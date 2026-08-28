//! R7.4 §10.1 — the pre-extraction characterization oracle for the CLI
//! lifecycle surfaces.
//!
//! The `vibe-orchestrator` extraction (R7-MCP-LIFECYCLE-ARCHITECTURE §4/§7)
//! will move the whole-command algorithm out of
//! `vibe-cli/src/commands/lifecycle*` behind ports; §7 makes "existing
//! byte/golden/e2e output" the oracle that judges the move. Existing tests
//! pin those facts FIELD BY FIELD; this file pins the complete documents.
//! Every golden parses the emitted stream and compares it against an
//! authored JSON text after replacing ONLY the genuine entropy — the random
//! 32-hex run id, the temp project root (both separator spellings) and CRLF
//! in captured handler streams. Semantic fields, row order, statuses,
//! message text and document count are never normalized.
//!
//! Coverage, and the existing tests that already pin neighbouring facts
//! (cited rather than duplicated here):
//!
//! * completed-run JSON framing (deferred plan FLUSHED, then the one root) —
//!   below; `cli_lifecycle.rs::json_parent_keeps_install_auto_approval_…`
//!   already pins the prerequisite install's child silence.
//! * park framing (plan DISCARDED, one total document) and the state/outbox
//!   contract — `cli_lifecycle_hosted.rs::a_first_hosted_invocation_parks_…`
//!   pins the state checkpoint; below pins the whole report document.
//! * executed-handler failure — `cli_lifecycle_fatal_outcomes.rs` pins the
//!   plan-then-report pair for a single failing row; below pins the retained
//!   successful prefix, the failing row last, the one-fail-step shape and
//!   the actionable stderr chain.
//! * install barrier slot-before-phase row order —
//!   `cli_lifecycle_exact_events.rs` and `cli_lifecycle_fatal_outcomes.rs`
//!   pin the sole-root/no-echo framing; below pins the complete five-document
//!   stream and the row order inside the root.
//! * quiet/human projection — `cli_lifecycle.rs::quiet_lifecycle_is_exactly_
//!   one_summary_line` (empty-world quiet), `cli_lifecycle_fatal_outcomes.rs`
//!   (quiet failure is one line), `cli_lifecycle_hosted.rs::human_and_quiet_
//!   print_exactly_one_fenced_contract` (the park fence in both modes); below
//!   pins the with-contributions human transcript and quiet line verbatim.

mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use common::agent_provider::{MockProvider, configure_provider};
use serde_json::Value;

const PROJECT_SENTINEL: &str = "<project>";
const RUN_ID_SENTINEL: &str = "<run-id>";
const EVIDENCE_ID_SENTINEL: &str = "<evidence-id>";
const INSTANT_SENTINEL: &str = "<instant>";

/// Two deterministic host builtin rows at `phase:generate` and `phase:build`.
const GREET_EXTENSIONS: &str = r#"
[[extension]]
id = "greet-generate"
point = "phase:generate"
handler = { kind = "builtin", name = "log" }
config = { message = "GOLDEN-GENERATE" }

[[extension]]
id = "greet-build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "GOLDEN-BUILD" }
"#;

fn golden_project(user: &UserScratch, extensions: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    fs::write(
        project.path().join("vibe.toml"),
        format!(
            "[project]\nname = \"golden\"\ngroup = \"org.golden\"\nversion = \"0.1.0\"\n{extensions}"
        ),
    )
    .unwrap();
    project
}

fn documents(bytes: &[u8]) -> Vec<Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap_or_else(|error| {
            panic!(
                "stdout is not a JSON document stream: {error}\n{}",
                String::from_utf8_lossy(bytes)
            )
        })
}

/// One authored golden stream, parsed from its exact JSON text.
fn golden(text: &str) -> Value {
    serde_json::from_str(text).expect("the authored golden is valid JSON")
}

/// The shape every durable run identity has (32 lowercase hex) — the one
/// predicate that proves a replaced value WAS entropy.
fn is_run_id(text: &str) -> bool {
    text.len() == 32 && text.bytes().all(hex_byte)
}

const fn hex_byte(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
}

fn project_spellings(project: &Path) -> Vec<String> {
    let native = project.to_string_lossy().into_owned();
    let forward = native.replace('\\', "/");
    if native == forward {
        vec![native]
    } else {
        vec![native, forward]
    }
}

