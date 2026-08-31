//! The hosted backend's reds: exact resolver delegation (the provider-self
//! half AND a selected-world cross-package `#embed`), the named completion
//! canary, the one-field storage law, and a real hosted PARK through the
//! engine this backend exists for — reusing the shared agent fixtures and
//! the real `LifecycleRun::execute_one`, never a second engine.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_spec::SelectedPackage;
use vibe_wire::generated::lifecycle::e1::context::{Project, RunAgentMode, World};
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

use crate::agent::tests::support::{PROMPT, TWO_OUTPUTS, row_at};
use crate::agent::{
    AgentBackend, AgentRequest, HostedAgentBackend, PromptRequest, SelectedWorldPromptResolver,
};
use crate::execution::HandlerExecution;
use crate::handlers::{HandlerRuntime, NoBinaryBackend, NoPackageBindingBackend};
use crate::process::{StreamMode, SystemProcessRunner};
use crate::{ExecutionReuse, LifecycleLease, LifecycleRun, RunMetadata};
use vibe_workspace::hooks::SystemProbe;

const RUN_ID: &str = "00112233445566778899aabbccddeeff";
const KEY: &str = "org.demo/tools#produce";

/// The workspace fixture: a project root, the executing provider's own
/// prompt document, and (when `embed`) a SECOND selected package whose
/// document the first one `#embed`s — so one resolve covers both halves.
struct Fixture {
    root: PathBuf,
    provider: PathBuf,
    _dir: tempfile::TempDir,
}

fn fixture(embed: bool) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    let provider = dir.path().join("vibedeps/org.demo.tools/1.0.0");
    let doc = provider.join("spec/common/PROMPT-001.md");
    fs::create_dir_all(doc.parent().unwrap()).unwrap();
    let body = if embed {
        "# Prompt {#root}\n\nWrite the guide.\n\n\
         #embed spec://org.demo/lib/common/STYLE-001#voice\n"
    } else {
        "# Prompt {#root}\n\nWrite the guide.\n"
    };
    fs::write(&doc, body).unwrap();
    let lib = dir.path().join("vibedeps/org.demo.lib/2.0.0");
    let style = lib.join("spec/common/STYLE-001.md");
    fs::create_dir_all(style.parent().unwrap()).unwrap();
    fs::write(&style, "# Style {#voice}\n\nPassive voice banned.\n").unwrap();
    Fixture {
        root: dir.path().to_path_buf(),
        provider,
        _dir: dir,
    }
}

fn request(at: &Fixture, embed: bool) -> PromptRequest {
    let mut selected = BTreeMap::new();
    if embed {
        selected.insert(
            ("org.demo".to_string(), "lib".to_string()),
            SelectedPackage::new("2.0.0", at.root.join("vibedeps/org.demo.lib/2.0.0")),
        );
    }
    PromptRequest {
        address: PROMPT.into(),
        provider_root: at.provider.clone(),
        provider_group: "org.demo".into(),
        provider_name: "tools".into(),
        selected_world: selected,
    }
}

/// `resolve_prompt` IS `SelectedWorldPromptResolver::resolve` — byte for
/// byte, for the provider-self half and for a selected-world cross-package
/// `#embed` alike, so a hosted prompt and a CLI prompt resolve identically.
#[test]
fn resolve_prompt_is_exactly_the_shared_resolver() {
    for embed in [false, true] {
        let at = fixture(embed);
        let backend = HostedAgentBackend::new(&at.root);
        let direct = SelectedWorldPromptResolver::new(&at.root).resolve(&request(&at, embed));
        let through = backend.resolve_prompt(&request(&at, embed));
        assert_eq!(through, direct, "exact delegation, embed={embed}");
        let resolved = through.expect("the fixture resolves");
        assert!(resolved.text.contains("Write the guide."));
        if embed {
            assert!(
                resolved.text.contains("Passive voice banned."),
                "the cross-package embed expanded through the selected world: {}",
                resolved.text
            );
        }
        assert!(resolved.unsupported.is_empty());
    }
}

/// The paid half refuses with the NAMED internal canary — never a
/// completion, never a configuration remediation (a hosted surface has no
/// paying half to configure).
#[test]
fn complete_is_the_named_internal_canary_and_never_completes() {
    let at = fixture(false);
    let backend = HostedAgentBackend::new(&at.root);
    let refusal = backend
        .complete(&AgentRequest {
            key: KEY.into(),
            phase: "create".into(),
            system: String::new(),
            user: String::new(),
        })
        .expect_err("the hosted backend has no paid half");
    assert!(
        refusal.contains("parks before paid dispatch"),
        "names the park law: {refusal}"
    );
    assert!(
        refusal.contains("internal invariant break"),
        "names the break: {refusal}"
    );
    assert!(
        !refusal.to_lowercase().contains("configure"),
        "no configuration remediation — a hosted surface cannot pay: {refusal}"
    );
}

