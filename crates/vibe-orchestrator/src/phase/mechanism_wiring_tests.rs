//! The §6.0.2 wiring pins — the mechanism fences inside the ONE contribution
//! walk.
//!
//! Its own cell because it is its own responsibility: the suite next door
//! owns the resume-failure mapping and the validate-only lease gate, and this
//! owns the phase line's own edges. The harness it drives — `Silent` and
//! `manifested` — is the one next door's, borrowed rather than copied.
use std::sync::Arc;

use vibe_lifecycle::{AgentBackend, Phase, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use super::validate_only_gate::{Silent, manifested};
use crate::failure::Measurement;
use crate::install::{InstallInputs, InstallPolicy, SelectedManifest, resolve_project_root};
use crate::ports::NoManifestMutation;
use crate::{PhaseOutcome, PhaseRun, run_phases};

/// One manifest that declares a static skill and nothing else.
const WITH_PACKAGE_TARGET: &str = concat!(
    "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
    "[[artifacts.package]]\nid = \"demo\"\nmechanism = \"package:static-skill\"\n",
    "outputs = [{ id = \"demo.md\", kind = \"file\" }]\n",
    "config = { source = \"skills/demo\" }\n",
);

/// Drive the executed region over one prepared project for exactly the
/// build and package phases.
///
/// The phase slice is narrowed deliberately: the inclusive chain would
/// enter the prerequisite install and its registry epoch, which this pin
/// is not about, and `run_phases` executes the phases it is GIVEN. What
/// is exercised is exactly the region the wiring lives in.
fn run(root: &std::path::Path) -> PhaseOutcome {
    run_phases_over(root, vec![Phase::Build, Phase::Package])
}

/// The same drive, over an explicit phase slice and no deploy selection.
fn run_phases_over(root: &std::path::Path, phases: Vec<Phase>) -> PhaseOutcome {
    run_deploying(root, phases, None)
}

/// The same drive again, carrying a resolved deploy-profile selection —
/// §7.0.5's "travels as data", from the position a command layer would
/// hand it in at.
fn run_deploying(
    root: &std::path::Path,
    phases: Vec<Phase>,
    deploy: Option<vibe_lifecycle::DeploySelection>,
) -> PhaseOutcome {
    let root = match resolve_project_root(root) {
        Ok(root) => root,
        Err(error) => panic!("the fixture root resolves: {error}"),
    };
    let lease = match vibe_lifecycle::LifecycleLease::acquire(&root) {
        Ok(lease) => Arc::new(lease),
        Err(error) => panic!("the fixture leases: {error}"),
    };
    let selection = SelectedManifest::read(&root).prepare();
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);
    // The chain is DERIVED from the phases, because the fences rank a
    // phase's position in the chain the run was asked for: a fixture whose
    // chain and phase slice disagreed would arm a fence the walk never
    // reaches, and prove nothing.
    let requested = phases.last().copied().unwrap_or(Phase::Package);
    let chain: Vec<String> = phases.iter().map(|phase| phase.to_string()).collect();
    let metadata = RunMetadata {
        requested: requested.as_str().to_string(),
        chain: chain.clone(),
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: match vibe_lifecycle::process::allocate_run_id(&root) {
            Ok(id) => id,
            Err(error) => panic!("the fixture run id allocates: {error}"),
        },
        started: vibe_core::timestamp::now_utc(),
        selected: ".".to_string(),
    };
    let silent = Silent;
    run_phases(PhaseRun {
        requested,
        phases,
        chain,
        metadata,
        install_args: InstallInputs::default(),
        policy: InstallPolicy {
            offline: true,
            ..InstallPolicy::default()
        },
        lease,
        selection,
        steps: Vec::new(),
        contributions: Vec::new(),
        notices: Vec::new(),
        observer: &silent,
        install_observer: &silent,
        confirm_gate: &silent,
        sources: &silent,
        environment: &silent,
        manifest_mutation: &NoManifestMutation,
        agent,
        trace: None,
        deploy,
        observed_at: match "2026-08-30T12:00:05Z".parse() {
            Ok(instant) => instant,
            Err(error) => panic!("the fixture instant parses: {error}"),
        },
    })
}

