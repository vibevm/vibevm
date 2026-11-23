//! `vibe prompt prepare` — bands P, U and G as separate ordered materials.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CONTINUATION-CACHEABLE");

use anyhow::{Result, bail};
use specmark::spec;
use vibe_core::manifest::TargetOs;
use vibe_wire::generated::prompt_prepare as wire;
use vibe_workspace::system_prompt::{
    PreparedBand, PreparedRequest, check_node_prompt, prepare_request,
};

use crate::output;
use crate::output::language::{Block, Cell, Glyph, List, Meta, Role, Row};

use super::Selected;

/// The protocol tag `vibe-prompt-prepare/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-prepare/1";

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CONTINUATION-CACHEABLE")]
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
    let breakpoints = prepared
        .cache_boundaries
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "system prompt prepared {} · {} unresolved · breakpoints {breakpoints}",
            selected.node_rel,
            prepared.unresolved.len()
        ));
        return Ok(());
    }
    ctx.block(
        &Block::new(
            Glyph::Done,
            format!("system prompt  prepared {}", selected.node_rel),
        )
        .with_meta(vec![
            Meta::note(format!("{} unresolved", prepared.unresolved.len())),
            Meta::note(format!("breakpoints {breakpoints}")),
        ])
        .render(),
    );
    ctx.block(&band_list(&prepared).render());
    for warning in &prepared.warnings {
        ctx.hint(warning);
    }
    Ok(())
}

/// Present resolved materials and unresolved conditions without rewriting either.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CONTINUATION-CACHEABLE")]
fn band_list(prepared: &PreparedRequest) -> List {
    let mut rows = Vec::new();
    for (name, bands) in [
        ("P", std::slice::from_ref(&prepared.prefix)),
        ("U", prepared.unconditional.as_slice()),
        ("G", prepared.gated.as_slice()),
    ] {
        for member in bands {
            rows.push(
                Row::new(vec![Cell::new(name, Role::Dim), Cell::text(&member.path)])
                    .meta(vec![Meta::note(format!("{} bytes", member.len))]),
            );
        }
    }
    for row in &prepared.unresolved {
        rows.push(
            Row::new(vec![Cell::new("?", Role::Warn), Cell::text(&row.path)])
                .meta(vec![Meta::note(&row.when), Meta::note(&row.reason)]),
        );
    }
    List::new(rows).meta_column()
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

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_workspace::system_prompt::{PreparedRequest, UnresolvedRow};

    #[test]
    fn prompt_prepare_bands_are_measured_rows() {
        let band = |path: &str, len: usize| PreparedBand {
            path: path.into(),
            sha256: "0".repeat(64),
            len,
        };
        let prepared = PreparedRequest {
            prefix: band("SYSTEM.md", 1214),
            unconditional: vec![band("vibevm/vibespecs/boot/00-core.md", 1487)],
            gated: Vec::new(),
            unresolved: vec![UnresolvedRow {
                path: "vibevm/vibespecs/boot/50-os.md".into(),
                when: "os:windows".into(),
                reason: "no session target was given".into(),
            }],
            cache_boundaries: vec![1214, 2701],
            warnings: Vec::new(),
        };
        let expected = format!(
            "  P  {:<34}1214 bytes\n  U  {:<34}1487 bytes\n  ?  {:<34}os:windows · no session target was given",
            "SYSTEM.md", "vibevm/vibespecs/boot/00-core.md", "vibevm/vibespecs/boot/50-os.md"
        );
        assert_eq!(band_list(&prepared).render_at(100), expected);
    }
}