/// The paid vocabulary stays out: the new cell names no HTTP transport,
/// provider client or credential shape, and the crate gains no paying
/// dependency. Needles are spelled in halves so this checker never reports
/// its own source; the storage law above is the structural fence.
#[test]
fn no_paid_vocabulary_or_dependency_grows() {
    let source = include_str!("../hosted.rs");
    for needle in [
        "reqwest",
        concat!("vibe", "_llm"),
        concat!("api", "_key"),
        "Bearer",
        concat!("Cli", "Agent"),
        concat!("User", "Config"),
        "http",
    ] {
        assert!(
            !source.contains(needle),
            "hosted.rs names `{needle}` — it has no paid half"
        );
    }
    let manifest = include_str!("../../../Cargo.toml");
    for forbidden in [concat!("vibe", "-llm"), "reqwest", "tokio"] {
        assert!(
            !manifest.contains(forbidden),
            "the crate must not grow a paying dependency `{forbidden}`"
        );
    }
}

// ---- the real hosted park, through the real engine --------------------------

fn lease(root: &Path) -> Arc<LifecycleLease> {
    Arc::new(LifecycleLease::acquire(root).expect("a temp root is leasable"))
}

fn metadata() -> RunMetadata {
    RunMetadata {
        requested: "create".into(),
        chain: vec!["validate".into(), "install".into(), "create".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Agent,
        force: false,
        trace_compile: false,
        run_id: RUN_ID.into(),
        started: "2026-08-26T00:00:00Z".into(),
        selected: ".".into(),
    }
}

fn project_fixture(root: &Path) -> Project {
    let text = root.to_string_lossy().replace('\\', "/");
    Project {
        kind: "project".into(),
        manifest: format!("{text}/vibe.toml"),
        name: "demo".into(),
        root: text,
        spec_roots: Vec::new(),
        version: "0.1.0".into(),
    }
}

fn world_fixture(root: &Path) -> World {
    let text = root.to_string_lossy().replace('\\', "/");
    World {
        deps_root: format!("{text}/vibedeps"),
        lockfile: format!("{text}/vibe.lock"),
        packages: Vec::new(),
    }
}

/// A real hosted runner transition with THIS backend parks: task published,
/// delegated row checkpointed with the exact planned rows, no declared
/// output written. Because this backend's `complete` ALWAYS refuses, the
/// successful park is itself the proof that dispatch (and therefore
/// `complete`) was never reached.
#[test]
fn a_hosted_row_parks_through_the_real_engine_without_spend_or_writes() {
    let at = fixture(false);
    let backend = HostedAgentBackend::new(&at.root);
    let row = row_at(TWO_OUTPUTS, PROMPT, at.provider.clone());
    let mut run = LifecycleRun::begin(
        lease(&at.root),
        project_fixture(&at.root),
        world_fixture(&at.root),
        metadata(),
        vec!["validate".into(), "install".into(), "create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row);
    static PROCESS: SystemProcessRunner = SystemProcessRunner;
    let rt = HandlerRuntime {
        process: &PROCESS,
        binary: &NoBinaryBackend,
        native: &crate::handlers::NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: &backend,
        probe: &SystemProbe,
        streams: StreamMode::Capture,
    };
    let transition = run
        .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .expect("the hosted transition parks");

    assert_eq!(transition.status, ExecutionRecordStatus::Delegated);
    let handoff = transition.delegation.as_ref().expect("a typed handoff");
    assert_eq!(handoff.run_id, RUN_ID);
    assert_eq!(handoff.resume, "vibe create");
    assert_eq!(handoff.tasks.len(), 1);
    assert!(handoff.tasks[0].starts_with(".vibe/agentic/outbox/"));
    assert!(at.root.join(&handoff.tasks[0]).is_file());

    let state: LifecycleState =
        toml::from_str(&fs::read_to_string(at.root.join(".vibe/lifecycle.toml")).unwrap()).unwrap();
    let record = &state.execution[KEY];
    assert_eq!(record.status, ExecutionRecordStatus::Delegated);
    assert_eq!(record.tasks.len(), 1);
    assert_eq!(
        record.artifacts.len(),
        2,
        "the exact planned rows are recorded"
    );
    // No output mutation: parking precedes dispatch entirely.
    assert!(!at.root.join("docs/guide.md").exists());
    assert!(!at.root.join("docs/reference.md").exists());
}
