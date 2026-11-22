//! The shippable tree (PROP-024 §2.2): identity is the source,
//! never build artifacts. The content-hash algorithm exists in two recipes
//! (PROP-044 §4.7) and is duplicated verbatim-in-intent in `vibe-index`'s
//! `content_hash` port — the two MUST stay in lockstep (PROP-005 §3.2), and a
//! parity test (`vibe-index`'s `tests/content_hash_parity.rs`) gates that.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#shippable-tree");

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::git_source::GitSourceView;
use walkdir::WalkDir;

use crate::RegistryError;
use crate::hash_recipe::{LEGACY0_EXCLUDES, RecipeId, order_entries};

/// Prune build output from a [`WalkDir`] walk so the hash and slot cover only
/// the shippable tree — per-entry, so an excluded dir is skipped, not entered.
/// The list is recipe 0's frozen [`LEGACY0_EXCLUDES`]; recipe 1's data-driven
/// list is identical today and carries the same lockstep obligation.
fn is_shippable(entry: &walkdir::DirEntry, excludes: &[&str]) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|name| !excludes.contains(&name))
        .unwrap_or(true)
}

/// Every entry of `pkg_dir`'s shippable tree — the one enumeration the
/// content hash and the store copy share, so the two cannot disagree on
/// which files a package is.
///
/// When `pkg_dir` lies inside a Git worktree the walk is narrowed to what
/// Git would version there ([`GitSourceView`]): files Git ignores never
/// enter the identity, and two clones of one commit hash alike whatever was
/// built or run in either. Outside a worktree, or without a `git`
/// executable, the walk is the plain denylist walk. `excludes` applies on
/// top in both cases.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-024#SHIPPABLE-TREE-DEF")]
pub(crate) fn shippable_entries(
    pkg_dir: &Path,
    excludes: &[&str],
) -> Result<Vec<walkdir::DirEntry>, RegistryError> {
    let view = GitSourceView::of(pkg_dir)?;
    Ok(entries_within(pkg_dir, excludes, view.as_ref()))
}

/// The denylist walk of `dir`, narrowed to `view` when there is one.
fn entries_within(
    dir: &Path,
    excludes: &[&str],
    view: Option<&GitSourceView>,
) -> Vec<walkdir::DirEntry> {
    WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| is_shippable(e, excludes) && git_admits(view, dir, e))
        .filter_map(|e| e.ok())
        .collect()
}

