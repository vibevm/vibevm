//! The install pipeline (PROP-019 §2.7): locate the source, build it,
//! publish the binary atomically, and record metadata. The slow,
//! machine-touching step (the cargo build) sits behind the [`Builder`] seam
//! so the orchestration is testable without a real compile.

specmark::scope!("spec://vibevm/common/PROP-019#build");

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Select;
use specmark::spec;

use super::model::{InstallRecord, Kind, Profile, VersionId};
use super::store::{BINARY_NAME, VersionStore};
use super::{git, model};
use crate::output;

/// A selector resolved to a concrete version id and commit (PROP-019 §2.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVersion {
    pub id: VersionId,
    pub commit: String,
}

/// The product of a successful build: where the binary landed and which
/// toolchain produced it (PROP-019 §2.7).
#[derive(Debug, Clone)]
pub struct BuildOutput {
    pub binary: PathBuf,
    pub toolchain: String,
}

/// Builds a vibevm source tree into a `vibe` binary (PROP-019 §2.7). A
/// crate-internal seam (vibe-cli is a bin crate, so this is not a public
/// cross-crate contract) that lets tests drive the pipeline without a real
/// cargo build.
pub(crate) trait Builder {
    fn build(&self, source_root: &Path, target_dir: &Path, profile: Profile)
    -> Result<BuildOutput>;
}

/// The production builder: `cargo build [--release] -p vibe-cli`, honouring
/// the source tree's `rust-toolchain.toml` (PROP-019 §2.7, §2.8).
#[spec(implements = "spec://vibevm/common/PROP-019#build")]
pub struct CargoBuilder;

impl Builder for CargoBuilder {
    fn build(
        &self,
        source_root: &Path,
        target_dir: &Path,
        profile: Profile,
    ) -> Result<BuildOutput> {
        // Build into a VVM-managed `--target-dir`, never the source tree's
        // own `target/`. This keeps the dev tree clean and — load-bearing on
        // Windows — avoids cargo trying to relink a `vibe.exe` that is the
        // currently-running binary, which fails with "Access is denied"
        // (PROP-019 §2.7).
        let mut cmd = Command::new("cargo");
        cmd.current_dir(source_root)
            .args(["build", "-p", "vibe-cli"]);
        if profile == Profile::Release {
            cmd.arg("--release");
        }
        cmd.arg("--target-dir").arg(target_dir);
        let status = cmd
            .status()
            .with_context(|| format!("spawning cargo build in `{}`", source_root.display()))?;
        if !status.success() {
            bail!("cargo build failed (exit {:?})", status.code());
        }
        let binary = target_dir.join(profile.target_subdir()).join(BINARY_NAME);
        if !binary.is_file() {
            bail!(
                "build reported success but `{}` is missing",
                binary.display()
            );
        }
        let toolchain = Command::new("rustc")
            .current_dir(source_root)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Ok(BuildOutput { binary, toolchain })
    }
}

/// Walk up from `start` to the vibevm source root — the dir carrying the
/// workspace `Cargo.toml` and `crates/vibe-cli`. `None` when not inside a
/// checkout (PROP-019 §2.7).
pub fn find_source_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| {
            dir.join("Cargo.toml").is_file() && dir.join("crates").join("vibe-cli").is_dir()
        })
        .map(Path::to_path_buf)
}

/// Derive the version label + commit for an in-tree build from git HEAD
/// (PROP-019 §2.7): the current branch when on one, else the commit.
pub fn label_in_tree(root: &Path) -> Result<ResolvedVersion> {
    let commit = git::rev_parse(root, "HEAD").context("resolving HEAD in the source tree")?;
    let id = match git::current_branch(root) {
        Some(branch) => VersionId::new(Kind::Branch, branch),
        None => VersionId::new(Kind::Commit, short_commit(&commit)),
    };
    Ok(ResolvedVersion { id, commit })
}

fn short_commit(c: &str) -> String {
    c[..c.len().min(10)].to_string()
}

/// A best-effort install lock so two concurrent installs do not race
/// (PROP-019 §2.7). Removed on drop.
struct InstallLock {
    path: PathBuf,
}

