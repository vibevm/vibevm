//! Canonical physical identity and overlap for paths and receipt rows.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use anyhow::{Context, Result, bail};
use icu_normalizer::ComposingNormalizer;
use unicode_casefold::{Locale, UnicodeCaseFold, Variant};

/// The physical-identity key of one path or file row: **one** conservative
/// cross-platform key, deliberately coarser than any single filesystem.
///
/// Exact spelling is what ownership compares (see `super::transaction`); this
/// key exists only to detect that two *different* spellings would land on one
/// physical file somewhere we ship to. It therefore merges every alias any of
/// our target platforms merges, and a few none of them do.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FoldKey(String);

/// Build the canonical identity of one relative path or component:
///
/// 1. NFC-normalize — so a decomposed `e` + `U+0301` and a composed `é` are
///    one identity, which is exactly what macOS/HFS+ (NFD) and everyone else
///    (NFC) disagree about;
/// 2. **full, non-Turkic Unicode case fold** — `CaseFolding.txt`'s `C` + `F`
///    mappings, the operation APFS case-insensitive matching is defined on,
///    so `ß`/`ẞ`/`SS` are one key and `Θ`/`ϴ`/`θ`, `Σ`/`ς`/`σ` likewise;
/// 3. NFC-normalize again — folding can leave the result unnormalized (a
///    full mapping may emit a base plus a combining mark).
///
/// Case folding, not uppercasing: uppercasing is not idempotent across the
/// title/uppercase-symbol families, so `ẞ`≠`ß` and `ϴ`≠`Θ` under it — two
/// spellings APFS treats as one file. The fold table is pinned to Unicode
/// **9.0.0** ([`unicode_casefold::UNICODE_VERSION`], asserted by a test), the
/// version APFS case-insensitive volumes use; a dependency bump must not
/// change filesystem identity silently.
///
/// **Safe over-refusal, documented deliberately.** Folding merges `Maße`
/// with `MASSE`, `ﬁle` with `FILE`, and NFC with NFD on *every* host, not
/// only where the filesystem does. Over-merging costs a refusal the operator
/// resolves by renaming; under-merging silently overwrites or orphans
/// someone's bytes. We take the refusal. No handwritten Unicode table is
/// involved: NFC is `icu_normalizer`'s compiled data, the fold is
/// `unicode-casefold`'s generated `CaseFolding.txt` table.
pub(crate) fn fold_key(value: &str) -> FoldKey {
    const NFC: icu_normalizer::ComposingNormalizerBorrowed<'static> =
        ComposingNormalizer::new_nfc();
    let composed = NFC.normalize(value);
    let folded = composed
        .as_ref()
        .case_fold_with(Variant::Full, Locale::NonTurkic)
        .collect::<String>();
    FoldKey(NFC.normalize(&folded).into_owned())
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
            // A non-UTF-8 component cannot be spelled by any receipt row or
            // selected source path (both refuse it earlier), so folding its
            // lossy rendering here can only ever over-merge — the safe side.
            Component::Normal(value) => Some(fold_key(&value.to_string_lossy())),
            _ => None,
        })
        .collect()
}

fn prefix_of(prefix: &[FoldKey], whole: &[FoldKey]) -> bool {
    prefix.len() <= whole.len() && prefix.iter().zip(whole).all(|(left, right)| left == right)
}

/// Why one complete selected file set is not portable.
#[derive(Debug)]
pub(crate) enum SelectionFault {
    /// One relative path is not storeable as written on every host.
    Unsafe(String),
    /// Two spellings share one canonical physical identity.
    Collision { first: String, alias: String },
}

impl std::fmt::Display for SelectionFault {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsafe(path) => write!(formatter, "unsafe file path `{path}`"),
            Self::Collision { first, alias } => write!(
                formatter,
                "file paths `{first}` and `{alias}` are one file on a case-insensitive host"
            ),
        }
    }
}

/// Judge one **complete** selected file set through the shared portability
/// laws — every relative path storeable under [`valid_path_component`], and
/// no two paths sharing a fold key. Callers run this before anything is
/// staged, published, or written, so a non-portable selection never reaches
/// a durable intent or a target directory.
pub(crate) fn judge_selection<'a>(
    paths: impl IntoIterator<Item = &'a str>,
) -> Result<(), SelectionFault> {
    let mut seen = BTreeMap::<FoldKey, &str>::new();
    for path in paths {
        if !valid_relative_file(path) {
            return Err(SelectionFault::Unsafe(path.to_string()));
        }
        if let Some(first) = seen.insert(fold_key(path), path) {
            return Err(SelectionFault::Collision {
                first: first.to_string(),
                alias: path.to_string(),
            });
        }
    }
    Ok(())
}

