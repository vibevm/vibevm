//! Standalone line-oriented surface over the same typed route as structured CLI.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#DEBUGGER-VIEWER");
use super::*;
use vibe_wire::generated::{
    boot_debugger::e1::{request as q, result as r},
    machine::e1::operation_envelope as env,
};
use vibe_workspace::boot_debugger::viewer::{Action, Frame, Service, Viewer};

pub(super) struct RouteService<'a>(pub &'a BootDebuggerRoute);
impl Service for RouteService<'_> {
    fn query(&self, command: q::Command) -> std::result::Result<r::Result, String> {
        let request = boot_debugger::input(&q::Request {
            schema: 1,
            context_id: self.0.context_id().into(),
            command,
        });
        let bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        match self.0.exchange(&bytes).body {
            env::Body::Result(body) => {
                let value: r::Result = serde_json::from_str(
                    body.payload_json
                        .as_deref()
                        .ok_or("missing result payload")?,
                )
                .map_err(|e| e.to_string())?;
                if value.context_id != self.0.context_id() {
                    return Err("foreign result context".into());
                }
                Ok(value)
            }
            env::Body::Error(body) => Err(body
                .payload_json
                .unwrap_or_else(|| "debugger refused request".into())),
            _ => Err("expected terminal debugger result".into()),
        }
    }
}

const HELP: &str = "Boot inspector — commands (row and snapshot ordinals start at 0):\nshow | next | prev | parent | child | select ROW | detail | source [BYTE_OFFSET]\nrefresh | snapshot N | compare BEFORE AFTER | scenario OS_TARGET DRAFT_JSON | preview SELECTORS_JSON | help | quit\nScenario drafts contain baseline_selections (features, no_defaults, all_features, configuration) and edits.\nEdit actions: add (package, loading), remove (coordinate), toggle (package, enabled, loading), reconfigure (selections).\nLoading is null or a legal link value; configuration is null or an explicit consumer request.\nPreview selectors: scenario_id, expected_baseline, expected_candidate, expected_catalog, loading_overrides.\nSource displays captured emitted bytes, not authored source coordinates.\n";

fn parse(line: &str) -> std::result::Result<Action, String> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("scenario ") {
        let (target, draft) = rest
            .split_once(' ')
            .ok_or("expected scenario OS_TARGET DRAFT_JSON")?;
        // Validate the immutable control vocabulary before the shared producer.
        let _: vibe_workspace::boot_debugger::scenario::Draft =
            serde_json::from_str(draft).map_err(|e| e.to_string())?;
        return Ok(Action::Scenario {
            target: target.into(),
            draft_json: draft.into(),
        });
    }
    if let Some(rest) = line.strip_prefix("preview ") {
        return serde_json::from_str(rest)
            .map(Action::Preview)
            .map_err(|e| e.to_string());
    }
    let words: Vec<_> = line.split_whitespace().collect();
    let ordinal = |s: &str| {
        s.parse::<usize>()
            .map_err(|_| "expected row/snapshot ordinal, never a label or path".to_string())
    };
    Ok(match words.as_slice() {
        ["show"] => Action::Show,
        ["next"] => Action::Next,
        ["prev"] => Action::Previous,
        ["parent"] => Action::Parent,
        ["child"] => Action::Child,
        ["select", n] => Action::Select(ordinal(n)?),
        ["snapshot", n] => Action::Snapshot(ordinal(n)?),
        ["refresh"] => Action::Refresh,
        ["detail"] => Action::Detail,
        ["source"] => Action::Source(0),
        ["source", n] => Action::Source(n.parse().map_err(|_| "expected original-byte offset")?),
        ["compare", a, b] => Action::Compare(ordinal(a)?, ordinal(b)?),
        _ => return Err("unknown command or arguments; type help".into()),
    })
}

