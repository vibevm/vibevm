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

/// The same drive, over an explicit phase slice.
fn run_phases_over(root: &std::path::Path, phases: Vec<Phase>) -> PhaseOutcome {
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
