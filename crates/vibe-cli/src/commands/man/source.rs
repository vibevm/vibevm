//! Locating and resolving source trees (PROP-019 §2.7, §2.16): the in-tree
//! committer checkout (built in place, never touched) and managed mirror
//! clones, plus selector→commit resolution against a clone.

specmark::scope!("spec://vibevm/common/PROP-019#provenance");

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use dialoguer::Select;
use specmark::spec;
use thiserror::Error;

use super::builder::{ResolvedVersion, short_commit};
use super::git::GitError;
use super::model::{self, Kind, VersionId};
use super::store::{StoreError, VersionStore};
use crate::output;

/// The source-resolution layer's failure surface (PROP-019 §2.7, §2.16):
/// an unknown mirror token, a selector that resolves to no ref, or a clone
/// that cannot be prepared. Git and store failures pass through transparently
/// (their own Class-F messages already cite the requirement).
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/common/PROP-019#provenance")]
pub(crate) enum ResolveError {
    #[error(
        "unknown source mirror `{token}` \
         (violates spec://vibevm/common/PROP-019#provenance; \
          fix: pass `gitverse` or `github`)"
    )]
    UnknownMirror { token: String },

    #[error(
        "{what} not found in the clone \
         (violates spec://vibevm/common/PROP-019#selectors; \
          fix: pass a selector that exists upstream — check `git ls-remote`)"
    )]
    RefNotFound { what: String },

    #[error(
        "no semantic-version tags in the clone \
         (violates spec://vibevm/common/PROP-019#selectors; \
          fix: install `latest` or an explicit branch/commit instead of `stable`)"
    )]
    NoSemverTags,

    #[error(
        "preparing the managed clone at `{path}` failed: {source} \
         (violates spec://vibevm/common/PROP-019#provenance; \
          fix: ensure the VVM source directory is writable)"
    )]
    Clone {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Git(#[from] GitError),

    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Walk up from `start` to the vibevm source root — the dir carrying the
/// workspace `Cargo.toml` and `crates/vibe-cli`. `None` when not inside a
/// checkout (PROP-019 §2.7).
pub(crate) fn find_source_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| {
            dir.join("Cargo.toml").is_file() && dir.join("crates").join("vibe-cli").is_dir()
        })
        .map(Path::to_path_buf)
}

/// Derive the version label + commit for an in-tree build from git HEAD
/// (PROP-019 §2.7): the current branch when on one, else the commit.
pub(crate) fn label_in_tree(root: &Path) -> Result<ResolvedVersion, ResolveError> {
    let commit = super::git::rev_parse(root, "HEAD")?;
    let id = match super::git::current_branch(root) {
        Some(branch) => VersionId::new(Kind::Branch, branch),
        None => VersionId::new(Kind::Commit, short_commit(&commit)),
    };
    Ok(ResolvedVersion { id, commit })
}

/// A source mirror VVM clones from (PROP-016, PROP-019 §2.7). A closed set;
/// each variant maps to its public clone URL. The first is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mirror {
    GitVerse,
    Github,
}

impl Mirror {
    /// Every mirror, in preference order (the default is the first).
    pub(crate) const ALL: [Mirror; 2] = [Mirror::GitVerse, Mirror::Github];

    /// The lowercase selector token (`gitverse` / `github`).
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Mirror::GitVerse => "gitverse",
            Mirror::Github => "github",
        }
    }

    /// The public clone URL (PROP-016).
    pub(crate) fn url(self) -> &'static str {
        match self {
            Mirror::GitVerse => "https://gitverse.ru/anarchic/vibevm.git",
            Mirror::Github => "https://github.com/anarchic-pro/vibevm.git",
        }
    }

    /// Parse a `--mirror` selector token (PROP-019 §2.7).
    pub(crate) fn parse(s: &str) -> Result<Mirror, ResolveError> {
        Mirror::ALL
            .into_iter()
            .find(|m| m.as_str() == s)
            .ok_or_else(|| ResolveError::UnknownMirror {
                token: s.to_string(),
            })
    }
}

/// Pick the source mirror: an explicit `--mirror`, an interactive choice on
/// a TTY, else the GitVerse default (PROP-019 §2.7).
pub(crate) fn choose_mirror(
    ctx: &output::Context,
    flag: Option<&str>,
) -> Result<Mirror, ResolveError> {
    if let Some(f) = flag {
        return Mirror::parse(f);
    }
    if !ctx.is_unattended() && std::io::stdin().is_terminal() {
        let items = Mirror::ALL.map(|m| m.as_str());
        let pick = Select::new()
            .with_prompt("Source mirror")
            .items(items)
            .default(0)
            .interact()
            .unwrap_or(0);
        return Ok(Mirror::ALL[pick]);
    }
    Ok(Mirror::GitVerse)
}

