//! The fake external reference process (R7.5 P3b, architecture §7,
//! PROP-054 `##REFERENCE-PDSA`).
//!
//! **This module is the orchestrator.** VibeVM below it knows nothing about
//! PDSA: the four words live in this module's name and its prose, never in a
//! phase, an enum, a wire field, a verb or an automatic transition — which the
//! `fence` submodule proves by scanning production sources and pinning the
//! closed vocabularies, and which the invocation counters below prove at
//! runtime.
//!
//! Two adjacent laws (architecture §4.3), one per cell:
//!
//! * a hosted park RESUMES — calling the same phase re-enters the inclusive
//!   chain, so a deterministic predecessor invalidated by the host's own output
//!   reruns and checkpoints a new measurement before the delegated row is
//!   accepted, and verify may therefore match on the resume itself;
//! * an uninterrupted local create that changes an already measured input
//!   produces `stale` and does NOT jump back — only an external second
//!   invocation recomputes it.
//!
//! Both cells count their external invocations, and both read requirement
//! metadata before touching the lifecycle: the process chooses its attempt from
//! read-only facts, and that reading moves no project byte.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use vibe_mcp::ServerContext;
use vibe_mcp::tools::{LifecycleTasksMcpTool, McpTool, RequirementsQueryMcpTool, ToolOutput};
use vibe_wire::behaviour::requirements_report::validate as validate_requirements;
use vibe_wire::behaviour::verification_evidence::validate as validate_evidence;
use vibe_wire::generated::lifecycle_state::LifecycleState;
use vibe_wire::generated::lifecycle_tasks::{LifecycleTasks, LifecycleTasksStatus};
use vibe_wire::generated::requirements_report::{
    AdoptionObservationPresence, AuthoringObservationPresence, FactStatus, FactStatusStage,
    FactStatusState, RelationSourceProvenance, RelationSourceState, RequirementSourceKind,
    RequirementsReport, SourceResultState,
};
use vibe_wire::generated::shared::{EvidenceStatus, VerificationEvidence};

use super::support::{
    ONE_AGENT_ROW, context, hosted_project, project, report, run, state_bytes, task_bytes, tree,
};

/// The vocabulary/back-edge fence, split out only for the 600-line budget.
#[path = "pdsa_reference/fence.rs"]
mod fence;

/// The one addressed requirement fact both fixtures share: host-authored,
/// status-marked, and with no consumer adoption overlay to consult.
const FACT_DOC: &str = "# Rules\n\n@fact:PROCESS the external process chooses the attempt. \
     @status:spec/plan\n";
const FACT_ADDRESS: &str = "spec://org.demo/demo/RULE#PROCESS";
const ADDRESS_PREFIX: &str = "spec://org.demo/demo/";

/// The hosted backend's paid half is a named internal canary
/// (`vibe_lifecycle::agent::hosted`): reaching it means the engine dispatched a
/// paid call under a backend that has none. Its first words, and the `[llm]`
/// table a paying surface would need, must appear nowhere.
const HOSTED_CANARY: &str = "internal invariant break";
const PAID_CONFIGURATION: &str = "[llm]";

/// A build row that measures the directory the hosted agent will write into.
const DECLARED_DOCS_BUILD: &str = r#"
[[extension]]
id = "declared-build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "DECLARED-BUILD" }
inputs = ["docs/**"]
"#;

/// The same law over the deterministic fixture's own measured tree.
const DECLARED_DATA_BUILD: &str = r#"
[[extension]]
id = "declared-build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "DECLARED-BUILD" }
inputs = ["data/**"]
"#;

/// The stop oracle: a verify contribution that must NOT run while the
/// comparison is stale, and must run once it matches.
const VERIFY_SENTINEL: &str = r#"
[[extension]]
id = "verify-sentinel"
point = "phase:verify"
handler = { kind = "builtin", name = "log" }
config = { message = "SENTINEL-VERIFY" }
"#;

const SENTINEL_MESSAGE: &str = "SENTINEL-VERIFY";

/// Every external lifecycle call this process makes, counted. The count IS the
/// no-back-edge oracle: an engine that re-entered a phase by itself would move
/// the observable state without moving this number.
struct Invocations {
    count: u32,
}