/// Replace ONLY genuine entropy inside an already-parsed document: the temp
/// project root (either separator spelling), the observed 32-hex run id, and
/// CRLF inside captured handler streams (a Windows/Linux stdout difference,
/// not a semantic one). Everything else passes through verbatim.
fn scrub(key: &str, value: &mut Value, spellings: &[String], run_id: Option<&str>) {
    match value {
        Value::String(text) => {
            for spelling in spellings {
                if text.contains(spelling.as_str()) {
                    *text = text.replace(spelling.as_str(), PROJECT_SENTINEL);
                }
            }
            if let Some(run_id) = run_id {
                *text = text.replace(run_id, RUN_ID_SENTINEL);
            }
            if matches!(key, "stdout" | "stderr") {
                *text = text.replace("\r\n", "\n");
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub(key, item, spellings, run_id);
            }
        }
        Value::Object(map) => {
            for (child_key, child) in map {
                scrub(child_key, child, spellings, run_id);
            }
        }
        _ => {}
    }
}

fn normalized(bytes: &[u8], project: &Path, run_id: Option<&str>) -> Vec<Value> {
    let spellings = project_spellings(project);
    let mut docs = documents(bytes);
    for doc in &mut docs {
        scrub_verification(doc);
        scrub("", doc, &spellings, run_id);
    }
    docs
}

/// The R7.5 evidence member's four entropy fields, and ONLY those: the run id
/// it names, its two instants, and `evidence_id`. The digest inherits the run
/// identity/start entropy but deliberately excludes `observed_at`; that clock
/// is independently variable. Each value is validated as well-formed BEFORE
/// its sentinel stands in, so a malformed value cannot hide behind the
/// normalisation; status, epoch, row counts, chain and selected node pass
/// through verbatim — the member is pinned, not skipped.
fn scrub_verification(doc: &mut Value) {
    let Some(member) = doc.get_mut("verification") else {
        return;
    };
    let id = member["evidence_id"].as_str().expect("an id is a string");
    assert!(
        id.strip_prefix("sha256:")
            .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(hex_byte)),
        "evidence ids are `sha256:` + 64 lowercase hex: {id}",
    );
    member["evidence_id"] = Value::String(EVIDENCE_ID_SENTINEL.into());
    let run_id = member["run"]["run_id"].as_str().expect("it names its run");
    assert!(is_run_id(run_id), "run ids are 32-hex: {run_id}");
    member["run"]["run_id"] = Value::String(RUN_ID_SENTINEL.into());
    scrub_instant(&mut member["observed_at"]);
    scrub_instant(&mut member["run"]["started"]);
}

fn scrub_instant(instant: &mut Value) {
    let text = instant.as_str().expect("an instant is a string");
    assert!(
        text.ends_with('Z') && text.contains('T'),
        "RFC 3339: {text}"
    );
    *instant = Value::String(INSTANT_SENTINEL.into());
}

/// The observed run id of a parked report — validated as entropy before any
/// sentinel may stand in for it.
fn observed_run_id(doc: &Value) -> String {
    let run_id = doc["delegation"]["run_id"]
        .as_str()
        .expect("the parked root carries delegation.run_id");
    assert!(is_run_id(run_id), "run ids are 32-hex: {run_id}");
    run_id.to_string()
}

fn deploy_fixture(user: &UserScratch) -> tempfile::TempDir {
    golden_project(user, GREET_EXTENSIONS)
}

fn deploy(user: &UserScratch, project: &Path, mode: &str) -> std::process::Output {
    let mut command = user.vibe();
    command.arg("deploy");
    if mode != "human" {
        command.arg(mode);
    }
    command
        .arg("--path")
        .arg(project)
        .arg("--registry")
        .arg(common::fixture_registry())
        .arg("--assume-yes")
        .output()
        .unwrap()
}

