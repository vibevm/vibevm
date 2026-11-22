//! Fixed Codex Responses-native TOML sink. Native auth.json remains untouched.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use super::*;
use toml_edit::{DocumentMut, Item, Table, value};

pub struct CodexNativePreview(pub(super) PiNativePreview);
impl CodexNativePreview {
    pub fn preview_id(&self) -> &str {
        self.0.preview_id()
    }
}
impl std::fmt::Debug for CodexNativePreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexNativePreview([REDACTED])")
    }
}
fn document(bytes: &Option<Vec<u8>>) -> Result<DocumentMut, Error> {
    let text = match bytes {
        Some(bytes) => std::str::from_utf8(bytes).map_err(|_| Error::Invalid)?,
        None => "",
    };
    text.parse().map_err(|_| Error::Invalid)
}
pub(super) fn validate(bytes: &Option<Vec<u8>>) -> Result<(), Error> {
    let doc = document(bytes)?;
    if let Some(providers) = doc.get("model_providers") {
        let providers = providers.as_table().ok_or(Error::Invalid)?;
        if let Some(provider) = providers.get("vibevm") {
            let provider = provider.as_table().ok_or(Error::Invalid)?;
            for conflict in [
                "auth",
                "env_key",
                "gateway_oauth",
                "aws",
                "http_headers",
                "env_http_headers",
                "query_params",
            ] {
                if provider.contains_key(conflict) {
                    return Err(Error::Invalid);
                }
            }
        }
    }
    Ok(())
}
pub(super) fn merge(
    bytes: &Option<Vec<u8>>,
    endpoint: &str,
    model: &str,
    secret: &str,
) -> Result<Option<Vec<u8>>, Error> {
    validate(bytes)?;
    let mut doc = document(bytes)?;
    let original: toml::Value = toml::from_str(&doc.to_string()).map_err(|_| Error::Invalid)?;
    doc["model_provider"] = value("vibevm");
    doc["model"] = value(model);
    if !doc.contains_key("model_providers") {
        doc["model_providers"] = Item::Table(Table::new());
    }
    if !doc["model_providers"]
        .as_table()
        .ok_or(Error::Invalid)?
        .contains_key("vibevm")
    {
        doc["model_providers"]["vibevm"] = Item::Table(Table::new());
    }
    let provider = &mut doc["model_providers"]["vibevm"];
    provider["name"] = value("Vibe-managed native Responses");
    provider["base_url"] = value(endpoint);
    provider["wire_api"] = value("responses");
    provider["experimental_bearer_token"] = value(secret);
    provider["requires_openai_auth"] = value(false);
    let output = doc.to_string();
    let proposed: toml::Value = toml::from_str(&output).map_err(|_| Error::Invalid)?;
    if proposed == original {
        Ok(None)
    } else {
        Ok(Some(output.into_bytes()))
    }
}
impl Store {
    pub fn preview_codex_native_settings(
        &self,
        home: &Path,
        connection: &SecretId,
    ) -> Result<CodexNativePreview, Error> {
        Ok(CodexNativePreview(self.preview_native_settings(
            home,
            connection,
            Kind::Codex,
        )?))
    }
    pub fn apply_codex_native_settings(
        &self,
        preview: &CodexNativePreview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        self.apply_native_settings(&preview.0, confirmed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codex_native_toml_keeps_comments_neighbors_and_noop() {
        assert!(
            validate(&Some(
                b"[model_providers.vibevm.env_http_headers]\nAuthorization='SYNTHETIC_ENV'\n"
                    .to_vec()
            ))
            .is_err()
        );
        let before =
            Some(b"# keep\nmodel = 'old'\n[model_providers.neighbor]\nname = 'keep'\n".to_vec());
        let output = merge(
            &before,
            "http://127.0.0.1:9/v1",
            "synthetic",
            "synthetic-key",
        )
        .unwrap()
        .unwrap();
        assert!(std::str::from_utf8(&output).unwrap().contains("# keep"));
        let parsed: toml::Value = toml::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
        assert_eq!(
            parsed["model_providers"]["neighbor"]["name"].as_str(),
            Some("keep")
        );
        assert!(
            merge(
                &Some(output),
                "http://127.0.0.1:9/v1",
                "synthetic",
                "synthetic-key"
            )
            .unwrap()
            .is_none()
        );
        assert!(
            validate(&Some(
                b"[model_providers.vibevm]\nenv_key='SYNTHETIC'\n".to_vec()
            ))
            .is_err()
        );
    }
}
