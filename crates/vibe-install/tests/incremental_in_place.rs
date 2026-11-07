//! Real local Git preparation feeds the general install deferred apply owner.
//! Solve/fetch counters are fixture observations; commits, hashes, manifests and
//! changed/no-op outcomes come from the actual registry preparation/materializer.
//! This integration target avoids Windows UAC's installer-name test restriction.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION");

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use tempfile::TempDir;
use vibe_core::manifest::{Lockfile, Manifest, NamingConvention};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{
    CachedPackage, GitPerPackageRegistry, InPlaceMaterialised, PreparedInPlace, RegistryError,
    ShellGit,
};
use vibe_resolver::{FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};
use vibe_workspace::hooks::HookPolicy;

#[derive(Debug, Clone)]
struct PreparedObservation {
    slot: PathBuf,
    commit: String,
    content_hash: String,
    source_uri: String,
    source_ref: String,
}

/// Only graph selection is mocked. Source preparation uses actual local Git.
struct Source {
    registry: GitPerPackageRegistry,
    version: RefCell<semver::Version>,
    fetched: RefCell<Vec<String>>,
    prepared: RefCell<Vec<PreparedObservation>>,
    unbound_placements: Cell<usize>,
}
impl Source {
    fn graph(&self) -> ResolvedGraph {
        ResolvedGraph {
            packages: vec![ResolvedNode {
                group: Group::parse("org.vibevm").unwrap(),
                name: "giant".into(),
                version: self.version.borrow().clone(),
                dependencies: vec![],
                is_root: true,
            }],
        }
    }
}
impl InstallSource for Source {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        _store: &Path,
        _expected: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetched.borrow_mut().push(pkgref.name.to_string());
        Err(RegistryError::UnknownPackage {
            group: pkgref.group.clone().unwrap(),
            name: pkgref.name.to_string(),
        })
    }
    fn solve(&self, _roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(self.graph())
    }
    fn solve_masked(
        &self,
        _roots: &[PackageRef],
        _blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        Ok(self.graph())
    }
    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
        let read = || {
            let selected = self.registry.resolve(pkg)?;
            self.registry
                .fetch_dep_manifest(&selected.group, &selected.name, &selected.version)
        };
        read().map_err(|e: RegistryError| {
            vibe_resolver::DepProviderError::Other(e.to_string()).into()
        })
    }
    fn prepare_in_place(
        &self,
        pkg: &PackageRef,
        slot: &Path,
    ) -> Result<PreparedInPlace, RegistryError> {
        let selected = self.registry.resolve(pkg)?;
        let prepared = self.registry.prepare_in_place(&selected, slot)?;
        self.prepared.borrow_mut().push(PreparedObservation {
            slot: prepared.slot().to_owned(),
            commit: prepared.resolved_commit().into(),
            content_hash: prepared.content_hash().into(),
            source_uri: prepared.source_uri().into(),
            source_ref: prepared.source_ref().into(),
        });
        Ok(prepared)
    }
    fn materialise_in_place(
        &self,
        pkg: &PackageRef,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        self.unbound_placements
            .set(self.unbound_placements.get() + 1);
        Err(RegistryError::PreparedInPlace {
            path: slot.to_owned(),
            reason: format!("unexpected unbound placement of {pkg}"),
        })
    }
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "protocol.file.allow=always",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}

/// Keep backend Git subprocesses independent of user aliases, hooks and signing.
fn backend(root: &Path) -> ShellGit {
    write(root, "empty-gitconfig", "");
    let wrapper = root.join(if cfg!(windows) {
        "fixture-git.cmd"
    } else {
        "fixture-git"
    });
    let body = if cfg!(windows) {
        "@echo off\r\nset GIT_CONFIG_NOSYSTEM=1\r\nset GIT_CONFIG_GLOBAL=%~dp0empty-gitconfig\r\ngit -c core.hooksPath=\"%~dp0empty-hooks\" -c protocol.file.allow=always %*\r\nexit /b %errorlevel%\r\n"
    } else {
        "#!/bin/sh\nfixture_root=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd) || exit 1\nexport GIT_CONFIG_NOSYSTEM=1\nexport GIT_CONFIG_GLOBAL=\"$fixture_root/empty-gitconfig\"\nexec git -c core.hooksPath=\"$fixture_root/empty-hooks\" -c protocol.file.allow=always \"$@\"\n"
    };
    fs::write(&wrapper, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    }
    ShellGit::new().with_binary(wrapper)
}

