//! Report rendering: XML (native), Markdown table, and the six
//! resolution views (PROP-043 §5 `report`).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#CMD-REPORT");

use crate::doc::{Fact, ParsedDoc};
use crate::evidence::{Evidence, EvidenceProvider};
use crate::model::{ArtifactRequirements, Audience, Granularity, Marker, Stage, State};
#[cfg(test)]
use crate::rollup::DocRollup;
use crate::terminal::{ArtifactObservation, TerminalOutcome, fact_address, resolve_terminal};
use serde::{Deserialize, Serialize};

/// The six resolution views — filters over one model (PROP-043 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Done,
    Todo,
    Qa,
    Remove,
    Doc,
    Terminal,
}

impl View {
    pub fn parse(s: &str) -> Option<View> {
        match s {
            "done" => Some(View::Done),
            "todo" => Some(View::Todo),
            "qa" => Some(View::Qa),
            "remove" => Some(View::Remove),
            "doc" => Some(View::Doc),
            "terminal" => Some(View::Terminal),
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
            // Terminality is derived only after evidence resolution.
            View::Terminal => true,
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
    /// Exact fact address when the conditional terminal surface is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Authored terminal artifact set, kept in canonical order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires: Option<ArtifactRequirements>,
    /// Current per-artifact observations from the same provider snapshot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactObservation>,
    /// Derived overall result, including `unclassified`. Absent for invalid
    /// facts and when no classified fact activates the compatibility surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<TerminalOutcome>,
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
    let docs: Vec<&ParsedDoc> = docs.into_iter().collect();
    // Conditional compatibility boundary: an entirely legacy corpus keeps
    // every new row member absent. Once one valid classified fact activates
    // the surface, valid addressed peers explicitly report `unclassified`.
    let terminal_surface = docs.iter().any(|doc| {
        doc.error_count() == 0
            && doc
                .blocks
                .iter()
                .flat_map(|block| &block.facts)
                .any(|fact| {
                    fact.requirements.is_some()
                        && fact_address(doc, fact).is_some()
                        && fact
                            .marker_index
                            .is_some_and(|index| doc.markers.get(index).is_some())
                })
    });
    let mut out = Vec::new();
    for doc in docs {
        let mut facts_by_marker: Vec<Option<&Fact>> = vec![None; doc.markers.len()];
        for fact in doc.blocks.iter().flat_map(|block| &block.facts) {
            if let Some(slot) = fact
                .marker_index
                .and_then(|index| facts_by_marker.get_mut(index))
            {
                *slot = Some(fact);
            }
        }
        for (marker_index, m) in doc.markers.iter().enumerate() {
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
            let fact = facts_by_marker[marker_index];
            let exact_address = fact.and_then(|fact| fact_address(doc, fact));
            let evidence_address = exact_address.clone().or_else(|| {
                matches!(m.granularity, Granularity::Document | Granularity::Section)
                    .then(|| unit_addr_at(doc, m.line))
                    .flatten()
            });
            let found = evidence_address
                .as_deref()
                .and_then(|addr| evidence.evidence_for(addr));
            let valid_fact = terminal_surface
                && doc.error_count() == 0
                && fact.is_some()
                && exact_address.is_some();
            let classified = valid_fact && fact.is_some_and(|fact| fact.requirements.is_some());
            let observation = match (valid_fact, fact, exact_address.as_deref()) {
                (true, Some(fact), Some(address)) => Some(resolve_terminal(
                    address,
                    m,
                    fact.requirements.as_ref(),
                    evidence,
                )),
                _ => None,
            };
            if view == Some(View::Terminal)
                && !observation
                    .as_ref()
                    .is_some_and(|value| value.outcome == TerminalOutcome::Terminal)
            {
                continue;
            }
            let mismatch = found.as_ref().and_then(|ev| {
                if classified && m.stage == Stage::Freeze {
                    None
                } else {
                    crate::evidence::mismatch(m, ev)
                }
            });
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
                address: valid_fact.then_some(exact_address).flatten(),
                requires: valid_fact
                    .then(|| fact.and_then(|fact| fact.requirements.clone()))
                    .flatten(),
                artifacts: observation
                    .as_ref()
                    .map(|value| value.artifacts.clone())
                    .unwrap_or_default(),
                terminal: observation.map(|value| value.outcome),
            });
        }
    }
    out
}

mod render;
pub use render::{render_md, render_md_with_mode, render_xml, render_xml_with_mode};

#[cfg(test)]
#[path = "report/terminal_tests.rs"]
mod terminal_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::NoEvidence;
    use crate::parse::parse_document;

    /// The standalone marker addresses the heading; the fact row owns `b1`.
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
    fn unaddressed_markers_ask_nothing() {
        let text = "<status stage=\"impl\" state=\"done\"/>\n\n# T {#t}\n\nBody. @test/plan\n";
        let (doc, _) = fixture(text);
        let rows = rows([&doc], None, None, &Stub(Evidence::default()));
        assert!(rows[0].evidence.is_none(), "line 1 is inside no unit");
        assert!(
            rows[1].evidence.is_none(),
            "fact-grain evidence never falls back to a heading"
        );
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