fn git_admits(view: Option<&GitSourceView>, root: &Path, entry: &walkdir::DirEntry) -> bool {
    let Some(view) = view else {
        return true;
    };
    let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
    view.admits(rel, entry.file_type().is_dir())
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), RegistryError> {
    fs::create_dir_all(dst).map_err(|source| RegistryError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    for entry in shippable_entries(src, LEGACY0_EXCLUDES)? {
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(entry.path(), &target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        }
    }
    Ok(())
}

/// Compute the content hash of `pkg_dir`'s shippable tree under `recipe`.
///
/// This is the **identity** half of the `(group, name, version,
/// content_hash)` tuple (PROP-002 §2.1). The bytes fed to SHA-256 per file
/// are `normalised_path || 0x00 || file_bytes || 0x00`, identical across
/// recipes; the two recipes differ only in file ORDER (recipe 1 normalises
/// separators before ordering, recipe 0 orders the platform path first) and
/// in the wire label. [`compute_content_hash`] (recipe 0, this crate's
/// default) delegates here.
///
/// Reads every file of the shippable tree: when `pkg_dir` lies in a Git
/// worktree, only files Git would version (tracked, or untracked and not
/// ignored); a `git` failure there is an error, never a silent full walk.
///
/// ```no_run
/// use std::path::Path;
/// use vibe_registry::compute_content_hash;
///
/// let hash = compute_content_hash(Path::new("path/to/package")).unwrap();
/// assert!(hash.starts_with("sha256:"));
/// ```
pub fn compute_content_hash_with(
    recipe: RecipeId,
    pkg_dir: &Path,
) -> Result<String, RegistryError> {
    let view = GitSourceView::of(pkg_dir)?;
    hash_tree(recipe, pkg_dir, view.as_ref())
}

/// The content hash of a tree that is already a copy of a source's
/// shippable tree — a store entry, a cached checkout, a materialised slot —
/// checked against the hash its source was given.
///
/// A copy holds exactly the files its source's Git view admitted, so it is
/// walked with the denylist alone. Narrowing it again would consult whatever
/// worktree the copy happens to sit in, where the package's own `.gitignore`
/// can match a file the source tracked, and the copy would be rejected for a
/// file it rightly holds (PROP-024 `#SHIPPABLE-TREE-DEF`).
pub fn compute_copy_content_hash_with(
    recipe: RecipeId,
    copy_dir: &Path,
) -> Result<String, RegistryError> {
    hash_tree(recipe, copy_dir, None)
}

fn hash_tree(
    recipe: RecipeId,
    pkg_dir: &Path,
    view: Option<&GitSourceView>,
) -> Result<String, RegistryError> {
    let excludes: Vec<&str> = match recipe {
        RecipeId::Legacy0 => LEGACY0_EXCLUDES.to_vec(),
        RecipeId::Tree1 => crate::hash_recipe::recipe1_excludes(),
    };

    // Walk the shippable tree, pairing each file with its raw relative path
    // string. Recipe 0 is lossy on non-UTF-8 (frozen behaviour); recipe 1
    // makes non-UTF-8 a hard error — two distinct invalid names would
    // otherwise collide to one hash (PROP-044 §4.7).
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries_within(pkg_dir, &excludes, view)
        .into_iter()
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path().to_path_buf();
        let rel = path.strip_prefix(pkg_dir).unwrap_or(&path);
        let raw = match recipe {
            RecipeId::Legacy0 => rel.to_string_lossy().into_owned(),
            RecipeId::Tree1 => rel
                .to_str()
                .ok_or_else(|| RegistryError::Io {
                    path: path.clone(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "relative path is not valid UTF-8; recipe 1 (sha256-tree/1) requires \
                         UTF-8 paths so two distinct non-UTF-8 names cannot collide to one hash",
                    ),
                })?
                .to_owned(),
        };
        entries.push((raw, path));
    }

    // The file rides through the ordering beside its path, so nothing has to
    // be looked back up afterwards — see `order_entries`.
    let ordered = order_entries(recipe, entries);

    let mut hasher = Sha256::new();
    for (norm, path) in &ordered {
        hasher.update(norm.as_bytes());
        hasher.update([0]);
        let bytes = fs::read(path).map_err(|source| RegistryError::Io {
            path: path.clone(),
            source,
        })?;
        hasher.update(&bytes);
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let hex = digest.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
        s
    });
    Ok(format!("{}{hex}", recipe.label()))
}

/// Compute the content hash of `pkg_dir`'s shippable tree with this crate's
/// default recipe — recipe 0 (`sha256:`), the form every lockfile in
/// existence already carries. Thin wrapper over [`compute_content_hash_with`].
pub fn compute_content_hash(pkg_dir: &Path) -> Result<String, RegistryError> {
    compute_content_hash_with(RecipeId::Legacy0, pkg_dir)
}

/// Compute the current cross-platform tree recipe. Bridge authoring uses this
/// spelling for immutable upstream pins so Windows and Unix maintainers agree
/// on one digest before any package is published.
pub fn compute_portable_content_hash(pkg_dir: &Path) -> Result<String, RegistryError> {
    compute_content_hash_with(RecipeId::Tree1, pkg_dir)
}

#[cfg(test)]
mod shippable_tree_tests {
    use super::*;
    use tempfile::tempdir;
    use vibe_core::git_source::fixture::{FixtureGit, outside_any_worktree};

