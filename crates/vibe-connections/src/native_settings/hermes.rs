//! Hermes native config facade; secret remains inside fixed protected sink.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use super::*;
pub struct HermesSettingsAdapter;
impl NativeSettingsAdapter for HermesSettingsAdapter {
    type Preview = vibe_secrets::HermesNativePreview;
    fn id(&self) -> &'static str {
        "hermes"
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
        Ok(store.0.preview_hermes_native_settings(home, connection)?)
    }
    fn apply(
        &self,
        store: &Store,
        preview: &Self::Preview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        Ok(store.0.apply_hermes_native_settings(preview, confirmed)?)
    }
}
