//! Directory-layout registries — `monorepo` and `localdir` (PROP-002
//! `#registry-index`): the fixed places inside such a registry and the
//! listing that tells a reader whether the persisted index still describes
//! the tree.
//!
//! A layout registry keeps every package at `<group>/<name>/v<version>/`.
//! Beside the packages it carries its own index at [`INDEX_DIR`] — an
//! ordinary vibe-index data directory (`hello.json`, `repomd.json`,
//! `primary.jsonl`, `by-name/`, …) whose fact journal
//! (`state/journal/`) is committed with it, so every publisher shares the
//! yank and tombstone facts. The index is a projection of the tree and the
//! journal; directories may also be filled by hand, so a reader trusts the
//! persisted files only while [`index_state`] says they were computed from
//! exactly this tree and this journal, and otherwise computes the same
//! projection in memory.
//!
//! `_index` cannot collide with a package: `_` is not legal in a group.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-index");

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::Group;

/// The registry-relative directory holding the persisted index.
pub const INDEX_DIR: &str = "_index";

/// The listing file inside [`INDEX_DIR`]: what the index was computed from.
pub const LISTING_FILE: &str = "listing.txt";

/// The first line of [`LISTING_FILE`]; a different first line is a listing
/// this build does not read (treated as stale, never as current).
pub const LISTING_HEADER: &str = "# vibe layout-registry listing 1";

/// The persisted index directory of the registry rooted at `root`.
pub fn index_dir(root: &Path) -> PathBuf {
    root.join(INDEX_DIR)
}

/// The committed fact journal of the registry rooted at `root` — the
/// vibe-index journal location inside [`INDEX_DIR`].
pub fn journal_dir(root: &Path) -> PathBuf {
    index_dir(root).join("state").join("journal")
}

/// One version directory of a layout registry.
///
/// ```
/// use vibe_core::layout_registry::LayoutVersion;
///
/// let version = LayoutVersion {
///     group: vibe_core::Group::parse("org.corp").unwrap(),
///     name: "demo".into(),
///     version: semver::Version::new(1, 0, 0),
/// };
/// assert_eq!(version.path(), "org.corp/demo/v1.0.0");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LayoutVersion {
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
}

impl LayoutVersion {
    /// `<group>/<name>/v<version>` — the registry-relative path.
    pub fn path(&self) -> String {
        format!("{}/{}/v{}", self.group, self.name, self.version)
    }
}

/// Every `<group>/<name>/v<version>/` directory under `root`, sorted by
/// path. Top-level entries that are not a [`Group`] (`_index`, `.git`, a
/// README) and `v…` entries that are not a semver are not versions and are
/// left out, as are hidden siblings a `localdir` publish leaves while it
/// runs (`.v1.0.0.incoming-…`).
pub fn version_dirs(root: &Path) -> io::Result<Vec<LayoutVersion>> {
    let mut out = Vec::new();
    for group_dir in sorted_dirs(root)? {
        let Ok(group) = Group::parse(&file_name(&group_dir)) else {
            continue;
        };
        for name_dir in sorted_dirs(&group_dir)? {
            let name = file_name(&name_dir);
            if name.starts_with('.') {
                continue;
            }
            for version_dir in sorted_dirs(&name_dir)? {
                let Some(version) = file_name(&version_dir)
                    .strip_prefix('v')
                    .and_then(|raw| semver::Version::parse(raw).ok())
                else {
                    continue;
                };
                out.push(LayoutVersion {
                    group: group.clone(),
                    name: name.clone(),
                    version,
                });
            }
        }
    }
    out.sort_by_key(LayoutVersion::path);
    Ok(out)
}