impl InstallLock {
    fn acquire(store: &VersionStore) -> Result<InstallLock> {
        let dir = store.data_dir();
        fs::create_dir_all(&dir).with_context(|| format!("creating `{}`", dir.display()))?;
        let path = dir.join(".install.lock");
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => Ok(InstallLock { path }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => bail!(
                "another `vibe man install` is in progress (remove `{}` if it is stale)",
                path.display()
            ),
            Err(e) => Err(e).with_context(|| format!("creating lock `{}`", path.display())),
        }
    }
}

impl Drop for InstallLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Parameters for [`perform_install`] — bundled so the orchestration stays
/// under clippy's argument-count limit.
pub(crate) struct InstallRequest<'a> {
    pub resolved: &'a ResolvedVersion,
    pub profile: Profile,
    pub force: bool,
    /// The RFC3339 install timestamp, stamped at the composition layer so
    /// the pipeline itself reads no clock.
    pub now: &'a str,
}

/// Build and publish a resolved version into the store, recording its
/// metadata (PROP-019 §2.7). Idempotent: an already-installed version is a
/// no-op unless `force`.
#[spec(implements = "spec://vibevm/common/PROP-019#build")]
pub(crate) fn perform_install(
    ctx: &output::Context,
    store: &VersionStore,
    source_root: &Path,
    req: &InstallRequest,
    builder: &dyn Builder,
) -> Result<()> {
    let id = &req.resolved.id;
    let dest = store.binary_path(id);
    if dest.is_file() && !req.force {
        ctx.summary(&format!(
            "{id} already installed ({}) — use --force to rebuild",
            req.profile.as_str()
        ));
        return Ok(());
    }

    let _lock = InstallLock::acquire(store)?;
    ctx.step(&format!(
        "building {id} ({}) from {}",
        req.profile.as_str(),
        source_root.display()
    ));
    let out = builder.build(source_root, &store.build_dir(), req.profile)?;

    let prefix = store.version_prefix(id);
    fs::create_dir_all(&prefix).with_context(|| format!("creating `{}`", prefix.display()))?;
    let tmp = prefix.join(format!("{BINARY_NAME}.tmp"));
    fs::copy(&out.binary, &tmp)
        .with_context(|| format!("copying `{}` → `{}`", out.binary.display(), tmp.display()))?;
    fs::rename(&tmp, &dest).with_context(|| format!("publishing `{}`", dest.display()))?;

    store.record_install(InstallRecord {
        kind: id.kind,
        id: id.id.clone(),
        commit: req.resolved.commit.clone(),
        toolchain: out.toolchain,
        profile: req.profile.as_str().to_string(),
        installed_at: req.now.to_string(),
    })?;

    ctx.created(&dest.display().to_string());
    ctx.summary(&format!("installed {id} → {}", dest.display()));
    Ok(())
}

// ---------------------------------------------------------------------------
// the clone path — install a ref that is not the current working tree
// ---------------------------------------------------------------------------

const MIRROR_GITVERSE: &str = "https://gitverse.ru/anarchic/vibevm.git";
const MIRROR_GITHUB: &str = "https://github.com/anarchic-pro/vibevm.git";

/// Map a mirror name to its public clone URL (PROP-019 §2.7, PROP-016).
pub(crate) fn mirror_url(choice: &str) -> Result<&'static str> {
    match choice {
        "gitverse" => Ok(MIRROR_GITVERSE),
        "github" => Ok(MIRROR_GITHUB),
        other => bail!("unknown mirror `{other}` (want gitverse|github)"),
    }
}

/// Pick the source mirror: an explicit `--mirror`, an interactive choice on
/// a TTY, else the GitVerse default (PROP-019 §2.7).
pub(crate) fn choose_mirror(ctx: &output::Context, flag: Option<&str>) -> Result<&'static str> {
    if let Some(f) = flag {
        return mirror_url(f);
    }
    if !ctx.is_unattended() && std::io::stdin().is_terminal() {
        let items = ["gitverse", "github"];
        let pick = Select::new()
            .with_prompt("Source mirror")
            .items(items)
            .default(0)
            .interact()
            .unwrap_or(0);
        return mirror_url(items[pick]);
    }
    Ok(MIRROR_GITVERSE)
}