/// Surface (a): the successful/default lifecycle document. A COMPLETED run
/// flushes its held-back plan preview first, then emits exactly one
/// `lifecycle` root — the two-document framing `output::Context` owns.
#[test]
fn completed_default_lifecycle_document_is_pinned_in_full() {
    let user = UserScratch::new();
    let project = deploy_fixture(&user);
    let output = deploy(&user, project.path(), "--json");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let docs = normalized(&output.stdout, project.path(), None);
    assert_eq!(
        docs.iter()
            .map(|doc| doc["command"].clone())
            .collect::<Vec<_>>(),
        ["lifecycle:plan", "lifecycle"],
        "a completed run flushes its plan preview, then its one root",
    );
    let expected = golden(
        r#"[
{"chain":["validate","install","generate","build","test","create","verify","package","deploy"],"command":"lifecycle:plan","contributions":[
 {"handler":"builtin","key":"org.golden/golden#greet-generate","phase":"generate","point":"phase:generate","provider":"org.golden/golden","tier":"host-declaration","version":"0.1.0"},
 {"handler":"builtin","key":"org.golden/golden#greet-build","phase":"build","point":"phase:build","provider":"org.golden/golden","tier":"host-declaration","version":"0.1.0"}],
 "notices":[],"requested":"deploy"},
{"chain":["validate","install","generate","build","test","create","verify","package","deploy"],"command":"lifecycle","contributions":[
 {"handler":"builtin","key":"org.golden/golden#greet-generate","phase":"generate","point":"phase:generate","provider":"org.golden/golden","status":"ok","tier":"host-declaration","message":"GOLDEN-GENERATE","version":"0.1.0"},
 {"handler":"builtin","key":"org.golden/golden#greet-build","phase":"build","point":"phase:build","provider":"org.golden/golden","status":"ok","tier":"host-declaration","message":"GOLDEN-BUILD","version":"0.1.0"}],
 "notices":[],"ok":true,"requested":"deploy","steps":[
  {"phase":"validate","status":"ok"},{"phase":"install","status":"fresh"},{"phase":"generate","status":"ok"},
  {"phase":"build","status":"ok"},{"phase":"test","status":"no-op"},{"phase":"create","status":"no-op"},
  {"phase":"verify","status":"no-op"},{"phase":"package","status":"no-op"},{"phase":"deploy","status":"no-op"}],
 "verification":{"artifacts":[],"evidence":1,"evidence_id":"<evidence-id>","inputs":[],
  "observed_at":"<instant>","status":"unavailable","run":{
   "chain":["validate","install","generate","build","test","create","verify","package","deploy"],
   "requested":"deploy","run_id":"<run-id>","selected":".","started":"<instant>"}}}]"#,
    );
    assert_eq!(
        Value::Array(docs),
        expected,
        "the complete two-document stream"
    );
}

