//! Qwen adapter facade over its concrete protected native writer.
use super::*;
pub struct QwenSettingsAdapter;
impl NativeSettingsAdapter for QwenSettingsAdapter {
    type Preview = vibe_secrets::QwenNativePreview;
    fn id(&self) -> &'static str {
        "qwen"
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
        store.details(connection)?.metadata.validate()?;
        Ok(store.0.preview_qwen_native_settings(home, connection)?)
    }
    fn apply(
        &self,
        store: &Store,
        preview: &Self::Preview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        Ok(store.0.apply_qwen_native_settings(preview, confirmed)?)
    }
}
