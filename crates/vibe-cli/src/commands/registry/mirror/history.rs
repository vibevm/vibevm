//! The reverse-mirror record and `--keep-history` bundles a mirror run
//! commits into the target (PROP-002 `#registry-mirror`).
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_core::manifest::RegistrySection;

use super::plan::{self, Decision};
use super::record;
use super::source::{self, SourceReader};
use crate::output;

/// The target's mirror record for this source, merged with this run's
/// copies; returned as a batch file.
pub(super) fn mirror_record(
    source: &RegistrySection,
    planned: &[plan::Planned],
    select: Option<record::SelectRecord>,
    target_root: &Path,
    scratch: &Path,
) -> Result<Vec<(String, PathBuf)>> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let vibe = format!("vibe {}", env!("CARGO_PKG_VERSION"));
    let rows: Vec<record::VersionRecord> = planned
        .iter()
        .filter(|p| matches!(p.decision, Decision::Copy | Decision::Update))
        .filter_map(|p| {
            let git = p.fetched.as_ref()?.git.as_ref()?;
            Some(record::VersionRecord {
                package: format!("{}/{}", p.version.group, p.version.name),
                version: p.version.version.to_string(),
                repository: git.repository.clone(),
                repo: git.repo.clone(),
                tree_verified: true,
                facts: git.facts.clone(),
            })
        })
        .collect();
    let existing = record::MirrorRecord::read(target_root, &source.name)?;
    if rows.is_empty() && existing.is_none() && select.is_none() {
        return Ok(Vec::new());
    }
    let mut record = existing.unwrap_or_else(|| record::MirrorRecord {
        schema: record::SCHEMA,
        source: record::SourceRecord {
            name: source.name.clone(),
            url: source.url.clone(),
            kind: String::new(),
            naming: String::new(),
            refname: String::new(),
            index_url: None,
        },
        sync: record::SyncRecord {
            vibe: vibe.clone(),
            first: now.clone(),
            last: now.clone(),
        },
        select: None,
        versions: Vec::new(),
    });
    // A run that had a selection of its own records it, replacing what was
    // there: the record names the selection the target is kept to, and a
    // narrowed `--config` must not read as the old wider one. A bare run
    // repeating a recorded selection passes `None` and leaves it alone
    // (`Selection::to_record`).
    let reselected = select.is_some() && record.select != select;
    if let Some(select) = select {
        record.select = Some(select);
    }
    let changed = !rows.is_empty() || reselected;
    record.source = record::SourceRecord {
        name: source.name.clone(),
        url: source.url.clone(),
        kind: source.effective_type().as_str().to_string(),
        naming: serde_json::to_value(source.naming)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
        refname: source.r#ref.clone(),
        index_url: match vibe_registry::resolve_index_url(source) {
            vibe_registry::IndexUrlResolution::Url { base, .. } => Some(base),
            vibe_registry::IndexUrlResolution::Disabled => None,
        },
    };
    if changed {
        record.sync.vibe = vibe;
        record.sync.last = now;
        record.merge(rows);
    }
    let path = scratch.join("record.toml");
    std::fs::write(&path, record.render()?)
        .with_context(|| format!("writing `{}`", path.display()))?;
    Ok(vec![(record::record_path(&source.name), path)])
}

/// The largest history bundle `--keep-history` stores (per repository).
const MAX_BUNDLE_BYTES: u64 = 100 * 1024 * 1024;

/// `--keep-history`: one `git bundle` of every tag and branch per source
/// repository the run saw, written when the target's bundle carries other
/// refs; above [`MAX_BUNDLE_BYTES`] a bundle is skipped with a warning.
pub(super) fn history_bundles(
    ctx: &output::Context,
    reader: &SourceReader,
    source: &RegistrySection,
    planned: &[plan::Planned],
    target_root: &Path,
    scratch: &Path,
) -> Result<Vec<(String, PathBuf)>> {
    let Some(store) = reader.git_store() else {
        ctx.step("--keep-history: the source is not Git; no bundle is written");
        return Ok(Vec::new());
    };
    let mut repos: Vec<(String, String)> = planned
        .iter()
        .filter(|p| !matches!(p.decision, Decision::Alarm(_) | Decision::Unavailable(_)))
        .filter_map(|p| p.version.repository())
        .map(|repository| (source::repo_key(repository), repository.to_string()))
        .collect();
    repos.sort();
    repos.dedup();
    let mut out = Vec::new();
    for (repo, repository) in repos {
        let file = scratch.join(format!("{repo}.bundle"));
        reader.history_bundle(&repository, &file)?;
        let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
        if size > MAX_BUNDLE_BYTES {
            ctx.step(&format!(
                "warning: the history of {repo} is {size} bytes, above the {MAX_BUNDLE_BYTES}-byte \
                 limit; its bundle is not stored"
            ));
            continue;
        }
        let relative = record::bundle_path(&source.name, &repo);
        let existing = target_root.join(&relative);
        if existing.is_file()
            && store.bundle_heads(&existing).ok() == store.bundle_heads(&file).ok()
        {
            continue;
        }
        out.push((relative, file));
    }
    Ok(out)
}
