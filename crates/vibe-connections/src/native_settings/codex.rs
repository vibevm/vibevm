//! Codex native Responses adapter, never a Chat Completions translator.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use super::*;
pub struct CodexSettingsAdapter;
impl NativeSettingsAdapter for CodexSettingsAdapter {
    type Preview = vibe_secrets::CodexNativePreview;
    fn id(&self) -> &'static str {
        "codex"
    }
    fn supports(&self, api_kind: &str) -> bool {
        api_kind == "openai-responses"
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
        Ok(store.0.preview_codex_native_settings(home, connection)?)
    }
    fn apply(
        &self,
        store: &Store,
        preview: &Self::Preview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        Ok(store.0.apply_codex_native_settings(preview, confirmed)?)
    }
}
