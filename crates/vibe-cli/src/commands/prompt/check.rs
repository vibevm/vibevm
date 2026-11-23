//! `vibe prompt check` — read-only freshness, and the measured time it took.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK");

use anyhow::{Result, bail};
use specmark::spec;
use vibe_wire::generated::prompt_check as wire;
use vibe_workspace::system_prompt::{
    ArtifactFacts, CheckVerdict, PromptCheck, check_node_prompt, prompt_warnings,
};

use crate::output;
use crate::output::language::{Block, Cell, Glyph, List, Meta, Row};

use super::Selected;

/// The protocol tag `vibe-prompt-check/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-check/1";

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK")]
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
    bail!("{}", refusal_message(&answer))
}

fn refusal_message(answer: &PromptCheck) -> String {
    let verdict = answer.verdict.as_str();
    let mut message = format!(
        "prompt_{verdict}: the system prompt of {} is {} (violates spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK; fix: `{}`)",
        answer.node_rel,
        verdict.replace('_', " "),
        answer.remedy()
    );
    let reasons = answer.verdict.reasons();
    if !reasons.is_empty() {
        message.push_str(": ");
        message.push_str(&reasons.join("; "));
    }
    message
}

fn print_text(ctx: &output::Context, answer: &PromptCheck) {
    if !answer.opted_in {
        let template =
            vibe_workspace::path_to_slash(&vibe_core::layout::current_boot_system_template());
        ctx.block(
            &Block::new(Glyph::Skipped, "system prompt  not opted in")
                .with_meta(vec![Meta::note(format!("no {template}"))])
                .render(),
        );
        ctx.hint("vibe prompt init opts this project in");
        return;
    }
    if ctx.is_quiet() {
        if answer.verdict.is_equal() {
            ctx.summary(&format!("system prompt equal · {}", answer.node_rel));
        }
        return;
    }
    if answer.verdict.is_equal() {
        ctx.block(
            &Block::new(Glyph::Done, "system prompt  equal")
                .with_meta(vec![Meta::note(&answer.node_rel)])
                .render(),
        );
    }
    ctx.block(&artifact_list([&answer.system, &answer.meta, &answer.lane]).render());
    for warning in &answer.warnings {
        ctx.hint(warning);
    }
}

fn artifact_list(facts: [&ArtifactFacts; 3]) -> List {
    let rows = facts
        .into_iter()
        .map(|facts| {
            let name = facts.path.file_name().unwrap_or_default().to_string_lossy();
            let meta = match (facts.len, facts.sha256.as_deref()) {
                (Some(len), Some(sha256)) if facts.present => vec![
                    Meta::note(format!("{len} bytes")),
                    Meta::note(&sha256[..12.min(sha256.len())]),
                ],
                _ => vec![Meta::State("absent".into())],
            };
            Row::new(vec![Cell::text(name)]).meta(meta)
        })
        .collect();
    List::new(rows).meta_column()
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

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