/// `sha256:<hex>` over the journal shards (name and bytes, in name order);
/// the digest of an absent journal is the digest of nothing.
pub fn journal_digest(root: &Path) -> io::Result<String> {
    let dir = journal_dir(root);
    let mut shards: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "ndjson"))
            .collect(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error),
    };
    shards.sort();
    let mut hasher = Sha256::new();
    for shard in shards {
        hasher.update(file_name(&shard).as_bytes());
        hasher.update([0u8]);
        hasher.update(fs::read(&shard)?);
        hasher.update([0u8]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// The listing text for `versions` and `journal`: the header, the journal
/// digest, then one version path per line.
pub fn render_listing(versions: &[LayoutVersion], journal: &str) -> String {
    let mut text = format!("{LISTING_HEADER}\n# journal {journal}\n");
    for version in versions {
        text.push_str(&version.path());
        text.push('\n');
    }
    text
}

/// The listing of the registry as it is on disk now.
pub fn current_listing(root: &Path) -> io::Result<String> {
    Ok(render_listing(&version_dirs(root)?, &journal_digest(root)?))
}

/// Whether the persisted index of a layout registry describes its tree.
///
/// ```
/// use vibe_core::layout_registry::IndexState;
///
/// let stale = IndexState::Stale { reason: "a version was added by hand".into() };
/// assert_eq!(stale.as_str(), "stale");
/// assert_eq!(IndexState::Current.as_str(), "current");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexState {
    /// The listing matches the tree and the journal: read the files.
    Current,
    /// There is an index, but it was computed from another tree or journal
    /// (directories filled by hand, a publish by an older vibe).
    Stale { reason: String },
    /// No persisted index (a hand-made registry, or one never published to
    /// by a vibe that writes the index).
    Absent,
}

impl IndexState {
    pub const fn as_str(&self) -> &'static str {
        match self {
            IndexState::Current => "current",
            IndexState::Stale { .. } => "stale",
            IndexState::Absent => "absent",
        }
    }
}

/// Compare the persisted listing against the tree.
///
/// ```
/// use vibe_core::layout_registry::{IndexState, index_state};
///
/// let root = tempfile::tempdir().unwrap();
/// assert_eq!(index_state(root.path()).unwrap(), IndexState::Absent);
/// ```
pub fn index_state(root: &Path) -> io::Result<IndexState> {
    let listing_path = index_dir(root).join(LISTING_FILE);
    let recorded = match fs::read_to_string(&listing_path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(if index_dir(root).join("repomd.json").is_file() {
                IndexState::Stale {
                    reason: format!("`{INDEX_DIR}/{LISTING_FILE}` is missing"),
                }
            } else {
                IndexState::Absent
            });
        }
        Err(error) => return Err(error),
    };
    if !index_dir(root).join("repomd.json").is_file() {
        return Ok(IndexState::Stale {
            reason: format!("`{INDEX_DIR}/repomd.json` is missing"),
        });
    }
    if recorded.lines().next() != Some(LISTING_HEADER) {
        return Ok(IndexState::Stale {
            reason: "the listing has an unknown header".into(),
        });
    }
    let current = current_listing(root)?;
    if recorded == current {
        return Ok(IndexState::Current);
    }
    let differs = |prefix: &str| {
        let line = |text: &str| {
            text.lines()
                .find(|l| l.starts_with(prefix))
                .map(str::to_owned)
        };
        line(&recorded) != line(&current)
    };
    let reason = if differs("# journal ") {
        "the journal changed since the index was written".to_string()
    } else {
        "the version directories differ from the ones the index was computed from".to_string()
    };
    Ok(IndexState::Stale { reason })
}

