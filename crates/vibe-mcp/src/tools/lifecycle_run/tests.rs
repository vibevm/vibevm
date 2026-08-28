use vibe_lifecycle::process::StreamMode;
use vibe_orchestrator::failure::Measurement;
use vibe_orchestrator::ports::{InstallObserver, RunObserver};

use super::ports::{HostedInstallObserver, HostedRunObserver};
use super::{LifecycleRunMcpTool, absorb_owner_notices, failure_values, parse_phase};
use crate::tools::McpTool;

#[test]
fn descriptor_and_runtime_share_the_closed_phase_vocabulary() {
    let descriptor = LifecycleRunMcpTool.descriptor();
    let advertised = descriptor.input_schema["properties"]["phase"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        advertised,
        vibe_lifecycle::DEFAULT_PHASES
            .iter()
            .map(|phase| phase.as_str())
            .collect::<Vec<_>>()
    );
    for phase in advertised {
        assert_eq!(
            parse_phase(&serde_json::json!({ "phase": phase }))
                .unwrap()
                .as_str(),
            phase
        );
    }
}

#[test]
fn hosted_ports_capture_and_never_claim_an_immediate_machine_failure() {
    let run = HostedRunObserver;
    let install = HostedInstallObserver;
    assert_eq!(run.stream_mode(), StreamMode::Capture);
    assert!(run.binary_quiet());
    assert!(!run.emit_machine_failure());
    assert_eq!(install.stream_mode(), StreamMode::Capture);
    assert!(!install.emit_machine_failure());
}

#[test]
fn registry_seed_is_best_effort_and_precedes_the_fallible_load() {
    let source = include_str!("ports.rs");
    let seed = source
        .find("let _ = vibe_core::ensure_default_global_registry();")
        .expect("the optional default is attempted without becoming a veto");
    let load = source
        .find("vibe_core::GlobalRegistryConfig::load()?")
        .expect("the real config load remains fallible");
    assert!(seed < load, "seed-before-load is the registry epoch law");
}

#[test]
fn lifecycle_lease_owner_survives_execution_trace_serialization_and_tool_output() {
    let source = include_str!("../lifecycle_run.rs");
    let retain = source.find("retain_lease()").unwrap();
    let run = source.find("prepared.run(").unwrap();
    let finalize = source.find("finalize(").unwrap();
    let serialize = source.find("serde_json::to_value").unwrap();
    let output = source
        .find("let output = match finalized.original_error")
        .unwrap();
    let drop_owner = source.find("drop(lease_owner)").unwrap();
    assert!(retain < run);
    assert!(run < finalize);
    assert!(finalize < serialize);
    assert!(serialize < output);
    assert!(
        output < drop_owner,
        "the lease is released only after ToolOutput exists"
    );
}

#[test]
fn lifecycle_measurement_keeps_its_own_failure_identity_and_rows() {
    let metadata = vibe_lifecycle::RunMetadata {
        requested: "deploy".into(),
        chain: vec!["validate".into(), "deploy".into()],
        offline: true,
        assume_yes: true,
        agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Agent,
        force: false,
        trace_compile: false,
        run_id: "0".repeat(32),
        started: "2026-08-28T00:00:00Z".into(),
        selected: ".".into(),
    };
    let values = failure_values(
        Measurement::Lifecycle {
            rows: Vec::new(),
            stopped_phase: "build".into(),
            requested: "build".into(),
            chain: vec!["validate".into(), "build".into()],
        },
        &metadata,
    );
    assert_eq!(values.requested, "build");
    assert_eq!(values.chain, ["validate", "build"]);
    assert_eq!(values.steps.len(), 1);
    assert_eq!(values.steps[0].phase, "build");
}

#[test]
fn owner_notices_use_the_root_only_when_no_trace_member_carries_them() {
    let mut without = vibe_orchestrator::values::LifecycleValues::failed(
        "build",
        vec!["build".into()],
        "build",
        Vec::new(),
    );
    absorb_owner_notices(&mut without, false, ["owner notice".to_string()]);
    assert_eq!(without.notices, ["owner notice"]);

    let mut with = vibe_orchestrator::values::LifecycleValues::failed(
        "build",
        vec!["build".into()],
        "build",
        Vec::new(),
    );
    absorb_owner_notices(&mut with, true, ["owner notice".to_string()]);
    assert!(with.notices.is_empty(), "the trace warnings own this copy");
}

#[test]
fn lifecycle_run_source_and_dependencies_have_no_paid_or_surface_backedge() {
    let production = [
        include_str!("../lifecycle_run.rs"),
        include_str!("ports.rs"),
    ]
    .join("\n");
    for forbidden in [
        concat!("selected", "_manifest"),
        concat!("User", "Config"),
        concat!("Llm", "Section"),
        concat!(".", "llm"),
        concat!("vibe", "_llm"),
        concat!("api", "_key"),
        concat!("token", "_file"),
        "reqwest",
        concat!("Open", "AI"),
    ] {
        assert!(
            !production.contains(forbidden),
            "hosted lifecycle source names forbidden paid/config seam `{forbidden}`"
        );
    }

    let manifest: toml::Table = toml::from_str(include_str!("../../../Cargo.toml")).unwrap();
    let dependencies = manifest["dependencies"].as_table().unwrap();
    for forbidden in ["vibe-llm", "vibe-cli", "reqwest", "clap", "dialoguer"] {
        assert!(
            !dependencies.contains_key(forbidden),
            "vibe-mcp must not depend on `{forbidden}`"
        );
    }
    for required in [
        "chrono",
        "vibe-install",
        "vibe-lifecycle",
        "vibe-orchestrator",
        "vibe-package-source",
    ] {
        assert!(dependencies.contains_key(required), "missing `{required}`");
    }
}
