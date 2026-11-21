//! Concrete OpenCode native JSONC writer; never touches credential SQL.
use super::*;
const PACKAGE: &str = "aisdk:@ai-sdk/openai-compatible";
pub struct OpenCodeNativePreview(pub(super) PiNativePreview);
impl OpenCodeNativePreview {
    pub fn preview_id(&self) -> &str {
        self.0.preview_id()
    }
}
impl std::fmt::Debug for OpenCodeNativePreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpenCodeNativePreview([REDACTED])")
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
    for path in [
        &["providers"][..],
        &["providers", "vibevm"][..],
        &["providers", "vibevm", "settings"][..],
        &["providers", "vibevm", "models"][..],
    ] {
        if let Some(node) = at(&root, path)
            && node.object.is_none()
        {
            return Err(Error::Invalid);
        }
    }
    for path in [
        &["providers", "vibevm", "headers"][..],
        &["providers", "vibevm", "body"][..],
        &["providers", "vibevm", "settings", "auth"][..],
        &["providers", "vibevm", "env"][..],
    ] {
        if at(&root, path).is_some() {
            return Err(Error::Invalid);
        }
    }
    Ok(())
}
fn literal(value: &str) -> Result<(), Error> {
    if value.contains("{env:") || value.contains("{file:") {
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
    if model.contains('#') {
        return Err(Error::Invalid);
    }
    let before = text(bytes)?;
    let mut output = before.to_owned();
    let root = jsonc::parse(&output)?;
    if let Some(node) = at(&root, &["model"])
        && node.object.is_some()
    {
        if at(node, &["variant"]).is_some() {
            return Err(Error::Invalid);
        }
        for (field, value) in [("providerID", "vibevm"), ("model", model)] {
            let root = jsonc::parse(&output)?;
            if let Some(node) = at(&root, &["model", field])
                && serde_json::from_str::<serde_json::Value>(&output[node.range.clone()])
                    .map_err(|_| Error::Invalid)?
                    .as_str()
                    == Some(value)
            {
                continue;
            }
            output = jsonc::set(
                &output,
                &["model", field],
                &serde_json::to_string(value).map_err(|_| Error::Invalid)?,
            )?;
        }
    } else {
        let selection = serde_json::json!({"providerID":"vibevm","model":model}).to_string();
        output = jsonc::set(&output, &["model"], &selection)?;
    }
    for (path, value) in [
        (&["providers", "vibevm", "package"][..], PACKAGE),
        (&["providers", "vibevm", "settings", "apiKey"][..], secret),
        (
            &["providers", "vibevm", "settings", "baseURL"][..],
            endpoint,
        ),
    ] {
        let encoded = serde_json::to_string(value).map_err(|_| Error::Invalid)?;
        let root = jsonc::parse(&output)?;
        if let Some(node) = at(&root, path)
            && serde_json::from_str::<serde_json::Value>(&output[node.range.clone()])
                .map_err(|_| Error::Invalid)?
                .as_str()
                == Some(value)
        {
            continue;
        }
        output = jsonc::set(&output, path, &encoded)?;
    }
    let model_path = ["providers", "vibevm", "models", model];
    let root = jsonc::parse(&output)?;
    if at(&root, &model_path).is_none() {
        output = jsonc::set(
            &output,
            &model_path,
            "{\"name\":\"Vibe-managed native model\"}",
        )?;
    } else if at(&root, &model_path)
        .and_then(|n| n.object.as_ref())
        .is_none()
    {
        return Err(Error::Invalid);
    }
    // Model-level connection/auth overlays could defeat the confirmed provider.
    let root = jsonc::parse(&output)?;
    for key in ["settings", "headers", "body", "package", "modelID"] {
        if at(&root, &["providers", "vibevm", "models", model, key]).is_some() {
            return Err(Error::Invalid);
        }
    }
    if output == before {
        Ok(None)
    } else {
        Ok(Some(output.into_bytes()))
    }
}
impl Store {
    pub fn preview_opencode_native_settings(
        &self,
        home: &Path,
        connection: &SecretId,
    ) -> Result<OpenCodeNativePreview, Error> {
        Ok(OpenCodeNativePreview(self.preview_native_settings(
            home,
            connection,
            Kind::OpenCode,
        )?))
    }
    pub fn apply_opencode_native_settings(
        &self,
        preview: &OpenCodeNativePreview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        self.apply_native_settings(&preview.0, confirmed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opencode_comments_neighbors_and_substitutions_are_preserved_or_refused() {
        let bytes=Some(b"{// keep\n\"model\":{/* model keep */\"providerID\":\"old\",\"model\":\"old\",\"foreign\":42},\"providers\":{\"neighbor\":{\"name\":\"keep\"}},\"foreign\":true}".to_vec());
        let out = merge(
            &bytes,
            "http://127.0.0.1:9/v1",
            "synthetic",
            "synthetic-key",
        )
        .unwrap()
        .unwrap();
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("// keep")
                && s.contains("/* model keep */")
                && s.contains("\"foreign\":42")
                && s.contains("\"neighbor\":{\"name\":\"keep\"}")
                && s.contains("\"foreign\":true")
        );
        assert!(
            merge(
                &Some(out),
                "http://127.0.0.1:9/v1",
                "synthetic",
                "synthetic-key"
            )
            .unwrap()
            .is_none()
        );
        for key in ["{env:HOME}", "{file:private}"] {
            assert!(merge(&bytes, "http://127.0.0.1:9/v1", "synthetic", key).is_err());
        }
    }
}