struct Fixture {
    _temp: TempDir,
    project: PathBuf,
    upstream: PathBuf,
    slot: PathBuf,
    source: Source,
}
impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        let upstream = temp.path().join("org/org.vibevm.giant.git");
        fs::create_dir_all(&upstream).unwrap();
        git(&upstream, &["init", "--initial-branch=main"]);
        // Disable hooks in the fixture repository without changing user config.
        let hooks = temp.path().join("empty-hooks");
        fs::create_dir(&hooks).unwrap();
        git(
            &upstream,
            &["config", "core.hooksPath", hooks.to_str().unwrap()],
        );
        write(
            &project,
            "vibe.toml",
            "[project]\nname='demo'\nversion='0.0.1'\n",
        );
        let registry = GitPerPackageRegistry::open_with(
            "fixture",
            upstream.parent().unwrap().to_str().unwrap(),
            "main",
            NamingConvention::Fqdn,
            &temp.path().join("cache"),
            Arc::new(backend(temp.path())),
            0,
        )
        .unwrap();
        let slot = project
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.vibevm.giant");
        Self {
            _temp: temp,
            project,
            upstream,
            slot,
            source: Source {
                registry,
                version: RefCell::new(semver::Version::parse("1.0.0").unwrap()),
                fetched: RefCell::new(vec![]),
                prepared: RefCell::new(vec![]),
                unbound_placements: Cell::new(0),
            },
        }
    }
    fn publish(&self, version: &str, extra: &str) -> String {
        write(
            &self.upstream,
            "vibe.toml",
            &format!(
                "[package]\ngroup='org.vibevm'\nname='giant'\nkind='feat'\nversion='{version}'\nmaterialization='in-place'\n\n[hooks]\npre-install='hooks/prepare'\npost-install='hooks/finalise'\n{extra}"
            ),
        );
        write(&self.upstream, "SENTINEL", "tracked source must survive\n");
        write(&self.upstream, "payload.txt", version);
        for phase in ["prepare", "finalise"] {
            write(
                &self.upstream,
                &format!("hooks/{phase}.sh"),
                "printf '%s\\n' \"$VIBE_HOOK_PHASE\" >> hook.log\n",
            );
            write(
                &self.upstream,
                &format!("hooks/{phase}.ps1"),
                "$env:VIBE_HOOK_PHASE | Add-Content -Path hook.log\n",
            );
        }
        git(&self.upstream, &["add", "-A"]);
        git(&self.upstream, &["commit", "-m", "fixture version"]);
        git(&self.upstream, &["tag", &format!("v{version}")]);
        *self.source.version.borrow_mut() = semver::Version::parse(version).unwrap();
        git(&self.upstream, &["rev-parse", "HEAD"])
    }
    fn seed(&self) -> InPlaceMaterialised {
        let selected = self
            .source
            .registry
            .resolve(&PackageRef::parse("org.vibevm/giant@=1.0.0").unwrap())
            .unwrap();
        let prepared = self
            .source
            .registry
            .prepare_in_place(&selected, &self.slot)
            .unwrap();
        let placed = prepared.materialise().unwrap();
        assert!(placed.changed);
        assert_eq!(
            git(&self.slot, &["rev-parse", "HEAD"]),
            prepared.resolved_commit()
        );
        // JSON string quoting is also valid for these TOML basic-string fields.
        let quoted = |text: &str| serde_json::to_string(text).unwrap();
        write(
            &self.project,
            "vibe.lock",
            &format!(
                "[meta]\ngenerated_by='vibe test'\ngenerated_at='2026-11-05T00:00:00Z'\nschema_version=7\n[[package]]\nkind='feat'\ngroup='org.vibevm'\nname='giant'\nversion='1.0.0'\nsource_url={}\nsource_ref={}\ncontent_hash={}\nsource_kind='registry'\nresolved_commit={}\nmaterialization='in-place'\n",
                quoted(&placed.source_uri),
                quoted(&placed.source_ref),
                quoted(&placed.content_hash),
                quoted(placed.resolved_commit.as_deref().unwrap()),
            ),
        );
        write(
            &self.slot,
            ".git/sentinel",
            "same repository must survive\n",
        );
        placed
    }
    fn plan(&self) -> Result<Plan, vibe_install::Error> {
        vibe_install::plan(
            &self.source,
            &self.project,
            InstallRequest {
                roots: vec![PackageRef::parse("org.vibevm/giant").unwrap()],
                features: FeatureRequest::default(),
                language: None,
                exact: false,
                generated_by: "vibe test".into(),
                offline: true,
            },
            &NullObserver,
        )
    }
    fn ready(&self) -> Box<vibe_install::PlannedInstall> {
        match self.plan().unwrap() {
            Plan::Ready(plan) => plan,
            Plan::Fresh => panic!("explicit-root install must produce a real plan"),
        }
    }
    fn assert_no_reclone(&self) {
        assert!(self.source.fetched.borrow().is_empty());
        assert_eq!(self.source.unbound_placements.get(), 0);
        assert_eq!(
            fs::read_to_string(self.slot.join(".git/sentinel")).unwrap(),
            "same repository must survive\n"
        );
        assert_eq!(
            fs::read_to_string(self.slot.join("SENTINEL")).unwrap(),
            "tracked source must survive\n"
        );
    }
    fn assert_provenance(&self, expected: &PreparedObservation) {
        let lock = Lockfile::read(self.project.join("vibe.lock")).unwrap();
        let giant = lock
            .find(&Group::parse("org.vibevm").unwrap(), "giant")
            .unwrap();
        assert!(giant.materialization.is_in_place());
        assert_eq!(
            giant.resolved_commit.as_deref(),
            Some(expected.commit.as_str())
        );
        assert_eq!(giant.content_hash.as_str(), expected.content_hash);
        assert_eq!(giant.source_url.as_str(), expected.source_uri);
        assert_eq!(
            giant.source_ref.as_deref(),
            Some(expected.source_ref.as_str())
        );
        assert_eq!(git(&self.slot, &["rev-parse", "HEAD"]), expected.commit);
    }
}
fn policy() -> HookPolicy {
    HookPolicy {
        allowed_groups: vec!["org.vibevm".into()],
        allow_hooks: false,
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn general_install_defers_in_place_instead_of_recloning() {
    let f = Fixture::new();
    let first_commit = f.publish("1.0.0", "");
    let original = f.seed();
    assert_eq!(
        original.resolved_commit.as_deref(),
        Some(first_commit.as_str())
    );
    let plan = f.ready();
    assert_eq!(f.source.prepared.borrow().len(), 1);
    assert_eq!(f.source.prepared.borrow()[0].slot, f.slot);
    let first = f.source.prepared.borrow()[0].clone();
    assert_eq!(first.content_hash, original.content_hash);
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), first_commit);
    f.assert_no_reclone();
    let applied = vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
    assert!(applied.outcome.hook_reports.is_empty());
    assert!(applied.post_install_reports.is_empty());
    assert!(
        !f.slot.join("hook.log").exists(),
        "actual no-op suppresses both hook phases"
    );
    f.assert_provenance(&first);
    f.assert_no_reclone();

    let next_commit = f.publish("1.1.0", "");
    assert_ne!(next_commit, first_commit);
    let plan = f.ready();
    assert_eq!(f.source.prepared.borrow().len(), 2);
    let next = f.source.prepared.borrow()[1].clone();
    assert_eq!(next.commit, next_commit);
    assert_ne!(next.content_hash, first.content_hash);
    assert_eq!(
        git(&f.slot, &["rev-parse", "HEAD"]),
        first_commit,
        "prepare must not checkout the new revision"
    );
    let applied = vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
    assert_eq!(applied.outcome.hook_reports.len(), 1);
    assert_eq!(applied.post_install_reports.len(), 1);
    assert_eq!(
        fs::read_to_string(f.slot.join("hook.log"))
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        ["pre-install", "post-install"]
    );
    f.assert_provenance(&next);
    f.assert_no_reclone();
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-EVIDENCE-PROJECT-EVENTS"
)]
fn held_project_observation_and_next_lock_use_current_preparation_alias() {
    let f = Fixture::new();
    let old_commit = f.publish("1.0.0", "");
    f.seed();
    let lock_path = f.project.join("vibe.lock");
    let mut old = Lockfile::read(&lock_path).unwrap();
    old.packages[0].registry = Some("historical-alias".into());
    old.write(&lock_path).unwrap();
    let commit = f.publish("1.1.0", "");
    let mut plan = f.ready();
    let captured = plan
        .package_sources()
        .map(|(g, n, v, e)| (format!("{g}/{n}@{v}"), e))
        .collect::<Vec<_>>();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].0, "org.vibevm/giant@1.1.0");
    assert_eq!(captured[0].1.registry.as_deref(), Some("fixture"));
    assert_eq!(
        captured[0].1.resolved_commit.as_deref(),
        Some(commit.as_str())
    );
    assert_eq!(captured[0].1.store_reused, Some(false));
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), old_commit);
    let before = worktree(&f.project);
    let prepared_count = f.source.prepared.borrow().len();
    // Public resolution rows and later on-disk history cannot be zipped into held facts.
    let original_name = std::mem::replace(&mut plan.resolution[0].name, "unrelated".into());
    let old_bytes = fs::read(&lock_path).unwrap();
    old.packages[0].registry = Some("later-history".into());
    old.write(&lock_path).unwrap();
    assert_eq!(
        plan.package_sources()
            .map(|(g, n, v, e)| (format!("{g}/{n}@{v}"), e))
            .collect::<Vec<_>>(),
        captured
    );
    fs::write(&lock_path, old_bytes).unwrap();
    plan.resolution[0].name = original_name;
    assert_eq!(f.source.prepared.borrow().len(), prepared_count);
    assert_eq!(worktree(&f.project), before);
    vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
    let written = Lockfile::read(&lock_path).unwrap();
    assert_eq!(written.packages[0].registry.as_deref(), Some("fixture"));
    f.assert_provenance(f.source.prepared.borrow().last().unwrap());
    f.assert_no_reclone();
}

