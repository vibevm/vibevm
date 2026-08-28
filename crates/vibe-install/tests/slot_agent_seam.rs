//! The slot lifecycle's agent seam, proven by dispatching a real slot agent
//! row rather than by inspecting a field.
//!
//! `agent` is legal at `slot:` points, so every caller that runs the slot
//! lifecycle must reach the backend it configured. Two facts keep that true:
//!
//! 1. [`SlotLifecycleSeams`] is a **required** constructor parameter and has
//!    no `Default` — a call site cannot silently fall back to the refusing
//!    backend, so forgetting it is a compile error, not a behaviour change
//!    nobody notices.
//! 2. A resolution carrying a `slot:post-install` agent contribution, actually
//!    dispatched, moves the injected backend's counters. Without that the test
//!    would only be asserting that a struct holds what it was given.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_core::manifest::Manifest;
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_install::{InstallSlotLifecycle, SlotLifecycleSeams};
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::agent::{
    AgentBackend, AgentCompletion, AgentRequest, PromptRequest, ResolvedPrompt,
};
use vibe_lifecycle::process::StreamMode;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_workspace::install::{ResolvedDep, SlotLifecycle, SlotLifecycleContext};

/// Counts what the runtime asked of it, so "the injected backend was reached"
/// is a measurement, not a claim about a pointer.
#[derive(Default)]
struct CountingBackend {
    resolved: AtomicUsize,
    completed: AtomicUsize,
}

impl AgentBackend for CountingBackend {
    fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
        self.resolved.fetch_add(1, Ordering::SeqCst);
        Ok(ResolvedPrompt {
            text: format!("instructions for {}", request.address),
            unsupported: Vec::new(),
        })
    }

    fn complete(&self, _request: &AgentRequest) -> Result<AgentCompletion, String> {
        self.completed.fetch_add(1, Ordering::SeqCst);
        Ok(AgentCompletion {
            text: r#"{"outputs":[{"path":"docs/slot.md","content":"slot body\n"}]}"#.into(),
            usage: None,
        })
    }
}

const SLOT_AGENT: &str = "\n[[extension]]\nid = \"slot-produce\"\npoint = \"slot:post-install\"\n\
     handler = { kind = \"agent\", prompt = \"spec://org.demo/tools/common/agent-prompt#root\" }\n\
     config.outputs = [\n  \
     { path = \"docs/slot.md\", kind = \"file\", accept = \"non-empty file\" },\n]\n";

fn metadata(project_root: &std::path::Path) -> RunMetadata {
    RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(project_root).unwrap(),
        started: "2026-08-26T00:00:00Z".into(),
        selected: ".".into(),
    }
}

/// A project with one materialised dependency slot whose manifest declares the
/// slot-scoped agent row, plus the resolution that names it.
fn fixture() -> (tempfile::TempDir, Manifest, Vec<ResolvedDep>) {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\ngroup = \"org.demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.demo/tools\" = \"=1.0.0\"\n",
    )
    .unwrap();
    let host = Manifest::read(project.path().join("vibe.toml")).unwrap();

    let slot = project
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.demo.tools")
        .join("1.0.0");
    std::fs::create_dir_all(&slot).unwrap();
    let slot_manifest_text = format!(
        "[package]\ngroup = \"org.demo\"\nname = \"tools\"\nkind = \"tool\"\n\
         version = \"1.0.0\"\n{SLOT_AGENT}"
    );
    std::fs::write(slot.join("vibe.toml"), &slot_manifest_text).unwrap();
    let specs = slot.join("vibevm/vibespecs/common");
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("agent-prompt.md"),
        "# Prompt {#root}\n\nWrite the slot document.\n",
    )
    .unwrap();

    let manifest = Manifest::read(slot.join("vibe.toml")).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Tool,
        group: Group::parse("org.demo").unwrap(),
        name: "tools".into(),
        version: "1.0.0".parse().unwrap(),
        content_dir: slot.clone(),
        source_hash: Some(ContentHash::parse("sha256:aa").unwrap()),
        manifest,
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    };
    (project, host, vec![dep])
}

fn dispatch_post_install(
    lifecycle: &InstallSlotLifecycle,
    dep: &ResolvedDep,
    slot: &std::path::Path,
) {
    let context = SlotLifecycleContext {
        group: &dep.group,
        name: &dep.name,
        version: &dep.version,
        kind: &dep.kind,
        slot,
        manifest: &dep.manifest,
    };
    let _ = lifecycle.post_install(context);
}

/// The injected backend is the one the slot runtime actually calls.
#[test]
fn an_injected_backend_executes_a_slot_scoped_agent_row() {
    let (project, host, resolution) = fixture();
    let backend = Arc::new(CountingBackend::default());
    let lifecycle = InstallSlotLifecycle::from_resolution_observed(
        project.path(),
        &host,
        &resolution,
        metadata(project.path()),
        StreamMode::Capture,
        SlotLifecycleSeams {
            observer: Arc::new(vibe_install::NoSlotLifecycleObserver),
            agent: backend.clone(),
        },
        std::sync::Arc::new(vibe_lifecycle::LifecycleLease::acquire(project.path()).unwrap()),
    )
    .expect("the slot lifecycle constructs");

    let slot = resolution[0].content_dir.clone();
    dispatch_post_install(&lifecycle, &resolution[0], &slot);

    assert_eq!(
        backend.resolved.load(Ordering::SeqCst),
        1,
        "the injected backend must resolve the slot row's prompt",
    );
    assert_eq!(
        backend.completed.load(Ordering::SeqCst),
        1,
        "and must be the one that performs the paid call",
    );
    assert_eq!(
        std::fs::read_to_string(project.path().join("docs/slot.md")).unwrap(),
        "slot body\n",
        "a slot-scoped agent row writes its declared output",
    );
}

/// The named refusing seams keep the refusing backend, so the same row fails
/// with remediation instead of being skipped — and writes nothing.
#[test]
fn the_refusing_seams_fail_the_same_row_with_remediation() {
    let (project, host, resolution) = fixture();
    let lifecycle = InstallSlotLifecycle::from_resolution_observed(
        project.path(),
        &host,
        &resolution,
        metadata(project.path()),
        StreamMode::Capture,
        SlotLifecycleSeams::refusing(),
        std::sync::Arc::new(vibe_lifecycle::LifecycleLease::acquire(project.path()).unwrap()),
    )
    .expect("the slot lifecycle constructs");

    let slot = resolution[0].content_dir.clone();
    let context = SlotLifecycleContext {
        group: &resolution[0].group,
        name: &resolution[0].name,
        version: &resolution[0].version,
        kind: &resolution[0].kind,
        slot: &slot,
        manifest: &resolution[0].manifest,
    };
    let error = lifecycle
        .post_install(context)
        .expect_err("a selected agent row is never skipped");
    assert!(
        error.contains("configure user `[llm]`") && error.contains("agent host"),
        "the refusal must carry the remediation: {error}"
    );
    assert!(
        !project.path().join("docs/slot.md").exists(),
        "and must write nothing",
    );
}
