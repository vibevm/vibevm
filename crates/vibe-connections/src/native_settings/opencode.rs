//! OpenCode native config adapter over its concrete protected sink.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use super::*;
pub struct OpenCodeSettingsAdapter;
impl NativeSettingsAdapter for OpenCodeSettingsAdapter {
    type Preview = vibe_secrets::OpenCodeNativePreview;
    fn id(&self) -> &'static str {
        "opencode"
    }
    fn supports(&self, kind: &str) -> bool {
        kind == "openai-chat-completions"
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
        Ok(store.0.preview_opencode_native_settings(home, connection)?)
    }
    fn apply(
        &self,
        store: &Store,
        preview: &Self::Preview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        Ok(store.0.apply_opencode_native_settings(preview, confirmed)?)
    }
}
