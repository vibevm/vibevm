//! The bounded human text projection of a validated report — the
//! shared twin the CLI prints and the JSON surface mirrors.
//!
//! Metadata only: source states, one bounded line per addressed fact
//! with its three observation columns, and the truncation summary. No
//! prose, no body, no recommendation, no ranking, no next task and no
//! synthetic verdict — the four axes stay four columns.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT");

use vibe_wire::generated::requirements_report::{
    AdoptionObservationPresence, AuthoringObservationPresence, RequirementsReport,
    SourceResultState,
};

/// Render the report's bounded text projection. Every scalar rendered
/// is one the wire validator already bounded; no new text is minted
/// from sources.
pub fn render(report: &RequirementsReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "requirements {}: selected={} sources={} rows={} truncated={}\n",
        report.requirements,
        report.observation.selected,
        report.sources.len(),
        report.rows.len(),
        report.truncated
    ));
    out.push_str(&format!(
        "observation_id={} source_digest={}\n",
        report.observation.observation_id, report.observation.source_digest
    ));
    for source in &report.sources {
        out.push_str(&format!(
            "source {} {}: {}",
            kind(&source.source.kind),
            source.source.package,
            state(&source.state),
        ));
        if let Some(reason) = &source.reason_code {
            out.push_str(&format!(" ({reason})"));
        }
        if let Some(entries) = source.adoption_entries {
            out.push_str(&format!(" adoption_entries={entries}"));
        }
        out.push('\n');
    }
    for relation in &report.relation_sources {
        // EVERY relation-source state is visible in the human output —
        // successful enrichment (not-requested/current/carried with no
        // reason) may not disappear just because it lost nothing.
        let mut line = format!(
            "relations {}: {}",
            relation.package,
            relation_state(relation)
        );
        if let Some(reason) = &relation.reason_code {
            line.push_str(&format!(" ({reason})"));
        }
        line.push('\n');
        out.push_str(&line);
    }
    for row in &report.rows {
        out.push_str(&format!(
            "{} authoring={} adoption={} relations={}\n",
            row.address,
            authoring(row),
            adoption(row),
            row.relations.len()
        ));
    }
    if report.truncated {
        out.push_str(&format!(
            "truncated: row set cut at the query's limit ({})\n",
            report.query.limit
        ));
    }
    out
}

fn kind(kind: &vibe_wire::generated::requirements_report::RequirementSourceKind) -> &'static str {
    match kind {
        vibe_wire::generated::requirements_report::RequirementSourceKind::Host => "host",
        vibe_wire::generated::requirements_report::RequirementSourceKind::Package => "package",
    }
}

fn state(state: &SourceResultState) -> &'static str {
    match state {
        SourceResultState::Available => "available",
        SourceResultState::Unavailable => "unavailable",
        SourceResultState::Invalid => "invalid",
        SourceResultState::Orphaned => "orphaned",
    }
}

fn relation_state(relation: &vibe_wire::generated::requirements_report::RelationSource) -> String {
    let spelling = match relation.state {
        vibe_wire::generated::requirements_report::RelationSourceState::NotRequested => {
            "not-requested"
        }
        vibe_wire::generated::requirements_report::RelationSourceState::Current => "current",
        vibe_wire::generated::requirements_report::RelationSourceState::Carried => "carried",
        vibe_wire::generated::requirements_report::RelationSourceState::Stale => "stale",
        vibe_wire::generated::requirements_report::RelationSourceState::Unavailable => {
            "unavailable"
        }
        vibe_wire::generated::requirements_report::RelationSourceState::Invalid => "invalid",
    };
    spelling.to_string()
}

fn authoring(row: &vibe_wire::generated::requirements_report::RequirementRow) -> String {
    match row.authoring.presence {
        AuthoringObservationPresence::Unmarked => "unmarked".to_string(),
        AuthoringObservationPresence::Marked => match &row.authoring.status {
            Some(status) => format!("marked={}/{}", stage(status), fact_state(status)),
            None => "marked".to_string(),
        },
    }
}

fn adoption(row: &vibe_wire::generated::requirements_report::RequirementRow) -> String {
    match row.adoption.presence {
        AdoptionObservationPresence::NotApplicable => "not-applicable".to_string(),
        AdoptionObservationPresence::Absent => "absent".to_string(),
        AdoptionObservationPresence::Indeterminate => "indeterminate".to_string(),
        AdoptionObservationPresence::Recorded => match &row.adoption.status {
            Some(status) => format!("recorded={}/{}", stage(status), fact_state(status)),
            None => "recorded".to_string(),
        },
    }
}

fn stage(status: &vibe_wire::generated::requirements_report::FactStatus) -> String {
    use vibe_wire::generated::requirements_report::FactStatusStage as S;
    match status.stage {
        S::Unknown => "unknown",
        S::Idea => "idea",
        S::Spec => "spec",
        S::Impl => "impl",
        S::Test => "test",
        S::Doc => "doc",
        S::Freeze => "freeze",
    }
    .to_string()
}

fn fact_state(status: &vibe_wire::generated::requirements_report::FactStatus) -> String {
    use vibe_wire::generated::requirements_report::FactStatusState as S;
    match status.state {
        S::Hold => "hold",
        S::Plan => "plan",
        S::Work => "work",
        S::Done => "done",
        S::Void => "void",
    }
    .to_string()
}
