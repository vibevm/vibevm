//! The deploy executor's own laws: the ONE selection path, its two typed
//! refusals, the read-only planner and the receipt listing.

use specmark::verifies;
use vibe_core::manifest::{
    DeployTarget, ExtensionHandler, MechanismDecl, MechanismFreshness, MechanismRole,
    MechanismRoutes,
};
use vibe_extension_registry::collect_mechanisms;

use super::plan::plan_resolved;
use super::support::{Fixture, FixtureProvider, Witness, selected, selection, target};
use super::{
    DeployError, DeployExecution, DeploySelection, apply_selection, deploy_state_home,
    execute_deploy_targets, list_deployments, plan_deploy_targets, undeploy_targets,
};
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::package::support::{
    config as configured, empty_world, key, pin, registry, temp,
};
use crate::mechanism::{BUILTIN_VIBE_BIN_NAME, BUILTIN_VIBE_BIN_PIN, MechanismError};

/// One `[[deploy.target]]` with no provider pin, so §3.1 step 3 answers.
fn builtin_target(id: &str, config: Option<&str>) -> DeployTarget {
    DeployTarget {
        id: id.to_owned(),
        artifact: "helper.exe".to_owned(),
        mechanism: key("deploy:vibe-bin"),
        provider: None,
        depends_on: None,
        config: config.map(configured),
    }
}

/// R8-DEPLOY's `the_reserved_deploy_builtin_refuses_as_provider_not_landed`
/// pinned §7.0.2's staging: "the one deploy builtin row (`#vibe-bin`)
/// refuses as provider-not-landed — a typed refusal, never a stub."
///
/// §7.1.0 ruling 1 retires that staging — "the provider-not-landed arm
/// becomes the real one" — so the pin becomes its successor: the SAME
/// resolvable builtin target now PLANS, through the same one selection
/// path, and reports the two resources §7.1.0 ruling 2 names. The refusal
/// it replaced no longer exists as a variant, which is why this is an
/// evolution rather than a deletion: what the old test really guarded was
/// that the builtin row reaches a decision of the engine's own, and it
/// still does.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_deploy_builtin_row_now_plans_through_the_landed_provider() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [builtin_target(
        "local-helper",
        Some("command = \"vibe-helper\""),
    )];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);

    let reports = plan_deploy_targets(&execution).expect("the landed builtin plans");

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.mechanism, "deploy:vibe-bin");
    assert_eq!(report.provider, BUILTIN_VIBE_BIN_PIN);
    assert_eq!(report.via, "the shipped builtin default");
    assert!(report.planned, "an undeployed target is planned work");
    let owned: Vec<&str> = report
        .resources
        .iter()
        .map(|resource| resource.resource.as_str())
        .collect();
    let suffix = if cfg!(windows) { ".cmd" } else { "" };
    assert_eq!(
        owned,
        [
            format!("bin/vibe-helper{suffix}").as_str(),
            "bin/vibe-helper.current",
        ],
        "the launcher and the active-payload pointer, and NOT the payload",
    );
    assert!(
        report
            .resources
            .iter()
            .all(|resource| resource.change == "create" && resource.desired_digest.len() == 64),
    );
    // And a read-only planner still wrote nothing at all.
    assert!(
        !fixture.settings.path().join("bin").exists(),
        "a plan mutates no destination",
    );
    assert!(
        std::fs::read_dir(&state_home)
            .expect("the planner opened its state home to read receipts")
            .next()
            .is_none(),
        "and it recorded no deployment in it",
    );
}

/// The same row, reached through the same arm, refusing on its OWN law:
/// the vibe-bin config's one required member.
///
/// This is the other half of the evolved pin — what proves the arm
/// constructs the real provider rather than merely returning `Ok`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_deploy_builtin_row_refuses_a_target_that_names_no_command() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [builtin_target("local-helper", None)];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);

    let error = execute_deploy_targets(&execution).expect_err("no `command` is a refusal");

    let DeployError::Provider(MechanismError::Deploy(DeployProviderError::Config {
        target,
        member,
        ..
    })) = &error
    else {
        panic!("expected the vibe-bin config refusal, got: {error}");
    };
    assert_eq!(target, "local-helper");
    assert_eq!(member, "command");
    assert!(
        !fixture.settings.path().join("bin").exists(),
        "a refused target installs nothing",
    );
}

