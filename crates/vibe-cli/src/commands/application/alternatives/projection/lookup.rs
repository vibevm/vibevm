//! Read the physically published global choice, independent of index intent.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use vibe_core::manifest::is_portable_token;
use vibe_safefs::ensure_no_follow_walk;

use super::{CURRENT_PROTOCOL, files::path_present, normalized_settings};

pub(in crate::commands::application::alternatives) fn published_global(
    settings: &Path,
    slot: &str,
) -> Result<Option<String>> {
    if !is_portable_token(slot) {
        bail!("invalid application alternative slot");
    }
    let settings = normalized_settings(settings)?;
    let slot_dir = settings.join("opt/alternatives").join(slot);
    let current = slot_dir.join("current.json");
    ensure_no_follow_walk(&settings, &current, true)?;
    if !path_present(&current)? {
        return Ok(None);
    }
    let pointer = bounded_json(&current, 1024)?;
    if pointer.get("protocol").and_then(Value::as_str) != Some(CURRENT_PROTOCOL) {
        bail!("unsupported published application current pointer");
    }
    let projection = pointer
        .get("projection")
        .and_then(Value::as_str)
        .context("published application pointer has no projection")?;
    if projection.len() != 64
        || !projection
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("published application projection id is invalid");
    }
    let owner_path = slot_dir
        .join("projections")
        .join(projection)
        .join("owner.json");
    ensure_no_follow_walk(&settings, &owner_path, false)?;
    let owner = bounded_json(&owner_path, 4096)?;
    if owner.get("protocol").and_then(Value::as_str) != Some("vibe-app-projection/1")
        || owner.get("slot").and_then(Value::as_str) != Some(slot)
    {
        bail!("published application projection owner is invalid");
    }
    match owner.get("installation") {
        Some(Value::Null) => Ok(None),
        Some(Value::String(key)) if !key.is_empty() => Ok(Some(key.clone())),
        _ => bail!("published application projection has no valid installation key"),
    }
}

fn bounded_json(path: &Path, limit: u64) -> Result<Value> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading projection metadata `{}`", path.display()))?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        bail!("projection metadata is not a bounded regular file");
    }
    let bytes = fs::read(path)?;
    if bytes.len() as u64 > limit {
        bail!("projection metadata exceeds size limit");
    }
    serde_json::from_slice(&bytes).context("parsing projection metadata")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::published_global;

    #[test]
    fn published_choice_comes_from_pointer_owner() {
        let root = tempfile::tempdir().unwrap();
        let slot = root.path().join("opt/alternatives/editor");
        let owner = slot.join(format!("projections/{}/owner.json", "a".repeat(64)));
        fs::create_dir_all(owner.parent().unwrap()).unwrap();
        fs::write(
            &owner,
            serde_json::to_vec(&json!({
                "protocol":"vibe-app-projection/1", "slot":"editor",
                "installation":"org.example/editor@1.2.3",
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            slot.join("current.json"),
            serde_json::to_vec(&json!({
                "protocol":"vibe-app-current/1", "projection":"a".repeat(64),
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            published_global(root.path(), "editor").unwrap().as_deref(),
            Some("org.example/editor@1.2.3")
        );
        fs::write(owner, b"{}").unwrap();
        assert!(published_global(root.path(), "editor").is_err());
    }
}
