//! Internal native-settings adapter composition; no raw credential surface.
use super::*;

/// A prepared native-settings change. Its id is what the operator confirms;
/// applying requires presenting that same id back.
///
/// ```
/// use vibe_connections::NativeSettingsPreview;
///
/// struct Prepared(String);
/// impl NativeSettingsPreview for Prepared {
///     fn preview_id(&self) -> &str {
///         &self.0
///     }
/// }
/// assert_eq!(Prepared("sha256:synthetic".into()).preview_id(), "sha256:synthetic");
/// ```
pub trait NativeSettingsPreview {
    fn preview_id(&self) -> &str;
}
impl NativeSettingsPreview for vibe_secrets::PiNativePreview {
    fn preview_id(&self) -> &str {
        self.preview_id()
    }
}
impl NativeSettingsPreview for vibe_secrets::QwenNativePreview {
    fn preview_id(&self) -> &str {
        self.preview_id()
    }
}

impl NativeSettingsPreview for vibe_secrets::CodexNativePreview {
    fn preview_id(&self) -> &str {
        self.preview_id()
    }
}
mod codex;
pub use codex::CodexSettingsAdapter;

/// One harness's native settings writer: it previews a change for a stored
/// connection and applies exactly the preview the operator confirmed.
///
/// ```
/// use vibe_connections::{ClaudeSettingsAdapter, NativeSettingsAdapter, PiSettingsAdapter};
///
/// fn admits(adapter: &impl NativeSettingsAdapter, api_kind: &str) -> bool {
///     adapter.supports(api_kind)
/// }
/// assert_eq!(PiSettingsAdapter.id(), "pi");
/// assert!(admits(&PiSettingsAdapter, "openai-chat-completions"));
/// // An adapter refuses every protocol it has no verified writer for.
/// assert!(!admits(&PiSettingsAdapter, "anthropic-messages-bearer"));
/// assert!(admits(&ClaudeSettingsAdapter, "anthropic-messages-bearer"));
/// ```
pub trait NativeSettingsAdapter {
    type Preview: NativeSettingsPreview;
    fn id(&self) -> &'static str;
    fn supports(&self, api_kind: &str) -> bool;
    fn preview(
        &self,
        store: &Store,
        home: &Path,
        connection: &SecretId,
    ) -> Result<Self::Preview, Error>;
    fn apply(&self, store: &Store, preview: &Self::Preview, confirmed: &str)
    -> Result<bool, Error>;
}

mod qwen;
pub use qwen::QwenSettingsAdapter;

/// Pi's concrete writer stays with the private keyring reader, not this facade.
pub struct PiSettingsAdapter;
impl NativeSettingsAdapter for PiSettingsAdapter {
    type Preview = vibe_secrets::PiNativePreview;
    fn id(&self) -> &'static str {
        "pi"
    }
    fn supports(&self, api_kind: &str) -> bool {
        api_kind == "openai-chat-completions"
    }
    fn preview(
        &self,
        store: &Store,
        home: &Path,
        connection: &SecretId,
    ) -> Result<Self::Preview, Error> {
        let metadata = store.details(connection)?.metadata;
        metadata.validate()?;
        if !self.supports(&metadata.api_kind) {
            return Err(Error::Invalid);
        }
        Ok(store.0.preview_pi_native_settings(home, connection)?)
    }
    fn apply(
        &self,
        store: &Store,
        preview: &Self::Preview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        Ok(store.0.apply_pi_native_settings(preview, confirmed)?)
    }
}

impl NativeSettingsPreview for vibe_secrets::OpenCodeNativePreview {
    fn preview_id(&self) -> &str {
        self.preview_id()
    }
}
mod opencode;
pub use opencode::OpenCodeSettingsAdapter;

impl NativeSettingsPreview for vibe_secrets::HermesNativePreview {
    fn preview_id(&self) -> &str {
        self.preview_id()
    }
}
mod hermes;
pub use hermes::HermesSettingsAdapter;

impl NativeSettingsPreview for vibe_secrets::ClaudeNativePreview {
    fn preview_id(&self) -> &str {
        self.preview_id()
    }
}
mod claude;
pub use claude::ClaudeSettingsAdapter;