fn sorted_dirs(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .collect();
    dirs.sort();
    Ok(dirs)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(root: &Path, path: &str) {
        fs::create_dir_all(root.join(path)).unwrap();
    }

    #[test]
    fn version_dirs_skip_the_index_hidden_and_non_version_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        version(root, "org.corp/demo/v1.0.0");
        version(root, "org.corp/demo/v1.10.0");
        version(root, "org.corp/demo/.v2.0.0.incoming-1");
        version(root, "org.corp/demo/vnext");
        version(root, "_index/by-name");
        version(root, ".git/objects");
        version(root, "a.b/z/v0.1.0");
        let paths: Vec<String> = version_dirs(root)
            .unwrap()
            .iter()
            .map(LayoutVersion::path)
            .collect();
        assert_eq!(
            paths,
            [
                "a.b/z/v0.1.0",
                "org.corp/demo/v1.0.0",
                "org.corp/demo/v1.10.0"
            ]
        );
    }

    #[test]
    fn state_follows_the_tree_and_the_journal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        version(root, "org.corp/demo/v1.0.0");
        assert_eq!(index_state(root).unwrap(), IndexState::Absent);

        fs::create_dir_all(index_dir(root)).unwrap();
        fs::write(index_dir(root).join("repomd.json"), "{}").unwrap();
        assert!(matches!(
            index_state(root).unwrap(),
            IndexState::Stale { .. }
        ));
        fs::write(
            index_dir(root).join(LISTING_FILE),
            current_listing(root).unwrap(),
        )
        .unwrap();
        assert_eq!(index_state(root).unwrap(), IndexState::Current);

        // A version directory filled by hand.
        version(root, "org.corp/demo/v1.1.0");
        let IndexState::Stale { reason } = index_state(root).unwrap() else {
            panic!("a hand-added version must make the index stale");
        };
        assert!(reason.contains("version directories"), "{reason}");
        fs::write(
            index_dir(root).join(LISTING_FILE),
            current_listing(root).unwrap(),
        )
        .unwrap();

        // A journal fact appended without re-projecting.
        fs::create_dir_all(journal_dir(root)).unwrap();
        fs::write(journal_dir(root).join("2026-11.ndjson"), "{}\n").unwrap();
        let IndexState::Stale { reason } = index_state(root).unwrap() else {
            panic!("a changed journal must make the index stale");
        };
        assert!(reason.contains("journal"), "{reason}");
    }
    /// A mirror's template snapshot grows `_index/templates/`, and that
    /// **cannot** make the index stale: the listing is rendered over
    /// `version_dirs`, which skips `_index` altogether, and the journal
    /// digest is over `state/journal/`. P4's plan carried the opposite as
    /// a risk and this is its closure (PROP-075 `#P4-AS-BUILT`
    /// amendment 6) — so a reader cannot reintroduce it by teaching the
    /// listing to walk `_index/`.
    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#P4-AS-BUILT")]
    fn a_template_snapshot_under_the_index_leaves_the_index_current() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        version(root, "org.corp/demo/v1.0.0");
        fs::create_dir_all(index_dir(root)).unwrap();
        fs::write(index_dir(root).join("repomd.json"), "{}").unwrap();
        fs::write(
            index_dir(root).join(LISTING_FILE),
            current_listing(root).unwrap(),
        )
        .unwrap();
        assert_eq!(index_state(root).unwrap(), IndexState::Current);
        let before = current_listing(root).unwrap();

        // The whole snapshot layout of `#MIRROR-SNAPSHOT-LAYOUT`, with a
        // directory named like a group (`org.vibevm.node`) and one named
        // like a version (`v24.22.2`) under it, which is the shape that
        // would catch a listing that walked `_index/`.
        let snapshot = index_dir(root).join("templates/org.vibevm.node/node");
        fs::create_dir_all(snapshot.join("digests")).unwrap();
        fs::create_dir_all(snapshot.join("v24.22.2")).unwrap();
        fs::write(snapshot.join("snapshot.toml"), "schema = 1\n").unwrap();
        fs::write(snapshot.join("versions.json"), "{}\n").unwrap();
        fs::write(snapshot.join("digests/24.22.2"), "aa  f\n").unwrap();

        assert_eq!(current_listing(root).unwrap(), before);
        assert_eq!(index_state(root).unwrap(), IndexState::Current);
    }
}
