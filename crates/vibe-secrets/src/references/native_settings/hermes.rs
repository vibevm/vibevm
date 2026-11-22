//! Fixed native Hermes block-mapping YAML sink; no command or pool mutation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use super::*;
pub struct HermesNativePreview(pub(super) PiNativePreview);
impl HermesNativePreview {
    pub fn preview_id(&self) -> &str {
        self.0.preview_id()
    }
}
impl std::fmt::Debug for HermesNativePreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HermesNativePreview([REDACTED])")
    }
}
fn text(bytes: &Option<Vec<u8>>) -> Result<&str, Error> {
    match bytes {
        Some(bytes) => std::str::from_utf8(bytes).map_err(|_| Error::Invalid),
        None => Ok(""),
    }
}
pub(super) fn validate(bytes: &Option<Vec<u8>>) -> Result<(), Error> {
    let text = text(bytes)?;
    yaml_mapping::validate(text)?;
    for key in [
        "key_cmd",
        "key_env",
        "api_key_env",
        "headers",
        "extra_headers",
        "extra_body",
        "enabled",
    ] {
        if yaml_mapping::contains(text, &["providers", "vibevm", key])? {
            return Err(Error::Invalid);
        }
    }
    // Legacy custom aliases and credential strategies can select an alternate
    // credential/endpoint. They require a separately witnessed native operation.
    for key in ["custom_providers", "credential_pool_strategies"] {
        if yaml_mapping::contains(text, &[key])? {
            return Err(Error::Invalid);
        }
    }
    Ok(())
}
pub(super) fn auth_witness(dir: &Pinned) -> Result<Option<String>, Error> {
    let bytes = dir
        .read_private_metadata("auth.json", 4 * 1024 * 1024)
        .map_err(|_| Error::Unavailable)?;
    if let Some(bytes) = &bytes {
        let data: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        let root = data.as_object().ok_or(Error::Invalid)?;
        if let Some(pool) = root.get("credential_pool")
            && !pool.as_object().is_some_and(|p| p.is_empty())
        {
            return Err(Error::Invalid);
        }
        if let Some(providers) = root.get("providers") {
            let providers = providers.as_object().ok_or(Error::Invalid)?;
            if providers
                .keys()
                .any(|k| matches!(k.as_str(), "vibevm" | "custom:vibevm" | "custom"))
            {
                return Err(Error::Invalid);
            }
        }
    }
    Ok(bytes.as_ref().map(|b| digest(b)))
}
pub(super) fn merge(
    bytes: &Option<Vec<u8>>,
    endpoint: &str,
    model: &str,
    secret: &str,
) -> Result<Option<Vec<u8>>, Error> {
    validate(bytes)?;
    // Hermes expands config strings through its own environment resolver;
    // unsupported interpolation is refused, never changed or executed.
    if [endpoint, model, secret].iter().any(|s| s.contains('$')) {
        return Err(Error::Invalid);
    }
    let before = text(bytes)?;
    let mut out = before.to_owned();
    for (path, value) in [
        (&["model", "provider"][..], "custom:vibevm"),
        (&["model", "default"][..], model),
        (&["providers", "vibevm", "base_url"][..], endpoint),
        (&["providers", "vibevm", "api_key"][..], secret),
        (&["providers", "vibevm", "api_mode"][..], "chat_completions"),
    ] {
        out = yaml_mapping::set(&out, path, value)?;
    }
    if out == before {
        Ok(None)
    } else {
        Ok(Some(out.into_bytes()))
    }
}
impl Store {
    pub fn preview_hermes_native_settings(
        &self,
        home: &Path,
        connection: &SecretId,
    ) -> Result<HermesNativePreview, Error> {
        Ok(HermesNativePreview(self.preview_native_settings(
            home,
            connection,
            Kind::Hermes,
        )?))
    }
    pub fn apply_hermes_native_settings(
        &self,
        preview: &HermesNativePreview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        self.apply_native_settings(&preview.0, confirmed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hermes_yaml_comments_neighbors_noop_and_command_conflict() {
        let before=Some(b"# keep\n_config_version: 12\nmodel:\n  provider: old # inline\n  default: old\nproviders:\n  neighbor:\n    api_key: synthetic-neighbor\n".to_vec());
        let out = merge(&before, "http://127.0.0.1:9/v1", "sentinel", "synthetic")
            .unwrap()
            .unwrap();
        let text = std::str::from_utf8(&out).unwrap();
        assert!(
            text.contains("# keep")
                && text.contains("# inline")
                && text.contains("  neighbor:\n    api_key: synthetic-neighbor\n")
        );
        assert!(
            merge(&Some(out), "http://127.0.0.1:9/v1", "sentinel", "synthetic")
                .unwrap()
                .is_none()
        );
        assert!(
            validate(&Some(
                b"providers:\n  vibevm:\n    key_cmd: synthetic-command\n".to_vec()
            ))
            .is_err()
        );
    }
}
