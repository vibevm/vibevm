//! `vibe prompt path` — where an artifact is, and never an empty answer.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS");

use anyhow::{Result, bail};
use vibe_wire::generated::prompt_path as wire;
use vibe_workspace::system_prompt::{ArtifactFacts, PromptCheck, check_node_prompt};

use crate::cli::PromptPart;
use crate::output;

use super::Selected;

/// The protocol tag `vibe-prompt-path/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-path/1";

pub(crate) fn run(ctx: &output::Context, selected: &Selected, part: PromptPart) -> Result<()> {
    let tracked = selected.tracked();
    let request = selected.request(&tracked, None);
    let answer = check_node_prompt(&request)?;
    let facts = match part {
        PromptPart::Lane => &answer.lane,
        PromptPart::System => &answer.system,
    };
    let report = report(&answer, part, facts);
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        // The path goes to stdout alone, so `$(vibe prompt path)` is the
        // path and nothing else.
        println!("{}", facts.path.display());
    }
    // A file that is not there is not a path a caller may use, so it is a
    // refusal naming the build — never an empty line a shell alias turns
    // silently into a broken launch. A *stale* artifact exists and is
    // usable, so it is a warning beside the path rather than a refusal.
    if !facts.present {
        bail!(
            "`{}` has not been generated ({}) — run `{}`",
            facts.path.display(),
            freshness_word(&report),
            answer.remedy()
        );
    }
    if !answer.verdict.is_equal() {
        eprintln!(
            "warning: `{}` is {} ({}) — run `{}`",
            facts.path.display(),
            answer.verdict.as_str(),
            answer.verdict.reasons().join("; "),
            answer.remedy()
        );
    }
    Ok(())
}

fn report(answer: &PromptCheck, part: PromptPart, facts: &ArtifactFacts) -> wire::PromptPathReport {
    wire::PromptPathReport {
        protocol: PROTOCOL.to_owned(),
        node: answer.node_rel.clone(),
        part: match part {
            PromptPart::Lane => wire::Part::Lane,
            PromptPart::System => wire::Part::System,
        },
        path: facts.path.display().to_string(),
        present: facts.present,
        len: facts.len.map(|len| u32::try_from(len).unwrap_or(u32::MAX)),
        sha256: facts.sha256.clone(),
        freshness: if answer.opted_in {
            freshness(&answer.verdict)
        } else {
            wire::Freshness::OptedOut
        },
        remedy: answer.remedy(),
    }
}

fn freshness(verdict: &vibe_workspace::system_prompt::CheckVerdict) -> wire::Freshness {
    use vibe_workspace::system_prompt::CheckVerdict as V;

    match verdict {
        V::Equal => wire::Freshness::Equal,
        V::Stale(_) => wire::Freshness::Stale,
        V::Missing(_) => wire::Freshness::Missing,
        V::Conflict(_) => wire::Freshness::Conflict,
        V::Unavailable(_) => wire::Freshness::Unavailable,
        V::FreshnessUnverifiable(_) => wire::Freshness::FreshnessUnverifiable,
    }
}

/// The freshness word a refusal quotes, so the message and the report
/// cannot disagree.
fn freshness_word(report: &wire::PromptPathReport) -> &'static str {
    {
        match report.freshness {
            wire::Freshness::Equal => "equal",
            wire::Freshness::Stale => "stale",
            wire::Freshness::Missing => "missing",
            wire::Freshness::Conflict => "conflict",
            wire::Freshness::Unavailable => "unavailable",
            wire::Freshness::FreshnessUnverifiable => "freshness_unverifiable",
            wire::Freshness::OptedOut => "this project has not opted in",
        }
    }
}