impl Invocations {
    fn new() -> Self {
        Self { count: 0 }
    }

    /// One external `lifecycle_run({phase:"verify"})`. A park and an executed
    /// failure are both tool results, so neither is unwrapped away.
    fn verify(&mut self, ctx: &ServerContext) -> ToolOutput {
        self.count += 1;
        run(ctx, "verify").expect("an external lifecycle invocation is a tool result")
    }
}

fn write_fact(root: &Path) {
    let specs = root.join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(specs.join("RULE.md"), FACT_DOC).unwrap();
}

/// The deterministic create contribution: it rewrites a MEASURED build input
/// inside the same invocation and declares no freshness input of its own.
fn mutating_create(root: &Path) {
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(
        root.join("scripts/touch.sh"),
        "printf 'two' > data/a.txt\n\
         printf '%s' '{\"artifacts\":[],\"envelope\":1,\"status\":\"ok\",\"tasks\":[]}' > \"$VIBE_REPLY\"\n",
    )
    .unwrap();
    fs::write(
        root.join("scripts/touch.ps1"),
        "'two' | Set-Content -NoNewline data/a.txt\n\
         '{\"artifacts\":[],\"envelope\":1,\"status\":\"ok\",\"tasks\":[]}' | Set-Content -NoNewline $env:VIBE_REPLY\n",
    )
    .unwrap();
    let manifest = root.join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        "\n[[extension]]\nid = \"mutating-create\"\npoint = \"phase:create\"\n\
         handler = { kind = \"script\", base = \"scripts/touch\" }\n",
    );
    fs::write(manifest, body).unwrap();
}

/// One read-only requirements answer through the real MCP surface.
fn requirements(ctx: &ServerContext, relations: bool) -> RequirementsReport {
    let output = RequirementsQueryMcpTool
        .run(
            &json!({ "address_prefix": ADDRESS_PREFIX, "relations": relations }),
            ctx,
        )
        .expect("the read-only requirements answer");
    assert!(!output.is_error(), "a bounded question is answerable");
    let root: RequirementsReport = serde_json::from_value(output.structured().clone())
        .expect("the structured content is the generated root");
    validate_requirements(&root).expect("what the surface published is a valid report");
    root
}

/// The answer with exactly the axes the two calls are ALLOWED to differ on
/// removed: the injected clock, the identity derived over the members we vary
/// on purpose, and the relation enrichment itself. Everything left must be
/// byte-identical, which is a content claim rather than a clock comparison.
fn base_metadata(source: &RequirementsReport) -> Value {
    let mut value = serde_json::to_value(source).unwrap();
    let observation = value["observation"].as_object_mut().unwrap();
    observation.remove("observed_at");
    observation.remove("observation_id");
    value["query"].as_object_mut().unwrap().remove("relations");
    value.as_object_mut().unwrap().remove("relation_sources");
    for row in value["rows"].as_array_mut().unwrap() {
        row.as_object_mut().unwrap().remove("relations");
    }
    value
}

