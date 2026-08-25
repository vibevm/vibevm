//! Payload-diff scheduling oracles for PROP-020 hooks.

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::Manifest;
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{ContentHash, Group, PackageKind};

use super::super::test_helpers::{deps_rel, ver, write};
use super::super::{
    Materialised, PostInstallPlan, ResolvedDep, SlotCheck, SlotVerifier, materialise_resolution,
};
use super::run_post_install_with;
use crate::hooks::{HookInvocation, HookPolicy, HookRunner, InterpreterProbe};
use crate::vibedeps;

struct BashProbe;

impl InterpreterProbe for BashProbe {
    fn has(&self, program: &str) -> bool {
        program == "bash"
    }
}

#[derive(Default)]
struct CountingRunner(RefCell<Vec<String>>);

impl CountingRunner {
    fn phases(&self) -> Vec<String> {
        self.0.borrow().clone()
    }
}

impl HookRunner for CountingRunner {
    fn run(
        &self,
        _invocation: &HookInvocation,
        _cwd: &Path,
        env: &[(String, String)],
    ) -> Result<i32, String> {
        let phase = env
            .iter()
            .find(|(key, _)| key == "VIBE_HOOK_PHASE")
            .map(|(_, value)| value.clone())
            .expect("the hook contract always carries VIBE_HOOK_PHASE");
        self.0.borrow_mut().push(phase);
        Ok(0)
    }
}

#[derive(Default)]
struct NamedRunner(RefCell<Vec<String>>);

impl NamedRunner {
    fn invocations(&self) -> Vec<String> {
        self.0.borrow().clone()
    }
}

impl HookRunner for NamedRunner {
    fn run(
        &self,
        _invocation: &HookInvocation,
        _cwd: &Path,
        env: &[(String, String)],
    ) -> Result<i32, String> {
        let value = |key: &str| {
            env.iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(_, value)| value.as_str())
                .expect("the hook contract carries package and phase identity")
        };
        self.0.borrow_mut().push(format!(
            "{}:{}",
            value("VIBE_PACKAGE_NAME"),
            value("VIBE_HOOK_PHASE")
        ));
        Ok(0)
    }
}

#[derive(Default)]
struct PayloadMutatingRunner(RefCell<Vec<String>>);

impl PayloadMutatingRunner {
    fn invocations(&self) -> Vec<String> {
        self.0.borrow().clone()
    }
}

impl HookRunner for PayloadMutatingRunner {
    fn run(
        &self,
        _invocation: &HookInvocation,
        cwd: &Path,
        env: &[(String, String)],
    ) -> Result<i32, String> {
        let phase = env
            .iter()
            .find(|(key, _)| key == "VIBE_HOOK_PHASE")
            .map(|(_, value)| value.as_str())
            .expect("the hook contract carries its phase");
        let (relative, bytes) = match phase {
            "pre-install" => ("payload/changed.txt", "pre hook wrote\n"),
            "post-install" => ("payload/stable.txt", "post hook wrote\n"),
            other => return Err(format!("unexpected hook phase {other}")),
        };
        fs::write(cwd.join(relative), bytes).map_err(|error| error.to_string())?;
        self.0.borrow_mut().push(phase.to_string());
        Ok(0)
    }
}

struct Verified;

impl SlotVerifier for Verified {
    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        SlotCheck::Verified
    }
}

struct Diverged;

impl SlotVerifier for Diverged {
    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        SlotCheck::DivergedDetail {
            reason: "fixture payload drift".to_string(),
        }
    }
}

fn hash(digit: char) -> ContentHash {
    ContentHash::parse(&format!("sha256:{}", digit.to_string().repeat(64))).unwrap()
}

fn policy() -> HookPolicy {
    HookPolicy {
        allowed_groups: vec!["org.vibevm".to_string()],
        allow_hooks: false,
    }
}

fn hooked_dep() -> (ResolvedDep, TempDir) {
    hooked_dep_named("hooked")
}

