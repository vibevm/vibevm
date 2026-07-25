//! Report rendering: XML (native), Markdown table, and the five
//! resolution views (PROP-043 §5 `report`).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#report");

use crate::doc::ParsedDoc;
use crate::evidence::{Evidence, EvidenceProvider};
use crate::model::{Audience, Granularity, Marker, Stage, State};
use crate::rollup::DocRollup;
use serde::{Deserialize, Serialize};

/// The five resolution views — filters over one model (PROP-043 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Done,
    Todo,
    Qa,
    Remove,
    Doc,
}

impl View {
    pub fn parse(s: &str) -> Option<View> {
        match s {
            "done" => Some(View::Done),
            "todo" => Some(View::Todo),
            "qa" => Some(View::Qa),
            "remove" => Some(View::Remove),
            "doc" => Some(View::Doc),
            _ => None,
        }
    }

    /// The filter predicate of each view.
    pub fn matches(self, m: &Marker) -> bool {
        match self {
            View::Done => m.state == State::Done,
            View::Todo => m.action == Some(crate::model::Action::Continue),
            View::Qa => {
                m.stage == Stage::Test && (m.state == State::Plan || m.state == State::Work)
            }
            View::Remove => m.action == Some(crate::model::Action::Remove),
            View::Doc => m.actionstage == Some(Stage::Doc),
        }
    }
}

/// Does a marker speak to `audience`? Absent audience list ⇒ `dev`.
pub fn audience_matches(m: &Marker, audience: Audience) -> bool {
    if m.audience.is_empty() {
        audience == Audience::Dev
    } else {
        m.audience.contains(&audience)
    }
}

/// One row of the rendered report.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub path: String,
    pub line: usize,
    pub granularity: Granularity,
    pub stage: Stage,
    pub state: State,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// What the wired provider knows about this row's unit (PROP-043 §6).
    /// `None` = the provider had no answer, which is NOT "zero edges".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
    /// The marker claims more than the evidence shows — `mismatch`'s verdict.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<String>,
}

/// The address of the innermost anchored unit containing `line` —
/// `path#anchor`, or `path#L<start>` when the unit carries no anchor.
/// `None` when the line precedes every heading: no unit, no address, and
/// therefore no evidence claim. Units come in document order, so the last
/// containing one is the innermost.
fn unit_addr_at(doc: &ParsedDoc, line: usize) -> Option<String> {
    doc.units
        .iter()
        .enumerate()
        .rfind(|(_, u)| u.line_start <= line && line <= u.line_end)
        .map(|(i, _)| crate::baseline::unit_addr(doc, i))
}