/// A manifest whose `phase:generate` and `phase:build` contributions
/// bracket a build target that always refuses.
///
/// `workdir` names a directory that does not exist, so the builtin's
/// `fingerprint` cannot even spawn the toolchain there. The refusal is
/// deterministic, costs no compile, and — crucially — is the BUILTIN's
/// own, so reaching it proves the mechanism executor really ran.
const GENERATE_THEN_BUILD: &str = concat!(
    "[project]
name = \"demo\"
version = \"0.1.0\"

",
    "[[extension]]
id = \"gen\"
point = \"phase:generate\"
",
    "handler = { kind = \"builtin\", name = \"log\" }
",
    "config = { message = \"GENERATED\" }

",
    "[[extension]]
id = \"bld\"
point = \"phase:build\"
",
    "handler = { kind = \"builtin\", name = \"log\" }
",
    "config = { message = \"BUILT\" }

",
    "[[artifacts.build]]
id = \"tool\"
mechanism = \"build:cargo\"
",
    "workdir = \"never-generated\"
",
    "outputs = [{ id = \"tool.exe\", kind = \"executable\" }]
",
);

/// The same shape one phase later: a package target that always refuses,
/// bracketed by a `phase:build` and a `phase:package` contribution.
const BUILD_THEN_PACKAGE: &str = concat!(
    "[project]
name = \"demo\"
version = \"0.1.0\"

",
    "[[extension]]
id = \"bld\"
point = \"phase:build\"
",
    "handler = { kind = \"builtin\", name = \"log\" }
",
    "config = { message = \"BUILT\" }

",
    "[[extension]]
id = \"pkg\"
point = \"phase:package\"
",
    "handler = { kind = \"builtin\", name = \"log\" }
",
    "config = { message = \"PACKAGED\" }

",
    "[[artifacts.package]]
id = \"demo\"
mechanism = \"package:static-skill\"
",
    "outputs = [{ id = \"demo.md\", kind = \"file\" }]
",
    "config = { source = \"never-generated\" }
",
);

/// The phases whose contributions this run had really dispatched when it
/// stopped, in order.
fn measured_phases(outcome: PhaseOutcome) -> Vec<String> {
    let PhaseOutcome::Failed {
        measurement,
        original,
        ..
    } = outcome
    else {
        panic!("a refusing mechanism target stops the run");
    };
    // The refusal is the BUILTIN provider's own, which is what proves the
    // executor really ran rather than something upstream of it.
    let rendered = format!("{original:#}");
    assert!(
        rendered.contains("PROP-054#ONE-MACHINE"),
        "the mechanism provider refused: {rendered}",
    );
    match measurement {
        Measurement::Lifecycle { rows, .. } => rows.into_iter().map(|row| row.phase).collect(),
        other => panic!("a dispatch failure is lifecycle-shaped, got {other:?}"),
    }
}

#[test]
fn every_generate_contribution_is_dispatched_before_the_mechanism_build() {
    let dir = manifested(GENERATE_THEN_BUILD);

    let phases = measured_phases(run_phases_over(
        dir.path(),
        vec![Phase::Generate, Phase::Build],
    ));

    // §2's primary law: `generate` owns deterministic derived source and
    // `build` produces artifacts FROM it, so the generate contribution
    // must already have run when the mechanism build fires.
    assert!(
        phases.iter().any(|phase| phase == "generate"),
        "the generate contribution ran before the build fence: {phases:?}",
    );
    // And the in-phase position: the fence fires BEFORE its own phase's
    // contributions, exactly as the verify boundary does.
    assert!(
        !phases.iter().any(|phase| phase == "build"),
        "the build fence fired before the build contributions: {phases:?}",
    );
}

#[test]
fn the_package_fence_fires_after_build_contributions_and_before_package_ones() {
    let dir = manifested(BUILD_THEN_PACKAGE);

    let phases = measured_phases(run_phases_over(
        dir.path(),
        vec![Phase::Build, Phase::Package],
    ));

    assert!(
        phases.iter().any(|phase| phase == "build"),
        "every earlier phase's contributions ran first: {phases:?}",
    );
    assert!(
        !phases.iter().any(|phase| phase == "package"),
        "the package fence fired before the package contributions: {phases:?}",
    );
}

#[test]
fn a_manifest_with_no_artifact_targets_moves_no_mechanism_bytes() {
    let dir = manifested("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n");

    let outcome = run(dir.path());

    let PhaseOutcome::Completed(values) = outcome else {
        panic!("a ritual with nothing to do completes");
    };
    assert!(values.ok);
    assert!(
        values.steps.iter().all(|step| step.status == "no-op"),
        "the historical ritual is unchanged: {:?}",
        values.steps,
    );
    assert!(
        values.contributions.is_empty(),
        "and it contributed nothing: {:?}",
        values.contributions,
    );
    assert!(
        !dir.path().join("target").exists(),
        "no engine-owned build or package root was created",
    );
    assert!(
        !dir.path().join(".vibe/state/artifacts").exists(),
        "and no artifact record was written",
    );
}