/// Resolve a selector against a local clone to a concrete version id +
/// commit (PROP-019 §2.3). A clone exposes branches as
/// `refs/remotes/origin/*`.
pub(crate) fn resolve_in_clone(
    repo: &Path,
    selector: &model::Selector,
) -> Result<ResolvedVersion, ResolveError> {
    match selector {
        model::Selector::Latest => {
            let commit = super::git::verify(repo, "refs/remotes/origin/main")
                .or_else(|| super::git::verify(repo, "main"))
                .ok_or_else(|| ResolveError::RefNotFound {
                    what: "branch `main`".to_string(),
                })?;
            Ok(ResolvedVersion {
                id: VersionId::new(Kind::Branch, "main"),
                commit,
            })
        }
        model::Selector::Stable => {
            let (tag, commit) = highest_semver_tag(repo)?;
            Ok(ResolvedVersion {
                id: VersionId::new(Kind::Tag, tag),
                commit,
            })
        }
        model::Selector::Explicit(v) => {
            let commit = match v.kind {
                Kind::Tag => super::git::verify(repo, &format!("refs/tags/{}", v.id))
                    .or_else(|| super::git::verify(repo, &format!("refs/tags/v{}", v.id)))
                    .ok_or_else(|| ResolveError::RefNotFound {
                        what: format!("tag `{}`", v.id),
                    })?,
                Kind::Branch => super::git::verify(repo, &format!("refs/remotes/origin/{}", v.id))
                    .or_else(|| super::git::verify(repo, &v.id))
                    .ok_or_else(|| ResolveError::RefNotFound {
                        what: format!("branch `{}`", v.id),
                    })?,
                Kind::Commit => super::git::verify(repo, &format!("{}^{{commit}}", v.id))
                    .ok_or_else(|| ResolveError::RefNotFound {
                        what: format!("commit `{}`", v.id),
                    })?,
            };
            let id = match v.kind {
                Kind::Commit => VersionId::new(Kind::Commit, short_commit(&commit)),
                _ => v.clone(),
            };
            Ok(ResolvedVersion { id, commit })
        }
        model::Selector::Ambiguous(name) => {
            if let Some(commit) = super::git::verify(repo, &format!("refs/remotes/origin/{name}")) {
                return Ok(ResolvedVersion {
                    id: VersionId::new(Kind::Branch, name.clone()),
                    commit,
                });
            }
            if let Some(commit) = super::git::verify(repo, &format!("refs/tags/{name}")) {
                return Ok(ResolvedVersion {
                    id: VersionId::new(Kind::Tag, name.clone()),
                    commit,
                });
            }
            if let Some(commit) = super::git::verify(repo, &format!("{name}^{{commit}}")) {
                return Ok(ResolvedVersion {
                    id: VersionId::new(Kind::Commit, short_commit(&commit)),
                    commit,
                });
            }
            Err(ResolveError::RefNotFound {
                what: format!("`{name}` (as a branch, tag, or commit)"),
            })
        }
    }
}

fn highest_semver_tag(repo: &Path) -> Result<(String, String), ResolveError> {
    let mut best: Option<(semver::Version, String)> = None;
    for tag in super::git::list_tags(repo)? {
        if let Ok(v) = semver::Version::parse(tag.strip_prefix('v').unwrap_or(&tag))
            && best.as_ref().map(|(bv, _)| &v > bv).unwrap_or(true)
        {
            best = Some((v, tag));
        }
    }
    let (_, tag) = best.ok_or(ResolveError::NoSemverTags)?;
    let commit = super::git::verify(repo, &format!("refs/tags/{tag}")).ok_or_else(|| {
        ResolveError::RefNotFound {
            what: format!("tag `{tag}`"),
        }
    })?;
    Ok((tag, commit))
}

/// The result of cloning and resolving a mirror (PROP-019 §2.7).
pub(crate) struct CloneOutcome {
    pub src_dir: PathBuf,
    pub resolved: ResolvedVersion,
}

