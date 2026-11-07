//! Closed version dispatch for the one existing finite import-work owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION");
use super::{document::ImportWork, pending, store};
use crate::{AppError, SettingsRoot};
use vibe_wire::generated::{application::e3::import_work as wire, shared as w};
pub(crate) const PROTOCOL: &str = "vibe-app-import-work/3";
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION"
)]
pub(crate) fn load(settings: &SettingsRoot, id: &str) -> Result<ImportWork, AppError> {
    let directory = store::path(settings, id)?;
    let bytes = std::fs::read(directory.join("import.json"))
        .map_err(|_| store::failure(settings, id, "exact cohort state is unavailable"))?;
    #[derive(serde::Deserialize)]
    struct Header {
        protocol: String,
        kind: Option<String>,
    }
    let header: Header = serde_json::from_slice(&bytes)
        .map_err(|_| store::failure(settings, id, "invalid cohort protocol header"))?;
    match header.protocol.as_str() {
        "vibe-app-import-work/1" => store::load(settings, id).map(ImportWork::Legacy),
        PROTOCOL if header.kind.as_deref() == Some("legacy") => {
            store::load(settings, id).map(ImportWork::Legacy)
        }
        "vibe-app-import-work/2" | PROTOCOL => {
            let value: wire::ImportWork = if header.protocol == "vibe-app-import-work/2" {
                let old: vibe_wire::generated::application::e2::import_work::ImportWork =
                    serde_json::from_slice(&bytes)
                        .map_err(|_| store::failure(settings, id, "invalid frozen full cohort"))?;
                wire::ImportWork {
                    protocol: PROTOCOL.into(),
                    kind: "full".into(),
                    source_evidence: Default::default(),
                    target: old.target,
                    phase: old.phase,
                    context: old.context,
                    cohort: old.cohort,
                }
            } else {
                serde_json::from_slice(&bytes).map_err(|_| {
                    store::failure(settings, id, "invalid closed full cohort record")
                })?
            };
            validate(settings, id, &value)?;
            Ok(ImportWork::Full(value))
        }
        _ => Err(store::failure(settings, id, "unsupported cohort protocol")),
    }
}
pub(crate) fn save(settings: &SettingsRoot, document: &ImportWork) -> Result<(), AppError> {
    match document {
        ImportWork::Legacy(d) => store::save(settings, d),
        ImportWork::Full(d) => save_full(settings, d),
    }
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION"
)]
pub(super) fn save_full(
    settings: &SettingsRoot,
    document: &wire::ImportWork,
) -> Result<(), AppError> {
    validate(settings, &document.target.operation, document)?;
    let mut bytes = encode(document).map_err(|_| {
        store::failure(
            settings,
            &document.target.operation,
            "cannot encode full cohort",
        )
    })?;
    bytes.push(b'\n');
    crate::settings::write_atomic(
        &store::path(settings, &document.target.operation)?.join("import.json"),
        &bytes,
    )
    .map_err(Into::into)
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION"
)]
pub(super) fn encode(document: &wire::ImportWork) -> Result<Vec<u8>, serde_json::Error> {
    if document.source_evidence.is_empty() {
        serde_json::to_vec_pretty(
            &vibe_wire::generated::application::e2::import_work::ImportWork {
                protocol: "vibe-app-import-work/2".into(),
                target: document.target.clone(),
                phase: document.phase.clone(),
                context: document.context.clone(),
                cohort: document.cohort.clone(),
            },
        )
    } else {
        serde_json::to_vec_pretty(document)
    }
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION"
)]
fn validate(settings: &SettingsRoot, id: &str, d: &wire::ImportWork) -> Result<(), AppError> {
    let bad = |reason: &str| store::failure(settings, id, reason);
    if d.kind != "full"
        || d.protocol != PROTOCOL
        || d.target.operation != id
        || d.target.home != pending::actual_home(settings)?
        || settings.is_project()
    {
        return Err(bad(
            "cohort belongs to another home, generation or protocol",
        ));
    }
    super::receipts::validate(d).map_err(|reason| bad(&reason))?;
    let c = &d.cohort;
    if d.source_evidence
        .keys()
        .any(|k| !c.bindings.contains_key(k))
    {
        return Err(bad("source evidence key is outside the captured cohort"));
    }
    crate::portable::validate_rows(&c.roots, c.bindings.values().map(|b| &b.application))
        .map_err(|_| bad("full cohort contains invalid portable roots or bindings"))?;
    for (key, binding) in &c.bindings {
        if key != &binding.application.key || pending::coordinate(key).is_err() {
            return Err(bad("full binding key differs from its exact application"));
        }
    }
    if c.cursor as usize > c.roots.len()
        || c.current.as_ref().is_some_and(|t| {
            t.home != d.target.home
                || !pending::valid_operation(&t.operation)
                || c.roots.get(c.cursor as usize) != Some(&t.key)
        })
    {
        return Err(bad("full cohort cursor/current attempt is inconsistent"));
    }
    if matches!(
        d.phase,
        w::ApplicationImportPhase::Arrangement | w::ApplicationImportPhase::Complete
    ) && (c.cursor as usize != c.roots.len() || c.current.is_some() || c.waiting.is_some())
    {
        return Err(bad(
            "full cohort has unsettled roots before arrangement or completion",
        ));
    }
    if d.phase == w::ApplicationImportPhase::Complete && !c.arrangement_done {
        return Err(bad("full cohort completion is inconsistent"));
    }
    if let Some(waiting) = &c.waiting {
        let root = c
            .roots
            .get(c.cursor as usize)
            .ok_or_else(|| bad("waiting has no active root"))?;
        let mut reachable = std::collections::BTreeSet::new();
        let mut queue = vec![root.as_str()];
        while let Some(key) = queue.pop() {
            if reachable.insert(key)
                && let Some(binding) = c.bindings.get(key)
            {
                queue.extend(
                    binding
                        .application
                        .requires
                        .iter()
                        .map(|edge| edge.key.as_str()),
                );
            }
        }
        if !reachable.contains(waiting.key.as_str())
            || waiting.parameters.is_empty()
            || waiting.parameters.iter().any(|id| id.trim().is_empty())
            || waiting
                .parameters
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != waiting.parameters.len()
            || &waiting.key != root && c.current.is_none()
        {
            return Err(bad(
                "waiting input is not an active prepared recipient/attempt",
            ));
        }
    }
    if c.results.len() != c.cursor as usize
        || c.results.iter().enumerate().any(|(row, result)| {
            result.row_index != Some(row as u32) || c.roots.get(row) != Some(&result.key)
        })
    {
        return Err(bad("settled cursor lacks its exact ordered outcome prefix"));
    }
    for (key, ids) in &c.secret_requirements {
        if !c.bindings.contains_key(key) || ids.iter().any(|id| id.trim().is_empty()) {
            return Err(bad(
                "secret obligations name an unknown recipient or empty ID",
            ));
        }
    }
    for result in &c.results {
        if result.action == w::ApplicationPendingImportResultAction::Installed
            && (result.row_index.is_none() || result.target.is_none())
        {
            return Err(bad("Installed outcome needs its exact root/attempt"));
        }
        if result
            .row_index
            .is_some_and(|row| row >= c.cursor || c.roots.get(row as usize) != Some(&result.key))
        {
            return Err(bad("outcome does not name a completed root"));
        }
        if result.target.as_ref().is_some_and(|t| {
            result.row_index.is_none()
                || result.action == w::ApplicationPendingImportResultAction::Skipped
                || t.home != d.target.home
                || t.key != result.key
                || !pending::valid_operation(&t.operation)
        }) {
            return Err(bad("outcome attempt binding is inconsistent"));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "full_store/tests.rs"]
mod tests;