/// One forward-slash relative file row is portable: no root, prefix, or
/// non-normal component, and every segment legal under the one component
/// law below.
fn valid_relative_file(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && value.split('/').all(valid_path_component)
}

/// The **one** portable path-component law, shared by receipt rows, source
/// projection, and every capability-relative mutation: a component is legal
/// only when the exact same spelling is storeable on Windows, macOS, and
/// Linux alike, and is visible when printed. Legal Unicode is preserved
/// untouched.
pub(crate) fn valid_path_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && !component.ends_with('.')
        && !component.ends_with(' ')
        && !component.chars().any(is_reserved_character)
        && !is_device_name(component)
}

/// What may never appear inside a name: Windows' reserved punctuation —
/// which also covers both separators and the drive-prefix colon — and
/// **every** control character. `char::is_control` is the whole `Cc`
/// category: C0 (`U+0000..=U+001F`), DEL (`U+007F`), and C1
/// (`U+0080..=U+009F`). An invisible name is not a portability feature; it
/// is an unreviewable path, so the law refuses it everywhere.
fn is_reserved_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
        )
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

    /// Every character Windows itself refuses is refused here, wherever it
    /// sits in the component, while legal Unicode survives untouched.
    #[test]
    fn the_one_component_law_rejects_every_windows_invalid_character() {
        for character in ['<', '>', ':', '"', '\\', '|', '?', '*'] {
            for path in [
                format!("a{character}b.md"),
                format!("{character}lead.md"),
                format!("trail{character}"),
                format!("references/a{character}b.md"),
                format!("a{character}b/guide.md"),
            ] {
                assert!(!valid_relative_file(&path), "{path:?}");
                assert!(
                    !valid_path_component(&format!("a{character}b")),
                    "{character}"
                );
            }
        }
        // `/` is a separator, never a character inside a component.
        assert!(!valid_path_component("a/b"));
        // Legal Unicode survives the law untouched.
        for path in [
            "references/Maße.md",
            "références/guía.md",
            "スキル/説明.md",
            "com10.md",
            "confidence.md",
            "e\u{301}.md",
        ] {
            assert!(valid_relative_file(path), "{path}");
        }
    }

    /// **Every** control character is refused — the whole `Cc` category, not
    /// just C0: an invisible name is an unreviewable path, never a
    /// portability feature.
    #[test]
    fn the_one_component_law_rejects_every_control_character() {
        for code in (0x00u32..=0x1f).chain(0x7fu32..=0x9f) {
            let control = char::from_u32(code).unwrap();
            assert!(control.is_control(), "U+{code:04X}");
            assert!(
                !valid_relative_file(&format!("a{control}b.md")),
                "U+{code:04X}"
            );
            assert!(
                !valid_path_component(&format!("a{control}b")),
                "U+{code:04X}"
            );
        }
        // The nearest legal neighbours of the control blocks stay legal.
        for path in ["a\u{20}b.md", "a\u{a0}b.md", "a\u{7e}b.md"] {
            assert!(valid_relative_file(path), "{path:?}");
        }
    }

    #[test]
    fn folded_aliases_collapse_to_one_identity() {
        assert!(judge_selection(["SKILL.md", "OTHER.md"]).is_ok());
        for alias in ["skill.md", "Skill.MD"] {
            let fault = judge_selection(["SKILL.md", alias]).unwrap_err();
            assert!(
                matches!(&fault, SelectionFault::Collision { first, alias: second }
                    if first == "SKILL.md" && second == alias),
                "{fault}"
            );
        }
    }

    /// The fold table is the one APFS case-insensitive matching is defined
    /// on. Pinning it is a *filesystem identity* decision, not a dependency
    /// detail: a bump that changed the Unicode version would silently change
    /// which two names this build considers one file.
    #[test]
    fn the_case_fold_table_is_pinned_to_the_apfs_unicode_version() {
        assert_eq!(
            unicode_casefold::UNICODE_VERSION,
            (9, 0, 0),
            "APFS case-insensitive matching uses Unicode 9.0.0; a dependency update that \
             changes this table silently changes filesystem identity"
        );
    }

    /// Full, non-Turkic case folding — including every equivalence family a
    /// full *uppercase* mapping misses, because uppercasing is not idempotent
    /// across the capital-sharp-s and symbol-letter families.
    #[test]
    fn full_unicode_folds_share_one_physical_identity() {
        for (owned, alias) in [
            // Sharp s: uppercase gives `ß`→`SS` but `ẞ`→`ẞ`, so `ß`/`ẞ` did
            // not collide. Under the fold all three are `ss`.
            ("Maße.md", "MASSE.md"),
            ("Maße.md", "Masse.md"),
            ("Maße.md", "masse.md"),
            ("straße.md", "STRAẞE.md"),
            ("STRAẞE.md", "STRASSE.md"),
            ("ß.md", "ẞ.md"),
            ("ß.md", "SS.md"),
            ("ẞ.md", "ss.md"),
            // Theta: `ϴ` (GREEK CAPITAL THETA SYMBOL) is already uppercase,
            // so uppercasing never merged it with `Θ`. The fold does.
            ("θ.md", "Θ.md"),
            ("θ.md", "ϴ.md"),
            ("Θ.md", "ϴ.md"),
            // `ϑ` GREEK THETA SYMBOL folds onto `θ` too (CaseFolding `C`).
            ("θ.md", "ϑ.md"),
            ("μέθοδος.md", "ΜΈϴΟΔΟΣ.md"),
            // Sigma, medial vs. final vs. capital.
            ("ΟΔΟΣ.md", "οδος.md"),
            ("οδός.md", "ΟΔΌΣ.md"),
            ("ὀδυσσεύς.md", "ὈΔΥΣΣΕΎΣ.md"),
        ] {
            assert_ne!(owned, alias, "the spellings must differ");
            let fault = judge_selection([owned, alias]).unwrap_err();
            assert!(
                matches!(fault, SelectionFault::Collision { .. }),
                "`{owned}` must collide with `{alias}`"
            );
            assert_eq!(fold_key(owned), fold_key(alias));
        }
        // Unrelated legal Unicode stays distinct.
        for distinct in [
            ["Maße.md", "Größe.md"],
            ["ΟΔΟΣ.md", "ΟΔΟΙ.md"],
            ["θ.md", "δ.md"],
            ["ß.md", "s.md"],
            ["ẞ.md", "sss.md"],
        ] {
            assert!(judge_selection(distinct).is_ok(), "{distinct:?}");
        }
    }

    /// NFC and NFD spellings of the same text are one physical file on
    /// macOS; the canonical key merges them before anything is staged.
    #[test]
    fn canonical_composition_aliases_share_one_physical_identity() {
        for (composed, decomposed) in [
            ("é.md", "e\u{301}.md"),
            ("references/café.md", "references/cafe\u{301}.md"),
            ("Ångström.md", "A\u{30a}ngstro\u{308}m.md"),
            ("ﬀ/ü.md", "ﬀ/u\u{308}.md"),
        ] {
            assert_ne!(composed, decomposed, "the spellings must differ");
            let fault = judge_selection([composed, decomposed]).unwrap_err();
            assert!(
                matches!(fault, SelectionFault::Collision { .. }),
                "`{composed}` must collide with `{decomposed}`"
            );
            assert_eq!(fold_key(composed), fold_key(decomposed));
        }
        // Unrelated Unicode stays distinct.
        assert!(judge_selection(["é.md", "e.md"]).is_ok());
        assert!(judge_selection(["ü.md", "u.md", "ǔ.md"]).is_ok());
    }

    /// The selection judge refuses on the first non-portable path, whatever
    /// the fault, so no caller ever stages or publishes one.
    #[test]
    fn selection_judge_refuses_unsafe_paths_before_collisions() {
        for path in [
            "a<b.md",
            "a\u{1}b.md",
            "a\u{7f}b.md",
            "a\u{85}b.md",
            "references/CON.txt",
            "trail.",
            "..",
        ] {
            let fault = judge_selection(["SKILL.md", path]).unwrap_err();
            assert!(
                matches!(&fault, SelectionFault::Unsafe(value) if value == path),
                "{fault}"
            );
        }
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
