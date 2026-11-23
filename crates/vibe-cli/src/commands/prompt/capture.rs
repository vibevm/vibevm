//! `vibe prompt capture` — the request envelope as a document.
//!
//! **Why this is a verb and not a field on `prepare`.** The plan put the
//! envelope projection inside `vibe prompt prepare --json`. It cannot go
//! there honestly: `prepare`'s own contract is "materials and observations,
//! never a ready-to-send request", and the envelope is precisely the
//! request — which role each band sits in, where the tool block goes, where
//! band T goes. Folding it into that document would contradict the sentence
//! `prepare` is built around, and would move `vibe-prompt-prepare/1`'s
//! epoch for every consumer that wanted none of it. `SP-FMT-011` already
//! names a **separate** format record, so the separate verb is the shape
//! the format was written for (`#SYS-REQUEST-ENVELOPE`).
//!
//! **It sends nothing.** No credentials, no model choice, no connection, no
//! provider. `--usage <file>` reads a response a caller already has on
//! disk; that is the only provider data in the command, and it is read from
//! a file rather than received.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-REQUEST-ENVELOPE");

use specmark::spec;
use std::path::Path;

use anyhow::{Context, Result};
use vibe_wire::generated::prompt_request_capture as wire;
use vibe_workspace::system_prompt::{check_node_prompt, prepare_request};

use crate::output;
use crate::output::language::{self as display};

use super::Selected;
use super::adapter::{CacheDiagnostics, ProviderUsage, RequestEnvelope, TOOLS_POSITION};

/// The protocol tag `vibe-prompt-capture/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-capture/1";

