//! Authored valid and invalid documents for the epoch-1 OpenAI-compatible
//! Chat Completions request/response wire and the agent-result document the
//! create-phase handler demands back inside the assistant message.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::llm::openai_compatible::e1::agent_result::AgentResult;
use vibe_wire::generated::llm::openai_compatible::e1::chat_request::{ChatRequest, MessageRole};
use vibe_wire::generated::llm::openai_compatible::e1::chat_response::{
    ChatResponse, MessageRole as ResponseMessageRole,
};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/llm/openai_compatible/e1")
}

fn read_valid<T: DeserializeOwned + Serialize>(name: &str) -> T {
    let path = corpus().join("valid").join(name);
    let bytes = std::fs::read(&path).unwrap();
    let authored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let value: T = serde_json::from_value(authored.clone()).unwrap();
    let round_trip = serde_json::to_value(&value).unwrap();
    assert_eq!(round_trip, authored, "{} loses data", path.display());
    value
}

fn rejects_all<T: DeserializeOwned>(dir: &str, prefix: &str) {
    let dir = corpus().join(dir);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no invalid {prefix} corpus documents");
    for path in paths {
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            serde_json::from_slice::<T>(&bytes).is_err(),
            "{} unexpectedly parsed",
            path.display()
        );
    }
}

fn schema_is_closed_and_has_no_catch_all(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    let schema: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let text = serde_json::to_string(&schema).unwrap();
    assert!(!text.contains("additionalProperties"));
    assert!(!text.contains("\"values\""));
    assert!(!text.contains("\"type\":{}"));
}

#[test]
fn request_corpus_round_trips_ordered_closed_roles_and_sync_mode() {
    let request: ChatRequest = read_valid("chat_request.json");
    assert_eq!(request.model, "demo-chat-model");
    assert!(!request.stream);
    assert_eq!(
        request
            .messages
            .iter()
            .map(|message| &message.role)
            .collect::<Vec<_>>(),
        [
            &MessageRole::System,
            &MessageRole::User,
            &MessageRole::Assistant,
            &MessageRole::User,
        ]
    );
}

#[test]
fn response_corpus_round_trips_ordered_assistant_text() {
    let response: ChatResponse = read_valid("chat_response.json");
    assert_eq!(response.id, "chatcmpl-demo-1");
    assert_eq!(response.model, "demo-chat-model");
    assert_eq!(response.choices.len(), 2);
    assert_eq!(
        response.choices[0].message.role,
        ResponseMessageRole::Assistant
    );
    assert_eq!(response.choices[0].message.content, "validate");
    assert_eq!(response.choices[1].message.content, "install");
    assert!(response.usage.is_none());
}

#[test]
fn response_projection_accepts_the_standard_external_superset() {
    let path = corpus()
        .join("valid")
        .join("chat_response_official_superset.json");
    let authored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let response: ChatResponse = serde_json::from_value(authored).unwrap();
    assert_eq!(response.id, "chatcmpl-demo-official-1");
    assert_eq!(response.model, "demo-chat-model");
    assert_eq!(response.choices[0].message.content, "validate");
    let usage = response.usage.as_ref().unwrap();
    assert_eq!(usage.prompt_tokens, 12);
    assert_eq!(usage.completion_tokens, 1);
    assert_eq!(usage.total_tokens, 13);

    let projected = serde_json::to_value(response).unwrap();
    assert_eq!(
        projected.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["choices", "id", "model", "usage"]
    );
    assert_eq!(
        projected["choices"][0]["message"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["content", "role"]
    );
    assert_eq!(
        projected["usage"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["completion_tokens", "prompt_tokens", "total_tokens"]
    );
}

#[test]
fn agent_result_corpus_round_trips_ordered_multi_output_rows() {
    let result: AgentResult = read_valid("agent_result.json");
    assert_eq!(
        result
            .outputs
            .iter()
            .map(|output| output.path.as_str())
            .collect::<Vec<_>>(),
        ["docs/guide.md", "docs/reference.md"],
        "declaration order is the wire order the handler compares against"
    );
    assert!(result.outputs[0].content.starts_with("# Guide"));
    assert!(result.outputs[1].content.starts_with("# Reference"));
}

/// The empty array is representable — the refusal is the handler's contract
/// comparison, not a wire-level trick that would also reject a legitimate
/// future zero-output contract at the parser.
#[test]
fn agent_result_wire_represents_an_empty_output_set() {
    let empty: AgentResult = serde_json::from_str(r#"{"outputs":[]}"#).unwrap();
    assert!(empty.outputs.is_empty());
    assert_eq!(
        serde_json::to_string(&empty).unwrap(),
        r#"{"outputs":[]}"#,
        "a required collection is emitted, never omitted"
    );
}

#[test]
fn invalid_request_corpus_is_rejected_by_generated_types() {
    rejects_all::<ChatRequest>("invalid", "chat_request_");
}

#[test]
fn invalid_response_corpus_is_rejected_by_generated_types() {
    rejects_all::<ChatResponse>("invalid", "chat_response_");
}

#[test]
fn invalid_agent_result_corpus_is_rejected_by_generated_types() {
    rejects_all::<AgentResult>("invalid", "agent_result_");
}

/// The agent result is OUR contract, published in the prompt and read only
/// here, so an unknown member is a provider that did not honour it — not a
/// newer version of someone else's API. Forward-compatible tolerance belongs
/// to the two Chat Completions wires next door, and this asserts the two
/// policies really do differ rather than merely being described differently.
#[test]
fn the_agent_result_is_strict_while_the_foreign_chat_wire_stays_tolerant() {
    let extra_root = r#"{"outputs":[{"path":"a.md","content":"x"}],"notes":"extra"}"#;
    let extra_member = r#"{"outputs":[{"path":"a.md","content":"x","mode":"0755"}]}"#;
    for (label, document) in [("root", extra_root), ("output", extra_member)] {
        assert!(
            serde_json::from_str::<AgentResult>(document).is_err(),
            "an extra {label} member must be refused",
        );
    }
    let foreign = r#"{"id":"x","model":"m","choices":[{"index":0,
        "message":{"role":"assistant","content":"hi","refusal":null}}],"system_fingerprint":"fp"}"#;
    assert!(
        serde_json::from_str::<ChatResponse>(foreign).is_ok(),
        "the foreign chat wire must stay forward-compatible",
    );
}

#[test]
fn every_jtd_contract_is_closed_and_contains_no_json_catch_all() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for schema in [
        "chat_request.jtd.json",
        "chat_response.jtd.json",
        "agent_result.jtd.json",
    ] {
        schema_is_closed_and_has_no_catch_all(
            &root.join("schemas/llm/openai_compatible/e1").join(schema),
        );
    }
}