/// The plan-side reading, done twice, proven read-only and enrichment-only.
fn read_requirements_twice(ctx: &ServerContext, root: &Path) -> RequirementsReport {
    let before = tree(root);
    let without = requirements(ctx, false);
    assert_eq!(tree(root), before, "a query moves no project byte");
    let with = requirements(ctx, true);
    assert_eq!(tree(root), before, "relation enrichment moves none either");

    assert_eq!(without.rows.len(), 1, "one addressed fact: {without:?}");
    let row = &without.rows[0];
    assert_eq!(row.address, FACT_ADDRESS);
    assert_eq!(row.source.kind, RequirementSourceKind::Host);
    assert_eq!(row.authoring.presence, AuthoringObservationPresence::Marked);
    assert_eq!(
        row.authoring.status,
        Some(FactStatus {
            stage: FactStatusStage::Spec,
            state: FactStatusState::Plan,
        })
    );
    // A host-authored fact has no consumer overlay to consult at all, which is
    // a different statement from "the registry has no row for it".
    assert_eq!(
        row.adoption.presence,
        AdoptionObservationPresence::NotApplicable
    );
    assert!(row.adoption.status.is_none());
    assert!(row.relations.is_empty());
    let source = without
        .sources
        .iter()
        .find(|result| result.source.kind == RequirementSourceKind::Host)
        .expect("the host source result");
    assert_eq!(source.state, SourceResultState::Available);

    // `relations: false` — no map was loaded, and the answer says so.
    assert!(!without.query.relations);
    assert!(!without.relation_sources.is_empty());
    for source in &without.relation_sources {
        assert_eq!(source.state, RelationSourceState::NotRequested);
        assert_eq!(source.provenance, RelationSourceProvenance::None);
        assert!(source.reason_code.is_none());
    }

    // `relations: true` with no map configured — a TYPED loss, never a false
    // zero-edge answer, and the base fact rows still come back.
    assert!(with.query.relations);
    assert_eq!(with.rows.len(), 1);
    assert_eq!(with.rows[0].address, FACT_ADDRESS);
    assert!(with.rows[0].relations.is_empty());
    assert!(!with.relation_sources.is_empty());
    for source in &with.relation_sources {
        assert_eq!(source.state, RelationSourceState::Unavailable);
        assert_eq!(source.provenance, RelationSourceProvenance::None);
        assert!(
            source
                .reason_code
                .as_deref()
                .is_some_and(|code| !code.trim().is_empty()),
            "a typed loss carries its machine reason: {source:?}"
        );
    }

    // Only the enrichment moved. The scope digest is the sharpest half: it is
    // taken over the selected sources and registry witnesses, so it cannot
    // move unless the measured scope did.
    assert_eq!(base_metadata(&without), base_metadata(&with));
    assert_eq!(
        without.observation.source_digest,
        with.observation.source_digest
    );
    assert_ne!(
        without.observation.observation_id, with.observation.observation_id,
        "the identity covers the query members, which is why it is excluded above"
    );
    with
}

/// No paid dispatch was made, and none was configurable: neither the hosted
/// backend's canary nor a paying `[llm]` table appears in any channel this
/// process saw, or in any byte the project holds.
fn assert_no_paid_dispatch(root: &Path, outputs: &[&ToolOutput]) {
    let mut haystacks: Vec<String> = Vec::new();
    for output in outputs {
        haystacks.push(output.text().to_string());
        haystacks.push(serde_json::to_string(output.structured()).unwrap());
    }
    for (relative, bytes) in tree(root) {
        haystacks.push(format!("{relative}\n{}", String::from_utf8_lossy(&bytes)));
    }
    for haystack in haystacks {
        assert!(
            !haystack.contains(HOSTED_CANARY),
            "the hosted backend has no paid half; reaching it is an invariant break: {haystack}"
        );
        assert!(
            !haystack.contains(PAID_CONFIGURATION),
            "no paying provider configuration exists on this surface: {haystack}"
        );
    }
}

/// Reading requirements after the lifecycle ran changes nothing the lifecycle
/// owns. Evidence is a GENERATED member of a verify answer, never a document
/// stored in lifecycle state, so the member this process captured is the only
/// copy it holds; the durable state bytes are the physical oracle.
fn assert_reading_requirements_changes_no_lifecycle_state(
    ctx: &ServerContext,
    root: &Path,
    member: &VerificationEvidence,
) {
    let captured = serde_json::to_value(member).unwrap();
    let state_after = state_bytes(root);
    let tree_after = tree(root);

    let plain = requirements(ctx, false);
    assert_eq!(state_bytes(root), state_after, "a query writes no state");
    let enriched = requirements(ctx, true);
    assert_eq!(state_bytes(root), state_after);
    assert_eq!(tree(root), tree_after, "and no project byte at all");

    assert_eq!(base_metadata(&plain), base_metadata(&enriched));
    assert_eq!(
        plain.observation.lifecycle_run_id.as_deref(),
        Some(member.run.run_id.as_str()),
        "the join key is READ from durable state, never minted"
    );
    assert_eq!(
        serde_json::to_value(member).unwrap(),
        captured,
        "the generated member this process captured is unchanged"
    );
}

fn state(root: &Path) -> LifecycleState {
    toml::from_str(&String::from_utf8(state_bytes(root)).unwrap()).unwrap()
}

