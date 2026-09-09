specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#TERMINAL-OBSERVATION-SURFACE"
);

use super::Row;
use crate::model::ArtifactRequirements;
use crate::rollup::DocRollup;
use crate::terminal::ArtifactObservation;

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The native XML report. Schema 1 is byte-compatible legacy output;
/// schema 2 announces the conditional terminal observation members.
pub fn render_xml(rows: &[Row], rollups: &[(String, DocRollup)]) -> String {
    render_xml_with_mode(rows, rollups, false)
}

/// Render XML while optionally preserving an explicitly selected terminal
/// surface even when its filter produced zero rows.
pub fn render_xml_with_mode(
    rows: &[Row],
    rollups: &[(String, DocRollup)],
    terminal_mode: bool,
) -> String {
    let schema = if terminal_mode || rows.iter().any(|row| row.terminal.is_some()) {
        2
    } else {
        1
    };
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<progress-report schema=\"{schema}\">\n"
    );
    s.push_str("  <files>\n");
    for (path, r) in rollups {
        let eff = r
            .effective
            .map(|(st, sta)| format!(" stage=\"{st}\" state=\"{sta}\""))
            .unwrap_or_default();
        s.push_str(&format!(
            "    <file path=\"{}\"{} markers=\"{}\" facts=\"{}\" unmarked=\"{}\"/>\n",
            xml_escape(path),
            eff,
            r.marker_count,
            r.fact_count,
            r.unmarked_facts
        ));
    }
    s.push_str("  </files>\n  <markers>\n");
    for row in rows {
        let mut attrs = format!(
            "    <marker path=\"{}\" line=\"{}\" granularity=\"{:?}\" stage=\"{}\" state=\"{}\"{}{}",
            xml_escape(&row.path),
            row.line,
            row.granularity,
            row.stage,
            row.state,
            row.action
                .as_deref()
                .map(|a| format!(" action=\"{}\"", xml_escape(a)))
                .unwrap_or_default(),
            row.comment
                .as_deref()
                .map(|c| format!(" comment=\"{}\"", xml_escape(c)))
                .unwrap_or_default(),
        );
        if let Some(address) = &row.address {
            attrs.push_str(&format!(" address=\"{}\"", xml_escape(address)));
        }
        if let Some(requirements) = &row.requires {
            attrs.push_str(&format!(" requires=\"{}\"", requirements.to_csv()));
        }
        if let Some(terminal) = row.terminal {
            attrs.push_str(&format!(" terminal=\"{:?}\"", terminal).to_lowercase());
        }
        s.push_str(&attrs);
        if row.evidence.is_none() && row.artifacts.is_empty() {
            s.push_str("/>\n");
            continue;
        }
        s.push_str(">\n");
        if let Some(ev) = &row.evidence {
            s.push_str(&format!(
                "      <evidence implements=\"{}\" verifies=\"{}\"{}/>\n",
                ev.implements,
                ev.verifies,
                row.mismatch
                    .as_deref()
                    .map(|m| format!(" mismatch=\"{}\"", xml_escape(m)))
                    .unwrap_or_default(),
            ));
        }
        for artifact in &row.artifacts {
            push_artifact_xml(&mut s, artifact);
        }
        s.push_str("    </marker>\n");
    }
    s.push_str("  </markers>\n</progress-report>\n");
    s
}

