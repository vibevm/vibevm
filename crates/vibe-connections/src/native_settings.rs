//! Internal native-settings adapter composition; no raw credential surface.
use super::*;

pub trait NativeSettingsAdapter {
    type Preview;
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
        store.details(connection)?.metadata.validate()?;
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
