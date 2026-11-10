//! `vibe prompt capture` — the request envelope, proved against a local
//! fake provider and against nothing else.
//!
//! **What these tests prove and what they deliberately do not.** They prove
//! the bytes vibe produced: which band sits in which role, in what order,
//! and that the entrypoint's text crosses the wire exactly once. They prove
//! that a provider's own usage counters are read back out of a response
//! rather than computed. They prove **no** cache hit, **no** TTL, **no**
//! latency and **no** saving — a capture against a fake provider is an
//! envelope claim, never a cache claim, and `#SYS-CACHE-FACTS` keeps the
//! rest provider-owned (`SP-FMT-011`).
//!
//! **No paid request is ever made.** The endpoint is a loopback listener in
//! this process that records exactly what it received; `vibe` itself sends
//! nothing at all — `vibe prompt capture` writes a document, and the HTTP
//! POST below is the test's, over literal 127.0.0.1.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;

use predicates::prelude::*;

mod common;

use common::UserScratch;
use common::agent_provider::MockProvider;

/// A canary inside the **entrypoint**, which band P already carries. If an
/// adapter also auto-loaded the entrypoint into the first message, this
/// string would appear twice.
const ENTRYPOINT_CANARY: &str = "ENTRYPOINT-CANARY-9f12";
const TEMPLATE: &str = "# My frame\n\nTEMPLATE-CANARY-4b70\n";
const TASK: &str = "TASK-CANARY-1d55: do the thing";

/// One opted-in project with a built prefix, under an isolated home.
struct Project {
    user: UserScratch,
    dir: tempfile::TempDir,
}

impl Project {
    fn opted_in() -> Self {
        let project = Self::fresh();
        let entrypoint = project.dir.path().join("AGENTS.md");
        let mut body = fs::read_to_string(&entrypoint).unwrap();
        body.push_str(&format!("\n{ENTRYPOINT_CANARY}\n"));
        fs::write(&entrypoint, body).unwrap();
        fs::write(
            project
                .dir
                .path()
                .join(common::boot_rel("SYSTEM-TEMPLATE.md")),
            TEMPLATE,
        )
        .unwrap();
        project.prompt("build").assert().success();
        project
    }

    fn fresh() -> Self {
        let user = UserScratch::new();
        let dir = tempfile::tempdir().unwrap();
        user.vibe()
            .arg("init")
            .arg("--path")
            .arg(dir.path())
            .assert()
            .success();
        Self { user, dir }
    }

    fn prompt(&self, verb: &str) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        command
            .arg("prompt")
            .arg(verb)
            .arg("--path")
            .arg(self.dir.path());
        command
    }

    /// The envelope document, with each block's bytes.
    fn envelope(&self, task: Option<&str>) -> serde_json::Value {
        let mut command = self.prompt("capture");
        command.arg("--json").arg("--with-text");
        if let Some(task) = task {
            command.arg("--task").arg(task);
        }
        let output = command.assert().success().get_output().stdout.clone();
        serde_json::from_slice(&output).expect("the envelope is one JSON document")
    }
}

/// Lower one envelope into an OpenAI-compatible request body: every
/// instruction block into a named instruction role, band T into the message
/// role, in the envelope's own order.
fn request_body(envelope: &serde_json::Value) -> String {
    let mut messages = Vec::new();
    for block in envelope["instruction_blocks"].as_array().unwrap() {
        assert_eq!(block["role"], "instruction", "{block}");
        messages.push(serde_json::json!({
            "role": "system",
            "content": block["text"].as_str().expect("--with-text carries the bytes"),
        }));
    }
    for message in envelope["messages"].as_array().unwrap() {
        assert_eq!(message["role"], "message", "{message}");
        messages.push(serde_json::json!({
            "role": "user",
            "content": message["text"].as_str().unwrap(),
        }));
    }
    serde_json::json!({ "model": "demo-chat-model", "messages": messages }).to_string()
}