fn contribution_status(
    report: &vibe_wire::generated::lifecycle_report::LifecycleReport,
    id: &str,
) -> String {
    let suffix = format!("#{id}");
    report
        .contributions
        .iter()
        .find(|row| row.key.ends_with(&suffix))
        .unwrap_or_else(|| panic!("no `{id}` contribution row: {report:?}"))
        .status
        .clone()
}

fn step_status(
    report: &vibe_wire::generated::lifecycle_report::LifecycleReport,
    phase: &str,
) -> String {
    report
        .steps
        .iter()
        .find(|step| step.phase == phase)
        .unwrap_or_else(|| panic!("no `{phase}` step: {report:?}"))
        .status
        .clone()
}

/// **Hosted control.** The requested phase is `verify`; an agent create row
/// parks before it. The process reads the mailbox, writes the exact declared
/// output — which lands inside a MEASURED build input — and calls the SAME
/// phase again. The resume re-enters the inclusive chain: build reruns and
/// checkpoints a new measurement BEFORE the hosted row is accepted, so verify
/// matches on the resume itself. Two external invocations, no paid call.
#[test]
fn pdsa_hosted_control_resumes_and_remeasures_before_accepting_the_host() {
    let project = hosted_project(&format!("{DECLARED_DOCS_BUILD}{ONE_AGENT_ROW}"));
    let root = project.path();
    write_fact(root);
    // The seed keeps the FIRST measurement a non-empty match set, so the law
    // under test is "the host's output changed a measured input" rather than
    // "a pattern that matched nothing now matches something".
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/seed.md"), "seed\n").unwrap();
    let ctx = context(root);

    assert!(!root.join(".vibe").exists());
    read_requirements_twice(&ctx, root);
    assert!(
        !root.join(".vibe").exists(),
        "reading requirements created no state directory"
    );

    let mut invocations = Invocations::new();

    // --- external invocation 1: the park ---------------------------------
    let parked = invocations.verify(&ctx);
    assert!(!parked.is_error(), "a park is a successful tool result");
    let parked_report = report(&parked);
    assert_eq!(parked_report.requested, "verify");
    assert_eq!(parked_report.steps.last().unwrap().status, "delegated");
    assert!(
        parked_report.verification.is_none(),
        "verify never ran, so it owes no member"
    );
    let parked_structured = serde_json::to_string(parked.structured()).unwrap();
    assert!(
        !parked_structured.contains("verification"),
        "an absent member is an absent key: {parked_structured}"
    );
    let handoff = parked_report
        .delegation
        .as_ref()
        .expect("one hosted handoff")
        .clone();
    assert_eq!(handoff.tasks.len(), 1);
    assert_eq!(handoff.resume, "vibe verify");
    assert!(
        !parked_report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("#after-agent")),
        "the chain stopped AT the park"
    );
    assert!(parked.text().contains("`lifecycle_tasks`"));
    let parked_started = state(root).run.started.clone();

    // --- the process acts as the host ------------------------------------
    let mailbox = LifecycleTasksMcpTool.run(&json!({}), &ctx).unwrap();
    let mailbox: LifecycleTasks = serde_json::from_value(mailbox.structured().clone()).unwrap();
    assert_eq!(mailbox.status, LifecycleTasksStatus::Parked);
    assert_eq!(mailbox.tasks.len(), 1);
    let task = String::from_utf8(task_bytes(root, &handoff.tasks[0])).unwrap();
    assert!(
        task.contains("docs/guide.md"),
        "the task document declares the exact output: {task}"
    );
    fs::write(root.join("docs/guide.md"), "hosted body\n").unwrap();

    // --- external invocation 2: the resume --------------------------------
    let resumed = invocations.verify(&ctx);
    assert!(!resumed.is_error());
    let resumed_report = report(&resumed);
    assert!(
        resumed_report.ok,
        "the resume completed: {resumed_report:?}"
    );
    assert!(resumed_report.delegation.is_none(), "nothing is still owed");

    // The inclusive predecessor RERAN — the same fixture resumed WITHOUT the
    // host write reports `build = fresh` and re-parks, so `ok` here is caused
    // by the host's own output landing inside the measured pattern — and the
    // hosted row was accepted after it.
    assert_eq!(step_status(&resumed_report, "build"), "ok");
    assert_eq!(contribution_status(&resumed_report, "declared-build"), "ok");
    assert_eq!(contribution_status(&resumed_report, "after-agent"), "ok");

    let member = resumed_report
        .verification
        .clone()
        .unwrap_or_else(|| panic!("the resume owes the member: {resumed_report:?}"));
    validate_evidence(&member).expect("what the surface published is a valid member");
    assert_eq!(member.status, EvidenceStatus::Matched);
    assert_eq!(member.run.requested, "verify");
    // The resume stayed INSIDE the parked run: this is linear resume, not a
    // fresh attempt and not a hidden retry.
    assert_eq!(member.run.run_id, handoff.run_id);
    assert_eq!(state(root).run.started, parked_started);
    let measurement = member
        .inputs
        .iter()
        .find(|row| row.execution.ends_with("#declared-build"))
        .unwrap_or_else(|| panic!("the build measurement: {member:?}"));
    assert_eq!(measurement.phase, "build");
    assert_eq!(measurement.patterns, ["docs/**"]);

    assert_eq!(
        invocations.count, 2,
        "the PROCESS invoked the lifecycle twice; the engine never called itself"
    );
    assert_no_paid_dispatch(root, &[&parked, &resumed]);
    assert_reading_requirements_changes_no_lifecycle_state(&ctx, root, &member);
}

