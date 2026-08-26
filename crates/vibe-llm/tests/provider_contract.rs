use std::error::Error;
use std::sync::{Arc, Mutex};

use serde_json::json;
use vibe_llm::{
    ApiKey, ChatInput, ChatInputError, ChatMessage, ChatRole, ChatTransport, Endpoint, LLMProvider,
    OpenAiCompatibleProvider, ProviderError, TransportError, TransportResponse,
};

const KEY_CANARY: &str = "key-canary-must-never-appear";
const QUERY_CANARY: &str = "query-canary-must-never-appear";
const REQUEST_CANARY: &str = "request-body-canary-must-never-appear";
const BODY_CANARY: &str = "body-canary-must-never-appear";

#[derive(Debug, Clone)]
struct SeenRequest {
    endpoint: String,
    has_key: bool,
    body: Vec<u8>,
}

struct MockTransport {
    status: u16,
    request_id: Option<String>,
    body: Vec<u8>,
    seen: Mutex<Vec<SeenRequest>>,
}

impl MockTransport {
    fn responding(status: u16, request_id: Option<&str>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            request_id: request_id.map(ToOwned::to_owned),
            body: body.into(),
            seen: Mutex::new(Vec::new()),
        }
    }
}

impl ChatTransport for MockTransport {
    fn post_json(
        &self,
        endpoint: &Endpoint,
        api_key: Option<&ApiKey>,
        body: &[u8],
    ) -> Result<TransportResponse, TransportError> {
        self.seen.lock().unwrap().push(SeenRequest {
            endpoint: endpoint.as_str().to_owned(),
            has_key: api_key.is_some(),
            body: body.to_vec(),
        });
        Ok(TransportResponse::new(
            self.status,
            self.request_id.clone(),
            self.body.clone(),
        ))
    }
}

fn input() -> ChatInput {
    ChatInput::new(vec![
        ChatMessage::new(ChatRole::System, "Be concise."),
        ChatMessage::new(ChatRole::User, "Name the first phase."),
        ChatMessage::new(ChatRole::Assistant, "validate"),
    ])
    .unwrap()
}

fn canary_input() -> ChatInput {
    ChatInput::new(vec![ChatMessage::new(ChatRole::User, REQUEST_CANARY)]).unwrap()
}

fn provider(transport: Arc<MockTransport>, api_key: Option<ApiKey>) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new(
        "demo-chat-model",
        Endpoint::parse(
            "https://api.example.invalid/v1/chat/completions",
            api_key.is_some(),
        )
        .unwrap(),
        api_key,
        transport,
    )
    .unwrap()
}

fn error_chain_text(error: &(dyn Error + 'static)) -> String {
    let mut out = format!("{error:?}\n{error}");
    let mut source = error.source();
    while let Some(next) = source {
        out.push('\n');
        out.push_str(&format!("{next:?}\n{next}"));
        source = next.source();
    }
    out
}

#[test]
fn object_safe_chat_sends_exact_generated_json_with_stream_false() {
    let transport = Arc::new(MockTransport::responding(
        200,
        Some("req_demo_1"),
        br#"{"id":"chatcmpl-1","model":"demo-chat-model","choices":[{"message":{"role":"assistant","content":"validate"}}]}"#,
    ));
    let provider: Box<dyn LLMProvider> = Box::new(provider(
        transport.clone(),
        Some(ApiKey::new(KEY_CANARY).unwrap()),
    ));
    let output = provider.chat(&input()).unwrap();
    assert_eq!(output.id(), "chatcmpl-1");
    assert_eq!(output.model(), "demo-chat-model");
    assert_eq!(output.content(), "validate");
    assert!(output.usage().is_none());

    let seen = transport.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].endpoint,
        "https://api.example.invalid/v1/chat/completions"
    );
    assert!(seen[0].has_key);
    let request: serde_json::Value = serde_json::from_slice(&seen[0].body).unwrap();
    assert_eq!(
        request,
        json!({
            "model": "demo-chat-model",
            "messages": [
                {"role": "system", "content": "Be concise."},
                {"role": "user", "content": "Name the first phase."},
                {"role": "assistant", "content": "validate"}
            ],
            "stream": false
        })
    );
    assert_eq!(request.as_object().unwrap().len(), 3);
}

