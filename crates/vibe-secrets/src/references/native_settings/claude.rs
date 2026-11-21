//! Claude Code native Messages bearer config only, no OpenAI translation.
use super::*;
use serde_json::{Value, json};
pub struct ClaudeNativePreview(pub(super) PiNativePreview);
impl ClaudeNativePreview {
    pub fn preview_id(&self) -> &str {
        self.0.preview_id()
    }
}
impl std::fmt::Debug for ClaudeNativePreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClaudeNativePreview([REDACTED])")
    }
}
fn document(bytes: &Option<Vec<u8>>) -> Result<Value, Error> {
    let value = match bytes {
        Some(bytes) => serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?,
        None => json!({}),
    };
    if !value.is_object() {
        return Err(Error::Invalid);
    }
    Ok(value)
}
pub(super) fn validate(bytes: &Option<Vec<u8>>) -> Result<(), Error> {
    let value = document(bytes)?;
    if value.get("apiKeyHelper").is_some() || value.get("forceLoginMethod").is_some() {
        return Err(Error::Invalid);
    }
    if let Some(env) = value.get("env") {
        let env = env.as_object().ok_or(Error::Invalid)?;
        for key in [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_CUSTOM_HEADERS",
            "CLAUDE_CODE_USE_BEDROCK",
            "CLAUDE_CODE_USE_VERTEX",
            "CLAUDE_CODE_USE_FOUNDRY",
            "CLAUDE_CODE_OAUTH_TOKEN",
        ] {
            if env.contains_key(key) {
                return Err(Error::Invalid);
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
    let mut value = document(bytes)?;
    let original = value.clone();
    let env = value
        .as_object_mut()
        .ok_or(Error::Invalid)?
        .entry("env")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(Error::Invalid)?;
    env.insert("ANTHROPIC_BASE_URL".into(), json!(endpoint));
    env.insert("ANTHROPIC_AUTH_TOKEN".into(), json!(secret));
    value
        .as_object_mut()
        .ok_or(Error::Invalid)?
        .insert("model".into(), json!(model));
    if value == original {
        Ok(None)
    } else {
        Ok(Some(
            serde_json::to_vec_pretty(&value).map_err(|_| Error::Invalid)?,
        ))
    }
}
impl Store {
    pub fn preview_claude_native_settings(
        &self,
        home: &Path,
        connection: &SecretId,
    ) -> Result<ClaudeNativePreview, Error> {
        Ok(ClaudeNativePreview(self.preview_native_settings(
            home,
            connection,
            Kind::Claude,
        )?))
    }
    pub fn apply_claude_native_settings(
        &self,
        preview: &ClaudeNativePreview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        self.apply_native_settings(&preview.0, confirmed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_claude_bearer_preserves_neighbors_and_refuses_other_auth_sources() {
        let before = Some(br#"{"env":{"OTHER":"keep"},"foreign":{"keep":true}}"#.to_vec());
        let out = merge(&before, "http://127.0.0.1:9", "synthetic", "synthetic-key")
            .unwrap()
            .unwrap();
        let v: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["env"]["OTHER"], "keep");
        assert_eq!(v["foreign"]["keep"], true);
        assert!(
            merge(
                &Some(out),
                "http://127.0.0.1:9",
                "synthetic",
                "synthetic-key"
            )
            .unwrap()
            .is_none()
        );
        for value in [
            br#"{"apiKeyHelper":"synthetic-command"}"#.as_slice(),
            br#"{"env":{"ANTHROPIC_API_KEY":"synthetic-other"}}"#,
        ] {
            assert!(validate(&Some(value.to_vec())).is_err());
        }
    }
}