    /// `copy_dir_recursive` and `compute_content_hash` both skip build output,
    /// so neither the materialised slot nor the content hash carries `target/`
    /// & friends (PROP-024 §2.2). Inline because `copy_dir_recursive` is
    /// `pub(crate)` — a `tests/` target could not reach it.
    #[test]
    fn copy_and_hash_exclude_build_output() {
        let src = tempdir().unwrap();
        fs::write(src.path().join("vibe.toml"), b"x").unwrap();
        fs::create_dir_all(src.path().join("target/debug")).unwrap();
        fs::write(src.path().join("target/debug/x.bin"), b"ARTIFACT").unwrap();

        let dst = tempdir().unwrap();
        copy_dir_recursive(src.path(), dst.path()).unwrap();
        assert!(dst.path().join("vibe.toml").exists());
        assert!(
            !dst.path().join("target").exists(),
            "target/ must not be copied"
        );

        let clean = tempdir().unwrap();
        fs::write(clean.path().join("vibe.toml"), b"x").unwrap();
        assert_eq!(
            compute_content_hash(src.path()).unwrap(),
            compute_content_hash(clean.path()).unwrap(),
            "build output must not affect the content hash"
        );
    }

    const PACKAGE: &str = "registry/org.example/demo/v1.0.0";

    /// A committed repository holding one local-registry package whose
    /// `.gitignore` covers interpreter caches and boot lock files — the two
    /// kinds of debris observed counted into a recorded identity.
    fn committed_repo(git: &FixtureGit) -> tempfile::TempDir {
        let root = tempdir().unwrap();
        git.init(root.path());
        fs::write(
            root.path().join(".gitignore"),
            "__pycache__/\n**/.vibe-boot-*\n",
        )
        .unwrap();
        let pkg = root.path().join(PACKAGE);
        fs::create_dir_all(pkg.join("skills/a/scripts")).unwrap();
        fs::create_dir_all(pkg.join("boot")).unwrap();
        fs::write(pkg.join("vibe.toml"), b"x").unwrap();
        fs::write(pkg.join("skills/a/scripts/run.py"), b"print()").unwrap();
        fs::write(pkg.join("boot/rules.md"), b"rules").unwrap();
        git.commit_all(root.path(), "fixture");
        root
    }

    fn add_ignored_debris(pkg: &Path) {
        fs::create_dir_all(pkg.join("skills/a/scripts/__pycache__")).unwrap();
        fs::create_dir_all(pkg.join("boot")).unwrap();
        fs::write(
            pkg.join("skills/a/scripts/__pycache__/run.cpython-314.pyc"),
            b"BYTECODE",
        )
        .unwrap();
        fs::write(pkg.join("boot/.vibe-boot-artifacts.lock"), b"").unwrap();
    }

