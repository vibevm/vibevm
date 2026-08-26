//! Synchronous LLM provider seam and the epoch-1 OpenAI-compatible adapter.
//!
//! Provider JSON is owned by JTD schemas and generated in `vibe-wire`; this
//! crate owns domain values, configuration resolution, endpoint policy, secret
//! handling, and transport behaviour around those generated types.

#![forbid(unsafe_code)]

mod config;
mod endpoint;
mod openai_compatible;
mod provider;
mod secret;
mod transport;

pub use config::{
    CredentialReadError, CredentialReader, CredentialSource, EffectiveLlmConfig,
    EffectiveLlmConfigError, SystemCredentialReader, resolve_effective_config,
};
pub use endpoint::{Endpoint, EndpointError};
pub use openai_compatible::OpenAiCompatibleProvider;
pub use provider::{
    ChatInput, ChatInputError, ChatMessage, ChatOutput, ChatRole, ChatUsage, LLMProvider,
    ProviderError,
};
pub use secret::{ApiKey, ApiKeyError};
pub use transport::{
    CHAT_CONNECT_TIMEOUT, CHAT_REQUEST_TIMEOUT, ChatTransport, MAX_CHAT_RESPONSE_BYTES,
    ReqwestChatTransport, TransportError, TransportResponse,
};

/// The one provider id R7 implements. Names of future adapters are not aliases.
pub const OPENAI_COMPATIBLE_PROVIDER_ID: &str = "openai-compatible";
