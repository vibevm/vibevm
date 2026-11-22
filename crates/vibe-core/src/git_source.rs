//! Git's view of a package source directory (PROP-024 §2.2).
//!
//! A package's identity is its source. When the source directory lies inside
//! a Git worktree, the source is exactly what Git would version there: the
//! tracked files plus the untracked files no ignore rule matches. Files Git
//! ignores — interpreter caches, engine lock files, editor droppings — never
//! enter the hash, the store copy or the slot record, so two clones of one
//! commit record one identity whatever was built or run in either.
//!
//! The view is a filter, never an enumerator of its own: every walker keeps
//! its existing traversal and build-output denylist, and only drops what the
//! view does not admit. Outside a Git worktree, or when no `git` executable
//! is installed, there is no view and the walkers behave as before.
//!
//! Git prints a name as its index records it; a walker sees the spelling on
//! disk. Where the repository says the two may differ — `core.ignorecase` on
//! a case-insensitive filesystem, `core.precomposeunicode` on macOS — the
//! view compares names in a folded form, so a case-only rename or a
//! decomposed name on disk never drops a file from the source.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#shippable-tree");

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use icu_normalizer::{ComposingNormalizer, ComposingNormalizerBorrowed};
use specmark::spec;
use thiserror::Error;
use unicode_casefold::{Locale, UnicodeCaseFold, Variant};

/// A hermetic `git` for test fixtures, shared by every crate whose tests
/// build a repository around a package source.
#[cfg(any(test, feature = "git-fixture"))]
pub mod fixture;

/// Environment that would point `git` at a repository other than the one
/// enclosing the directory — a hook's `GIT_DIR`, a temporary index. The view
/// is always the one discovered from the directory itself.
const REPOSITORY_OVERRIDES: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CEILING_DIRECTORIES",
    "GIT_PREFIX",
];

/// `git` ran inside a worktree and could not say what it versions. The walk
/// is never substituted silently: a hash taken without the view would count
/// ignored files and differ from every other checkout of the same commit.
///
/// Every variant names the directory the view was asked for, and its
/// message cites the shippable-tree decision it protects:
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::git_source::GitSourceError;
///
/// let err = GitSourceError::UnexpectedEntry {
///     path: PathBuf::from("pkg"),
///     entry: "../outside".to_string(),
/// };
/// assert!(err.to_string().contains("`../outside`"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-024#SHIPPABLE-TREE-DEF")]
#[derive(Debug, Error)]
pub enum GitSourceError {
    #[error(
        "cannot run `git` to list the versioned files of `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-024#SHIPPABLE-TREE-DEF; \
          fix: make the `git` executable runnable, or move the package source out of the Git worktree)"
    )]
    Spawn {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error(
        "`git {command}` failed in `{path}` ({status}): {stderr} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-024#SHIPPABLE-TREE-DEF; \
          fix: repair the repository so that `git -C <dir> {command}` succeeds, \
          e.g. add it to `safe.directory`)"
    )]
    Failed {
        path: PathBuf,
        command: &'static str,
        status: String,
        stderr: String,
    },

    #[error(
        "`git ls-files` in `{path}` reported `{entry}`, which is not a path below that directory \
         (violates spec://org.vibevm.core/vibevm/common/PROP-024#SHIPPABLE-TREE-DEF; \
          fix: report the git version in use)"
    )]
    UnexpectedEntry { path: PathBuf, entry: String },
}

/// The files Git would version below one directory, relative to it.
///
/// [`GitSourceView::of`] yields `None` when Git has no say over the
/// directory; callers then keep their plain walk. A walker consults
/// [`GitSourceView::admits_file`] and [`GitSourceView::admits_dir`] for each
/// relative entry, so a directory holding nothing Git versions is pruned
/// without being entered.
///
/// ```
/// use std::path::Path;
/// use vibe_core::git_source::GitSourceView;
///
/// let view = GitSourceView::from_relative_files(["vibe.toml", "skills/a/run.py"]);
/// assert!(view.admits_file(Path::new("skills/a/run.py")));
/// assert!(view.admits_dir(Path::new("skills/a")));
/// assert!(!view.admits_dir(Path::new("skills/a/__pycache__")));
/// assert!(!view.admits_file(Path::new("skills/a/__pycache__/run.cpython-314.pyc")));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSourceView {
    fold: NameFold,
    files: BTreeSet<PathBuf>,
    dirs: BTreeSet<PathBuf>,
}

/// How names are compared: exactly, or in the folded form the repository's
/// own settings say Git and the filesystem treat as one name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct NameFold {
    /// `core.ignorecase`: full, non-Turkic Unicode case folding.
    case: bool,
    /// `core.precomposeunicode` (macOS): Git prints NFC while the
    /// filesystem may hand back NFD, so both sides compose to NFC.
    compose: bool,
}