/// The reserved row's handler name and pin are the two spellings this
/// engine matches on — pinned so a rename in the registry cannot silently
/// stop the refusal above from firing.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_reserved_deploy_row_keeps_the_identity_the_executor_matches() {
    let registry = registry(&empty_world());
    let row = registry
        .builtin_default(&key("deploy:vibe-bin"))
        .expect("the engine ships the deploy row");
    assert_eq!(row.pin().to_string(), BUILTIN_VIBE_BIN_PIN);
    assert_eq!(
        row.handler(),
        &ExtensionHandler::Builtin {
            name: BUILTIN_VIBE_BIN_NAME.to_owned()
        },
    );
}

/// §7.0.2: "a non-builtin selection refuses by the unlanded transport's
/// name (the R8-CARGO law)". Routing is real, and the builtin did not run.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_routed_away_deploy_target_refuses_by_the_unlanded_transport() {
    let fixture = Fixture::new("helper-bytes");
    let declaration = MechanismDecl {
        id: "installer".into(),
        role: MechanismRole::Deploy,
        name: "vibe-bin".into(),
        handler: ExtensionHandler::Native {
            crate_dir: Some(std::path::PathBuf::from("crates/installer")),
            prebuilt: None,
        },
        protocol: 1,
        config_schema: std::path::PathBuf::from("schemas/deploy-v1.jtd.json"),
        freshness: MechanismFreshness::Provider,
    };
    let mut world = empty_world();
    world.installed.push(installed(declaration));
    let plane = collect_mechanisms(&world).expect("the fixture world collects");
    let mut routes = MechanismRoutes::default();
    routes.insert(
        key("deploy:vibe-bin"),
        pin("org.example/deployers#installer"),
    );
    let targets = [builtin_target("local-helper", None)];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = DeployExecution {
        project_root: fixture.project.path(),
        targets: &targets,
        selection: &chosen,
        registry: &plane,
        routes: &routes,
        state_home: &state_home,
        settings_root: fixture.settings.path(),
        project: "org.example/demo",
        package: None,
        created_at: "2026-08-30T12:00:00Z",
    };

    let error = execute_deploy_targets(&execution).expect_err("the transport is a later atom");

    let DeployError::TransportNotLanded { key, pin, kind } = &error else {
        panic!("expected the transport refusal, got: {error}");
    };
    assert_eq!(key, "deploy:vibe-bin");
    assert_eq!(pin, "org.example/deployers#installer");
    assert_eq!(kind, "native");
    assert!(
        error
            .to_string()
            .contains("NOT deployed by a builtin instead")
    );
}

/// One installed package declaring one deploy provider.
fn installed(declaration: MechanismDecl) -> crate::DependencyExtensionSource {
    use vibe_core::manifest::ExtensionsControl;
    use vibe_core::{ContentHash, Group, PackageKind, PackageName};
    let (group, name, hash) = match (
        Group::parse("org.example"),
        PackageName::parse("deployers"),
        ContentHash::parse("sha256:aa"),
    ) {
        (Ok(group), Ok(name), Ok(hash)) => (group, name, hash),
        _ => panic!("the fixture identity parses"),
    };
    crate::DependencyExtensionSource {
        provider: crate::DependencyProvider {
            id: crate::DependencyProviderId::new(group, name),
            root: std::path::PathBuf::from("vibedeps/deployers"),
            version: "1.0.0".into(),
            kind: PackageKind::Tool,
            content_hash: hash,
        },
        declarations: Vec::new(),
        controls: ExtensionsControl::default(),
        mechanisms: vec![declaration],
    }
}

/// A selection naming a target the manifest does not declare refuses by
/// name and lists what IS declared.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_selection_naming_an_undeclared_target_refuses() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [builtin_target("local-helper", None)];
    let chosen = selection("local", &["ghost"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    let error = execute_deploy_targets(&execution).expect_err("a ghost target refuses");

    let DeployError::UnknownTarget {
        profile,
        target,
        declared,
    } = &error
    else {
        panic!("expected the unknown-target refusal, got: {error}");
    };
    assert_eq!(profile, "local");
    assert_eq!(target, "ghost");
    assert_eq!(declared, "local-helper");
}

/// An empty selection deploys nothing and says so — the no-op that keeps
/// `vibe deploy` on a project with no deploy section byte-identical to
/// the historical run.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_empty_selection_deploys_nothing() {
    let fixture = Fixture::new("helper-bytes");
    let targets: [DeployTarget; 0] = [];
    let chosen = DeploySelection {
        profile: "local".to_owned(),
        targets: Vec::new(),
    };
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    assert!(
        execute_deploy_targets(&execution)
            .expect("an empty selection runs")
            .is_empty()
    );
    assert!(
        !state_home.exists(),
        "and it does not even create a state home",
    );
}

