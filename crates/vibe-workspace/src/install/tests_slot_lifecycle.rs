use std::cell::RefCell;

use super::test_helpers::*;
use super::*;

#[derive(Default)]
struct RecordingLifecycle {
    events: RefCell<Vec<String>>,
    ready: RefCell<Vec<Vec<String>>>,
    fail_pre: bool,
}

impl SlotLifecycle for RecordingLifecycle {
    fn targets_ready(&self, targets: &[SlotLifecycleTarget]) -> Result<(), String> {
        self.ready
            .borrow_mut()
            .push(targets.iter().map(|target| target.name.clone()).collect());
        Ok(())
    }

    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        assert!(context.slot.is_dir(), "pre runs after slot materialisation");
        self.events
            .borrow_mut()
            .push(format!("pre:{}/{}", context.group, context.name));
        if self.fail_pre {
            Err("fixture pre failure".to_string())
        } else {
            Ok(())
        }
    }

    fn post_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        assert!(context.slot.is_dir(), "post runs against the durable slot");
        self.events
            .borrow_mut()
            .push(format!("post:{}/{}", context.group, context.name));
        Ok(())
    }
}

#[test]
fn identity_only_reconcile_publishes_an_empty_exact_event_set() {
    let workspace = tempfile::tempdir().unwrap();
    let (mut dep, _source) = dep_with_boot(
        "identity-only-callback",
        "1.0.0",
        "",
        "payload.txt",
        "same payload",
    );
    let initial = RecordingLifecycle::default();
    materialise_with_callback(workspace.path(), &dep, &initial).unwrap();
    dep.source_mutable = true;
    dep.source_hash = Some(vibe_core::ContentHash::from_validated(format!(
        "sha256:{}",
        "3".repeat(64)
    )));
    let callback = RecordingLifecycle::default();
    let outcome = materialise_with_callback(workspace.path(), &dep, &callback).unwrap();
    assert!(outcome.post_install_deps.is_empty());
    assert_eq!(callback.ready.borrow().as_slice(), [Vec::<String>::new()]);
    assert!(callback.events.borrow().is_empty());
}

fn materialise_with_callback(
    workspace: &Path,
    dep: &ResolvedDep,
    callback: &dyn SlotLifecycle,
) -> Result<Materialised, WorkspaceError> {
    materialise_resolution_with_spec_format(
        workspace,
        std::slice::from_ref(dep),
        MaterialiseOptions {
            slot_integrity: SlotIntegrity::TrustPresence,
            spec_format: SpecFormat::Mixed,
            slot_verifier: None,
            lifecycle: MaterialiseLifecycle::Callback(callback),
        },
    )
}

#[test]
fn callback_mode_runs_pre_and_defers_post_to_the_one_shot_plan() {
    let workspace = tempfile::tempdir().unwrap();
    let (dep, _source) = dep_with_boot(
        "callback-demo",
        "1.0.0",
        "[boot_snippet]\nsource = \"vibevm/vibespecs/boot/demo.xml\"\ncategory = \"flow\"\n",
        "vibevm/vibespecs/boot/demo.xml",
        "<spec><title id=\"root\">demo</title></spec>",
    );
    let callback = RecordingLifecycle::default();

    let mut materialised = materialise_with_callback(workspace.path(), &dep, &callback).unwrap();
    assert_eq!(
        callback.events.borrow().as_slice(),
        ["pre:org.vibevm/callback-demo"]
    );
    let plan = PostInstallPlan::new(
        workspace.path(),
        std::mem::take(&mut materialised.post_install_deps),
    )
    .expect("payload-changing install produces a post plan");

    run_post_install_slot_lifecycle(plan, SlotLifecycleMode::Callback(&callback)).unwrap();
    assert_eq!(
        callback.events.borrow().as_slice(),
        [
            "pre:org.vibevm/callback-demo",
            "post:org.vibevm/callback-demo"
        ]
    );
}