impl NameFold {
    /// The comparison key of a relative path; the path itself when no fold
    /// is in force, so names stay byte-exact on a case-sensitive host.
    fn key(self, path: &Path) -> PathBuf {
        if !self.case && !self.compose {
            return path.to_path_buf();
        }
        path.components()
            .map(|component| match component {
                Component::Normal(name) => self.fold_name(name),
                other => other.as_os_str().to_os_string(),
            })
            .collect()
    }

    fn fold_name(self, name: &OsStr) -> OsString {
        const NFC: ComposingNormalizerBorrowed<'static> = ComposingNormalizer::new_nfc();
        let text = name.to_string_lossy();
        let composed = NFC.normalize(&text);
        if !self.case {
            return OsString::from(composed.into_owned());
        }
        let folded: String = composed
            .case_fold_with(Variant::Full, Locale::NonTurkic)
            .collect();
        OsString::from(NFC.normalize(&folded).into_owned())
    }
}

impl GitSourceView {
    /// Ask Git what it would version below `dir`.
    ///
    /// `Ok(None)` — keep the plain walk — when no ancestor of `dir` holds a
    /// `.git` entry, when `dir` cannot be resolved, when `git` is not
    /// installed, or when Git ignores `dir` as a whole (a store or cache
    /// under an ignored directory is not a versioned source). Otherwise the
    /// set is exactly `git ls-files --cached --others --exclude-standard`
    /// for `dir`, minus entries that are not regular files on disk (tracked
    /// files deleted from the worktree, symlinks, submodules), compared in
    /// the folded form the repository's `core.ignorecase` and
    /// `core.precomposeunicode` call for.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-024#SHIPPABLE-TREE-DEF")]
    pub fn of(dir: &Path) -> Result<Option<Self>, GitSourceError> {
        let Some(dir) = resolve(dir) else {
            return Ok(None);
        };
        if !inside_git_worktree(&dir) {
            return Ok(None);
        }
        let Some(ignored) = run_git(&dir, &["check-ignore", "-q", "--", "."])? else {
            return Ok(None);
        };
        match ignored.status.code() {
            Some(0) => return Ok(None),
            Some(1) => {}
            _ => return Err(failure(&dir, "check-ignore", &ignored)),
        }
        let Some(listed) = run_git(
            &dir,
            &[
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
                "--",
                ".",
            ],
        )?
        else {
            return Ok(None);
        };
        if !listed.status.success() {
            return Err(failure(&dir, "ls-files", &listed));
        }
        let Some(fold) = name_fold(&dir)? else {
            return Ok(None);
        };
        let mut files = BTreeSet::new();
        for raw in listed.stdout.split(|byte| *byte == 0) {
            if raw.is_empty() {
                continue;
            }
            let relative = relative_path(raw);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err(GitSourceError::UnexpectedEntry {
                    path: dir.clone(),
                    entry: String::from_utf8_lossy(raw).into_owned(),
                });
            }
            let on_disk = std::fs::symlink_metadata(dir.join(&relative));
            if on_disk.is_ok_and(|meta| meta.file_type().is_file()) {
                files.insert(relative);
            }
        }
        Ok(Some(Self::from_files(fold, files)))
    }

    /// A view over an explicit set of `/`-separated relative file paths,
    /// compared exactly.
    pub fn from_relative_files<I, S>(files: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::from_files(
            NameFold::default(),
            files
                .into_iter()
                .map(|file| PathBuf::from(file.as_ref()))
                .collect(),
        )
    }

    fn from_files(fold: NameFold, files: BTreeSet<PathBuf>) -> Self {
        let files: BTreeSet<PathBuf> = files.iter().map(|file| fold.key(file)).collect();
        let dirs = files
            .iter()
            .flat_map(|file| file.ancestors().skip(1))
            .filter(|dir| !dir.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .collect();
        Self { fold, files, dirs }
    }

    /// Whether Git versions the file at `relative`.
    pub fn admits_file(&self, relative: &Path) -> bool {
        self.files.contains(&self.fold.key(relative))
    }

    /// Whether any file Git versions lies below the directory `relative`.
    /// The root (an empty path) is always admitted.
    pub fn admits_dir(&self, relative: &Path) -> bool {
        relative.as_os_str().is_empty() || self.dirs.contains(&self.fold.key(relative))
    }

    /// Whether a walked entry survives the view: [`Self::admits_dir`] for a
    /// directory, [`Self::admits_file`] for anything else.
    pub fn admits(&self, relative: &Path, is_dir: bool) -> bool {
        if is_dir {
            self.admits_dir(relative)
        } else {
            self.admits_file(relative)
        }
    }
}

