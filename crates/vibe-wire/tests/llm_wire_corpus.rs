//! Authored valid and invalid documents for the epoch-1 OpenAI-compatible
//! Chat Completions request/response wire.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
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
fn invalid_request_corpus_is_rejected_by_generated_types() {
    rejects_all::<ChatRequest>("invalid", "chat_request_");
}

#[test]
fn invalid_response_corpus_is_rejected_by_generated_types() {
    rejects_all::<ChatResponse>("invalid", "chat_response_");
}

#[test]
fn both_jtd_contracts_are_closed_and_contain_no_json_catch_all() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for schema in ["chat_request.jtd.json", "chat_response.jtd.json"] {
        schema_is_closed_and_has_no_catch_all(
            &root.join("schemas/llm/openai_compatible/e1").join(schema),
        );
    }
}
