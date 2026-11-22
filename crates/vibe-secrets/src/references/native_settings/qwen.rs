//! Qwen native JSONC merge only; no credential export or alternate transport.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use super::*;

pub struct QwenNativePreview(pub(super) PiNativePreview);
impl QwenNativePreview {
    pub fn preview_id(&self) -> &str {
        self.0.preview_id()
    }
}
impl std::fmt::Debug for QwenNativePreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QwenNativePreview([REDACTED])")
    }
}
fn text(bytes: &Option<Vec<u8>>) -> Result<&str, Error> {
    match bytes {
        Some(bytes) => std::str::from_utf8(bytes).map_err(|_| Error::Invalid),
        None => Ok("{}"),
    }
}
fn at<'a>(node: &'a jsonc::Node, path: &[&str]) -> Option<&'a jsonc::Node> {
    if path.is_empty() {
        return Some(node);
    }
    at(node.object.as_ref()?.get(path[0])?, &path[1..])
}
pub(super) fn validate(bytes: &Option<Vec<u8>>) -> Result<(), Error> {
    let source = text(bytes)?;
    let root = jsonc::parse(source)?;
    // The native early operator-policy reader strips comments, but rejects
    // trailing commas. Preserve/refuse the subject instead of normalizing it.
    if root.trailing {
        return Err(Error::Invalid);
    }
    // Existing explicit model-provider routes can override the ordinary auth
    // endpoint. First slice refuses rather than silently rewriting that route.
    if let Some(providers) = at(&root, &["modelProviders"])
        && !providers.object.as_ref().is_some_and(|map| map.is_empty())
    {
        return Err(Error::Invalid);
    }
    for path in [&["security"][..], &["security", "auth"][..], &["model"][..]] {
        if let Some(node) = at(&root, path)
            && node.object.is_none()
        {
            return Err(Error::Invalid);
        }
    }
    if let Some(node) = at(&root, &["security", "auth", "enforcedType"]) {
        let value: serde_json::Value =
            serde_json::from_str(&source[node.range.clone()]).map_err(|_| Error::Invalid)?;
        if !value.is_null() && value.as_str() != Some("openai") {
            return Err(Error::Invalid);
        }
    }
    if let Some(node) = at(&root, &["security", "auth", "useExternal"])
        && source[node.range.clone()].trim() != "false"
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn literal(value: &str) -> Result<(), Error> {
    // Native resolver has no literal escape for $VAR/${VAR}; never change the
    // credential to avoid that resolver. Static refusal precedes publication.
    if value
        .as_bytes()
        .windows(2)
        .any(|w| w[0] == b'$' && (w[1].is_ascii_alphanumeric() || w[1] == b'_' || w[1] == b'{'))
    {
        return Err(Error::Invalid);
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
    for value in [endpoint, model, secret] {
        literal(value)?;
    }
    let before = text(bytes)?;
    let mut output = before.to_owned();
    for (path, value) in [
        (&["security", "auth", "selectedType"][..], "openai"),
        (&["security", "auth", "apiKey"][..], secret),
        (&["security", "auth", "baseUrl"][..], endpoint),
        (&["model", "name"][..], model),
        (&["model", "baseUrl"][..], endpoint),
    ] {
        // Keep byte-identical no-ops, including comments around other fields.
        let tree = jsonc::parse(&output)?;
        if let Some(node) = at(&tree, path) {
            let current: serde_json::Value =
                serde_json::from_str(&output[node.range.clone()]).map_err(|_| Error::Invalid)?;
            if current.as_str() == Some(value) {
                continue;
            }
        }
        output = jsonc::set(
            &output,
            path,
            &serde_json::to_string(value).map_err(|_| Error::Invalid)?,
        )?;
    }
    if output == before {
        Ok(None)
    } else {
        Ok(Some(output.into_bytes()))
    }
}
impl Store {
    pub fn preview_qwen_native_settings(
        &self,
        home: &Path,
        connection: &SecretId,
    ) -> Result<QwenNativePreview, Error> {
        Ok(QwenNativePreview(self.preview_native_settings(
            home,
            connection,
            Kind::Qwen,
        )?))
    }
    pub fn apply_qwen_native_settings(
        &self,
        preview: &QwenNativePreview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        self.apply_native_settings(&preview.0, confirmed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comments_unknown_fields_and_noop_survive_native_merge() {
        let input = Some(b"{// root\n\"security\":{\"auth\":{/*keep*/\"selectedType\":\"qwen-oauth\"},\"foreign\":true},\"nested\":{\"provider\":\"keep\\\"quoted\"}}".to_vec());
        assert!(validate(&Some(b"{\"security\":{},}".to_vec())).is_err());
        let output = merge(
            &input,
            "http://127.0.0.1:9/v1",
            "synthetic",
            "synthetic-key",
        )
        .unwrap()
        .unwrap();
        let value = std::str::from_utf8(&output).unwrap();
        assert!(
            value.contains("// root")
                && value.contains("/*keep*/")
                && value.contains("\"foreign\":true")
                && value.contains("keep\\\"quoted")
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
        assert!(merge(&input, "http://127.0.0.1:9/v1", "synthetic", "$HOME-key").is_err());
        assert!(
            validate(&Some(
                br#"{"modelProviders":{"openai":[{"id":"other"}]}}"#.to_vec()
            ))
            .is_err()
        );
    }
}