/// Surface (b): the hosted park. The plan preview is DISCARDED, so the whole
/// output is one document: the executed prefix (nothing downstream of the
/// parked phase), the one `delegated` row, and the typed handoff whose task
/// path relates to the reported run id.
#[test]
fn hosted_park_document_is_pinned_in_full() {
    let provider =
        MockProvider::serving(r#"{"outputs":[{"path":"docs/guide.md","content":"paid\n"}]}"#);
    let user = UserScratch::new();
    let project = golden_project(
        &user,
        r#"
[[extension]]
id = "produce-docs"
point = "phase:create"
handler = { kind = "agent", prompt = "spec://org.golden/golden/common/agent-prompt#root" }
config.outputs = [
  { path = "docs/guide.md", kind = "file", accept = "non-empty file" },
]

[[extension]]
id = "after-agent"
point = "phase:create"
handler = { kind = "builtin", name = "log" }
config = { message = "SENTINEL-AFTER-AGENT" }
"#,
    );
    let specs = project.path().join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        "# Golden prompt {#root}\n\nWrite the declared documentation files.\n",
    )
    .unwrap();
    configure_provider(&user, &provider.endpoint());

    let output = user
        .vibe()
        .args(["create", "--json", "--assume-yes", "--agent-mode", "agent"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "a durable handoff exits 0: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(provider.hits(), 0, "parking never constructs a provider");

    let raw = documents(&output.stdout);
    assert_eq!(raw.len(), 1, "a parked run emits ONE total document");
    let run_id = observed_run_id(&raw[0]);
    let docs = normalized(&output.stdout, project.path(), Some(&run_id));
    let expected = golden(
        r#"[
{"chain":["validate","install","generate","build","test","create"],"command":"lifecycle","contributions":[
 {"handler":"agent","key":"org.golden/golden#produce-docs","phase":"create","point":"phase:create","provider":"org.golden/golden","status":"delegated","tier":"host-declaration","message":"parked for the hosting agent; 1 declared output(s) awaited; resume with `vibe create`","version":"0.1.0"}],
 "notices":[],"ok":true,"requested":"create","steps":[
  {"phase":"validate","status":"ok"},{"phase":"install","status":"fresh"},{"phase":"generate","status":"no-op"},
  {"phase":"build","status":"no-op"},{"phase":"test","status":"no-op"},{"phase":"create","status":"delegated"}],
 "delegation":{"resume":"vibe create","run_id":"<run-id>","tasks":[".vibe/agentic/outbox/<run-id>/task-org.golden%2Fgolden%23produce-docs.md"]}}]"#,
    );
    assert_eq!(Value::Array(docs), expected, "the complete parked document");
    // The task really is durably published under the reported run.
    assert!(
        project
            .path()
            .join(format!(
                ".vibe/agentic/outbox/{run_id}/task-org.golden%2Fgolden%23produce-docs.md"
            ))
            .is_file()
    );
}

/// Surface (c): an executed handler failure AFTER a successful row. Exactly
/// one `ok:false` root follows the flushed plan; the successful row is
/// retained, the failing row is last with its streams, the step list is the
/// single failing phase, and stderr still carries the original actionable
/// chain with a non-zero exit.
#[test]
fn executed_handler_failure_document_is_pinned_in_full() {
    let user = UserScratch::new();
    let project = golden_project(
        &user,
        r#"
[[extension]]
id = "greet-generate"
point = "phase:generate"
handler = { kind = "builtin", name = "log" }
config = { message = "GOLDEN-GENERATE" }
"#,
    );
    let scripts = project.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    fs::write(
        scripts.join("fail.sh"),
        "printf PHASE-OUT\nprintf PHASE-ERR >&2\nexit 29\n",
    )
    .unwrap();
    fs::write(
        scripts.join("fail.ps1"),
        "Write-Output PHASE-OUT\n[Console]::Error.Write('PHASE-ERR')\nexit 29\n",
    )
    .unwrap();
    let manifest = project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(
        "\n[[extension]]\nid='fatal-row'\npoint='phase:build'\nhandler={ kind = \"script\", base = \"scripts/fail\" }\n",
    );
    fs::write(&manifest, text).unwrap();

    let output = user
        .vibe()
        .args(["build", "--json"])
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(common::fixture_registry())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1), "the exit classification");

    let docs = normalized(&output.stdout, project.path(), None);
    assert_eq!(
        docs.iter()
            .map(|doc| doc["command"].clone())
            .collect::<Vec<_>>(),
        ["lifecycle:plan", "lifecycle"],
        "a failed run still flushes its plan, then emits its one ok:false root",
    );
    let expected = golden(
        r#"[
{"chain":["validate","install","generate","build"],"command":"lifecycle:plan","contributions":[
 {"handler":"builtin","key":"org.golden/golden#greet-generate","phase":"generate","point":"phase:generate","provider":"org.golden/golden","tier":"host-declaration","version":"0.1.0"},
 {"handler":"script","key":"org.golden/golden#fatal-row","phase":"build","point":"phase:build","provider":"org.golden/golden","tier":"host-declaration","version":"0.1.0"}],
 "notices":[],"requested":"build"},
{"chain":["validate","install","generate","build"],"command":"lifecycle","contributions":[
 {"handler":"builtin","key":"org.golden/golden#greet-generate","phase":"generate","point":"phase:generate","provider":"org.golden/golden","status":"ok","tier":"host-declaration","message":"GOLDEN-GENERATE","version":"0.1.0"},
 {"handler":"script","key":"org.golden/golden#fatal-row","phase":"build","point":"phase:build","provider":"org.golden/golden","status":"fail","tier":"host-declaration",
  "message":"extension `org.golden/golden#fatal-row` handler failed: extension `org.golden/golden#fatal-row` exited nonzero (Some(29)); reply is ignoredPHASE-ERR (governed by spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; fix: correct the handler and rerun the stopped lifecycle) (governed by spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; fix: correct the named handler or its process/reply wire)",
  "stderr":"PHASE-ERR","stdout":"PHASE-OUT\n","version":"0.1.0"}],
 "notices":[],"ok":false,"requested":"build","steps":[{"phase":"build","status":"fail"}]}]"#,
    );
    assert_eq!(
        Value::Array(docs),
        expected,
        "the complete failed two-document stream"
    );

    // The original actionable error chain survives on stderr, as one JSON
    // error document, naming the stopped phase, the exit code and the
    // checkpointed transition.
    let error: Value =
        serde_json::from_slice(&output.stderr).expect("stderr is one JSON error document");
    assert_eq!(error["ok"], false);
    let chain = error["error"].as_str().unwrap();
    for fragment in [
        "phase `build` stopped before any later lifecycle contribution",
        "exited nonzero (Some(29))",
        "failed lifecycle transition checkpointed",
    ] {
        assert!(
            chain.contains(fragment),
            "the chain must keep `{fragment}`: {chain}",
        );
    }
}