/// What `vibe prompt capture` was asked for.
pub(crate) struct Request<'a> {
    /// The session's operating system, as `os=<name>`.
    pub(crate) session_target: Option<&'a str>,
    /// Band T, supplied by the caller because it is the caller's.
    pub(crate) task: Option<&'a str>,
    /// Include each block's bytes in the document.
    pub(crate) with_text: bool,
    /// A provider response on disk, whose usage record is read back.
    pub(crate) usage: Option<&'a Path>,
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-REQUEST-ENVELOPE")]
pub(crate) fn run(ctx: &output::Context, selected: &Selected, request: Request<'_>) -> Result<()> {
    let target = request
        .session_target
        .map(super::prepare::parse_target)
        .transpose()?;
    let tracked = selected.tracked();
    let check = selected.request(&tracked, None);
    let answer = check_node_prompt(&check)?;
    let prepared = prepare_request(&check, &answer, target)?;
    let mut envelope = RequestEnvelope::build(
        &prepared,
        &answer.system.path,
        &selected.workspace.root,
        request.task,
    )?;
    super::personal::append(ctx, &mut envelope, target, &selected.node_dir);
    let read = request.usage.map(read_usage).transpose()?;
    let cache = read.as_ref().and_then(Read::diagnostics);

    let report = wire::PromptCaptureReport {
        protocol: PROTOCOL.to_owned(),
        node: selected.node_rel.clone(),
        session_target: target.map(|target| target.as_str().to_owned()),
        tools: wire::Tools {
            count: u32::try_from(envelope.tools.len()).unwrap_or(u32::MAX),
            position: TOOLS_POSITION.to_owned(),
        },
        instruction_blocks: envelope
            .instruction_blocks
            .iter()
            .map(|block| wire::Block {
                band: block.band.as_str().to_owned(),
                role: block.role.as_str().to_owned(),
                path: block.path.clone(),
                sha256: block.sha256.clone(),
                len: truncate(block.text.len()),
                offset: truncate(block.offset),
                text: request.with_text.then(|| block.text.clone()),
            })
            .collect(),
        messages: envelope
            .messages
            .iter()
            .map(|message| wire::Message {
                band: message.band.as_str().to_owned(),
                role: message.role.as_str().to_owned(),
                len: truncate(message.text.len()),
                text: request.with_text.then(|| message.text.clone()),
            })
            .collect(),
        cache_boundaries: prepared
            .cache_boundaries
            .iter()
            .copied()
            .map(truncate)
            .collect(),
        cache: cache.as_ref().map(|diagnostics| wire::Cache {
            source: diagnostics.source().to_owned(),
            write_tokens: diagnostics.write_tokens().map(truncate64),
            read_tokens: diagnostics.read_tokens().map(truncate64),
            uncached_tokens: diagnostics.uncached_tokens().map(truncate64),
        }),
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    let blocks = envelope.instruction_blocks.len();
    let tools = envelope.tools.len();
    let messages = envelope.messages.len();
    let block_plural = if blocks == 1 { "" } else { "s" };
    let tool_plural = if tools == 1 { "" } else { "s" };
    let message_plural = if messages == 1 { "" } else { "s" };
    let breakpoints = prepared
        .cache_boundaries
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    if ctx.is_quiet() {
        ctx.summary(&format!("system prompt envelope for {} · {blocks} instruction block{block_plural} · {tools} tool{tool_plural} {TOOLS_POSITION} · {messages} task message{message_plural} · {} bytes of instructions · breakpoints {breakpoints}", selected.node_rel, envelope.instruction_len()));
        return Ok(());
    }
    ctx.block(
        &display::Block::new(
            display::Glyph::Done,
            format!("system prompt  envelope for {}", selected.node_rel),
        )
        .with_meta(vec![
            display::Meta::note(format!("{blocks} instruction block{block_plural}")),
            display::Meta::note(format!("{tools} tool{tool_plural} {TOOLS_POSITION}")),
            display::Meta::note(format!("{messages} task message{message_plural}")),
            display::Meta::note(format!(
                "{} bytes of instructions",
                envelope.instruction_len()
            )),
            display::Meta::note(format!("breakpoints {breakpoints}")),
        ])
        .render(),
    );
    let rows = envelope
        .instruction_blocks
        .iter()
        .map(|block| {
            band_row(
                block.band.as_str(),
                &block.path,
                block.text.len(),
                Some(block.offset),
            )
        })
        .chain(
            envelope
                .messages
                .iter()
                .map(|message| band_row("T", "caller's own", message.text.len(), None)),
        )
        .collect();
    ctx.block(&display::List::new(rows).meta_column().render());
    match &read {
        Some(Read::Diagnostics(diagnostics)) => ctx.hint(&format!(
            "provider usage ({}): write {}, read {}, uncached {}",
            diagnostics.source(),
            reported(diagnostics.write_tokens()),
            reported(diagnostics.read_tokens()),
            reported(diagnostics.uncached_tokens())
        )),
        Some(Read::NoCounters(names)) => ctx.hint(&format!(
            "provider usage: the record carries {} and no cache counter this build knows, so \
             none are reported — absence is not zero",
            names.join(", ")
        )),
        None => {}
    }
    Ok(())
}

/// A counter the provider did not report prints as "not reported", never as
/// `0`: the two are different statements and only one of them is a
/// measurement.
fn reported(counter: Option<u64>) -> String {
    counter.map_or_else(|| "not reported".to_owned(), |count| count.to_string())
}

/// What a supplied provider response turned out to carry.
///
/// A record with no cache counters is its own answer, not a missing one:
/// three zeros would read as a measured miss. So the command reports the
/// absence, and names the counters the record *did* carry — which is the
/// sentence a caller holding such a response actually needs.
enum Read {
    /// Counters a provider reported.
    Diagnostics(CacheDiagnostics),
    /// A usage record with no cache counters, and the counter names it did
    /// carry.
    NoCounters(Vec<String>),
}

impl Read {
    fn diagnostics(&self) -> Option<&CacheDiagnostics> {
        match self {
            Read::Diagnostics(diagnostics) => Some(diagnostics),
            Read::NoCounters(_) => None,
        }
    }
}

/// Read one saved provider response's usage record.
fn read_usage(path: &Path) -> Result<Read> {
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("reading the provider response `{}`", path.display()))?;
    let usage = ProviderUsage::read(&body)
        .with_context(|| format!("reading usage out of `{}`", path.display()))?;
    Ok(match CacheDiagnostics::from_usage(&usage) {
        Some(diagnostics) => Read::Diagnostics(diagnostics),
        None => Read::NoCounters(
            usage
                .counter_names()
                .into_iter()
                .map(|name| format!("`{name}`"))
                .collect(),
        ),
    })
}

fn truncate(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn truncate64(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Keep instruction offsets separate from the caller's task-message bytes.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-REQUEST-ENVELOPE")]
fn band_row(band: &str, path: &str, len: usize, offset: Option<usize>) -> display::Row {
    let mut meta = vec![display::Meta::note(format!("{len} bytes"))];
    if let Some(offset) = offset {
        meta.push(display::Meta::note(format!("offset {offset}")));
    }
    display::Row::new(vec![
        display::Cell::new(band, display::Role::Dim),
        display::Cell::text(path),
    ])
    .meta(meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_capture_rows_carry_offsets() {
        let rows = vec![
            band_row("P", "SYSTEM.md", 1214, Some(0)),
            band_row("T", "caller's own", 12, None),
        ];
        let expected = format!(
            "  P  {:<14}1214 bytes · offset 0\n  T  {:<14}12 bytes",
            "SYSTEM.md", "caller's own"
        );
        assert_eq!(
            crate::output::language::List::new(rows)
                .meta_column()
                .render_at(100),
            expected
        );
    }
}
