//! Request-local personal boot; never writes project or personal artifacts.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#USER-LANE-LAST");
use super::adapter::{Band, Block, RequestEnvelope, Role};
use crate::output;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use vibe_core::manifest::{TargetOs, WhenCondition};
use vibe_workspace::system_prompt::sha256_hex;

const CAP: usize = 16 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: u32,
    #[serde(rename = "static")]
    static_lane: Option<String>,
    #[serde(default)]
    entry: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    kind: String,
    when: Option<WhenCondition>,
}

pub(super) fn append(
    ctx: &output::Context,
    envelope: &mut RequestEnvelope,
    target: Option<TargetOs>,
    project: &std::path::Path,
) {
    let mut blocks = layered_blocks(ctx, target, project);
    let mut offset = envelope.instruction_len();
    for block in &mut blocks {
        block.offset = offset;
        offset += block.text.len();
    }
    envelope.instruction_blocks.extend(blocks);
}

pub(super) fn layered_blocks(
    ctx: &output::Context,
    target: Option<TargetOs>,
    project: &std::path::Path,
) -> Vec<Block> {
    let mut blocks = blocks(ctx, target);
    let loaded = crate::commands::user_override::selected_root(project)
        .and_then(|root| root.map(|root| load(&root, target)).transpose());
    match loaded {
        Ok(Some(mut layer)) => {
            layer.retain(|block| block.path != "personal:memory-policy");
            for block in &mut layer {
                block.band = Band::Override;
                block.path = block.path.replacen("personal:", "override:", 1);
                if block.path == "override:precedence" {
                    block.text = "Project override preferences supplement the project and take precedence over general personal preferences only for rules the project marks overridable. Safety, attribution, commit discipline and all unmarked project rules remain binding; report conflicts.\n".into();
                    block.sha256 = sha256_hex(block.text.as_bytes());
                }
            }
            blocks.extend(layer);
        }
        Ok(None) => {}
        Err(_) => ctx
            .diagnostic("project override boot skipped: binding or compiled layer is unavailable"),
    }
    blocks
}

fn blocks(ctx: &output::Context, target: Option<TargetOs>) -> Vec<Block> {
    let selected = vibe_core::user_project::admit_root(vibe_core::settings::settings_dir());
    let root = match selected {
        Ok(root) => root,
        Err(_) => {
            ctx.diagnostic("personal boot skipped: settings root is unavailable");
            return Vec::new();
        }
    };
    use vibe_workspace::user_project::Observation;
    match vibe_workspace::user_project::status(&root) {
        Observation::Absent | Observation::Empty => return Vec::new(),
        Observation::Unavailable(_) => {
            ctx.diagnostic(
                "personal boot skipped: personal project is unavailable; run `vibe user status`",
            );
            return Vec::new();
        }
        Observation::Initialized { .. } => {}
    }
    match load(&root, target) {
        Ok(blocks) => blocks,
        Err(_) => {
            ctx.diagnostic("personal boot skipped: compiled personal lane is unavailable, invalid or stale; install the personal project's requirements and retry");
            Vec::new()
        }
    }
}
fn load(root: &std::path::Path, target: Option<TargetOs>) -> Result<Vec<Block>> {
    let project = vibe_safefs::Project::open(root)?;
    let index_path = vibe_core::machine_json_path(&vibe_core::layout::current_boot_index());
    let index = project
        .read_file_snapshot_bounded(&index_path, CAP)?
        .context("personal boot index absent")?;
    let parsed: Index = toml::from_str(std::str::from_utf8(&index.bytes)?)?;
    if parsed.schema != 1 {
        bail!("unsupported personal boot index");
    }
    let workspace = vibe_workspace::Workspace::load(root)?;
    let format = crate::commands::reinstall::effective_rederive_format(&workspace.root_manifest)?;
    if !vibe_workspace::install::verify_boot_graph_with_spec_format(&workspace, format)?.is_empty()
    {
        bail!("personal boot is stale");
    }
    let mut paths = Vec::new();
    if let Some(path) = parsed.static_lane {
        paths.push(path);
    }
    for entry in parsed.entry {
        if entry.kind != "static" && entry.kind != "dynamic" {
            bail!("unsupported personal entry kind");
        }
        let include = match entry.when {
            None => true,
            Some(WhenCondition::Os(os)) => {
                target.context("personal boot requires explicit session target")? == os
            }
            Some(_) => bail!("unresolved install-time predicate in personal boot"),
        };
        if include {
            paths.push(entry.path);
        }
    }
    let mut blocks = Vec::new();
    let mut total = 0usize;
    // The explicit project precedence instruction is request-local as well.
    let precedence = "Personal preferences supplement project instructions. Project safety, attribution, commit discipline and other non-overridable rules remain binding. Report conflicts instead of replacing those rules.\n";
    blocks.push(Block {
        band: Band::Personal,
        role: Role::Instruction,
        path: "personal:precedence".into(),
        sha256: sha256_hex(precedence.as_bytes()),
        offset: 0,
        text: precedence.into(),
    });
    let memory = vibe_core::user_project::memory::INSTRUCTIONS;
    blocks.push(Block {
        band: Band::Personal,
        role: Role::Instruction,
        path: "personal:memory-policy".into(),
        sha256: sha256_hex(memory.as_bytes()),
        offset: 0,
        text: memory.into(),
    });
    for path in paths {
        let snapshot = project
            .read_file_snapshot_bounded(&path, CAP - total)?
            .context("personal boot member absent")?;
        total = total
            .checked_add(snapshot.bytes.len())
            .context("personal lane size overflow")?;
        if total > CAP {
            bail!("personal lane exceeds request budget");
        }
        let text = String::from_utf8(snapshot.bytes)?;
        blocks.push(Block {
            band: Band::Personal,
            role: Role::Instruction,
            path: format!("personal:{path}"),
            sha256: sha256_hex(text.as_bytes()),
            offset: 0,
            text,
        });
    }
    Ok(blocks)
}