/// Resolve a selector against a local clone to a concrete version id +
/// commit (PROP-019 §2.3). A clone exposes branches as
/// `refs/remotes/origin/*`.
pub(crate) fn resolve_in_clone(repo: &Path, selector: &model::Selector) -> Result<ResolvedVersion> {
    match selector {
        model::Selector::Latest => {
            let commit = git::verify(repo, "refs/remotes/origin/main")
                .or_else(|| git::verify(repo, "main"))
                .ok_or_else(|| anyhow!("branch `main` not found in the clone"))?;
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
                Kind::Tag => git::verify(repo, &format!("refs/tags/{}", v.id))
                    .or_else(|| git::verify(repo, &format!("refs/tags/v{}", v.id)))
                    .ok_or_else(|| anyhow!("tag `{}` not found in the clone", v.id))?,
                Kind::Branch => git::verify(repo, &format!("refs/remotes/origin/{}", v.id))
                    .or_else(|| git::verify(repo, &v.id))
                    .ok_or_else(|| anyhow!("branch `{}` not found in the clone", v.id))?,
                Kind::Commit => git::verify(repo, &format!("{}^{{commit}}", v.id))
                    .ok_or_else(|| anyhow!("commit `{}` not found in the clone", v.id))?,
            };
            let id = match v.kind {
                Kind::Commit => VersionId::new(Kind::Commit, short_commit(&commit)),
                _ => v.clone(),
            };
            Ok(ResolvedVersion { id, commit })
        }
        model::Selector::Ambiguous(name) => {
            // A bare word: branch, then tag, then commit (hex commits and
            // `X.Y.Z` tags are already classified before reaching here).
            if let Some(commit) = git::verify(repo, &format!("refs/remotes/origin/{name}")) {
                return Ok(ResolvedVersion {
                    id: VersionId::new(Kind::Branch, name.clone()),
                    commit,
                });
            }
            if let Some(commit) = git::verify(repo, &format!("refs/tags/{name}")) {
                return Ok(ResolvedVersion {
                    id: VersionId::new(Kind::Tag, name.clone()),
                    commit,
                });
            }
            if let Some(commit) = git::verify(repo, &format!("{name}^{{commit}}")) {
                return Ok(ResolvedVersion {
                    id: VersionId::new(Kind::Commit, short_commit(&commit)),
                    commit,
                });
            }
            bail!("`{name}` is not a branch, tag, or commit in the clone")
        }
    }
}

/// The highest semantic-version tag in the clone, with its commit.
fn highest_semver_tag(repo: &Path) -> Result<(String, String)> {
    let mut best: Option<(semver::Version, String)> = None;
    for tag in git::list_tags(repo)? {
        if let Ok(v) = semver::Version::parse(tag.strip_prefix('v').unwrap_or(&tag))
            && best.as_ref().map(|(bv, _)| &v > bv).unwrap_or(true)
        {
            best = Some((v, tag));
        }
    }
    let (_, tag) = best.ok_or_else(|| anyhow!("no semantic-version tags in the clone"))?;
    let commit = git::verify(repo, &format!("refs/tags/{tag}"))
        .ok_or_else(|| anyhow!("tag `{tag}` did not resolve"))?;
    Ok((tag, commit))
}

/// The result of cloning and resolving a mirror (PROP-019 §2.7).
pub(crate) struct CloneOutcome {
    pub src_dir: PathBuf,
    pub resolved: ResolvedVersion,
}

