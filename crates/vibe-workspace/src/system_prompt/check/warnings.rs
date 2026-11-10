//! The observations `check` reports that are **not** freshness.
//!
//! Out of the verdict on purpose, and out of line along that seam. None of
//! them can make a published prefix wrong — a lane whose bodies have drifted
//! is *expected*, because `vibe agent` recomposes it at launch — so folding
//! any of them into `stale` would make the one assertion the band model
//! rests on fail (`#SYS-NO-FROZEN-INDEX`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK");

use vibe_core::layout;

use crate::WorkspaceError;
use crate::boot_artifacts;

use super::super::constraints::delivery_constraints;
use super::super::markers::verify_lane_markers;
use super::super::verify::ImportInertness;
use super::{CheckRequest, PromptCheck, read_owned};

/// The observations that are not freshness: marker integrity, the import
/// census, the Codex budget and the lane's size against a stated window.
///
/// Out of the verdict on purpose. None of them can make a published prefix
/// wrong — a lane whose bodies have drifted is *expected*, because `vibe
/// agent` recomposes it at launch — so folding any of them into `stale`
/// would make the one assertion the band model rests on fail.
pub fn prompt_warnings(
    request: &CheckRequest<'_>,
    answer: &PromptCheck,
) -> Result<Vec<String>, WorkspaceError> {
    let mut warnings = Vec::new();
    if let Some(bytes) = read_owned(&answer.lane.path)? {
        match verify_lane_markers(&bytes) {
            Ok(segments) => {
                let inertness =
                    ImportInertness::of(layout::AGENT_LANE_MD, &String::from_utf8_lossy(&bytes));
                if !inertness.is_inert() {
                    warnings.push(format!(
                        "the agent lane carries {} line-initial `@` token(s) outside a fence; \
                         the `import` delivery tier stays refused in version 1 for exactly that \
                         reason",
                        inertness.line_initial()
                    ));
                }
                if let Some(window) = request.model_window.filter(|window| bytes.len() > *window) {
                    warnings.push(format!(
                        "the agent lane is {} bytes against a stated {window}-byte context \
                         window, so it does not fit",
                        bytes.len()
                    ));
                }
                warnings.push(format!(
                    "the agent lane delivers {} segment(s), {} bytes",
                    segments.len(),
                    bytes.len()
                ));
            }
            Err(error) => warnings.push(format!("the agent lane's markers do not verify: {error}")),
        }
    }
    warnings.extend(entrypoint_warnings(request)?);
    // The one-delivery-mode constraints of `HARNESS-DELIVERY.md` §4, which
    // that section places "at install and in `vibe prompt check`". They are
    // observations rather than freshness for the same reason as the rest of
    // this cell: a divergent instruction file does not make a published
    // prefix wrong, and folding it into `stale` would send a caller to
    // `vibe prompt build`, which is not the remedy.
    warnings.extend(delivery_constraints(request.node_dir, request.manifest)?);
    Ok(warnings)
}

/// The entrypoint's own two: how many managed blocks it carries, and what a
/// Codex truncation at the declared limit would cost.
fn entrypoint_warnings(request: &CheckRequest<'_>) -> Result<Vec<String>, WorkspaceError> {
    let mut warnings = Vec::new();
    let Some(name) = request
        .manifest
        .system_prompt
        .as_ref()
        .map(|section| section.entrypoint)
        .unwrap_or_default()
        .file_name()
    else {
        return Ok(warnings);
    };
    let Some(bytes) = read_owned(&request.node_dir.join(name))? else {
        return Ok(warnings);
    };
    let text = String::from_utf8_lossy(&bytes);
    let opens = text
        .lines()
        .filter(|line| line.trim() == boot_artifacts::BLOCK_OPEN)
        .count();
    if opens != 1 {
        warnings.push(format!(
            "`{name}` carries {opens} `{}` marker line(s); exactly one managed block is the \
             only shape composition can splice",
            boot_artifacts::BLOCK_OPEN
        ));
    }
    let limit = request
        .manifest
        .system_prompt
        .as_ref()
        .map_or(boot_artifacts::PROJECT_DOC_MAX_BYTES, |section| {
            section.codex_doc_max_bytes()
        });
    if let Some(message) = boot_artifacts::CodexBudget::whole_file(name, bytes.len(), limit) {
        warnings.push(message);
    }
    Ok(warnings)
}
