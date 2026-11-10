//! `vibe prompt check` — read-only freshness, and the measured time it took.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK");

use anyhow::{Result, bail};
use vibe_wire::generated::prompt_check as wire;
use vibe_workspace::system_prompt::{
    ArtifactFacts, CheckVerdict, PromptCheck, check_node_prompt, prompt_warnings,
};

use crate::output;

use super::Selected;

/// The protocol tag `vibe-prompt-check/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-check/1";

pub(crate) fn run(
    ctx: &output::Context,
    selected: &Selected,
    model_window: Option<usize>,
) -> Result<()> {
    let tracked = selected.tracked();
    let request = selected.request(&tracked, model_window);
    let mut answer = check_node_prompt(&request)?;
    answer.warnings = prompt_warnings(&request, &answer)?;
    let report = report(&answer);
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        print_text(ctx, &answer);
    }
    // Equal and "never opted in" are the two answers that need no action;
    // everything else names a remedy, so exiting zero would make a wrapper
    // believe a stale prefix was current.
    if answer.verdict.is_equal() {
        return Ok(());
    }
    bail!(
        "the system prompt of `{}` is {}: {} — run `{}`",
        answer.node_rel,
        answer.verdict.as_str(),
        answer.verdict.reasons().join("; "),
        answer.remedy()
    )
}

fn print_text(ctx: &output::Context, answer: &PromptCheck) {
    if !answer.opted_in {
        ctx.summary(&format!(
            "system prompt: {} — `{}` has not opted in (no authored `{}`)",
            answer.verdict.as_str(),
            answer.node_rel,
            vibe_workspace::path_to_slash(&vibe_core::layout::current_boot_system_template())
        ));
        return;
    }
    ctx.summary(&format!(
        "system prompt: {} — `{}`",
        answer.verdict.as_str(),
        answer.node_rel
    ));
    if ctx.is_quiet() {
        return;
    }
    for reason in answer.verdict.reasons() {
        println!("  reason: {reason}");
    }
    if !answer.verdict.is_equal() {
        println!("  fix: {}", answer.remedy());
    }
    for facts in [&answer.system, &answer.meta, &answer.lane] {
        println!("  {}", artifact_line(facts));
    }
    for warning in &answer.warnings {
        println!("  note: {warning}");
    }
}

fn artifact_line(facts: &ArtifactFacts) -> String {
    match (facts.len, facts.sha256.as_deref()) {
        (Some(len), Some(sha256)) => format!(
            "{} — {len} bytes, {}",
            facts.path.display(),
            &sha256[..12.min(sha256.len())]
        ),
        _ => format!("{} — absent", facts.path.display()),
    }
}

/// Lower one answer into its registered report.
pub(crate) fn report(answer: &PromptCheck) -> wire::PromptCheckReport {
    wire::PromptCheckReport {
        protocol: PROTOCOL.to_owned(),
        node: answer.node_rel.clone(),
        opted_in: answer.opted_in,
        verdict: verdict(&answer.verdict),
        reasons: answer.verdict.reasons(),
        remedy: answer.remedy(),
        artifacts: vec![
            artifact(wire::Part::System, &answer.system),
            artifact(wire::Part::Meta, &answer.meta),
            artifact(wire::Part::Lane, &answer.lane),
        ],
        warnings: answer.warnings.clone(),
    }
}

pub(crate) fn verdict(verdict: &CheckVerdict) -> wire::Verdict {
    match verdict {
        CheckVerdict::Equal => wire::Verdict::Equal,
        CheckVerdict::Stale(_) => wire::Verdict::Stale,
        CheckVerdict::Missing(_) => wire::Verdict::Missing,
        CheckVerdict::Conflict(_) => wire::Verdict::Conflict,
        CheckVerdict::Unavailable(_) => wire::Verdict::Unavailable,
        CheckVerdict::FreshnessUnverifiable(_) => wire::Verdict::FreshnessUnverifiable,
    }
}

fn artifact(part: wire::Part, facts: &ArtifactFacts) -> wire::Artifact {
    wire::Artifact {
        part,
        path: facts.path.display().to_string(),
        present: facts.present,
        len: facts.len.map(|len| u32::try_from(len).unwrap_or(u32::MAX)),
        sha256: facts.sha256.clone(),
    }
}