/// Build the report rows, asking `evidence` about each row's unit.
///
/// A project with nothing wired passes [`crate::evidence::NoEvidence`] and
/// gets exactly the rows it always got — the separability law (PROP-043 §2):
/// the core must run with no provider at all.
pub fn rows<'a>(
    docs: impl IntoIterator<Item = &'a ParsedDoc>,
    view: Option<View>,
    audience: Option<Audience>,
    evidence: &dyn EvidenceProvider,
) -> Vec<Row> {
    let mut out = Vec::new();
    for doc in docs {
        for m in &doc.markers {
            if let Some(v) = view
                && !v.matches(m)
            {
                continue;
            }
            if let Some(a) = audience
                && !audience_matches(m, a)
            {
                continue;
            }
            let action = m.actionstage.map_or_else(
                || m.action.map(|a| a.to_string()),
                |ast| m.action.map(|a| format!("{a}+{ast}")),
            );
            let found = unit_addr_at(doc, m.line).and_then(|addr| evidence.evidence_for(&addr));
            let mismatch = found
                .as_ref()
                .and_then(|ev| crate::evidence::mismatch(m, ev));
            out.push(Row {
                path: doc.path.clone(),
                line: m.line,
                granularity: m.granularity,
                stage: m.stage,
                state: m.state,
                action,
                comment: m.comment.clone(),
                evidence: found,
                mismatch,
            });
        }
    }
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The native XML report (PROP-043 §5: XML is the native output).
pub fn render_xml(rows: &[Row], rollups: &[(String, DocRollup)]) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<progress-report schema=\"1\">\n",
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
        s.push_str(&format!(
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
        ));
        // A row the provider could not answer closes exactly as it always
        // did — an evidence-less run is byte-identical (PROP-043 §6).
        match &row.evidence {
            None => s.push_str("/>\n"),
            Some(ev) => s.push_str(&format!(
                ">\n      <evidence implements=\"{}\" verifies=\"{}\"{}/>\n    </marker>\n",
                ev.implements,
                ev.verifies,
                row.mismatch
                    .as_deref()
                    .map(|m| format!(" mismatch=\"{}\"", xml_escape(m)))
                    .unwrap_or_default(),
            )),
        }
    }
    s.push_str("  </markers>\n</progress-report>\n");
    s
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// The Markdown table render (source · stage · state · action · comment),
/// plus a right-most `evidence` column when any row carries evidence.
pub fn render_md(rows: &[Row], rollups: &[(String, DocRollup)]) -> String {
    // The column exists only when the wired provider answered something:
    // an evidence-less run renders the table it always rendered.
    let with_evidence = rows.iter().any(|r| r.evidence.is_some());
    let mut s = if with_evidence {
        String::from(
            "| source | stage | state | action | comment | evidence |\n|---|---|---|---|---|---|\n",
        )
    } else {
        String::from("| source | stage | state | action | comment |\n|---|---|---|---|---|\n")
    };
    for (path, r) in rollups {
        if let Some((st, sta)) = r.effective {
            s.push_str(&format!(
                "| **{}** ({} markers, {}/{} unmarked) | {} | {} |  |  |",
                md_escape(path),
                r.marker_count,
                r.unmarked_facts,
                r.fact_count,
                st,
                sta
            ));
        } else {
            s.push_str(&format!(
                "| **{}** (no markers, {} facts) | — | — |  |  |",
                md_escape(path),
                r.fact_count
            ));
        }
        // A file rollup is not a unit — its evidence cell is always empty.
        if with_evidence {
            s.push_str("  |");
        }
        s.push('\n');
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
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::NoEvidence;
    use crate::parse::parse_document;

    /// Every marker of this fixture sits inside the `{#t}` unit, so both
    /// rows address `spec/t.md#t`.
    const FIXTURE: &str = "\
# T {#t}

<status stage=\"impl\" state=\"done\"/>

##b1 @test/plan Body.
";

    fn fixture(text: &str) -> (ParsedDoc, Vec<(String, DocRollup)>) {
        let doc = parse_document("spec/t.md", text);
        let rollups = vec![(doc.path.clone(), crate::rollup::rollup_doc(&doc))];
        (doc, rollups)
    }

    /// A provider that answers every address with the same fixed facts.
    struct Stub(Evidence);

    impl EvidenceProvider for Stub {
        fn evidence_for(&self, _unit: &str) -> Option<Evidence> {
            Some(self.0.clone())
        }
    }

    /// The separability guarantee: wiring the seam changed no byte of an
    /// evidence-less run. The literals are the renderers' output captured
    /// before the column existed.
    #[test]
    fn report_without_provider_is_unchanged() {
        let (doc, rollups) = fixture(FIXTURE);
        let rows = rows([&doc], None, None, &NoEvidence);
        assert_eq!(
            render_md(&rows, &rollups),
            "| source | stage | state | action | comment |\n\
             |---|---|---|---|---|\n\
             | **spec/t.md** (2 markers, 0/1 unmarked) | impl | done |  |  |\n\
             | spec/t.md:3 | impl | done |  |  |\n\
             | spec/t.md:5 | test | plan |  |  |\n"
        );
        assert_eq!(
            render_xml(&rows, &rollups),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <progress-report schema=\"1\">\n\
             \x20 <files>\n\
             \x20   <file path=\"spec/t.md\" stage=\"impl\" state=\"done\" markers=\"2\" facts=\"1\" unmarked=\"0\"/>\n\
             \x20 </files>\n\
             \x20 <markers>\n\
             \x20   <marker path=\"spec/t.md\" line=\"3\" granularity=\"Document\" stage=\"impl\" state=\"done\"/>\n\
             \x20   <marker path=\"spec/t.md\" line=\"5\" granularity=\"Paragraph\" stage=\"test\" state=\"plan\"/>\n\
             \x20 </markers>\n\
             </progress-report>\n"
        );
        // The json form is the rows themselves: no evidence, no field.
        let json = serde_json::to_string(&rows).expect("serialise rows");
        assert!(!json.contains("evidence"), "absent evidence stays absent");
    }

    #[test]
    fn report_with_provider_shows_evidence() {
        let (doc, rollups) = fixture(FIXTURE);
        let stub = Stub(Evidence {
            implements: 2,
            verifies: 1,
            refs: vec!["crates/x/src/y.rs:12".into()],
        });
        let rows = rows([&doc], None, None, &stub);
        let md = render_md(&rows, &rollups);
        assert!(
            md.starts_with("| source | stage | state | action | comment | evidence |\n|---|---|---|---|---|---|\n"),
            "the column is announced in the header:\n{md}"
        );
        assert!(
            md.contains("| spec/t.md:3 | impl | done |  |  | impl=2 ver=1 |\n"),
            "the row carries the counts:\n{md}"
        );
        assert!(
            render_xml(&rows, &rollups).contains(
                "<marker path=\"spec/t.md\" line=\"3\" granularity=\"Document\" stage=\"impl\" \
                 state=\"done\">\n      <evidence implements=\"2\" verifies=\"1\"/>\n    </marker>\n"
            ),
            "the marker element nests the evidence child"
        );
        let json = serde_json::to_string(&rows).expect("serialise rows");
        assert!(json.contains("\"evidence\":{\"implements\":2,\"verifies\":1"));
        assert!(!json.contains("mismatch"), "proven claims are not flagged");
    }

    /// The markup-vs-reality flag. `mismatch` (the already-built seam)
    /// fires on `test/done` with no `verifies` edge and on `freeze` with no
    /// `implements` edge; `impl/done` is NOT one of its rules, so a row
    /// marked that way stays unflagged however empty the evidence is.
    #[test]
    fn mismatch_is_flagged() {
        let claimed = "# T {#t}\n\n<status stage=\"test\" state=\"done\"/>\n";
        let (doc, rollups) = fixture(claimed);
        let empty = Stub(Evidence::default());
        let flagged = rows([&doc], None, None, &empty);
        assert!(
            flagged[0].mismatch.is_some(),
            "test/done with zero verifying edges is a mismatch"
        );
        assert!(
            render_md(&flagged, &rollups).contains("| impl=0 ver=0 ⚠ |\n"),
            "md flags the evidence cell"
        );
        assert!(
            render_xml(&flagged, &rollups).contains(
                "<evidence implements=\"0\" verifies=\"0\" mismatch=\"marked test/done but no \
                 verifying evidence (0 `verifies` edges)\"/>"
            ),
            "xml carries the message"
        );
        assert!(
            serde_json::to_string(&flagged)
                .expect("serialise rows")
                .contains("\"mismatch\":\"marked test/done"),
            "json carries the message"
        );

        let unclaimed = "# T {#t}\n\n<status stage=\"impl\" state=\"done\"/>\n";
        let (unclaimed_doc, _) = fixture(unclaimed);
        let unclaimed_rows = rows([&unclaimed_doc], None, None, &empty);
        assert!(
            unclaimed_rows[0].mismatch.is_none(),
            "impl/done is outside `mismatch`'s rules — zero edges is not a claim it makes"
        );
    }

    /// A marker whose line precedes every heading has no unit address, so
    /// the provider is never asked: "no unit" cannot become "zero edges".
    #[test]
    fn unaddressed_marker_asks_nothing() {
        let text = "<status stage=\"impl\" state=\"done\"/>\n\n# T {#t}\n\nBody. @test/plan\n";
        let (doc, _) = fixture(text);
        let rows = rows([&doc], None, None, &Stub(Evidence::default()));
        assert!(rows[0].evidence.is_none(), "line 1 is inside no unit");
        assert!(rows[1].evidence.is_some(), "the body row is inside `#t`");
    }

    #[test]
    fn views_filter_one_model() {
        let text = "\
<status stage=\"impl\" state=\"done\"/>

# T {#t}

<status stage=\"test\" state=\"plan\"/>

Body. <status stage=\"impl\" state=\"work\" action=\"continue\" actionstage=\"doc\" audience=\"user\"/>
";
        let doc = parse_document("x.md", text);
        let all = rows([&doc], None, None, &NoEvidence);
        assert_eq!(all.len(), 3);
        assert_eq!(rows([&doc], Some(View::Done), None, &NoEvidence).len(), 1);
        assert_eq!(rows([&doc], Some(View::Qa), None, &NoEvidence).len(), 1);
        assert_eq!(rows([&doc], Some(View::Todo), None, &NoEvidence).len(), 1);
        assert_eq!(rows([&doc], Some(View::Doc), None, &NoEvidence).len(), 1);
        // audience=user matches only the explicitly-user marker.
        assert_eq!(
            rows([&doc], None, Some(Audience::User), &NoEvidence).len(),
            1
        );
        // default audience is dev.
        assert_eq!(
            rows([&doc], None, Some(Audience::Dev), &NoEvidence).len(),
            2
        );
    }
}