#[test]
fn standard_external_response_superset_projects_to_domain_output() {
    let body = include_bytes!(
        "../../../formats/corpora/llm/openai_compatible/e1/valid/chat_response_official_superset.json"
    );
    let transport = Arc::new(MockTransport::responding(200, None, body.as_slice()));
    let output = provider(transport, None).chat(&input()).unwrap();
    assert_eq!(output.id(), "chatcmpl-demo-official-1");
    assert_eq!(output.model(), "demo-chat-model");
    assert_eq!(output.content(), "validate");
    let usage = output.usage().unwrap();
    assert_eq!(usage.prompt_tokens(), 12);
    assert_eq!(usage.completion_tokens(), 1);
    assert_eq!(usage.total_tokens(), 13);
}

#[test]
fn empty_input_choices_and_content_null_fail_as_typed_values() {
    assert_eq!(
        ChatInput::new(Vec::new()).unwrap_err(),
        ChatInputError::EmptyMessages
    );

    let empty_choices = Arc::new(MockTransport::responding(
        200,
        Some("req-empty"),
        br#"{"id":"chatcmpl-empty","model":"m","choices":[]}"#,
    ));
    assert!(matches!(
        provider(empty_choices, None).chat(&input()),
        Err(ProviderError::EmptyChoices { .. })
    ));

    let empty_text = Arc::new(MockTransport::responding(
        200,
        None,
        br#"{"id":"chatcmpl-empty","model":"m","choices":[{"message":{"role":"assistant","content":"  "}}]}"#,
    ));
    assert!(matches!(
        provider(empty_text, None).chat(&input()),
        Err(ProviderError::EmptyContent { .. })
    ));

    let null_content = Arc::new(MockTransport::responding(
        200,
        None,
        br#"{"id":"chatcmpl-null","model":"m","choices":[{"message":{"role":"assistant","content":null}}]}"#,
    ));
    assert!(matches!(
        provider(null_content, None).chat(&input()),
        Err(ProviderError::InvalidResponse { .. })
    ));
}

#[test]
fn all_secret_canaries_reflected_as_request_id_are_omitted_from_diagnostics() {
    let key = ApiKey::new(KEY_CANARY).unwrap();
    assert!(!format!("{key:?}").contains(KEY_CANARY));
    assert!(!format!("{key}").contains(KEY_CANARY));

    for request_id in [KEY_CANARY, QUERY_CANARY, REQUEST_CANARY, BODY_CANARY] {
        for (status, body) in [
            (401, BODY_CANARY.as_bytes().to_vec()),
            (200, format!("not-json-{BODY_CANARY}").into_bytes()),
            (
                200,
                format!("{{\"model\":\"m\",\"choices\":[],\"secret\":\"{BODY_CANARY}\"}}")
                    .into_bytes(),
            ),
        ] {
            let transport = Arc::new(MockTransport::responding(status, Some(request_id), body));
            let provider = provider(transport, Some(ApiKey::new(KEY_CANARY).unwrap()));
            assert!(!format!("{provider:?}").contains(KEY_CANARY));
            let error = provider.chat(&canary_input()).unwrap_err();
            let rendered = error_chain_text(&error);
            for canary in [KEY_CANARY, QUERY_CANARY, REQUEST_CANARY, BODY_CANARY] {
                assert!(!rendered.contains(canary), "{rendered}");
            }
            assert!(rendered.contains(&status.to_string()), "{rendered}");
        }
    }

    for canary in [KEY_CANARY, QUERY_CANARY, REQUEST_CANARY, BODY_CANARY] {
        let reflected =
            TransportResponse::new(500, Some(canary.into()), BODY_CANARY.as_bytes().to_vec());
        let rendered = format!("{reflected:?}");
        assert!(!rendered.contains(canary), "{rendered}");
        assert!(!rendered.contains(BODY_CANARY), "{rendered}");
    }
}