/// Ensure the shared managed clone is present and up to date, resolve the
/// selector against it, check out the commit, and return it as the build
/// source (PROP-019 §2.7, §2.16). The clone is updated incrementally
/// (`git fetch`), never re-cloned — a full rebuild can take hours.
pub(crate) fn prepare_from_mirror(
    store: &VersionStore,
    mirror: &str,
    selector: &model::Selector,
) -> Result<CloneOutcome, ResolveError> {
    let dir = store.mirror_dir();
    if dir.join(".git").is_dir() {
        super::git::fetch(&dir)?;
    } else {
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|source| ResolveError::Clone {
                path: dir.clone(),
                source,
            })?;
        }
        if let Some(parent) = dir.parent() {
            fs::create_dir_all(parent).map_err(|source| ResolveError::Clone {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        super::git::clone(mirror, &dir)?;
    }
    let resolved = resolve_in_clone(&dir, selector)?;
    super::git::checkout(&dir, &resolved.commit)?;
    Ok(CloneOutcome {
        src_dir: dir,
        resolved,
    })
}

/// The friendly absolute path of an external source for provenance — strips
/// the Windows verbatim `\\?\` prefix (PROP-019 §2.16).
pub(crate) fn external_path(root: &Path) -> String {
    let canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let s = canon.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

/// If the selector matches an installed *external* version whose recorded
/// source path is still a vibevm checkout, return that path for a linked
/// rebuild — so `man install <id>` rebuilds from the remembered location
/// without being in the checkout (PROP-019 §2.16).
pub(crate) fn linked_source(
    store: &VersionStore,
    selector: &model::Selector,
    raw: &str,
) -> Result<Option<PathBuf>, ResolveError> {
    let state = store.load_state()?;
    let Ok(rec) = super::resolve_installed(&state, selector, raw) else {
        return Ok(None);
    };
    if rec.origin == model::Origin::External
        && let Some(p) = rec.source_path
    {
        let path = PathBuf::from(p);
        if find_source_root(&path).is_some() {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::man::model::{Kind, Selector, VersionId};
    use specmark::verifies;

    fn make_source_repo(dir: &Path) {
        let g = |args: &[&str]| super::super::git::run(dir, args).unwrap();
        g(&["init", "-q", "-b", "main"]);
        g(&["config", "user.email", "t@example.com"]);
        g(&["config", "user.name", "tester"]);
        fs::write(dir.join("a.txt"), "1").unwrap();
        g(&["add", "."]);
        g(&["commit", "-q", "-m", "one"]);
        g(&["tag", "1.2.0"]);
        fs::write(dir.join("a.txt"), "2").unwrap();
        g(&["commit", "-aqm", "two"]);
        g(&["tag", "1.10.0"]);
        g(&["branch", "feature"]);
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#build", r = 1)]
    fn find_source_root_walks_up_to_the_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        let nested = root.join("crates").join("vibe-cli").join("src");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(
            find_source_root(&nested).unwrap().file_name(),
            root.file_name()
        );
        assert!(find_source_root(tempfile::tempdir().unwrap().path()).is_none());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#selectors", r = 1)]
    fn resolve_in_clone_against_a_local_clone() {
        let src = tempfile::tempdir().unwrap();
        make_source_repo(src.path());
        let holder = tempfile::tempdir().unwrap();
        let clone = holder.path().join("c");
        super::super::git::clone(&src.path().display().to_string(), &clone).unwrap();

        let r = |s| resolve_in_clone(&clone, &s).unwrap();
        assert_eq!(r(Selector::Latest).id, VersionId::new(Kind::Branch, "main"));
        assert_eq!(r(Selector::Stable).id, VersionId::new(Kind::Tag, "1.10.0"));
        assert_eq!(
            r(Selector::Explicit(VersionId::new(Kind::Tag, "1.2.0"))).id,
            VersionId::new(Kind::Tag, "1.2.0")
        );
        assert_eq!(
            r(Selector::Ambiguous("feature".into())).id,
            VersionId::new(Kind::Branch, "feature")
        );
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#build", r = 1)]
    fn mirror_parses_names_and_maps_urls() {
        assert_eq!(Mirror::parse("gitverse").unwrap(), Mirror::GitVerse);
        assert_eq!(Mirror::parse("github").unwrap(), Mirror::Github);
        assert!(Mirror::parse("nope").is_err());
        assert!(Mirror::GitVerse.url().starts_with("https://"));
        assert_eq!(Mirror::ALL[0], Mirror::GitVerse);
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#provenance", r = 1)]
    fn prepare_from_mirror_clones_then_fetches() {
        let upstream = tempfile::tempdir().unwrap();
        make_source_repo(upstream.path());
        let root = tempfile::tempdir().unwrap();
        let store = VersionStore::new(root.path());
        let url = upstream.path().display().to_string();

        // First call clones into the shared mirror.
        let out = prepare_from_mirror(&store, &url, &Selector::Stable).unwrap();
        assert_eq!(out.resolved.id, VersionId::new(Kind::Tag, "1.10.0"));
        assert_eq!(out.src_dir, store.mirror_dir());
        assert!(store.mirror_dir().join(".git").is_dir());

        // Second call reuses + fetches (no re-clone) and still resolves.
        let out2 = prepare_from_mirror(&store, &url, &Selector::Latest).unwrap();
        assert_eq!(out2.resolved.id, VersionId::new(Kind::Branch, "main"));
    }
}