/// A repository marker in `dir` or one of its ancestors — decided without
/// spawning anything, so a tree outside any repository never depends on Git
/// at all. A marker is what Git's own discovery accepts: a `.git` directory
/// holding `HEAD`, `objects/` and `refs/`, or a `.git` file naming its
/// `gitdir:` (a linked worktree or submodule). A bare `.git` placeholder is
/// skipped, as Git skips it. A directory inside a `.git` directory is
/// repository metadata, never a worktree.
fn inside_git_worktree(dir: &Path) -> bool {
    if dir
        .ancestors()
        .any(|ancestor| ancestor.file_name() == Some(OsStr::new(".git")))
    {
        return false;
    }
    dir.ancestors()
        .any(|ancestor| is_repository_marker(&ancestor.join(".git")))
}

fn is_repository_marker(marker: &Path) -> bool {
    match std::fs::symlink_metadata(marker) {
        Ok(meta) if meta.is_dir() => {
            marker.join("HEAD").is_file()
                && marker.join("objects").is_dir()
                && marker.join("refs").is_dir()
        }
        Ok(meta) if meta.is_file() => {
            std::fs::read(marker).is_ok_and(|bytes| bytes.starts_with(b"gitdir:"))
        }
        _ => false,
    }
}

/// The name fold the repository enclosing `dir` calls for. An unset
/// `core.precomposeunicode` composes on macOS, as Git itself does there.
fn name_fold(dir: &Path) -> Result<Option<NameFold>, GitSourceError> {
    let Some(output) = run_git(
        dir,
        &[
            "config",
            "--bool",
            "--get-regexp",
            r"^core\.(ignorecase|precomposeunicode)$",
        ],
    )?
    else {
        return Ok(None);
    };
    let mut fold = NameFold {
        case: false,
        compose: cfg!(target_os = "macos"),
    };
    match output.status.code() {
        Some(0) => {}
        Some(1) => return Ok(Some(fold)),
        _ => return Err(failure(dir, "config", &output)),
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((key, value)) = line.trim().split_once(' ') else {
            continue;
        };
        let enabled = value.trim() == "true";
        match key.to_ascii_lowercase().as_str() {
            "core.ignorecase" => fold.case = enabled,
            "core.precomposeunicode" => fold.compose = enabled,
            _ => {}
        }
    }
    Ok(Some(fold))
}