#[test]
fn callback_pre_failure_rolls_back_the_materialised_slot() {
    let workspace = tempfile::tempdir().unwrap();
    let (dep, _source) = dep_with_boot(
        "callback-fails",
        "1.0.0",
        "[boot_snippet]\nsource = \"vibevm/vibespecs/boot/demo.xml\"\ncategory = \"flow\"\n",
        "vibevm/vibespecs/boot/demo.xml",
        "<spec><title id=\"root\">demo</title></spec>",
    );
    let callback = RecordingLifecycle {
        fail_pre: true,
        ..RecordingLifecycle::default()
    };

    let error = materialise_with_callback(workspace.path(), &dep, &callback)
        .expect_err("pre callback must fail the install");
    assert!(matches!(error, WorkspaceError::SlotLifecycle { .. }));
    assert!(
        !workspace
            .path()
            .join(deps_rel("org.vibevm.callback-fails/1.0.0"))
            .exists(),
        "pre failure removes the slot"
    );
}

struct BarrierLifecycle {
    workspace: std::path::PathBuf,
    expected: Vec<&'static str>,
    fail_on: Option<&'static str>,
    events: RefCell<Vec<String>>,
}

impl SlotLifecycle for BarrierLifecycle {
    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        for name in &self.expected {
            assert!(
                self.workspace
                    .join(deps_rel(format!("org.vibevm.{name}/1.0.0")))
                    .is_dir(),
                "the callback barrier must place `{name}` before the first pre event"
            );
        }
        self.events.borrow_mut().push(context.name.to_string());
        if self.fail_on == Some(context.name) {
            Err("barrier fixture failure".into())
        } else {
            Ok(())
        }
    }

    fn post_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        self.events
            .borrow_mut()
            .push(format!("post:{}", context.name));
        Ok(())
    }
}

fn barrier_dep(name: &str) -> (ResolvedDep, tempfile::TempDir) {
    dep_with_boot(
        name,
        "1.0.0",
        "[boot_snippet]\nsource = \"vibevm/vibespecs/boot/demo.xml\"\ncategory = \"flow\"\n",
        "vibevm/vibespecs/boot/demo.xml",
        "<spec><title id=\"root\">demo</title></spec>",
    )
}

#[test]
fn callback_pre_plan_waits_for_all_slots_and_preserves_target_order() {
    let workspace = tempfile::tempdir().unwrap();
    let (target, _target_source) = barrier_dep("a-target");
    let (provider, _provider_source) = barrier_dep("z-provider");
    let callback = BarrierLifecycle {
        workspace: workspace.path().to_path_buf(),
        expected: vec!["a-target", "z-provider"],
        fail_on: None,
        events: RefCell::new(Vec::new()),
    };

    materialise_resolution_with_spec_format(
        workspace.path(),
        &[target, provider],
        MaterialiseOptions {
            slot_integrity: SlotIntegrity::TrustPresence,
            spec_format: SpecFormat::Mixed,
            slot_verifier: None,
            lifecycle: MaterialiseLifecycle::Callback(&callback),
        },
    )
    .unwrap();

    assert_eq!(
        callback.events.borrow().as_slice(),
        ["a-target", "z-provider"]
    );
}

#[test]
fn deferred_pre_failure_rolls_back_only_its_target_and_aborts_the_plan() {
    let workspace = tempfile::tempdir().unwrap();
    let (target, _target_source) = barrier_dep("a-target");
    let (provider, _provider_source) = barrier_dep("z-provider");
    let callback = BarrierLifecycle {
        workspace: workspace.path().to_path_buf(),
        expected: vec!["a-target", "z-provider"],
        fail_on: Some("z-provider"),
        events: RefCell::new(Vec::new()),
    };

    materialise_resolution_with_spec_format(
        workspace.path(),
        &[target, provider],
        MaterialiseOptions {
            slot_integrity: SlotIntegrity::TrustPresence,
            spec_format: SpecFormat::Mixed,
            slot_verifier: None,
            lifecycle: MaterialiseLifecycle::Callback(&callback),
        },
    )
    .unwrap_err();

    assert!(
        workspace
            .path()
            .join(deps_rel("org.vibevm.a-target/1.0.0"))
            .is_dir()
    );
    assert!(
        !workspace
            .path()
            .join(deps_rel("org.vibevm.z-provider/1.0.0"))
            .exists()
    );
    assert_eq!(
        callback.events.borrow().as_slice(),
        ["a-target", "z-provider"]
    );
}
