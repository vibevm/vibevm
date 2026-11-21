//! Locked metadata protocol admission for existing managed Pi/Qwen bindings.
use super::*;
pub(super) fn binding_api(
    snapshot: &ReferenceSnapshot,
    harness: &str,
    binding: &ReferenceBinding,
) -> Result<(), Error> {
    let target = match binding {
        ReferenceBinding::Connection { connection } => Some(connection),
        ReferenceBinding::InheritDefault => match &snapshot.index.selection {
            Some(ReferenceSelection::Selected { connection }) => Some(connection),
            _ => None,
        },
    };
    let Some(target) = target else {
        return Ok(());
    };
    let bytes = snapshot.metadata.get(target).ok_or(Error::RefusedUnknown)?;
    let metadata: toml::Value =
        toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Invalid)?)
            .map_err(|_| Error::Invalid)?;
    if matches!(harness, "pi" | "qwen")
        && metadata.get("api_kind").and_then(toml::Value::as_str) != Some("openai-chat-completions")
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub(super) fn all(snapshot: &ReferenceSnapshot) -> Result<(), Error> {
    if let Some(bindings) = &snapshot.index.bindings {
        for (harness, binding) in bindings {
            binding_api(snapshot, harness, binding)?;
        }
    }
    Ok(())
}