/// Dependency order, not authored order: a target that depends on another
/// is applied after it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_selection_is_applied_in_dependency_order() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("late", "helper.exe", &["early"]),
        target("early", "helper.exe", &[]),
    ];
    let chosen = selection("local", &["late", "early"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let resolved = vec![
        selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/late"],
            )),
        ),
        selected(
            &targets[1],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/early"],
            )),
        ),
    ];

    // The selection is resolved in profile order above; ordering is the
    // executor's own walk, so this drives the ordered half directly.
    let ordered = super::order(resolved).expect("the selection orders");
    let outcomes = apply_selection(&execution, &ordered).expect("both targets deploy");

    assert_eq!(
        outcomes
            .iter()
            .map(|outcome| outcome.target.as_str())
            .collect::<Vec<_>>(),
        ["early", "late"],
        "a dependency is deployed before its dependant",
    );
}

/// §7.0.6: `--plan` is a read-only planner. It calls `plan` and NOTHING
/// else, and it mutates no destination and no state.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn plan_mode_calls_only_the_plan_verb_and_mutates_nothing() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let witness = std::rc::Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));
    let resolved = vec![selected(&targets[0], Box::new(Witness(witness.clone())))];

    let reports = plan_resolved(&execution, &resolved).expect("the planner runs");

    assert_eq!(witness.calls(), ["plan"], "only the plan verb ran");
    assert_eq!(reports.len(), 1);
    assert!(reports[0].planned, "an undeployed target is planned work");
    assert_eq!(reports[0].resources.len(), 1);
    assert_eq!(reports[0].resources[0].change, "create");
    assert!(reports[0].resources[0].recorded_digest.is_none());
    assert!(
        !fixture.destination.path().join("bin/helper").exists(),
        "a plan mutates no destination",
    );
    assert!(
        std::fs::read_dir(&state_home)
            .expect("the state home is readable")
            .next()
            .is_none(),
        "and it writes no deployment state",
    );
}

/// A target whose artifact has no record yet is reported as planned work
/// rather than refused — §7's "it reports preceding stale targets as
/// planned work", and a read-only planner never builds to find out.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_target_whose_artifact_is_unbuilt_is_planned_work_not_a_refusal() {
    let project = temp();
    let settings = temp();
    let destination = temp();
    let plane = registry(&empty_world());
    let routes = MechanismRoutes::default();
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = deploy_state_home(settings.path());
    let execution = DeployExecution {
        project_root: project.path(),
        targets: &targets,
        selection: &chosen,
        registry: &plane,
        routes: &routes,
        state_home: &state_home,
        settings_root: settings.path(),
        project: "org.example/demo",
        package: None,
        created_at: "2026-08-30T12:00:00Z",
    };
    let resolved = vec![selected(
        &targets[0],
        Box::new(FixtureProvider::new(destination.path(), &["bin/helper"])),
    )];

    let reports = plan_resolved(&execution, &resolved).expect("the planner runs");

    assert!(reports[0].planned);
    assert!(
        reports[0].reason.contains("has no record yet"),
        "{:?}",
        reports[0]
    );
    assert!(reports[0].resources.is_empty());
}

/// A target already deployed at the plan's own digests is NOT planned
/// work, and a target that depends on a stale one is.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_fresh_target_is_not_planned_and_a_dependant_of_a_stale_one_is() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("early", "helper.exe", &[]),
        target("late", "helper.exe", &["early"]),
    ];
    let chosen = selection("local", &["early", "late"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    // Deploy only the second target, so the first stays stale.
    let deployed = vec![selected(
        &targets[1],
        Box::new(FixtureProvider::new(
            fixture.destination.path(),
            &["bin/late"],
        )),
    )];
    apply_selection(&execution, &deployed).expect("the second target deploys");

    let resolved = vec![
        selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/early"],
            )),
        ),
        selected(
            &targets[1],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/late"],
            )),
        ),
    ];
    let reports = plan_resolved(&execution, &resolved).expect("the planner runs");

    assert!(reports[0].planned, "the undeployed target is stale");
    assert!(
        reports[1].planned,
        "and its dependant is planned work because a preceding target is stale",
    );
    assert!(
        reports[1].reason.contains("preceding target"),
        "{:?}",
        reports[1],
    );
    assert_eq!(reports[1].resources[0].change, "unchanged");
}

