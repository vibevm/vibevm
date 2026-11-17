//! Explicit private full-prefix composition, never a provider call.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#LAYERED-PREFIX");
use super::{Selected, adapter::RequestEnvelope};
use crate::output;
use anyhow::{Context, Result, bail};
use vibe_workspace::system_prompt::{check_node_prompt, prepare_request, sha256_hex};
pub(super) fn run(ctx: &output::Context, selected: &Selected, target: Option<&str>) -> Result<()> {
    let target = target.map(super::prepare::parse_target).transpose()?;
    let tracked = selected.tracked();
    let request = selected.request(&tracked, None);
    let answer = check_node_prompt(&request)?;
    let prepared = prepare_request(&request, &answer, target)?;
    if !prepared.unresolved.is_empty() {
        bail!("composed prefix requires an explicit session target for every gated continuation");
    }
    let envelope = RequestEnvelope::build(
        &prepared,
        &answer.system.path,
        &selected.workspace.root,
        None,
    )?;
    let layers = super::personal::layered_blocks(ctx, target, &selected.node_dir);
    let prefix = &envelope.instruction_blocks[0].text;
    let (path, text, local) = if layers.is_empty() {
        // No private layers: identity is the ordinary project prefix and no
        // local directory is created merely by asking to compose it.
        (answer.system.path.clone(), prefix.clone(), false)
    } else {
        let metadata = std::fs::read(&answer.meta.path)?;
        if answer.meta.sha256.as_deref() != Some(sha256_hex(&metadata).as_str()) {
            bail!("project prompt metadata changed during composition");
        }
        let meta: vibe_wire::generated::system_prompt_meta::SystemPromptMeta =
            serde_json::from_slice(&metadata)?;
        let mut text = prefix.clone();
        let mut remaining: Vec<_> = envelope.instruction_blocks.iter().skip(1).collect();
        let mut sources = Vec::<(String, Vec<u8>)>::new();
        for row in &meta.continuation {
            if let Some(index) = remaining.iter().position(|block| block.path == row.path) {
                let block = remaining.remove(index);
                sources.push((row.path.clone(), block.text.as_bytes().to_vec()));
            }
        }
        if !remaining.is_empty() {
            bail!("prepared continuation absent from verified metadata");
        }
        sources.extend(
            layers
                .into_iter()
                .map(|block| (block.path, block.text.into_bytes())),
        );
        let borrowed: Vec<_> = sources
            .iter()
            .map(|(path, bytes)| vibe_workspace::system_prompt::LaneSource { path, bytes })
            .collect();
        let tail = vibe_workspace::system_prompt::compose_agent_lane(&meta.node_id, &borrowed)?;
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(std::str::from_utf8(&tail.bytes).context("composed tail is not UTF-8")?);
        let path = super::local_artifact::publish(&selected.node_dir, "SYSTEM", &text)?;
        (path, text, true)
    };
    let report = vibe_wire::generated::prompt_compose::PromptComposeReport {
        command: "prompt:compose".into(),
        node: selected.node_rel.clone(),
        path: vibe_core::machine_json_path(&path),
        sha256: sha256_hex(text.as_bytes()),
        len: u32::try_from(text.len()).context("composed prefix exceeds report limit")?,
        local,
    };
    if ctx.is_json() {
        ctx.emit_json(&report)
    } else {
        println!("{}", path.display());
        Ok(())
    }
}