/// **Uninterrupted stale.** A deterministic create rewrites a measured build
/// input later in ONE invocation and declares no freshness input of its own.
/// Verify reports stale and stops before its sentinel — it does not jump back.
/// The TEST, and only the test, invokes verify a second time: build recomputes,
/// create fresh-skips, the sentinel runs and the comparison matches.
#[test]
fn pdsa_stale_needs_a_second_external_invocation() {
    let project = project(&format!("{DECLARED_DATA_BUILD}{VERIFY_SENTINEL}"));
    let root = project.path();
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(root.join("data/a.txt"), "one").unwrap();
    write_fact(root);
    mutating_create(root);
    let ctx = context(root);

    assert!(!root.join(".vibe").exists());
    read_requirements_twice(&ctx, root);
    assert!(!root.join(".vibe").exists());

    let mut invocations = Invocations::new();

    // --- external invocation 1: stale, stopping before the sentinel -------
    let first = invocations.verify(&ctx);
    let first_report = report(&first);
    assert!(!first_report.ok, "the command's own axis is false");
    let stale = first_report
        .verification
        .clone()
        .unwrap_or_else(|| panic!("a stop must carry its comparison: {first_report:?}"));
    validate_evidence(&stale).expect("a stopping member is still a valid member");
    assert_eq!(stale.status, EvidenceStatus::Stale);
    assert!(
        !first_report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("#verify-sentinel")),
        "a stale comparison stops the chain: {first_report:?}"
    );
    assert!(!first.text().contains(SENTINEL_MESSAGE));

    // --- external invocation 2: the process decides to recompute ----------
    let second = invocations.verify(&ctx);
    let second_report = report(&second);
    assert!(
        second_report.ok,
        "the recompute completed: {second_report:?}"
    );
    assert_eq!(step_status(&second_report, "build"), "ok");
    assert_eq!(contribution_status(&second_report, "declared-build"), "ok");
    assert_eq!(
        contribution_status(&second_report, "mutating-create"),
        "fresh",
        "the create contribution fresh-skips; nothing re-dirties the input"
    );
    assert_eq!(contribution_status(&second_report, "verify-sentinel"), "ok");
    let matched = second_report
        .verification
        .clone()
        .unwrap_or_else(|| panic!("the recompute owes the member: {second_report:?}"));
    validate_evidence(&matched).expect("what the surface published is a valid member");
    assert_eq!(matched.status, EvidenceStatus::Matched);
    assert_ne!(
        matched.run.run_id, stale.run.run_id,
        "the failed first invocation parked nothing, so the second is a fresh run"
    );

    assert_eq!(
        invocations.count, 2,
        "exactly two external lifecycle invocations; verify never called create"
    );
    assert_no_paid_dispatch(root, &[&first, &second]);
    assert_reading_requirements_changes_no_lifecycle_state(&ctx, root, &matched);
}