fn hooked_dep_named(name: &str) -> (ResolvedDep, TempDir) {
    hooked_dep_named_with_mode(name, false)
}

fn hooked_dep_named_with_mode(name: &str, hardlink: bool) -> (ResolvedDep, TempDir) {
    let source = TempDir::new().unwrap();
    let materialization = if hardlink {
        "materialization = \"hardlink\"\n"
    } else {
        ""
    };
    write(
        source.path(),
        "vibe.toml",
        &format!(
            "[package]\n\
         group = \"org.vibevm\"\n\
         name = \"{name}\"\n\
         kind = \"flow\"\n\
         version = \"1.0.0\"\n\n\
         {materialization}\
         [hooks]\n\
         pre-install = \"hooks/prepare\"\n\
         post-install = \"hooks/finalise\"\n"
        ),
    );
    write(source.path(), "hooks/prepare.sh", "echo prepare\n");
    write(source.path(), "hooks/finalise.sh", "echo finalise\n");
    write(source.path(), "payload/changed.txt", "canonical\n");
    write(source.path(), "payload/stable.txt", "stable\n");
    let manifest = Manifest::read(source.path().join(Manifest::FILENAME)).unwrap();
    (
        ResolvedDep {
            kind: PackageKind::Flow,
            group: Group::parse("org.vibevm").unwrap(),
            name: name.to_string(),
            version: ver("1.0.0"),
            content_dir: source.path().to_path_buf(),
            source_hash: Some(hash('1')),
            manifest,
            requires: Vec::new(),
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        },
        source,
    )
}

fn slot_label() -> String {
    deps_rel("org.vibevm.hooked/1.0.0")
}

fn slot_path(workspace: &Path) -> std::path::PathBuf {
    vibedeps::slot_abs_path(
        workspace,
        &Group::parse("org.vibevm").unwrap(),
        "hooked",
        &ver("1.0.0"),
    )
}

fn seed_slot(workspace: &Path, dep: &ResolvedDep) {
    materialise_resolution(
        workspace,
        std::slice::from_ref(dep),
        SlotIntegrity::Verify,
        None,
        None,
        &BashProbe,
        &CountingRunner::default(),
    )
    .unwrap();
}