/// Clone the mirror, resolve the selector against it, check out the commit,
/// and place the tree at `src/<kind>/<id>` (PROP-019 §2.7).
pub(crate) fn prepare_from_mirror(
    store: &VersionStore,
    mirror: &str,
    selector: &model::Selector,
) -> Result<CloneOutcome> {
    let staging = store.data_dir().join("src").join(".staging");
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .with_context(|| format!("clearing `{}`", staging.display()))?;
    }
    if let Some(parent) = staging.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating `{}`", parent.display()))?;
    }
    git::clone(mirror, &staging)?;
    let resolved = resolve_in_clone(&staging, selector)?;
    git::checkout(&staging, &resolved.commit)?;

    let dest = store.src_dir(&resolved.id);
    if dest.exists() {
        fs::remove_dir_all(&dest).with_context(|| format!("clearing `{}`", dest.display()))?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating `{}`", parent.display()))?;
    }
    fs::rename(&staging, &dest)
        .with_context(|| format!("placing source at `{}`", dest.display()))?;
    Ok(CloneOutcome {
        src_dir: dest,
        resolved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    /// A builder that fabricates a dummy binary instead of compiling, so the
    /// orchestration is exercised without a real (minutes-long) build.
    struct FakeBuilder {
        dir: tempfile::TempDir,
    }

    impl FakeBuilder {
        fn new() -> Self {
            FakeBuilder {
                dir: tempfile::tempdir().unwrap(),
            }
        }
    }

    impl Builder for FakeBuilder {
        fn build(
            &self,
            _source_root: &Path,
            _target_dir: &Path,
            _profile: Profile,
        ) -> Result<BuildOutput> {
            let binary = self.dir.path().join("vibe-built");
            fs::write(&binary, b"#!fake vibe\n").unwrap();
            Ok(BuildOutput {
                binary,
                toolchain: "rustc 0.0.0-fake".into(),
            })
        }
    }

    fn quiet_ctx() -> output::Context {
        // quiet + unattended → no stdout noise during tests.
        output::Context::from_flags(true, false, None, true)
    }

    fn init_repo(dir: &Path) {
        git::run(dir, &["init", "-q", "-b", "main"]).unwrap();
        git::run(dir, &["config", "user.email", "t@example.com"]).unwrap();
        git::run(dir, &["config", "user.name", "tester"]).unwrap();
        fs::write(dir.join("f.txt"), "x").unwrap();
        git::run(dir, &["add", "."]).unwrap();
        git::run(dir, &["commit", "-q", "-m", "init"]).unwrap();
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
        // An unrelated directory has no vibevm source root.
        let other = tempfile::tempdir().unwrap();
        assert!(find_source_root(other.path()).is_none());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#build", r = 1)]
    fn label_in_tree_reads_branch_then_commit() {
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());
        let on_branch = label_in_tree(tmp.path()).unwrap();
        assert_eq!(on_branch.id, VersionId::new(Kind::Branch, "main"));
        assert_eq!(on_branch.commit.len(), 40, "full commit hash");
        // Detached HEAD → commit label.
        git::run(tmp.path(), &["checkout", "-q", &on_branch.commit]).unwrap();
        assert_eq!(label_in_tree(tmp.path()).unwrap().id.kind, Kind::Commit);
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#build", r = 1)]
    fn perform_install_publishes_records_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let resolved = ResolvedVersion {
            id: VersionId::new(Kind::Branch, "main"),
            commit: "deadbeefcafe".into(),
        };
        let builder = FakeBuilder::new();
        let src = tempfile::tempdir().unwrap();
        let req = |force, now| InstallRequest {
            resolved: &resolved,
            profile: Profile::Debug,
            force,
            now,
        };

        perform_install(
            &quiet_ctx(),
            &store,
            src.path(),
            &req(false, "2026-06-17T00:00:00Z"),
            &builder,
        )
        .unwrap();
        let dest = store.binary_path(&resolved.id);
        assert!(dest.is_file(), "binary published to the version prefix");
        let state = store.load_state().unwrap();
        assert_eq!(state.installs.len(), 1);
        assert_eq!(state.installs[0].commit, "deadbeefcafe");

        // Idempotent: a second non-force install neither rebuilds nor dups.
        perform_install(
            &quiet_ctx(),
            &store,
            src.path(),
            &req(false, "2026-06-17T00:00:00Z"),
            &builder,
        )
        .unwrap();
        assert_eq!(store.load_state().unwrap().installs.len(), 1);

        // Force re-publishes and upserts (still one record, newer stamp).
        perform_install(
            &quiet_ctx(),
            &store,
            src.path(),
            &req(true, "2026-06-18T00:00:00Z"),
            &builder,
        )
        .unwrap();
        let state = store.load_state().unwrap();
        assert_eq!(state.installs.len(), 1);
        assert_eq!(state.installs[0].installed_at, "2026-06-18T00:00:00Z");
    }

    /// A source repo with two tags (1.2.0, 1.10.0) on `main` and a `feature`
    /// branch, for the clone-path tests.
    fn make_source_repo(dir: &Path) {
        git::run(dir, &["init", "-q", "-b", "main"]).unwrap();
        git::run(dir, &["config", "user.email", "t@example.com"]).unwrap();
        git::run(dir, &["config", "user.name", "tester"]).unwrap();
        fs::write(dir.join("a.txt"), "1").unwrap();
        git::run(dir, &["add", "."]).unwrap();
        git::run(dir, &["commit", "-q", "-m", "one"]).unwrap();
        git::run(dir, &["tag", "1.2.0"]).unwrap();
        fs::write(dir.join("a.txt"), "2").unwrap();
        git::run(dir, &["commit", "-aqm", "two"]).unwrap();
        git::run(dir, &["tag", "1.10.0"]).unwrap();
        git::run(dir, &["branch", "feature"]).unwrap();
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#build", r = 1)]
    fn mirror_url_maps_names() {
        assert!(mirror_url("gitverse").is_ok());
        assert!(mirror_url("github").is_ok());
        assert!(mirror_url("nope").is_err());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#selectors", r = 1)]
    fn resolve_in_clone_against_a_local_clone() {
        let src = tempfile::tempdir().unwrap();
        make_source_repo(src.path());
        let holder = tempfile::tempdir().unwrap();
        let clone = holder.path().join("c");
        git::clone(&src.path().display().to_string(), &clone).unwrap();

        let r = |s| resolve_in_clone(&clone, &s).unwrap();
        assert_eq!(
            r(model::Selector::Latest).id,
            VersionId::new(Kind::Branch, "main")
        );
        assert_eq!(
            r(model::Selector::Stable).id,
            VersionId::new(Kind::Tag, "1.10.0"),
            "stable picks the highest semver tag"
        );
        assert_eq!(
            r(model::Selector::Explicit(VersionId::new(
                Kind::Tag,
                "1.2.0"
            )))
            .id,
            VersionId::new(Kind::Tag, "1.2.0")
        );
        assert_eq!(
            r(model::Selector::Ambiguous("feature".into())).id,
            VersionId::new(Kind::Branch, "feature")
        );
        // A commit selector resolves and labels by the (short) hash.
        let head = git::verify(&clone, "HEAD").unwrap();
        let c = r(model::Selector::Explicit(VersionId::new(
            Kind::Commit,
            head.clone(),
        )));
        assert_eq!(c.id.kind, Kind::Commit);
        assert_eq!(c.commit, head);
        // An unknown ref errors.
        assert!(
            resolve_in_clone(
                &clone,
                &model::Selector::Explicit(VersionId::new(Kind::Tag, "9.9.9"))
            )
            .is_err()
        );
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#build", r = 1)]
    fn prepare_from_mirror_clones_resolves_and_places() {
        let src = tempfile::tempdir().unwrap();
        make_source_repo(src.path());
        let root = tempfile::tempdir().unwrap();
        let store = VersionStore::new(root.path());

        let outcome = prepare_from_mirror(
            &store,
            &src.path().display().to_string(),
            &model::Selector::Stable,
        )
        .unwrap();
        assert_eq!(outcome.resolved.id, VersionId::new(Kind::Tag, "1.10.0"));
        assert!(outcome.src_dir.exists());
        assert_eq!(
            outcome.src_dir,
            store.src_dir(&VersionId::new(Kind::Tag, "1.10.0"))
        );
    }
}
