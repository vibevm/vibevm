//! Stable, escaped editorial inputs; no net-delivery inference.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#TRANSITION-INDEX");

use super::Result;
use std::collections::BTreeSet;
use vibe_wire::generated::preview::e1::{release_impact::ReleaseImpact, transition::Transition};

fn text(value: &str) -> String {
    let mut escaped = String::new();
    for c in value.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            c if c.is_control()
                || matches!(c, '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}' | '\u{061c}') =>
            {
                escaped.push_str(&format!("\\\\u{{{:x}}}", c as u32));
            }
            c if c.is_ascii_punctuation() => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

fn state(value: &impl serde::Serialize) -> Result<String> {
    // Vocabulary spellings come from generated Serialize, not duplicated enums.
    let wire = serde_json::to_string(value)?;
    Ok(text(wire.trim_matches('"')))
}

fn field(out: &mut String, label: &str, value: &str) {
    out.push_str(&format!("- {label}: {}\n", text(value)));
}

fn header(transition: &Transition) -> Result<String> {
    let mut out = "Generated draft. Assembly does not accept records, establish net delivery, verify evidence, or complete migration/documentation proof.\n\n## Exact capture references\n\n".to_owned();
    field(&mut out, "DP1 capture", &transition.dp1.capture_id);
    field(&mut out, "DP1 commit", &transition.dp1.commit_sha1);
    field(&mut out, "DP1 tree", &transition.dp1.git_tree_sha1);
    field(
        &mut out,
        "DP1 capture SHA256",
        &transition.dp1.capture_file_sha256,
    );
    out.push_str(&format!("- DP2 state: {}\n", state(&transition.dp2.state)?));
    if let Some(id) = &transition.dp2.capture_id {
        field(&mut out, "DP2 capture", id);
    }
    for (label, value) in [
        ("DP2 commit", &transition.dp2.commit_sha1),
        ("DP2 tree", &transition.dp2.git_tree_sha1),
        ("DP2 capture SHA256", &transition.dp2.capture_file_sha256),
    ] {
        if let Some(value) = value {
            field(&mut out, label, value);
        }
    }
    if transition.dp2.capture_id.is_none() {
        out.push_str("- UNRESOLVED: DP2 has no capture; record after-states remain declared contributions, unbound to the final preview.\n");
    }
    if let Some(captures) = &transition.captures {
        if captures.is_empty() {
            out.push_str("- Additional capture catalogue: explicitly supplied empty.\n");
        }
        for capture in captures {
            out.push_str("\n### Additional supplied capture (unverified by assembly)\n\n");
            for (label, value) in [
                ("Capture", &capture.capture_id),
                ("Capture SHA256", &capture.capture_file_sha256),
                ("Commit", &capture.commit_sha1),
                ("Tree", &capture.git_tree_sha1),
                ("Source", &capture.source),
            ] {
                field(&mut out, label, value);
            }
        }
    }
    out.push_str(&format!(
        "- Documentation state: {}\n",
        state(&transition.documentation.state)?
    ));
    if let Some(id) = &transition.documentation.capture_id {
        field(&mut out, "Documentation capture", id);
    }
    out.push_str(&format!(
        "- Migration guide state: {}\n",
        state(&transition.migration_guide.state)?
    ));
    Ok(out)
}

fn record(out: &mut String, record: &ReleaseImpact, captures: &BTreeSet<&str>) -> Result<()> {
    out.push_str(&format!("\n## Change {}\n\n", text(&record.change_id)));
    field(out, "Before capture", &record.before_capture_id);
    field(out, "Declared after capture", &record.after_capture_id);
    for (subject, id) in [
        ("before", &record.before_capture_id),
        ("after", &record.after_capture_id),
    ] {
        if !captures.contains(id.as_str()) {
            out.push_str(&format!("- UNRESOLVED {subject} capture: {}. No supplied reference binds this declared subject; assembly does not verify it.\n", text(id)));
        }
    }
    out.push_str(&format!(
        "- Recorded acceptance: {} (not granted by assembly)\n",
        state(&record.acceptance.state)?
    ));
    field(out, "Review subject", &record.acceptance.review_subject);
    field(out, "Review evidence", &record.acceptance.review_evidence);
    out.push_str(&format!(
        "- User impact: {}; migration action: {}\n",
        state(&record.user_impact.state)?,
        state(&record.user_impact.migration_action)?
    ));
    field(out, "User impact reason", &record.user_impact.reason);
    out.push_str(&format!(
        "- Documentation impact: {}; action: {}\n",
        state(&record.documentation_impact.state)?,
        state(&record.documentation_impact.documentation_action)?
    ));
    field(
        out,
        "Documentation impact reason",
        &record.documentation_impact.reason,
    );
    let mut effects: Vec<_> = record.effects.iter().collect();
    effects.sort_by_key(|e| (&e.interface_id, &e.capability_id, &e.audience, &e.platform));
    if effects.is_empty() {
        out.push_str(
            "- No scoped effects recorded; this is not an independent net no-op conclusion.\n",
        );
    }
    for effect in effects {
        out.push_str("\n### Declared scoped effect\n\n");
        for (label, value) in [
            ("Interface", &effect.interface_id),
            ("Capability", &effect.capability_id),
            ("Audience", &effect.audience),
            ("Platform", &effect.platform),
            ("Before", &effect.before),
            ("After", &effect.after),
        ] {
            field(out, label, value);
        }
        out.push_str(&format!("- Kind: {}\n", state(&effect.kind)?));
        let mut anchors: Vec<_> = effect.contract_anchors.iter().collect();
        anchors.sort();
        for anchor in anchors {
            field(out, "Permanent contract anchor", anchor);
        }
    }
    let mut subjects: Vec<_> = record.implementation_subjects.iter().collect();
    subjects.sort_by_key(|s| &s.identity);
    for subject in subjects {
        out.push_str(&format!(
            "- Implementation subject ({}): {}\n",
            state(&subject.kind)?,
            text(&subject.identity)
        ));
    }
    let mut evidence: Vec<_> = record.evidence.iter().collect();
    evidence.sort_by_key(|e| (&e.claim_id, &e.subject_id, &e.source));
    for item in evidence {
        out.push_str(&format!(
            "- Recorded evidence: claim {}; subject {}; source {}; verdict {} (not reverified)\n",
            text(&item.claim_id),
            text(&item.subject_id),
            text(&item.source),
            state(&item.verdict)?
        ));
    }
    let mut relations: Vec<_> = record.relations.iter().collect();
    relations.sort_by_key(|r| &r.change_id);
    for relation in relations {
        out.push_str(&format!(
            "- Relation: {} {}. Net effect requires independent before-to-after reconciliation.\n",
            state(&relation.kind)?,
            text(&relation.change_id)
        ));
    }
    Ok(())
}

pub(super) fn render(transition: &Transition, records: &[ReleaseImpact]) -> Result<[String; 2]> {
    let references = transition.capture_references()?;
    let captures = references
        .iter()
        .map(|reference| reference.capture_id.as_str())
        .collect();
    let header = header(transition)?;
    let mut migration = format!("# Migration draft\n\n{header}\n## Contribution accounting\n\n");
    if records.is_empty() {
        migration
            .push_str("No change records supplied. Net transition effects remain unresolved.\n");
    }
    for item in records {
        record(&mut migration, item, &captures)?;
    }
    let mut worklist = format!(
        "# Worklist draft\n\n{header}\n## Unresolved obligations\n\n- Independently reconcile actual DP1-to-DP2 net effects, including reverts and supersession.\n- Review draft guidance and documentation at their owned sources; retain review evidence.\n- For migration actions, bind exact captures, prerequisites, ordered steps, expected results, verification, repeat behavior, recovery, preservation and backup boundaries. This draft supplies no checked recipe.\n- Bind documentation topics to exact source pages/anchors or explicit page-to-create obligations.\n"
    );
    for (label, complete) in [
        (
            "Validate capture evidence",
            transition.proof.capture_validated,
        ),
        (
            "Verify contribution coverage",
            transition.proof.contributions_verified,
        ),
        (
            "Verify documentation build/link/claim proof",
            transition.proof.documentation_verified,
        ),
        (
            "Rehearse migration and partial-failure recovery",
            transition.proof.migration_rehearsed,
        ),
        (
            "Complete final product proof panel",
            transition.proof.final_product_panel_passed,
        ),
    ] {
        worklist.push_str(&format!(
            "- {label}: {}.\n",
            if complete {
                "recorded complete; not reverified by assembly"
            } else {
                "UNRESOLVED"
            }
        ));
    }
    for item in records {
        record(&mut worklist, item, &captures)?;
        worklist.push_str("- Obligation: independently verify this record's claims, capture binding, migration disposition and documentation mapping.\n");
    }
    Ok([migration, worklist])
}
