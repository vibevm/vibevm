//! Exact hook scheduling for already-placed in-place updates.

use std::cell::RefCell;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::manifest::Manifest;
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{ContentHash, Group, PackageKind};

use super::super::test_helpers::{deps_rel, ver, write};
use super::super::{PostInstallPlan, ResolvedDep, materialise_resolution};
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
            .unwrap();
        self.0.borrow_mut().push(phase);
        Ok(0)
    }
}

fn hash() -> ContentHash {
    ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap()
}

fn policy() -> HookPolicy {
    HookPolicy {
        allowed_groups: vec!["org.vibevm".to_string()],
        allow_hooks: false,
    }
}

fn already_placed(changed: bool) -> (TempDir, ResolvedDep) {
    let workspace = TempDir::new().unwrap();
    let group = Group::parse("org.vibevm").unwrap();
    let slot = vibedeps::in_place_slot_abs_path(workspace.path(), &group, "giant");
    write(&slot, ".git/HEAD", "ref: refs/heads/main\n");
    write(
        &slot,
        "vibe.toml",
        "[package]\n\
         group = \"org.vibevm\"\n\
         name = \"giant\"\n\
         kind = \"feat\"\n\
         version = \"1.0.0\"\n\
         materialization = \"in-place\"\n\n\
         [hooks]\n\
         pre-install = \"hooks/prepare\"\n\
         post-install = \"hooks/finalise\"\n",
    );
    write(&slot, "hooks/prepare.sh", "echo prepare\n");
    write(&slot, "hooks/finalise.sh", "echo finalise\n");
    let manifest = Manifest::read(slot.join(Manifest::FILENAME)).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Feat,
        group,
        name: "giant".to_string(),
        version: ver("1.0.0"),
        content_dir: slot,
        source_hash: Some(hash()),
        manifest,
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: Some(changed),
    };
    (workspace, dep)
}

#[test]
fn unchanged_already_placed_in_place_runs_no_hook() {
    let (workspace, dep) = already_placed(false);
    let runner = CountingRunner::default();
    let outcome = materialise_resolution(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        None,
        Some(&policy()),
        &BashProbe,
        &runner,
    )
    .unwrap();

    assert!(outcome.materialised.is_empty());
    assert_eq!(outcome.skipped, [deps_rel("org.vibevm.giant")]);
    assert!(outcome.post_install_deps.is_empty());
    assert!(runner.phases().is_empty());
}

#[test]
fn changed_already_placed_in_place_runs_each_hook_once() {
    let (workspace, dep) = already_placed(true);
    let runner = CountingRunner::default();
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

    assert_eq!(outcome.materialised, [deps_rel("org.vibevm.giant")]);
    assert_eq!(runner.phases(), ["pre-install"]);
    let plan = PostInstallPlan::new(
        workspace.path(),
        std::mem::take(&mut outcome.post_install_deps),
    )
    .unwrap();
    run_post_install_with(plan, &policy(), &BashProbe, &runner).unwrap();
    assert_eq!(runner.phases(), ["pre-install", "post-install"]);
}
