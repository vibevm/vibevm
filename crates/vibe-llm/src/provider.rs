use specmark::spec;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI");

/// A role in the provider-independent text chat domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// One text-only chat message.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct ChatMessage {
    role: ChatRole,
    content: String,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn role(&self) -> ChatRole {
        self.role
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

/// Validated input for one synchronous chat call.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct ChatInput {
    messages: Vec<ChatMessage>,
}

impl ChatInput {
    pub fn new(messages: Vec<ChatMessage>) -> Result<Self, ChatInputError> {
        if messages.is_empty() {
            return Err(ChatInputError::EmptyMessages);
        }
        Ok(Self { messages })
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub enum ChatInputError {
    #[error(
        "a chat request requires at least one message \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: add a system or user message)"
    )]
    EmptyMessages,
}

/// The single assistant result consumed by the later create handler.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct ChatOutput {
    id: String,
    model: String,
    content: String,
    usage: Option<ChatUsage>,
}

impl ChatOutput {
    pub(crate) fn new(
        id: String,
        model: String,
        content: String,
        usage: Option<ChatUsage>,
    ) -> Self {
        Self {
            id,
            model,
            content,
            usage,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn usage(&self) -> Option<&ChatUsage> {
        self.usage.as_ref()
    }
}

/// Provider-independent token counts reported by a completed chat call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl ChatUsage {
    pub(crate) fn new(prompt_tokens: u32, completion_tokens: u32, total_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        }
    }

    pub fn prompt_tokens(self) -> u32 {
        self.prompt_tokens
    }

    pub fn completion_tokens(self) -> u32 {
        self.completion_tokens
    }

    pub fn total_tokens(self) -> u32 {
        self.total_tokens
    }
}

/// Object-safe, synchronous single-shot provider seam.
///
/// ```
/// use vibe_llm::{ChatInput, ChatMessage, ChatOutput, ChatRole, LLMProvider, ProviderError};
///
/// struct RefusingProvider;
/// impl LLMProvider for RefusingProvider {
///     fn chat(&self, _: &ChatInput) -> Result<ChatOutput, ProviderError> {
///         Err(ProviderError::CredentialRequiresHttps)
///     }
/// }
/// let provider: Box<dyn LLMProvider> = Box::new(RefusingProvider);
/// let input = ChatInput::new(vec![ChatMessage::new(ChatRole::User, "hello")]).unwrap();
/// assert!(provider.chat(&input).is_err());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub trait LLMProvider: Send + Sync {
    fn chat(&self, input: &ChatInput) -> Result<ChatOutput, ProviderError>;
}

/// Safe provider failures. No variant retains an API key or response body.
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub enum ProviderError {
    #[error(
        "could not encode the generated Chat Completions request \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: regenerate the JTD request type)"
    )]
    RequestEncoding,
    #[error(transparent)]
    Transport(#[from] crate::TransportError),
    #[error(
        "the OpenAI-compatible provider requires a non-empty model \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: configure a model in user or project LLM settings)"
    )]
    EmptyModel,
    #[error(
        "refused to pair an LLM credential with a non-HTTPS endpoint \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: configure an https endpoint or remove the credential)"
    )]
    CredentialRequiresHttps,
    #[error(
        "provider returned HTTP {status} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: inspect provider availability and configuration)"
    )]
    HttpStatus { status: u16 },
    #[error(
        "provider returned an invalid Chat Completions response after HTTP {status} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: select an OpenAI-compatible Chat Completions endpoint)"
    )]
    InvalidResponse { status: u16 },
    #[error(
        "provider returned no choices after HTTP {status} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: retry or select a provider that returns assistant choices)"
    )]
    EmptyChoices { status: u16 },
    #[error(
        "provider returned no usable assistant text after HTTP {status} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: use a text Chat Completions model)"
    )]
    EmptyContent { status: u16 },
}