fn render(frame: &Frame, json: bool, output: &mut impl Write) -> Result<()> {
    if json {
        serde_json::to_writer(&mut *output, frame)?;
        writeln!(output)?;
    } else {
        writeln!(
            output,
            "Snapshot {:?} — {} occurrences; snapshots {:?}",
            frame.snapshot_id, frame.total, frame.snapshots
        )?;
        for (i, node) in frame.nodes.iter().enumerate() {
            let ordinal = frame.start + i;
            writeln!(
                output,
                "{} {ordinal}: {}:{} origin={:?} path={:?} requested={:?} parent={:?}",
                if frame.selected == Some(ordinal) {
                    ">"
                } else {
                    " "
                },
                node.contribution,
                node.occurrence,
                node.origin,
                node.path,
                node.requested,
                node.parent
            )?;
            if frame.selected == Some(ordinal) {
                writeln!(
                    output,
                    "Source: {}",
                    serde_json::to_string(&frame.sources.iter().find(|s| s.source == node.source))?
                )?;
                writeln!(
                    output,
                    "Captured emitted lines: {}",
                    serde_json::to_string(&node.artifact_lines)?
                )?;
            }
        }
        if let Some(catalog) = &frame.derivative_catalog_json {
            writeln!(output, "Retained derivative history: {catalog}")?;
        }
        for gap in &frame.gaps {
            writeln!(output, "Gap: {gap:?}")?;
        }
        for (snapshot, gaps) in &frame.comparison_gaps {
            writeln!(output, "Comparison gaps {snapshot:?}: {gaps:?}")?;
        }
        if let Some(observation) = &frame.observation {
            writeln!(output, "{}", serde_json::to_string_pretty(observation)?)?;
            if let r::Body::Chunk(chunk) = &observation.body {
                let bytes = chunk
                    .hex_bytes
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| {
                        std::str::from_utf8(pair)
                            .ok()
                            .and_then(|s| u8::from_str_radix(s, 16).ok())
                            .unwrap_or(b'?')
                    })
                    .collect::<Vec<_>>();
                // Debug escaping prevents captured terminal controls from executing.
                writeln!(
                    output,
                    "Captured bytes: {:?}",
                    String::from_utf8_lossy(&bytes)
                )?;
                writeln!(output, "Next original-byte offset: {:?}", chunk.next_offset)?;
            }
        }
        if let Some(diagnostic) = &frame.diagnostic {
            writeln!(output, "Diagnostic: {diagnostic:?}")?;
        }
    }
    output.flush()?;
    Ok(())
}

pub(super) fn serve(
    route: &BootDebuggerRoute,
    mut input: impl BufRead,
    output: &mut impl Write,
    script: Option<&str>,
) -> Result<()> {
    let mut viewer = Viewer::new(RouteService(route)).map_err(anyhow::Error::msg)?;
    let json = script.is_some();
    if !json {
        write!(output, "{HELP}")?;
    }
    render(&viewer.frame(None, None), json, output)?;
    let mut script = script.map(|s| {
        // Commands may carry JSON strings containing semicolons. Only a
        // delimiter outside a quoted string separates script commands.
        let (mut quoted, mut escaped) = (false, false);
        s.split(move |c| {
            if escaped {
                escaped = false;
                return false;
            }
            if quoted && c == '\\' {
                escaped = true;
                return false;
            }
            if c == '"' {
                quoted = !quoted;
            }
            c == ';' && !quoted
        })
    });
    loop {
        let line = if let Some(script) = &mut script {
            let Some(line) = script.next() else {
                break;
            };
            line.to_owned()
        } else {
            write!(output, "boot> ")?;
            output.flush()?;
            let mut bytes = Vec::new();
            let n = input.by_ref().take(4098).read_until(b'\n', &mut bytes)?;
            if n == 0 {
                break;
            }
            if bytes.len() > 4097 {
                anyhow::bail!("viewer command exceeds frame bound");
            }
            String::from_utf8(bytes)?
        };
        if line.trim() == "quit" {
            break;
        }
        if line.trim() == "help" {
            if !json {
                write!(output, "{HELP}")?;
            }
            continue;
        }
        let (observation, diagnostic) = match parse(&line).and_then(|a| viewer.apply(a)) {
            Ok(value) => (value, None),
            Err(error) => (None, Some(error)),
        };
        render(&viewer.frame(observation, diagnostic), json, output)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "viewer_tests.rs"]
mod m_06_c_2_cases;
