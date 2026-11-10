//! `vibe prompt flags` — flags only, never a program.
//!
//! That is the whole property: the output is appended to whatever wrapper,
//! alias or launcher a developer already has, which is why the argument may
//! be either a harness **kind** or a **launcher name** and why no program
//! ever appears in the answer (`#SYS-HARNESS-EXECUTABLE`).
//!
//! It also writes nothing. Decision D4 allowed `vibe prompt flags` to write
//! Codex's launch-time profile file beside `vibe agent`; it does not, by the
//! narrower rule that a command whose job is to print is a command that
//! prints. The note in the answer says which command writes the file.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use anyhow::{Result, bail};
use vibe_app::{AgentsConfig, AppError};
use vibe_wire::generated::prompt_flags as wire;
use vibe_workspace::system_prompt::{HarnessKind, LaunchFlags, check_node_prompt, launch_flags};

use crate::commands::agent::launchers;
use crate::commands::app::global;
use crate::output;

use super::Selected;

/// The protocol tag `vibe-prompt-flags/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-flags/1";

pub(crate) fn run(ctx: &output::Context, selected: &Selected, harness: &str) -> Result<()> {
    let (kind, launcher) = resolve_kind(selected, harness)?;
    let composed = compose(selected, kind)?;
    let report = wire::PromptFlagsReport {
        protocol: PROTOCOL.to_owned(),
        node: selected.node_rel.clone(),
        launcher,
        kind: kind.as_str().to_owned(),
        delivery: kind.delivery(&selected.manifest).as_str().to_owned(),
        flags: composed.flags.clone(),
        notes: composed.notes.clone(),
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    // One line of flags on stdout, so `claude $(vibe prompt flags --harness
    // claude)` is the whole integration; everything else is stderr.
    println!("{}", shell_line(&composed.flags));
    for note in &composed.notes {
        eprintln!("note: {note}");
    }
    Ok(())
}

/// Either a kind or a launcher name. A kind is tried first, so `--harness
/// claude` means the dialect even on a machine with a launcher of another
/// name bound to it.
fn resolve_kind(selected: &Selected, harness: &str) -> Result<(HarnessKind, Option<String>)> {
    if let Some(kind) = HarnessKind::parse(harness) {
        return Ok((kind, None));
    }
    launchers::refuse_project_launchers(&selected.workspace.root)?;
    let settings = global::settings_root()?;
    let config = AgentsConfig::load(&settings).map_err(AppError::from)?;
    let launcher = launchers::resolve(&settings, &config, harness)?;
    Ok((launcher.kind, Some(launcher.name)))
}

/// Compose the flags for one kind over this node's current artifacts.
///
/// The lane is read from disk rather than recomposed, and a lane that is not
/// there is a refusal naming the build: flags pointing at a file that does
/// not exist are the one answer a launch turns silently into a session with
/// no boot lane at all.
pub(crate) fn compose(selected: &Selected, kind: HarnessKind) -> Result<LaunchFlags> {
    let tracked = selected.tracked();
    let request = selected.request(&tracked, None);
    let answer = check_node_prompt(&request)?;
    if !answer.opted_in {
        bail!(
            "`{}` has not opted in to a system prompt (no authored `{}`), so there is no lane to \
             deliver and no flag to pass",
            selected.node_rel,
            vibe_workspace::path_to_slash(&vibe_core::layout::current_boot_system_template())
        );
    }
    if !answer.lane.present {
        bail!(
            "`{}` has not been generated — run `{}`",
            answer.lane.path.display(),
            answer.remedy()
        );
    }
    let bytes = std::fs::read_to_string(&answer.lane.path)?;
    let limit = selected.manifest.system_prompt.as_ref().map_or(
        vibe_workspace::boot_artifacts::PROJECT_DOC_MAX_BYTES,
        |section| section.codex_doc_max_bytes(),
    );
    let node_id = vibe_workspace::boot_artifacts::router_node_id(
        Some(&selected.manifest),
        &vibe_spec::SelfCoordinate::new(None, String::new()),
    );
    let mut composed = launch_flags(
        kind,
        kind.delivery(&selected.manifest),
        &node_id,
        &answer.lane.path,
        &bytes,
        limit,
    );
    if !answer.verdict.is_equal() {
        composed.notes.push(format!(
            "the lane is {} ({}) — run `{}`; these flags point at the bytes on disk now",
            answer.verdict.as_str(),
            answer.verdict.reasons().join("; "),
            answer.remedy()
        ));
    }
    Ok(composed)
}

/// One pasteable line. Only an argument that needs quoting gets it, so the
/// common case stays copy-pasteable without a shell's help.
fn shell_line(flags: &[String]) -> String {
    flags
        .iter()
        .map(|flag| {
            if flag.is_empty() || flag.contains([' ', '\t', '"', '\'', '$', '`', '\\']) {
                format!("'{}'", flag.replace('\'', "'\\''"))
            } else {
                flag.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