/// Snapshot worktree bytes independently of Git status; preparation may add
/// object/FETCH_HEAD metadata, but every other Git/worktree byte stays bound.
fn worktree(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap();
            let parts = relative
                .components()
                .map(|c| c.as_os_str())
                .collect::<Vec<_>>();
            if parts
                .windows(2)
                .any(|pair| pair[0] == ".git" && (pair[1] == "objects" || pair[1] == "FETCH_HEAD"))
            {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(path, path, &mut out);
    out
}
fn deferred_common_refusal(source_input: bool) {
    let f = Fixture::new();
    let old = f.publish("1.0.0", "");
    f.seed();
    let declaration = if source_input {
        "\n[requires.packages.'org.example/dep']\nversion='*'\nconfiguration={policy='require-selection'}\n"
    } else {
        "\n[configuration]\n"
    };
    f.publish("1.1.0", declaration);
    let before = worktree(&f.project);
    let config = fs::read(f.slot.join(".git/config")).unwrap();
    let error = f.plan().unwrap_err();
    assert!(
        if source_input {
            matches!(
                error,
                vibe_install::Error::ConfigurationInputUnsupported { .. }
            )
        } else {
            matches!(error, vibe_install::Error::ConfigurationUnsupported { .. })
        },
        "{error}"
    );
    assert_eq!(
        f.source.prepared.borrow().len(),
        1,
        "object preparation precedes actual target admission"
    );
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), old);
    assert_eq!(worktree(&f.project), before);
    assert_eq!(fs::read(f.slot.join(".git/config")).unwrap(), config);
    assert!(!f.slot.join("hook.log").exists());
    f.assert_no_reclone();
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn deferred_common_definition_refuses_without_reclone_or_mutating_existing_state() {
    deferred_common_refusal(false);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn deferred_original_input_refuses_without_reclone_or_mutating_existing_state() {
    deferred_common_refusal(true);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn selected_legacy_target_can_replace_an_old_common_definition() {
    let f = Fixture::new();
    f.publish("1.0.0", "\n[configuration]\n");
    f.seed();
    f.publish("1.1.0", "");
    let plan = f.ready();
    let applied = vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
    assert_eq!(applied.outcome.hook_reports.len(), 1);
    assert_eq!(applied.post_install_reports.len(), 1);
    assert!(
        Manifest::read(f.slot.join("vibe.toml"))
            .unwrap()
            .configuration
            .is_none()
    );
    f.assert_provenance(f.source.prepared.borrow().last().unwrap());
    f.assert_no_reclone();
}

#[path = "incremental_in_place/configured.rs"]
mod configured;