/// The other half of the no-op law, and the one the first test cannot
/// reach: an `[artifacts]` section that EXISTS but declares no package
/// target. Here the package arm really is walked and really does call
/// `execute_package_targets` — with an empty slice — so anything the
/// wiring touches unconditionally shows up as a byte that moved.
#[test]
fn an_artifacts_section_with_no_package_target_still_moves_no_bytes() {
    let dir = manifested(concat!(
        "[project]
name = \"demo\"
version = \"0.1.0\"

",
        "[[artifacts.build]]
id = \"helper\"
mechanism = \"build:cargo\"
",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]
",
    ));

    // The PACKAGE phase only: the build arm is not walked, so no Cargo
    // process runs and the package arm is reached with an empty family.
    let outcome = run_phases_over(dir.path(), vec![Phase::Package]);

    let PhaseOutcome::Completed(values) = outcome else {
        panic!("an empty package family completes");
    };
    assert!(values.ok);
    assert!(
        !dir.path().join("target").exists(),
        "the engine-owned package root is created only for a real target",
    );
    assert!(
        !dir.path().join(".vibe/state/artifacts").exists(),
        "and no artifact record was written",
    );
}

#[test]
fn a_declared_package_target_really_runs_through_the_wiring() {
    let dir = manifested(WITH_PACKAGE_TARGET);
    std::fs::create_dir_all(dir.path().join("skills/demo")).unwrap();
    std::fs::write(
        dir.path().join("skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: A skill packaged through the phase wiring.\n---\n\nB.\n",
    )
    .unwrap();

    let outcome = run(dir.path());

    let PhaseOutcome::Completed(values) = outcome else {
        panic!("the declared package target executes");
    };
    assert!(values.ok);
    let distributable = dir.path().join("target/vibe-package/demo/SKILL.md");
    let document = std::fs::read_to_string(&distributable)
        .unwrap_or_else(|error| panic!("the distributable exists: {error}"));
    assert!(document.starts_with("---\nname: demo\n"), "{document}");
    assert!(
        dir.path()
            .join(".vibe/state/artifacts/demo.md.json")
            .is_file(),
        "and the engine wrote its A2 record beside the build records",
    );
}

/// The same shape one phase further: a deploy target that always refuses,
/// bracketed by a `phase:package` and a `phase:deploy` contribution.
///
/// The refusal is the ENGINE's own — §7.0.2's provider-not-landed, since
/// `deploy:vibe-bin` is collected, routable and deliberately unimplemented
/// — so reaching it proves the deploy executor really ran.
const PACKAGE_THEN_DEPLOY: &str = concat!(
    "[project]
name = \"demo\"
version = \"0.1.0\"

",
    "[[extension]]
id = \"pkg\"
point = \"phase:package\"
",
    "handler = { kind = \"builtin\", name = \"log\" }
",
    "config = { message = \"PACKAGED\" }

",
    "[[extension]]
id = \"dep\"
point = \"phase:deploy\"
",
    "handler = { kind = \"builtin\", name = \"log\" }
",
    "config = { message = \"DEPLOYED\" }

",
    "[[artifacts.build]]
id = \"tool\"
mechanism = \"build:cargo\"
",
    "outputs = [{ id = \"tool.exe\", kind = \"executable\" }]

",
    "[[deploy.target]]
id = \"local\"
artifact = \"tool.exe\"
",
    "mechanism = \"deploy:vibe-bin\"

",
    "[deploy.profiles.local]
targets = [\"local\"]
",
);

/// The resolved selection the command layer would hand down.
fn local_profile() -> vibe_lifecycle::DeploySelection {
    vibe_lifecycle::DeploySelection {
        profile: "local".to_string(),
        targets: vec!["local".to_string()],
    }
}

/// The phases whose contributions a DEPLOY-refusing run had really
/// dispatched, plus the rendered refusal.
fn measured_deploy(outcome: PhaseOutcome) -> Vec<String> {
    let PhaseOutcome::Failed {
        measurement,
        original,
        ..
    } = outcome
    else {
        panic!("a refusing deploy target stops the run");
    };
    let rendered = format!("{original:#}");
    assert!(
        rendered.contains("PROP-054#OPEN-DEPLOY-TARGETS"),
        "the deploy engine refused: {rendered}",
    );
    assert!(
        rendered.contains("R8-VIBE-BIN"),
        "and it named the atom that lands the provider: {rendered}",
    );
    match measurement {
        Measurement::Lifecycle { rows, .. } => rows.into_iter().map(|row| row.phase).collect(),
        other => panic!("a dispatch failure is lifecycle-shaped, got {other:?}"),
    }
}

#[test]
fn the_deploy_fence_fires_after_package_contributions_and_before_deploy_ones() {
    let dir = manifested(PACKAGE_THEN_DEPLOY);

    let phases = measured_deploy(run_deploying(
        dir.path(),
        vec![Phase::Package, Phase::Deploy],
        Some(local_profile()),
    ));

    assert!(
        phases.iter().any(|phase| phase == "package"),
        "every earlier phase's contributions ran first: {phases:?}",
    );
    assert!(
        !phases.iter().any(|phase| phase == "deploy"),
        "the deploy fence fired before the deploy contributions: {phases:?}",
    );
}

/// The verify edge of the same walk: a `phase:verify` contribution is
/// dispatched BEFORE the deploy fence, exactly as §2's phase line orders
/// build → verify → package → deploy.
#[test]
fn every_verify_contribution_is_dispatched_before_the_mechanism_deploy() {
    let dir = manifested(
        &PACKAGE_THEN_DEPLOY.replace("point = \"phase:package\"", "point = \"phase:verify\""),
    );

    let phases = measured_deploy(run_deploying(
        dir.path(),
        vec![Phase::Verify, Phase::Package, Phase::Deploy],
        Some(local_profile()),
    ));

    assert!(
        phases.iter().any(|phase| phase == "verify"),
        "the verify contribution ran before the deploy fence: {phases:?}",
    );
    assert!(
        !phases.iter().any(|phase| phase == "deploy"),
        "and the deploy fence still fired before the deploy contributions: {phases:?}",
    );
}

/// §7.0.5's arming law, in the direction that keeps every existing
/// `vibe deploy` working: a dispatch that carries NO resolved selection
/// arms no deploy fence, whatever the chain says.
#[test]
fn a_dispatch_with_no_resolved_selection_arms_no_deploy_fence() {
    let dir = manifested(PACKAGE_THEN_DEPLOY);

    let outcome = run_deploying(dir.path(), vec![Phase::Package, Phase::Deploy], None);

    let PhaseOutcome::Completed(values) = outcome else {
        panic!("without a selection the deploy fence arms nothing and the run completes");
    };
    assert!(values.ok);
    assert!(
        values.contributions.iter().any(|row| row.phase == "deploy"),
        "and the ordinary `phase:deploy` contributions still ran: {:?}",
        values.contributions,
    );
}

/// The deployment state home the carriage resolves is the ISOLATED one —
/// the operator's real `~/.vibe` is unreachable from this suite, and the
/// assertion says so rather than assuming it.
#[test]
fn the_deployment_state_home_resolves_inside_the_isolated_settings_dir() {
    let home = vibe_test_support::isolated_home().expect("the test process is isolated");
    let settings = vibe_core::settings::settings_dir().expect("an isolated settings dir");
    assert!(
        settings.starts_with(home),
        "`{}` must sit inside the isolated home `{}`",
        settings.display(),
        home.display(),
    );
    let state = vibe_lifecycle::deploy_state_home(&settings);
    assert!(state.starts_with(home));
    assert!(state.ends_with("deployments"));
}

/// §7.0.7's LOWERING CALL SITE, the one R8-PACKAGE named so it could not
/// be forgotten: a package that declares only a legacy `[[binary]]` and no
/// `[[artifacts.build]]` reaches the build executor anyway.
///
/// `crate` names a directory that does not exist, so the builtin Cargo
/// adapter refuses deterministically and at no compile cost — and the
/// refusal is the BUILTIN's own, which is what proves the lowered row
/// really became a live build target rather than being ignored.
const BINARY_ONLY: &str = concat!(
    "[package]
group = \"org.example\"
name = \"legacy-tools\"
kind = \"tool\"
version = \"0.1.0\"
authors = [\"Fixture\"]
license = \"EULA\"
description = \"fixture\"
keywords = [\"fixture\"]

",
    "[[binary]]
name = \"tool\"
crate = \"never-generated\"
",
);

#[test]
fn a_legacy_binary_row_reaches_the_build_executor_through_the_lowering() {
    let dir = manifested(BINARY_ONLY);

    let outcome = run_phases_over(dir.path(), vec![Phase::Build]);

    let PhaseOutcome::Failed { original, .. } = outcome else {
        panic!("the lowered target really runs, and the builtin refuses it");
    };
    let rendered = format!("{original:#}");
    assert!(
        rendered.contains("[[artifacts.build]] `tool`"),
        "the legacy row IS a build target after lowering: {rendered}",
    );
    assert!(
        rendered.contains("PROP-054#ONE-MACHINE"),
        "and the refusal is the builtin provider's own: {rendered}",
    );
}