/// Surface (d): the install barrier path. `vibe install --json` emits the
/// resolution plan, the slot plan, the phase-ritual plan and the closure
/// diff, then its ONE registered root LAST — and inside that root the slot
/// rows precede the `phase:install` ritual row.
#[test]
fn install_barrier_document_stream_is_pinned_in_full() {
    let registry = tempfile::tempdir().unwrap();
    let package = registry
        .path()
        .join("org.goldenpkg")
        .join("hooked")
        .join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.goldenpkg'\nname='hooked'\nkind='tool'\nversion='0.1.0'\n\n[hooks]\npre-install='hooks/pre'\npost-install='hooks/post'\n",
    )
    .unwrap();
    fs::write(package.join("hooks/pre.sh"), "printf PRE-RAN\n").unwrap();
    fs::write(package.join("hooks/pre.ps1"), "Write-Output PRE-RAN\n").unwrap();
    fs::write(package.join("hooks/post.sh"), "printf POST-RAN\n").unwrap();
    fs::write(package.join("hooks/post.ps1"), "Write-Output POST-RAN\n").unwrap();

    let user = UserScratch::new();
    let project = golden_project(
        &user,
        r#"
[[extension]]
id = "greet-install"
point = "phase:install"
handler = { kind = "builtin", name = "log" }
config = { message = "GOLDEN-INSTALL" }
"#,
    );
    let output = user
        .vibe()
        .args(["install", "org.goldenpkg/hooked@=0.1.0", "--json"])
        .arg("--registry")
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let docs = normalized(&output.stdout, project.path(), None);
    assert_eq!(
        docs.iter()
            .map(|doc| doc["command"].clone())
            .collect::<Vec<_>>(),
        [
            "install:plan",
            "lifecycle:plan",
            "lifecycle:plan",
            "install:closure-diff",
            "install"
        ],
        "the outermost command's root is the sole report and it is last",
    );
    // The one entropy-bearing member of the root is the project path.
    assert_eq!(docs[4]["project"], PROJECT_SENTINEL);
    // The slot-before-phase order, stated where it lives:
    let points: Vec<&str> = docs[4]["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["point"].as_str().unwrap())
        .collect();
    assert_eq!(
        points,
        ["slot:pre-install", "slot:post-install", "phase:install"],
        "slot rows precede phase rows in the command root",
    );

    let expected = golden(
        r#"[
{"command":"install:plan","packages":[{"package":"org.goldenpkg/hooked","version":"0.1.0"}]},
{"chain":["validate","install"],"command":"lifecycle:plan","contributions":[
 {"handler":"script","key":"org.goldenpkg/hooked#@vibe/hooks/pre-install@slot(org.goldenpkg/hooked@0.1.0)","phase":"install","point":"slot:pre-install","provider":"org.goldenpkg/hooked","tier":"dependency","reference":"org.goldenpkg/hooked#@vibe/hooks/pre-install","slot_target":{"group":"org.goldenpkg","kind":"tool","name":"hooked","root":"<project>/vibevm/vibedeps/org.goldenpkg.hooked/0.1.0","version":"0.1.0"},"version":"0.1.0"},
 {"handler":"script","key":"org.goldenpkg/hooked#@vibe/hooks/post-install@slot(org.goldenpkg/hooked@0.1.0)","phase":"install","point":"slot:post-install","provider":"org.goldenpkg/hooked","tier":"dependency","reference":"org.goldenpkg/hooked#@vibe/hooks/post-install","slot_target":{"group":"org.goldenpkg","kind":"tool","name":"hooked","root":"<project>/vibevm/vibedeps/org.goldenpkg.hooked/0.1.0","version":"0.1.0"},"version":"0.1.0"}],
 "notices":[],"requested":"install"},
{"chain":["validate","install"],"command":"lifecycle:plan","contributions":[
 {"handler":"builtin","key":"org.golden/golden#greet-install","phase":"install","point":"phase:install","provider":"org.golden/golden","tier":"host-declaration","version":"0.1.0"}],
 "notices":[],"requested":"install"},
{"command":"install:closure-diff","added":["org.goldenpkg/hooked@0.1.0 (root-edge)"],"removed":[],"changed":[],"lanes":[]},
{"command":"install","complete":true,"materialised":["vibevm/vibedeps/org.goldenpkg.hooked/0.1.0"],"nodes_regenerated":["."],"ok":true,
 "project":"<project>","pruned":[],"skipped":[],"unchanged":false,"contributions":[
 {"handler":"script","key":"org.goldenpkg/hooked#@vibe/hooks/pre-install@slot(org.goldenpkg/hooked@0.1.0)","phase":"install","point":"slot:pre-install","provider":"org.goldenpkg/hooked","status":"ok","tier":"dependency","reference":"org.goldenpkg/hooked#@vibe/hooks/pre-install","slot_target":{"group":"org.goldenpkg","kind":"tool","name":"hooked","root":"<project>/vibevm/vibedeps/org.goldenpkg.hooked/0.1.0","version":"0.1.0"},"stdout":"PRE-RAN\n","version":"0.1.0"},
 {"handler":"script","key":"org.goldenpkg/hooked#@vibe/hooks/post-install@slot(org.goldenpkg/hooked@0.1.0)","phase":"install","point":"slot:post-install","provider":"org.goldenpkg/hooked","status":"ok","tier":"dependency","reference":"org.goldenpkg/hooked#@vibe/hooks/post-install","slot_target":{"group":"org.goldenpkg","kind":"tool","name":"hooked","root":"<project>/vibevm/vibedeps/org.goldenpkg.hooked/0.1.0","version":"0.1.0"},"stdout":"POST-RAN\n","version":"0.1.0"},
 {"handler":"builtin","key":"org.golden/golden#greet-install","phase":"install","point":"phase:install","provider":"org.golden/golden","status":"ok","tier":"host-declaration","message":"GOLDEN-INSTALL","version":"0.1.0"}]}]"#,
    );
    assert_eq!(
        Value::Array(docs),
        expected,
        "the complete install-barrier stream"
    );
}