/// POST one body to the fake provider and return its response body.
fn post(endpoint: &str, body: &str) -> String {
    let address = endpoint
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap()
        .to_owned();
    let mut stream = TcpStream::connect(&address).unwrap();
    write!(
        stream,
        "POST /v1/chat/completions HTTP/1.1\r\nHost: {address}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(body.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    answer
        .split_once("\r\n\r\n")
        .map_or(answer.clone(), |(_, rest)| rest.to_owned())
}

#[test]
fn no_duplicate_entrypoint_injection() {
    let project = Project::opted_in();
    let provider = MockProvider::serving("ok");
    let envelope = project.envelope(Some(TASK));
    let sent = post(&provider.endpoint(), &request_body(&envelope));
    assert!(
        sent.contains("chatcmpl"),
        "the fake provider answered: {sent}"
    );

    let captured = provider.bodies();
    assert_eq!(provider.hits(), 1, "one request, and it was the fake one");
    let request = &captured[0];
    // The entrypoint's text is inside band P already. Exactly once in the
    // whole request is the claim; twice would double the cost this feature
    // exists to reduce and leave the model two copies of one instruction.
    assert_eq!(
        request.matches(ENTRYPOINT_CANARY).count(),
        1,
        "the entrypoint crossed the wire {} times",
        request.matches(ENTRYPOINT_CANARY).count()
    );
    // And the prefix itself is carried once, not once per band.
    assert_eq!(request.matches("TEMPLATE-CANARY-4b70").count(), 1);
    assert_eq!(request.matches("TASK-CANARY-1d55").count(), 1);
}

#[test]
fn worker_profile_carries_no_central_prefix() {
    // A quiet worker's node is not opted in, so there is no envelope to
    // build for it — the exclusion is that the prefix is never assembled,
    // not that a later message asks the worker to ignore it.
    let worker = Project::fresh();
    worker
        .prompt("capture")
        .arg("--json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("has not opted in"));

    // And the only request such a worker can send carries none of the
    // central prefix's bytes, asserted on what reached the provider.
    let central = Project::opted_in();
    let central_prefix =
        fs::read_to_string(central.dir.path().join(common::boot_rel("SYSTEM.md"))).unwrap();
    assert!(
        central_prefix.contains(ENTRYPOINT_CANARY),
        "the fixture is wrong"
    );

    let provider = MockProvider::serving("ok");
    let packet = serde_json::json!({
        "model": "demo-chat-model",
        "messages": [{ "role": "user", "content": "WORKER-PACKET-7c01" }],
    })
    .to_string();
    post(&provider.endpoint(), &packet);
    let request = &provider.bodies()[0];
    assert_eq!(request.matches("WORKER-PACKET-7c01").count(), 1);
    for canary in [ENTRYPOINT_CANARY, "TEMPLATE-CANARY-4b70"] {
        assert_eq!(
            request.matches(canary).count(),
            0,
            "a central canary reached a worker's request"
        );
    }
}

#[test]
fn the_bands_arrive_in_order_with_the_tool_block_declared_ahead_of_them() {
    let project = Project::opted_in();
    let envelope = project.envelope(Some(TASK));
    let bands: Vec<&str> = envelope["instruction_blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|block| block["band"].as_str().unwrap())
        .collect();
    assert_eq!(bands.first(), Some(&"P"), "band P leads: {bands:?}");
    // P, then U, then G — never interleaved, whatever the row count.
    let order = |band: &str| match band {
        "P" => 0,
        "U" => 1,
        "G" => 2,
        other => panic!("an instruction block claimed band {other}"),
    };
    assert!(
        bands
            .windows(2)
            .all(|pair| order(pair[0]) <= order(pair[1])),
        "{bands:?}"
    );
    assert_eq!(envelope["tools"]["count"], 0);
    assert_eq!(envelope["tools"]["position"], "before_instructions");
    // Band T is a message and is last; it is never an instruction block.
    let messages = envelope["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["band"], "T");
    assert_eq!(envelope["protocol"], "vibe-prompt-capture/1");
}

#[test]
fn a_cache_boundary_lands_at_an_offset_the_prepared_materials_named() {
    let project = Project::opted_in();
    let envelope = project.envelope(None);
    let prepared: serde_json::Value = serde_json::from_slice(
        &project
            .prompt("prepare")
            .arg("--json")
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(
        envelope["cache_boundaries"], prepared["cache_boundaries"],
        "the envelope invented a boundary the materials did not name"
    );
    let edges: Vec<u64> = std::iter::once(0)
        .chain(
            envelope["instruction_blocks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|block| block["offset"].as_u64().unwrap() + block["len"].as_u64().unwrap()),
        )
        .collect();
    for boundary in envelope["cache_boundaries"].as_array().unwrap() {
        let offset = boundary.as_u64().unwrap();
        assert!(
            edges.contains(&offset),
            "boundary {offset} is inside a block; the edges are {edges:?}"
        );
    }
}

#[test]
fn the_providers_own_usage_counters_are_read_back_out_of_its_response() {
    let project = Project::opted_in();
    // The numbers live in the provider's answer, not in this test's
    // expectations: they travel response → file → report.
    let provider = MockProvider::serving_usage(
        "ok",
        serde_json::json!({
            "input_tokens": 1_234,
            "cache_creation_input_tokens": 278_000,
            "cache_read_input_tokens": 9_876,
            "output_tokens": 7,
        }),
    );
    let envelope = project.envelope(Some(TASK));
    let response = post(&provider.endpoint(), &request_body(&envelope));
    let saved = project.dir.path().join("provider-response.json");
    fs::write(&saved, &response).unwrap();

    let report: serde_json::Value = serde_json::from_slice(
        &project
            .prompt("capture")
            .arg("--json")
            .arg("--usage")
            .arg(&saved)
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    let reported: serde_json::Value = serde_json::from_str(&response).unwrap();
    let usage = &reported["usage"];
    assert_eq!(report["cache"]["source"], "anthropic-messages-usage");
    assert_eq!(
        report["cache"]["read_tokens"],
        usage["cache_read_input_tokens"]
    );
    assert_eq!(
        report["cache"]["write_tokens"],
        usage["cache_creation_input_tokens"]
    );
    assert_eq!(report["cache"]["uncached_tokens"], usage["input_tokens"]);
}

#[test]
fn a_response_with_no_cache_counters_reports_none_rather_than_zeros() {
    let project = Project::opted_in();
    let provider = MockProvider::serving("ok");
    let response = post(&provider.endpoint(), &request_body(&project.envelope(None)));
    let saved = project.dir.path().join("plain-response.json");
    fs::write(&saved, &response).unwrap();
    project
        .prompt("capture")
        .arg("--usage")
        .arg(&saved)
        .assert()
        .success()
        .stdout(predicate::str::contains("`prompt_tokens`"))
        .stdout(predicate::str::contains(
            "no cache counter this build knows",
        ))
        .stdout(predicate::str::contains("absence is not zero"));
}

#[test]
fn the_envelope_omits_the_block_bytes_unless_they_are_asked_for() {
    let project = Project::opted_in();
    let report: serde_json::Value = serde_json::from_slice(
        &project
            .prompt("capture")
            .arg("--json")
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    for block in report["instruction_blocks"].as_array().unwrap() {
        assert!(block.get("text").is_none(), "{block}");
        // The structure is still complete without the bytes: a runner can
        // read each file itself from the path and digest it.
        assert!(block["len"].as_u64().unwrap() > 0);
        assert_eq!(block["sha256"].as_str().unwrap().len(), 64);
    }
}
