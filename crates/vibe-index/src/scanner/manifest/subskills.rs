//! Subskill walking: `<pkg_root>/subskills/<path>/vibe-subskill.toml`,
//! parsed through `vibe-core`'s own [`SubskillManifest`] for the same
//! reason the package manifest is — the index cannot drift from a schema
//! it does not re-implement.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::Path;

use super::delivery::delivery_from;

use vibe_core::manifest::{ActivationRules, SubskillManifest};
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::types::SubskillEntry;

// ---------------------------------------------------------------------------
// Subskill walking
// ---------------------------------------------------------------------------

/// Walk `<pkg_root>/subskills/<path>/vibe-subskill.toml`, parsing each
/// through `vibe-core`'s [`SubskillManifest`]. A directory without a
/// manifest is ignored; a malformed manifest surfaces as an error so the
/// authoring bug is loud at index time.
pub fn collect_subskills(pkg_root: &Path) -> Result<Vec<SubskillEntry>> {
    let subdir = pkg_root.join("subskills");
    if !subdir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(&subdir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() || entry.file_name() != SubskillManifest::FILENAME {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(pkg_root)
            .unwrap_or(entry.path())
            .display()
            .to_string();
        let sm = SubskillManifest::read(entry.path())
            .map_err(|e| Error::Malformed(format!("{rel}: {e}")))?;
        out.push(SubskillEntry {
            path: sm.subskill.path.clone(),
            delivery: delivery_from(sm.subskill.delivery),
            describes: sm.subskill.describes.as_ref().map(|p| p.to_string()),
            description: sm.subskill.description.clone(),
            channels: declared_channels(&sm.activation),
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// The activation channels a subskill declares — every non-empty
/// `[activation]` lane, surfaced in the index entry for discovery.
fn declared_channels(a: &ActivationRules) -> Vec<String> {
    let mut ch = Vec::new();
    if !a.if_present.is_empty() {
        ch.push("if_present".into());
    }
    if !a.if_provides.is_empty() {
        ch.push("if_provides".into());
    }
    if !a.if_files.is_empty() {
        ch.push("if_files".into());
    }
    if !a.if_command.is_empty() {
        ch.push("if_command".into());
    }
    if !a.if_env.is_empty() {
        ch.push("if_env".into());
    }
    if !a.if_os.is_empty() {
        ch.push("if_os".into());
    }
    if a.if_describes_match {
        ch.push("if_describes_match".into());
    }
    if !a.if_language.is_empty() {
        ch.push("if_language".into());
    }
    ch
}
