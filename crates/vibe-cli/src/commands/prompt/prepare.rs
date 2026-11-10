//! `vibe prompt prepare` — bands P, U and G as separate ordered materials.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CONTINUATION-CACHEABLE");

use anyhow::{Result, bail};
use vibe_core::manifest::TargetOs;
use vibe_wire::generated::prompt_prepare as wire;
use vibe_workspace::system_prompt::{PreparedBand, check_node_prompt, prepare_request};

use crate::output;

use super::Selected;

/// The protocol tag `vibe-prompt-prepare/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-prepare/1";

pub(crate) fn run(
    ctx: &output::Context,
    selected: &Selected,
    session_target: Option<&str>,
) -> Result<()> {
    let target = session_target.map(parse_target).transpose()?;
    let tracked = selected.tracked();
    let mut request = selected.request(&tracked, None);
    // `--session-target os=<name>` is this verb's explicit statement of the
    // session's own operating system (`#SYS-SESSION-TARGET`), so when it is
    // supplied it **is** the session OS the host mismatch is measured
    // against; without one, this process's host stands. It still decides
    // the gated band on its own — `session_os` decides nothing.
    request.session_os = target.or(request.session_os);
    let answer = check_node_prompt(&request)?;
    let prepared = prepare_request(&request, &answer, target)?;
    let report = wire::PromptPrepareReport {
        protocol: PROTOCOL.to_owned(),
        node: selected.node_rel.clone(),
        session_target: target.map(|target| target.as_str().to_owned()),
        prefix: band(&prepared.prefix),
        unconditional: prepared.unconditional.iter().map(band).collect(),
        gated: prepared.gated.iter().map(band).collect(),
        unresolved: prepared
            .unresolved
            .iter()
            .map(|row| wire::Unresolved {
                path: row.path.clone(),
                when: row.when.clone(),
                reason: row.reason.clone(),
            })
            .collect(),
        cache_boundaries: prepared
            .cache_boundaries
            .iter()
            .map(|offset| u32::try_from(*offset).unwrap_or(u32::MAX))
            .collect(),
        warnings: prepared.warnings.clone(),
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    ctx.summary(&format!(
        "system prompt: prepared `{}` — band P {} bytes, band U {} file(s), band G {} file(s), \
         {} unresolved",
        selected.node_rel,
        prepared.prefix.len,
        prepared.unconditional.len(),
        prepared.gated.len(),
        prepared.unresolved.len()
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    println!(
        "  P {} — {} bytes",
        prepared.prefix.path, prepared.prefix.len
    );
    for member in &prepared.unconditional {
        println!("  U {} — {} bytes", member.path, member.len);
    }
    for member in &prepared.gated {
        println!("  G {} — {} bytes", member.path, member.len);
    }
    for row in &prepared.unresolved {
        println!("  ? {} ({}) — {}", row.path, row.when, row.reason);
    }
    // `note:`, the same word `check` uses for the same genre of line: an
    // observation that changed no material above. A host mismatch never
    // makes this a failure, so the exit status is untouched.
    for warning in &prepared.warnings {
        println!("  note: {warning}");
    }
    println!(
        "  cache breakpoints at {}",
        prepared
            .cache_boundaries
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

fn band(member: &PreparedBand) -> wire::Band {
    wire::Band {
        path: member.path.clone(),
        sha256: member.sha256.clone(),
        len: u32::try_from(member.len).unwrap_or(u32::MAX),
    }
}

/// `--session-target os=<name>`. The shape carries the axis so a later
/// session fact needs no second flag, and an unknown selector refuses
/// rather than becoming `true` (`#SYS-SESSION-TARGET`).
///
/// Shared with `capture`, which decides the same gated rows from the same
/// flag: two parsers for one spelling is how the two verbs would come to
/// disagree about what `os=` means.
pub(super) fn parse_target(value: &str) -> Result<TargetOs> {
    let Some(name) = value.strip_prefix("os=") else {
        bail!(
            "`--session-target {value}` is not `os=<name>`; this version decides only the `os:` axis"
        );
    };
    match name {
        "windows" => Ok(TargetOs::Windows),
        "macos" => Ok(TargetOs::Macos),
        "linux" => Ok(TargetOs::Linux),
        other => bail!(
            "`--session-target os={other}` names no operating system this build knows; they are \
             `windows`, `macos` and `linux`"
        ),
    }
}
