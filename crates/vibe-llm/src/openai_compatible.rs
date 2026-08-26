use std::sync::Arc;

use specmark::spec;
use vibe_wire::generated::llm::openai_compatible::e1::chat_request::{
    ChatRequest, Message, MessageRole,
};
use vibe_wire::generated::llm::openai_compatible::e1::chat_response::ChatResponse;

use crate::{
    ApiKey, ChatInput, ChatOutput, ChatRole, ChatTransport, ChatUsage, Endpoint, LLMProvider,
    ProviderError, ReqwestChatTransport,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI");

/// The epoch-1 OpenAI-compatible synchronous Chat Completions adapter.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct OpenAiCompatibleProvider {
    model: String,
    endpoint: Endpoint,
    api_key: Option<ApiKey>,
    transport: Arc<dyn ChatTransport>,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        model: impl Into<String>,
        endpoint: Endpoint,
        api_key: Option<ApiKey>,
        transport: Arc<dyn ChatTransport>,
    ) -> Result<Self, ProviderError> {
        let model = model.into();
        if model.trim().is_empty() {
            return Err(ProviderError::EmptyModel);
        }
        if api_key.is_some() && !endpoint.accepts_api_key() {
            return Err(ProviderError::CredentialRequiresHttps);
        }
        Ok(Self {
            model,
            endpoint,
            api_key,
            transport,
        })
    }

    /// Construct the production blocking provider.
    ///
    /// # Panics
    ///
    /// Upstream `reqwest::blocking` may panic when its client is constructed
    /// or dropped inside an async runtime. Async/MCP callers must cross a
    /// dedicated blocking boundary before constructing or owning this value.
    pub fn with_reqwest(
        model: impl Into<String>,
        endpoint: Endpoint,
        api_key: Option<ApiKey>,
    ) -> Result<Self, ProviderError> {
        let transport = Arc::new(ReqwestChatTransport::new()?);
        Self::new(model, endpoint, api_key, transport)
    }
}

impl std::fmt::Debug for OpenAiCompatibleProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatibleProvider")
            .field("model", &self.model)
            .field("endpoint", &self.endpoint)
            .field("api_key", &self.api_key)
            .field("transport", &"<transport>")
            .finish()
    }
}

impl LLMProvider for OpenAiCompatibleProvider {
    fn chat(&self, input: &ChatInput) -> Result<ChatOutput, ProviderError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: input.messages().iter().map(message_to_wire).collect(),
            stream: false,
        };
        let body = serde_json::to_vec(&request).map_err(|_| ProviderError::RequestEncoding)?;
        let response = self
            .transport
            .post_json(&self.endpoint, self.api_key.as_ref(), &body)?;
        if !(200..300).contains(&response.status()) {
            return Err(ProviderError::HttpStatus {
                status: response.status(),
            });
        }

        let decoded: ChatResponse = serde_json::from_slice(response.body()).map_err(|_| {
            ProviderError::InvalidResponse {
                status: response.status(),
            }
        })?;
        let usage = decoded.usage.map(|usage| {
            ChatUsage::new(
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens,
            )
        });
        let choice =
            decoded
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| ProviderError::EmptyChoices {
                    status: response.status(),
                })?;
        if choice.message.content.trim().is_empty() {
            return Err(ProviderError::EmptyContent {
                status: response.status(),
            });
        }
        Ok(ChatOutput::new(
            decoded.id,
            decoded.model,
            choice.message.content,
            usage,
        ))
    }
}

fn message_to_wire(message: &crate::ChatMessage) -> Message {
    Message {
        role: match message.role() {
            ChatRole::System => MessageRole::System,
            ChatRole::User => MessageRole::User,
            ChatRole::Assistant => MessageRole::Assistant,
        },
        content: message.content().to_owned(),
    }
}