fn take_plan(workspace: &Path, outcome: &mut Materialised) -> Option<PostInstallPlan> {
    PostInstallPlan::new(workspace, std::mem::take(&mut outcome.post_install_deps))
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases",
    r = 1
)]
fn unchanged_verified_slot_runs_no_hook() {
    let workspace = TempDir::new().unwrap();
    let (dep, _source) = hooked_dep();
    seed_slot(workspace.path(), &dep);
    let runner = CountingRunner::default();

    let mut outcome = materialise_resolution(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&Verified),
        Some(&policy()),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert!(outcome.materialised.is_empty());
    assert_eq!(outcome.skipped, [slot_label()]);
    assert!(outcome.post_install_deps.is_empty());
    assert!(outcome.hook_reports.is_empty());
    assert!(take_plan(workspace.path(), &mut outcome).is_none());
    assert!(runner.phases().is_empty());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases",
    r = 1
)]
fn verify_heals_one_file_and_runs_each_hook_once() {
    let workspace = TempDir::new().unwrap();
    let (dep, _source) = hooked_dep();
    seed_slot(workspace.path(), &dep);
    let slot = slot_path(workspace.path());
    let stable = slot.join("payload/stable.txt");
    let stable_mtime = fs::metadata(&stable).unwrap().modified().unwrap();
    fs::write(slot.join("payload/changed.txt"), "drifted\n").unwrap();
    let runner = CountingRunner::default();

    let mut outcome = materialise_resolution(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&Diverged),
        Some(&policy()),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert_eq!(outcome.materialised, [slot_label()]);
    assert_eq!(outcome.integrity_warnings.len(), 1);
    assert_eq!(
        fs::read_to_string(slot.join("payload/changed.txt")).unwrap(),
        "canonical\n"
    );
    assert_eq!(
        fs::metadata(stable).unwrap().modified().unwrap(),
        stable_mtime
    );
    assert_eq!(runner.phases(), ["pre-install"]);

    let post = run_post_install_with(
        take_plan(workspace.path(), &mut outcome).unwrap(),
        &policy(),
        &BashProbe,
        &runner,
    )
    .unwrap();
    assert_eq!(post.len(), 1);
    assert_eq!(runner.phases(), ["pre-install", "post-install"]);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases",
    r = 1
)]
fn source_payload_change_runs_each_hook_once() {
    let workspace = TempDir::new().unwrap();
    let (mut dep, source) = hooked_dep();
    seed_slot(workspace.path(), &dep);
    write(source.path(), "payload/changed.txt", "source changed\n");
    dep.source_hash = Some(hash('2'));
    dep.source_mutable = true;
    let runner = CountingRunner::default();

    let mut outcome = materialise_resolution(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
        Some(&policy()),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert_eq!(outcome.materialised, [slot_label()]);
    assert_eq!(runner.phases(), ["pre-install"]);
    let post = run_post_install_with(
        take_plan(workspace.path(), &mut outcome).unwrap(),
        &policy(),
        &BashProbe,
        &runner,
    )
    .unwrap();
    assert_eq!(post.len(), 1);
    assert_eq!(runner.phases(), ["pre-install", "post-install"]);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases",
    r = 1
)]
fn two_dependency_plan_runs_only_the_exact_changed_subset() {
    let workspace = TempDir::new().unwrap();
    let (stable, _stable_source) = hooked_dep_named("stable");
    let (mut changed, changed_source) = hooked_dep_named("changed");
    seed_slot(workspace.path(), &stable);
    seed_slot(workspace.path(), &changed);
    write(
        changed_source.path(),
        "payload/changed.txt",
        "source changed\n",
    );
    changed.source_hash = Some(hash('2'));
    changed.source_mutable = true;
    let runner = NamedRunner::default();

    let mut outcome = materialise_resolution(
        workspace.path(),
        &[stable, changed],
        SlotIntegrity::TrustPresence,
        None,
        Some(&policy()),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert_eq!(runner.invocations(), ["changed:pre-install"]);
    let reports = run_post_install_with(
        take_plan(workspace.path(), &mut outcome).unwrap(),
        &policy(),
        &BashProbe,
        &runner,
    )
    .unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(
        runner.invocations(),
        ["changed:pre-install", "changed:post-install"]
    );
    assert!(take_plan(workspace.path(), &mut outcome).is_none());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#hardlink",
    r = 1
)]
fn hardlink_to_copy_transition_is_cow_before_pre_and_post_hooks() {
    let workspace = TempDir::new().unwrap();
    let (mut dep, source) = hooked_dep_named_with_mode("hardlinked", true);
    seed_slot(workspace.path(), &dep);
    let preserved_mtime = UNIX_EPOCH + Duration::from_secs(3_000_000);
    fs::File::options()
        .write(true)
        .open(source.path().join("hooks/finalise.sh"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(preserved_mtime))
        .unwrap();
    let manifest_path = source.path().join(Manifest::FILENAME);
    let manifest = fs::read_to_string(&manifest_path).unwrap().replace(
        "materialization = \"hardlink\"",
        "materialization = \"copy\"",
    );
    fs::write(&manifest_path, manifest).unwrap();
    dep.manifest = Manifest::read(&manifest_path).unwrap();
    dep.source_hash = Some(hash('5'));
    dep.source_mutable = true;
    let runner = PayloadMutatingRunner::default();

    let mut outcome = materialise_resolution(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        None,
        Some(&policy()),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert_eq!(runner.invocations(), ["pre-install"]);
    let slot = vibedeps::slot_abs_path(
        workspace.path(),
        &Group::parse("org.vibevm").unwrap(),
        "hardlinked",
        &ver("1.0.0"),
    );
    assert_eq!(
        fs::metadata(slot.join("hooks/finalise.sh"))
            .unwrap()
            .modified()
            .unwrap(),
        preserved_mtime,
        "copy-on-write detachment must preserve payload mtime"
    );
    assert_eq!(
        fs::read_to_string(source.path().join("payload/changed.txt")).unwrap(),
        "canonical\n",
        "a pre-install write must not reach the source/cache inode"
    );
    assert_eq!(
        fs::read_to_string(source.path().join("payload/stable.txt")).unwrap(),
        "stable\n"
    );

    let reports = run_post_install_with(
        take_plan(workspace.path(), &mut outcome).unwrap(),
        &policy(),
        &BashProbe,
        &runner,
    )
    .unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(runner.invocations(), ["pre-install", "post-install"]);
    assert_eq!(
        fs::read_to_string(source.path().join("payload/changed.txt")).unwrap(),
        "canonical\n"
    );
    assert_eq!(
        fs::read_to_string(source.path().join("payload/stable.txt")).unwrap(),
        "stable\n",
        "a post-install write must not reach the source/cache inode"
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#hardlink",
    r = 1
)]
fn post_hook_detaches_hardlinks_even_when_pre_hooks_were_not_run() {
    let workspace = TempDir::new().unwrap();
    let (dep, source) = hooked_dep_named_with_mode("post-only-run", true);
    let mut outcome = materialise_resolution(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        None,
        None,
        &BashProbe,
        &CountingRunner::default(),
    )
    .unwrap();
    let runner = PayloadMutatingRunner::default();

    run_post_install_with(
        take_plan(workspace.path(), &mut outcome).unwrap(),
        &policy(),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert_eq!(runner.invocations(), ["post-install"]);
    assert_eq!(
        fs::read_to_string(source.path().join("payload/stable.txt")).unwrap(),
        "stable\n",
        "post-install must establish its own copy-on-write boundary"
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases",
    r = 1
)]
fn source_payload_removal_runs_each_hook_once() {
    let workspace = TempDir::new().unwrap();
    let (mut dep, source) = hooked_dep();
    seed_slot(workspace.path(), &dep);
    fs::remove_file(source.path().join("payload/changed.txt")).unwrap();
    dep.source_hash = Some(hash('4'));
    dep.source_mutable = true;
    let runner = CountingRunner::default();

    let mut outcome = materialise_resolution(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
        Some(&policy()),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert_eq!(outcome.materialised, [slot_label()]);
    assert!(
        !slot_path(workspace.path())
            .join("payload/changed.txt")
            .exists()
    );
    assert_eq!(runner.phases(), ["pre-install"]);
    let post = run_post_install_with(
        take_plan(workspace.path(), &mut outcome).unwrap(),
        &policy(),
        &BashProbe,
        &runner,
    )
    .unwrap();
    assert_eq!(post.len(), 1);
    assert_eq!(runner.phases(), ["pre-install", "post-install"]);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases",
    r = 1
)]
fn identity_only_reconciliation_stays_materialised_but_runs_no_hook() {
    let workspace = TempDir::new().unwrap();
    let (mut dep, _source) = hooked_dep();
    seed_slot(workspace.path(), &dep);
    dep.source_hash = Some(hash('3'));
    dep.source_mutable = true;
    let runner = CountingRunner::default();

    let mut outcome = materialise_resolution(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
        Some(&policy()),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert_eq!(outcome.materialised, [slot_label()]);
    assert!(outcome.skipped.is_empty());
    assert!(outcome.post_install_deps.is_empty());
    assert!(outcome.hook_reports.is_empty());
    assert!(take_plan(workspace.path(), &mut outcome).is_none());
    assert!(runner.phases().is_empty());
}
