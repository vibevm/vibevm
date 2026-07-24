//! Rollup: explicit markers beat inheritance; unmarked nodes aggregate
//! worst-of over their children (PROP-043 §3.10).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#rollup");

use crate::doc::ParsedDoc;
use crate::model::{Granularity, Stage, State, rollup_key};
use serde::{Deserialize, Serialize};

/// A document's aggregate standing, explicit and computed side by side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRollup {
    /// The explicit document-level marker, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit: Option<(Stage, State)>,
    /// Worst-of over every non-fragment marker in the file, when any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed: Option<(Stage, State)>,
    /// The effective value a report shows: explicit wins, else computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective: Option<(Stage, State)>,
    pub marker_count: usize,
    pub paragraph_count: usize,
    pub unmarked_paragraphs: usize,
}

pub fn rollup_doc(doc: &ParsedDoc) -> DocRollup {
    let explicit = doc.document_marker().map(|m| (m.stage, m.state));
    let computed = doc
        .markers
        .iter()
        .filter(|m| m.granularity != Granularity::Fragment)
        .map(|m| (m.stage, m.state))
        .min_by_key(|(st, s)| rollup_key(*st, *s));
    let effective = explicit.or(computed);
    DocRollup {
        explicit,
        computed,
        effective,
        marker_count: doc.markers.len(),
        paragraph_count: doc.paragraph_count,
        unmarked_paragraphs: doc.unmarked_paragraphs.len(),
    }
}

/// Worst-of across many documents (the project row of a report).
pub fn rollup_project<'a>(
    rollups: impl IntoIterator<Item = &'a DocRollup>,
) -> Option<(Stage, State)> {
    rollups
        .into_iter()
        .filter_map(|r| r.effective)
        .min_by_key(|(st, s)| rollup_key(*st, *s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;

    #[test]
    fn explicit_beats_computed_and_worst_of_wins() {
        let text = "\
<status stage=\"impl\" state=\"work\"/>

# T {#t}

@test/done

Body paragraph. @idea
";
        let doc = parse_document("x.md", text);
        let r = rollup_doc(&doc);
        assert_eq!(r.explicit, Some((Stage::Impl, State::Work)));
        // Worst across doc/section/para markers: idea/work.
        assert_eq!(r.computed, Some((Stage::Idea, State::Work)));
        assert_eq!(r.effective, Some((Stage::Impl, State::Work)));
    }
}