/// The physical absolute form of `dir`, in a spelling `git -C` accepts
/// (no Windows verbatim prefix); `None` when it does not resolve.
fn resolve(dir: &Path) -> Option<PathBuf> {
    let resolved = dir.canonicalize().ok()?;
    #[cfg(windows)]
    if let Some(plain) = resolved
        .to_str()
        .and_then(|text| text.strip_prefix(r"\\?\"))
        .filter(|text| !text.starts_with("UNC\\"))
    {
        return Some(PathBuf::from(plain));
    }
    Some(resolved)
}

/// Run `git -C dir <args>`; `Ok(None)` when no `git` executable exists.
fn run_git(dir: &Path, args: &[&str]) -> Result<Option<std::process::Output>, GitSourceError> {
    let mut git = Command::new("git");
    git.arg("-C").arg(dir).args(args).stdin(Stdio::null());
    for name in REPOSITORY_OVERRIDES {
        git.env_remove(name);
    }
    // The marker scan above crosses mount points; discovery must agree.
    git.env("GIT_DISCOVERY_ACROSS_FILESYSTEM", "1");
    match git.output() {
        Ok(output) => Ok(Some(output)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(GitSourceError::Spawn {
            path: dir.to_path_buf(),
            source,
        }),
    }
}

fn failure(dir: &Path, command: &'static str, output: &std::process::Output) -> GitSourceError {
    GitSourceError::Failed {
        path: dir.to_path_buf(),
        command,
        status: output.status.to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    }
}

#[cfg(unix)]
fn relative_path(raw: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(raw))
}

#[cfg(not(unix))]
fn relative_path(raw: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(raw).into_owned())
}

#[cfg(test)]
mod tests {
    use super::fixture::{FixtureGit, outside_any_worktree};
    use super::*;
    use std::fs;

    fn repo_with_package(git: &FixtureGit) -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        git.init(root.path());
        fs::write(
            root.path().join(".gitignore"),
            "__pycache__/\n**/.vibe-boot-*\n",
        )
        .unwrap();
        let pkg = root.path().join("registry/org.example/demo/v1.0.0");
        fs::create_dir_all(pkg.join("boot")).unwrap();
        fs::write(pkg.join("vibe.toml"), "x").unwrap();
        fs::write(pkg.join("boot/rules.md"), "rules").unwrap();
        git.commit_all(root.path(), "fixture");
        (root, pkg)
    }

    fn listed(view: &GitSourceView) -> Vec<&Path> {
        view.files.iter().map(PathBuf::as_path).collect()
    }

    #[test]
    fn outside_a_worktree_there_is_no_view() {
        let dir = tempfile::tempdir().unwrap();
        if !outside_any_worktree(dir.path()) {
            return;
        }
        fs::write(dir.path().join("a"), "a").unwrap();
        assert_eq!(GitSourceView::of(dir.path()).unwrap(), None);
    }

    #[test]
    fn ignored_files_are_outside_and_untracked_files_inside() {
        let git = FixtureGit::new();
        let (_root, pkg) = repo_with_package(&git);
        fs::create_dir_all(pkg.join("__pycache__")).unwrap();
        fs::write(pkg.join("__pycache__/x.pyc"), "debris").unwrap();
        fs::write(pkg.join("boot/.vibe-boot-artifacts.lock"), "").unwrap();
        fs::write(pkg.join("notes.md"), "new").unwrap();
        let view = GitSourceView::of(&pkg).unwrap().unwrap();
        assert_eq!(
            listed(&view),
            [
                Path::new("boot/rules.md"),
                Path::new("notes.md"),
                Path::new("vibe.toml")
            ]
        );
        assert!(!view.admits_dir(Path::new("__pycache__")));
    }

    #[test]
    fn a_placeholder_git_directory_is_not_a_repository() {
        let dir = tempfile::tempdir().unwrap();
        if !outside_any_worktree(dir.path()) {
            return;
        }
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join("a"), "a").unwrap();
        assert_eq!(GitSourceView::of(dir.path()).unwrap(), None);
    }

    #[test]
    fn tracked_files_deleted_from_disk_are_absent() {
        let git = FixtureGit::new();
        let (_root, pkg) = repo_with_package(&git);
        fs::remove_file(pkg.join("boot/rules.md")).unwrap();
        let view = GitSourceView::of(&pkg).unwrap().unwrap();
        assert_eq!(listed(&view), [Path::new("vibe.toml")]);
    }

    #[test]
    fn a_directory_git_ignores_whole_has_no_view() {
        let git = FixtureGit::new();
        let (root, _pkg) = repo_with_package(&git);
        let cache = root.path().join("__pycache__/entry");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("vibe.toml"), "x").unwrap();
        assert_eq!(GitSourceView::of(&cache).unwrap(), None);
    }

    /// With `core.ignorecase` set, Git lists the index spelling while the
    /// walker meets the disk spelling: a case-only rename of a file or of a
    /// directory still admits the file and its whole subtree.
    #[test]
    fn ignorecase_admits_a_case_only_rename() {
        let git = FixtureGit::new();
        let (root, pkg) = repo_with_package(&git);
        git.run(root.path(), &["config", "core.ignorecase", "true"]);
        fs::create_dir_all(pkg.join("Docs")).unwrap();
        fs::write(pkg.join("Docs/Guide.md"), "guide").unwrap();
        fs::write(pkg.join("Readme.md"), "readme").unwrap();
        git.commit_all(root.path(), "docs");
        let view = GitSourceView::of(&pkg).unwrap().unwrap();
        assert!(view.admits_file(Path::new("README.md")));
        assert!(view.admits_dir(Path::new("docs")));
        assert!(view.admits_file(Path::new("docs/GUIDE.md")));
        assert!(!view.admits_file(Path::new("notes.md")));
    }

    /// Without `core.ignorecase` names compare exactly.
    #[test]
    fn a_case_sensitive_repository_compares_names_exactly() {
        let git = FixtureGit::new();
        let (root, pkg) = repo_with_package(&git);
        git.run(root.path(), &["config", "core.ignorecase", "false"]);
        git.run(root.path(), &["config", "core.precomposeunicode", "false"]);
        let view = GitSourceView::of(&pkg).unwrap().unwrap();
        assert!(view.admits_file(Path::new("vibe.toml")));
        assert!(!view.admits_file(Path::new("VIBE.toml")));
    }

    /// Git on macOS prints NFC names; the filesystem may hand the walker
    /// the decomposed spelling. Composing both sides keeps the file.
    #[test]
    fn a_composing_fold_admits_a_decomposed_name() {
        let composed = "caf\u{e9}.md";
        let decomposed = "cafe\u{301}.md";
        let fold = NameFold {
            case: false,
            compose: true,
        };
        let view = GitSourceView::from_files(fold, BTreeSet::from([PathBuf::from(composed)]));
        assert!(view.admits_file(Path::new(decomposed)));
        assert!(!view.admits_file(Path::new("CAF\u{c9}.md")));
        let exact = GitSourceView::from_relative_files([composed]);
        assert!(!exact.admits_file(Path::new(decomposed)));
    }
}
