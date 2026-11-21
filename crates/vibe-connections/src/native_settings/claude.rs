//! Claude native Messages bearer adapter, not an OpenAI compatibility bridge.
use super::*;
pub struct ClaudeSettingsAdapter;
impl NativeSettingsAdapter for ClaudeSettingsAdapter {
    type Preview = vibe_secrets::ClaudeNativePreview;
    fn id(&self) -> &'static str {
        "claude"
    }
    fn supports(&self, kind: &str) -> bool {
        kind == "anthropic-messages-bearer"
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
        Ok(store.0.preview_claude_native_settings(home, connection)?)
    }
    fn apply(
        &self,
        store: &Store,
        preview: &Self::Preview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        Ok(store.0.apply_claude_native_settings(preview, confirmed)?)
    }
}
