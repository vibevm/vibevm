//! Fold-keyed physical identity and overlap for paths and receipt rows.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use anyhow::{Context, Result, bail};
use unicase::UniCase;

/// The physical-identity key of one path or file row: a conservative full
/// Unicode case fold, so Windows case aliases (and their Unicode spellings)
/// collapse to one identity instead of naming one file twice.
pub(crate) type FoldKey = UniCase<String>;

pub(crate) fn fold_key(value: impl Into<String>) -> FoldKey {
    UniCase::new(value.into())
}

/// Lexical containment proof used before capability-relative walks: `path`
/// must sit below `root` through normal components only.
pub(crate) fn ensure_lexically_contained(root: &Path, path: &Path) -> Result<()> {
    if !root.is_absolute() || !path.is_absolute() {
        bail!(
            "containment requires absolute paths (root `{}`, path `{}`)",
            root.display(),
            path.display()
        );
    }
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "path `{}` escapes trusted root `{}`",
            path.display(),
            root.display()
        )
    })?;
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!(
            "path `{}` contains a non-normal component below `{}`",
            path.display(),
            root.display()
        );
    }
    Ok(())
}

/// Whether two paths overlap, comparing components under the fold key so
/// `.CLAUDE/skills/demo` cannot dodge `.claude/skills/demo`.
pub(crate) fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = components_folded(left);
    let right = components_folded(right);
    prefix_of(&left, &right) || prefix_of(&right, &left)
}

/// Ordinary no-follow walk used by the standalone `vibe skill` surface,
/// whose writes stay whole-directory user-invoked replacements. The
/// automatic package-phase path never uses this: its mutations go through
/// retained capabilities instead.
pub(crate) fn ensure_no_follow_walk(root: &Path, path: &Path, allow_missing: bool) -> Result<()> {
    ensure_lexically_contained(root, path)?;
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "path `{}` escapes trusted root `{}`",
            path.display(),
            root.display()
        )
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || is_reparse(&metadata) {
                    bail!(
                        "path `{}` traverses symlink/junction/reparse component `{}`",
                        path.display(),
                        current.display()
                    );
                }
                if current != path && !metadata.is_dir() {
                    bail!(
                        "path `{}` traverses non-directory ancestor `{}`",
                        path.display(),
                        current.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && allow_missing => break,
            Err(error) => {
                return Err(anyhow::Error::new(error)
                    .context(format!("inspecting `{}`", current.display())));
            }
        }
    }
    Ok(())
}

fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

fn components_folded(path: &Path) -> Vec<FoldKey> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(fold_key(value.to_string_lossy().into_owned())),
            _ => None,
        })
        .collect()
}

fn prefix_of(prefix: &[FoldKey], whole: &[FoldKey]) -> bool {
    prefix.len() <= whole.len() && prefix.iter().zip(whole).all(|(left, right)| left == right)
}

/// A fold-keyed set of relative file rows for duplicate detection.
pub(crate) struct FoldSet {
    seen: BTreeSet<FoldKey>,
}

impl FoldSet {
    pub(crate) fn new() -> Self {
        Self {
            seen: BTreeSet::new(),
        }
    }

    /// Insert one relative path; `false` when a folded alias already exists.
    pub(crate) fn insert(&mut self, relative: &str) -> bool {
        self.seen.insert(fold_key(relative))
    }
}

/// One forward-slash relative file row is portable and Windows-storeable:
/// no empty segments (`a//b`, trailing `/`), dot/dot-dot, device names, or
/// dot/space-ended components.
pub(crate) fn valid_relative_file(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && !value.contains(':')
        && !value.starts_with('/')
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && value.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && !is_device_name(segment)
                && !segment.ends_with('.')
                && !segment.ends_with(' ')
        })
}

/// Windows reserved device spellings, which cannot be stored as a directory
/// or file component on that filesystem. Delegates to the single shared
/// table in `vibe_core` (stem-before-extension, console/clock devices,
/// superscript ports).
pub(crate) fn is_device_name(component: &str) -> bool {
    vibe_core::manifest::SkillDecl::is_windows_device_name(component)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_owned_file_paths_are_portable_normal_relatives() {
        assert!(valid_relative_file("references/guide.md"));
        for path in [
            "",
            ".",
            "../x",
            "/x",
            "a\\b",
            "file:stream",
            "references/COM1",
            "references/trailing.",
            "references/trailing ",
            "a//b",
            "a/",
            "references//guide.md",
            "maße/",
            // Extension-bearing device aliases (shared core table).
            "references/CON.txt",
            "references/NUL.md",
            "references/COM1.json",
            "references/LPT9.log",
            "references/CONIN$",
            "references/CLOCK$",
            "references/COM²",
        ] {
            assert!(!valid_relative_file(path), "{path}");
        }
        // Non-ASCII spellings judge without panicking.
        assert!(valid_relative_file("references/Maße.md"));
    }

    #[test]
    fn folded_aliases_collapse_to_one_identity() {
        let mut set = FoldSet::new();
        assert!(set.insert("SKILL.md"));
        assert!(!set.insert("skill.md"), "case alias must dedup");
        assert!(!set.insert("Skill.MD"), "unicode fold alias must dedup");
        assert!(set.insert("OTHER.md"));
    }

    #[test]
    fn full_unicode_folds_share_one_physical_identity() {
        // Full (not ASCII-only) folding: `Maße` folds to the same physical
        // key as `MASSE` under Unicode CaseFolding.
        let mut set = FoldSet::new();
        assert!(set.insert("Maße.md"));
        assert!(
            !set.insert("MASSE.md"),
            "full Unicode fold must collide `Maße` with `MASSE`"
        );
    }

    #[test]
    fn overlap_is_fold_aware() {
        let left = Path::new(r"C:\proj\.claude\skills\demo");
        let right = Path::new(r"C:\proj\.CLAUDE\skills\demo");
        assert!(paths_overlap(left, right));
        let outside = Path::new(r"C:\proj\.claude\skills\other");
        assert!(!paths_overlap(left, outside));
    }
}