/// Surface (e): the quiet/human projections of the SAME fixture, verbatim —
/// the facts that live in `output::Context` today and must survive its
/// becoming a port.
#[test]
fn quiet_and_human_projection_of_the_same_fixture_is_pinned() {
    let human = {
        let user = UserScratch::new();
        let project = deploy_fixture(&user);
        let output = deploy(&user, project.path(), "human");
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    // Flush-left on purpose: a raw string whose bytes ARE the transcript.
    let expected_human = r#"  → will run `org.golden/golden#greet-generate` — point=phase:generate, handler=builtin:log, provider=org.golden/golden tier=host-declaration
  → will run `org.golden/golden#greet-build` — point=phase:build, handler=builtin:log, provider=org.golden/golden tier=host-declaration
  → log [org.golden/golden]: GOLDEN-GENERATE
  → log [org.golden/golden]: GOLDEN-BUILD
lifecycle `deploy`:
  → validate: ok
  → install: fresh
  → generate: ok
  → build: ok
  → test: no-op
  → create: no-op
  → verify: no-op
  → package: no-op
  → deploy: no-op
  → verification: unavailable (0 input(s), 0 artifact(s))
vibe lifecycle: deploy completed (9 phases, 2 contribution(s) selected, 2 executed, 2 ok, 0 fresh, 0 notice(s))
"#;
    assert_eq!(
        human, expected_human,
        "the full human transcript: narration, outcomes, heading, steps, summary",
    );

    let quiet = {
        let user = UserScratch::new();
        let project = deploy_fixture(&user);
        let output = deploy(&user, project.path(), "--quiet");
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    assert_eq!(
        quiet,
        "vibe lifecycle: deploy completed (9 phases, 2 contribution(s) selected, 2 executed, 2 ok, 0 fresh, 0 notice(s))\n",
        "quiet is exactly the one summary line",
    );
}