    fn copied_files(src: &Path) -> Vec<String> {
        let dst = tempdir().unwrap();
        copy_dir_recursive(src, dst.path()).unwrap();
        let mut files: Vec<String> = WalkDir::new(dst.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                e.path()
                    .strip_prefix(dst.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        files.sort();
        files
    }

    /// Files Git ignores never enter a local source's identity: neither the
    /// store copy nor the hash sees them, and adding or removing them leaves
    /// the hash unchanged (PROP-024 `#SHIPPABLE-TREE-DEF`).
    #[test]
    fn ignored_files_stay_out_of_copy_and_hash() {
        let git = FixtureGit::new();
        let repo = committed_repo(&git);
        let pkg = repo.path().join(PACKAGE);
        let clean = compute_content_hash(&pkg).unwrap();
        let clean_tree1 = compute_portable_content_hash(&pkg).unwrap();

        add_ignored_debris(&pkg);
        assert_eq!(compute_content_hash(&pkg).unwrap(), clean);
        assert_eq!(compute_portable_content_hash(&pkg).unwrap(), clean_tree1);
        assert_eq!(
            copied_files(&pkg),
            ["boot/rules.md", "skills/a/scripts/run.py", "vibe.toml"]
        );

        fs::remove_file(pkg.join("boot/.vibe-boot-artifacts.lock")).unwrap();
        fs::remove_dir_all(pkg.join("skills/a/scripts/__pycache__")).unwrap();
        assert_eq!(compute_content_hash(&pkg).unwrap(), clean);
    }

    /// An untracked file no ignore rule matches is source Git would version
    /// on the next commit, so it is part of the identity.
    #[test]
    fn untracked_unignored_file_is_part_of_the_source() {
        let git = FixtureGit::new();
        let repo = committed_repo(&git);
        let pkg = repo.path().join(PACKAGE);
        let before = compute_content_hash(&pkg).unwrap();
        fs::write(pkg.join("notes.md"), b"new").unwrap();
        assert_ne!(compute_content_hash(&pkg).unwrap(), before);
        assert!(copied_files(&pkg).contains(&"notes.md".to_string()));
    }

    /// Outside any Git worktree the walk is unchanged: only the denylist
    /// applies, so a `__pycache__` file is still copied and hashed.
    #[test]
    fn source_outside_git_keeps_the_plain_walk() {
        let src = tempdir().unwrap();
        if !outside_any_worktree(src.path()) {
            return;
        }
        fs::write(src.path().join("vibe.toml"), b"x").unwrap();
        let clean = compute_content_hash(src.path()).unwrap();
        add_ignored_debris(src.path());
        assert_ne!(compute_content_hash(src.path()).unwrap(), clean);
        assert_eq!(
            copied_files(src.path()),
            [
                "boot/.vibe-boot-artifacts.lock",
                "skills/a/scripts/__pycache__/run.cpython-314.pyc",
                "vibe.toml"
            ]
        );
    }

    /// Two clones of one commit record one identity, whatever debris the
    /// running of a package's tests left in either.
    #[test]
    fn two_clones_of_one_commit_hash_alike() {
        let git = FixtureGit::new();
        let origin = committed_repo(&git);
        let clones = tempdir().unwrap();
        for name in ["one", "two"] {
            git.clone(origin.path(), &clones.path().join(name));
        }
        let one = clones.path().join("one").join(PACKAGE);
        let two = clones.path().join("two").join(PACKAGE);
        add_ignored_debris(&two);
        assert_eq!(
            compute_content_hash(&one).unwrap(),
            compute_content_hash(&two).unwrap()
        );
        assert_eq!(copied_files(&one), copied_files(&two));
    }

    /// A copy is hashed as the copy it is. A package may ship its own
    /// `.gitignore` matching a file its author force-added; once copied into
    /// an untracked spot of some other worktree, that file is untracked and
    /// ignored there. The copy still holds exactly its source's files, and
    /// the copy hash agrees with the source hash.
    #[test]
    fn a_copy_hashes_like_its_source_wherever_it_lies() {
        let git = FixtureGit::new();
        let repo = committed_repo(&git);
        let pkg = repo.path().join(PACKAGE);
        fs::write(pkg.join(".gitignore"), b"*.generated\n").unwrap();
        fs::write(pkg.join("table.generated"), b"authored on purpose").unwrap();
        git.run(repo.path(), &["add", "-f", "."]);
        git.run(repo.path(), &["commit", "-q", "-m", "force-added table"]);
        let source_hash = compute_content_hash(&pkg).unwrap();
        let source_tree1 = compute_portable_content_hash(&pkg).unwrap();

        let host = tempdir().unwrap();
        git.init(host.path());
        let copy = host.path().join("cache/demo");
        copy_dir_recursive(&pkg, &copy).unwrap();
        assert!(copy.join("table.generated").is_file());
        // Narrowed by the host worktree, the copy would lose the file.
        assert_ne!(compute_content_hash(&copy).unwrap(), source_hash);
        assert_eq!(
            compute_copy_content_hash_with(RecipeId::Legacy0, &copy).unwrap(),
            source_hash
        );
        assert_eq!(
            compute_copy_content_hash_with(RecipeId::Tree1, &copy).unwrap(),
            source_tree1
        );
    }
}
