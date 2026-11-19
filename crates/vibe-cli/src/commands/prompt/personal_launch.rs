//! Launch-only personal composition, using ordinary witnessed request materials.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#USER-LANE-LAST");
use super::{Selected, adapter::RequestEnvelope};
use crate::output;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use vibe_core::manifest::TargetOs;
use vibe_workspace::system_prompt::{PromptCheck, prepare_request, sha256_hex};

pub(super) fn compose(
    ctx: &output::Context,
    selected: &Selected,
    answer: &PromptCheck,
) -> Result<Option<(PathBuf, String)>> {
    compose_inner(ctx, selected, answer, false)
}

/// Native bootstrap always observes current project continuation, even without
/// a personal layer; reuse the same witnessed composition owner.
pub(super) fn compose_native(
    ctx: &output::Context,
    selected: &Selected,
    answer: &PromptCheck,
) -> Result<Option<(PathBuf, String)>> {
    compose_inner(ctx, selected, answer, true)
}

fn compose_inner(
    ctx: &output::Context,
    selected: &Selected,
    answer: &PromptCheck,
    require_fresh_project: bool,
) -> Result<Option<(PathBuf, String)>> {
    let personal = super::personal::layered_blocks(ctx, TargetOs::current(), &selected.node_dir);
    if personal.is_empty() && !require_fresh_project {
        return Ok(None);
    }
    let tracked = selected.tracked();
    let request = selected.request(&tracked, None);
    let prepared = prepare_request(&request, answer, TargetOs::current())?;
    if !prepared.unresolved.is_empty() {
        bail!("launch project continuation contains unresolved conditions");
    }
    let envelope = RequestEnvelope::build(
        &prepared,
        &answer.system.path,
        &selected.workspace.root,
        None,
    )?;
    let meta_bytes = std::fs::read(&answer.meta.path)?;
    if answer.meta.sha256.as_deref() != Some(sha256_hex(&meta_bytes).as_str()) {
        bail!("project prompt metadata changed during launch preparation");
    }
    let meta: vibe_wire::generated::system_prompt_meta::SystemPromptMeta =
        serde_json::from_slice(&meta_bytes)?;
    let prefix = &envelope.instruction_blocks[0].text;
    // A harness already loaded its entrypoint. Only the static lane segment
    // belongs here, not the API template or the prospective entrypoint export.
    let mut sources = Vec::<(String, Vec<u8>)>::new();
    if let Some(segment) = meta.segments.iter().find(|row| row.id == "project.lane") {
        let start = segment.offset as usize;
        let end = start
            .checked_add(segment.len as usize)
            .context("lane segment overflow")?;
        let bytes = prefix
            .as_bytes()
            .get(start..end)
            .context("lane segment outside witnessed prefix")?;
        let input = meta
            .inputs
            .iter()
            .find(|row| {
                matches!(
                    row.role,
                    vibe_wire::generated::system_prompt_meta::Role::Lane
                )
            })
            .context("static lane has no witnessed source")?;
        let path = if selected.node_rel == "." {
            input.path.clone()
        } else {
            format!("{}/{}", selected.node_rel, input.path)
        };
        sources.push((path, bytes.to_vec()));
    }
    // The sidecar retains INDEX order, including interleaved OS rows; the
    // prepared envelope admits each selected member against its observed hash.
    let mut remaining: Vec<_> = envelope.instruction_blocks.iter().skip(1).collect();
    for row in &meta.continuation {
        if let Some(index) = remaining.iter().position(|block| block.path == row.path) {
            let block = remaining.remove(index);
            sources.push((row.path.clone(), block.text.as_bytes().to_vec()));
        }
    }
    if !remaining.is_empty() {
        bail!("prepared project continuation missing from verified metadata");
    }
    sources.extend(
        personal
            .into_iter()
            .map(|block| (block.path, block.text.into_bytes())),
    );
    let borrowed: Vec<_> = sources
        .iter()
        .map(|(path, bytes)| vibe_workspace::system_prompt::LaneSource { path, bytes })
        .collect();
    let lane = vibe_workspace::system_prompt::compose_agent_lane(&meta.node_id, &borrowed)?;
    let text = String::from_utf8(lane.bytes)?;
    let path = super::local_artifact::publish(&selected.node_dir, "AGENT", &text)?;
    Ok(Some((path, text)))
}