fn push_artifact_xml(out: &mut String, artifact: &ArtifactObservation) {
    let count = artifact
        .count
        .map(|value| format!(" count=\"{value}\""))
        .unwrap_or_default();
    let state = format!("{:?}", artifact.state).to_lowercase();
    match &artifact.locators {
        None => out.push_str(&format!(
            "      <artifact kind=\"{}\" state=\"{state}\"{count}/>\n",
            artifact.kind
        )),
        Some(locators) if locators.is_empty() => out.push_str(&format!(
            "      <artifact kind=\"{}\" state=\"{state}\"{count}/>\n",
            artifact.kind
        )),
        Some(locators) => {
            out.push_str(&format!(
                "      <artifact kind=\"{}\" state=\"{state}\"{count}>\n",
                artifact.kind
            ));
            for locator in locators {
                out.push_str(&format!(
                    "        <locator>{}</locator>\n",
                    xml_escape(locator)
                ));
            }
            out.push_str("      </artifact>\n");
        }
    }
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// The Markdown table, conditionally extended by evidence and terminal data.
pub fn render_md(rows: &[Row], rollups: &[(String, DocRollup)]) -> String {
    render_md_with_mode(rows, rollups, false)
}

/// Render Markdown while preserving an explicitly selected empty terminal
/// view as a terminal-shaped table.
pub fn render_md_with_mode(
    rows: &[Row],
    rollups: &[(String, DocRollup)],
    terminal_mode: bool,
) -> String {
    let with_evidence = rows.iter().any(|r| r.evidence.is_some());
    let with_terminal = terminal_mode || rows.iter().any(|r| r.terminal.is_some());
    let mut s = match (with_evidence, with_terminal) {
        (true, true) => String::from(
            "| source | stage | state | action | comment | evidence | requires | artifacts | terminal |\n|---|---|---|---|---|---|---|---|---|\n",
        ),
        (true, false) => String::from(
            "| source | stage | state | action | comment | evidence |\n|---|---|---|---|---|---|\n",
        ),
        (false, true) => String::from(
            "| source | stage | state | action | comment | requires | artifacts | terminal |\n|---|---|---|---|---|---|---|---|\n",
        ),
        (false, false) => {
            String::from("| source | stage | state | action | comment |\n|---|---|---|---|---|\n")
        }
    };
    for (path, r) in rollups {
        push_rollup_md(&mut s, path, r, with_evidence, with_terminal);
    }
    for row in rows {
        s.push_str(&format!(
            "| {}:{} | {} | {} | {} | {} |",
            md_escape(&row.path),
            row.line,
            row.stage,
            row.state,
            row.action.as_deref().map(md_escape).unwrap_or_default(),
            row.comment.as_deref().map(md_escape).unwrap_or_default(),
        ));
        if with_evidence {
            match &row.evidence {
                Some(ev) => s.push_str(&format!(
                    " impl={} ver={}{} |",
                    ev.implements,
                    ev.verifies,
                    if row.mismatch.is_some() { " ⚠" } else { "" },
                )),
                None => s.push_str("  |"),
            }
        }
        if with_terminal {
            s.push_str(&format!(
                " {} | {} | {} |",
                row.requires
                    .as_ref()
                    .map(ArtifactRequirements::to_csv)
                    .unwrap_or_default(),
                md_artifacts(&row.artifacts),
                row.terminal
                    .map(|value| format!("{value:?}").to_lowercase())
                    .unwrap_or_default(),
            ));
        }
        s.push('\n');
    }
    s
}

fn push_rollup_md(
    out: &mut String,
    path: &str,
    rollup: &DocRollup,
    with_evidence: bool,
    with_terminal: bool,
) {
    if let Some((stage, state)) = rollup.effective {
        out.push_str(&format!(
            "| **{}** ({} markers, {}/{} unmarked) | {} | {} |  |  |",
            md_escape(path),
            rollup.marker_count,
            rollup.unmarked_facts,
            rollup.fact_count,
            stage,
            state
        ));
    } else {
        out.push_str(&format!(
            "| **{}** (no markers, {} facts) | — | — |  |  |",
            md_escape(path),
            rollup.fact_count
        ));
    }
    if with_evidence {
        out.push_str("  |");
    }
    if with_terminal {
        out.push_str("  |  |  |");
    }
    out.push('\n');
}

fn md_artifacts(artifacts: &[ArtifactObservation]) -> String {
    artifacts
        .iter()
        .map(|artifact| {
            let state = format!("{:?}", artifact.state).to_lowercase();
            let mut value = format!("{}={state}", artifact.kind);
            if let Some(locators) = &artifact.locators
                && !locators.is_empty()
            {
                value.push_str(&format!(" [{}]", locators.join(", ")));
            }
            md_escape(&value)
        })
        .collect::<Vec<_>>()
        .join("; ")
}