/// `vibe deployments` reads the state home and reports receipt facts.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_listing_reports_receipt_facts_and_no_secret() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    apply_selection(&execution, &[selected(&targets[0], Box::new(provider))])
        .expect("the deployment applies");

    let rows = list_deployments(&state_home).expect("the state home lists");

    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.project, "org.example/demo");
    assert_eq!(row.package, None);
    assert_eq!(row.profile, "local");
    assert_eq!(row.target, "local-helper");
    assert_eq!(row.generation, 0);
    assert_eq!(row.status.as_str(), "verified");
    assert_eq!(row.scope, "user");
    assert_eq!(row.provider, super::support::FIXTURE_PIN);
    assert_eq!(row.resources, 1);
    assert!(row.reversible);
    assert!(row.finalized_at.is_some());
    assert_eq!(row.deployment.len(), 64, "the engine-owned deployment id");
}

/// An empty state home lists nothing rather than refusing.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_untouched_machine_lists_no_deployments() {
    let settings = temp();
    let rows = list_deployments(&deploy_state_home(settings.path()))
        .expect("an untouched machine still answers");
    assert!(rows.is_empty());
}

/// §7.1.0 ruling 4's undeploy clause through the ENGINE's own path: after
/// an UPDATE (a pointer-moving second generation), `undeploy` removes the
/// launcher and the pointer. It does not "restore" a generation nobody
/// asked for — restoration is the SAGA's inverse, and the engine says
/// which of the two it is performing by what it hands the provider.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn undeploy_after_a_pointer_moving_update_removes_both_owned_files() {
    let fixture = Fixture::new("original-bytes");
    let targets = [builtin_target(
        "local-helper",
        Some("command = \"vibe-helper\""),
    )];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    execute_deploy_targets(&execution).expect("generation 0 deploys");
    fixture.rebuild("updated-bytes");
    execute_deploy_targets(&execution).expect("generation 1 moves the pointer");
    let suffix = if cfg!(windows) { ".cmd" } else { "" };
    let launcher = fixture
        .settings
        .path()
        .join("bin")
        .join(format!("vibe-helper{suffix}"));
    let pointer = fixture
        .settings
        .path()
        .join("bin")
        .join("vibe-helper.current");
    assert!(launcher.is_file(), "the update left a live launcher");
    assert!(pointer.is_file(), "and a live pointer");

    undeploy_targets(&execution).expect("the undeploy runs");

    assert!(
        !launcher.exists(),
        "undeploy REMOVES the launcher — it does not restore a prior generation",
    );
    assert!(!pointer.exists(), "and removes the pointer with it");
}

/// §7.2's undeploy drift refusal COMPOSED with the real provider: a
/// pointer edited by hand after deployment is a changed path, and the
/// engine's drift law must see it through `vibe-bin`'s own `verify`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn undeploy_refuses_a_hand_edited_pointer_through_the_real_provider() {
    let fixture = Fixture::new("original-bytes");
    let targets = [builtin_target(
        "local-helper",
        Some("command = \"vibe-helper\""),
    )];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    execute_deploy_targets(&execution).expect("the deployment applies");
    let pointer = fixture
        .settings
        .path()
        .join("bin")
        .join("vibe-helper.current");
    std::fs::write(
        &pointer,
        format!(
            "{}
",
            "ab".repeat(32)
        ),
    )
    .expect("a human edits the pointer by hand");

    let error = undeploy_targets(&execution).expect_err("drift refuses the removal");
    let DeployError::UndeployDrift { target, resources } = &error else {
        panic!("expected the drift refusal, got: {error}");
    };
    assert_eq!(target, "local-helper");
    assert!(
        resources.contains("vibe-helper.current"),
        "the changed pointer is named: {resources}",
    );
}
