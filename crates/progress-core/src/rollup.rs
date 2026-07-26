//! Rollup: explicit markers beat inheritance; unmarked nodes aggregate
//! worst-of over their children (PROP-043 §3.10).
//!
//! The same tree walk answers the fold question (§3.9's
//! `POST-CAMPAIGN-FOLD`, and `check`'s "lossless folds" duty in §5): a
//! section whose units agree may collapse to one section marker, and that
//! collapse is only legal when the section marker keeps everything the
//! unit markers carried. [`fold_check`] verifies exactly that — it never
//! demands a fold, it only catches a fold that lies.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#rollup");

use crate::doc::{ParsedDoc, Unit};
use crate::model::{Audience, Granularity, Marker, Stage, State, rollup_key};
use serde::{Deserialize, Serialize};
use std::fmt;

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
    /// Countable facts (fact amendment: paragraphs + lead lines + list
    /// items + table body cells).
    pub fact_count: usize,
    pub unmarked_facts: usize,
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
        fact_count: doc.fact_count,
        unmarked_facts: doc.unmarked_facts.len(),
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

/// What a claimed fold would drop. A marker's standing is the one
/// `(stage, state)` value, reported here as `state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FoldLoss {
    State,
    Action,
    Actionstage,
    Audience,
    Ref,
    Comment,
}

impl FoldLoss {
    pub fn as_str(self) -> &'static str {
        match self {
            FoldLoss::State => "state",
            FoldLoss::Action => "action",
            FoldLoss::Actionstage => "actionstage",
            FoldLoss::Audience => "audience",
            FoldLoss::Ref => "ref",
            FoldLoss::Comment => "comment",
        }
    }
}

impl fmt::Display for FoldLoss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One section whose marker claims a fold that loses information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldIssue {
    /// The folding section: its `{#anchor}` when it has one, else its
    /// heading text.
    pub section: String,
    /// 1-based line of the section marker — where `check` reports.
    pub line: usize,
    /// The unit the fold would silence: its `##<ID>` fact anchor when it
    /// has one, else `line <n>` (cells are anchor-exempt, §3.8).
    pub unit: String,
    /// 1-based line of the losing unit's marker.
    pub unit_line: usize,
    /// The attribute the section marker fails to carry.
    pub lost: FoldLoss,
}

impl fmt::Display for FoldIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "fold over section `{}` is lossy — unit `{}` (line {}) carries a `{}` \
             the section marker does not",
            self.section, self.unit, self.unit_line, self.lost
        )
    }
}

/// Verify every claimed marker-density fold in one document.
///
/// A section-level marker **folds** the unit markers under it when all of
/// them carry the same `(stage, state)`; the fold is **lossless** when the
/// section marker carries exactly that pair and every `action`,
/// `actionstage`, `audience`, `ref` and `comment` those units carried.
/// One issue per section — the first unit whose information the fold drops.
///
/// Silent by design: a section with no section-level marker, a section
/// with no unit markers, and a section whose units **disagree** — a mixed
/// section is not a fold at all and stays fact-marked (§3.9).
#[specmark::spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#tool")]
pub fn fold_check(doc: &ParsedDoc) -> Vec<FoldIssue> {
    let sections = section_markers(doc);
    let mut out = Vec::new();
    for (i, u) in doc.units.iter().enumerate() {
        let Some(section) = sections[i] else { continue };
        let folded = folded_units(doc, i, &sections);
        let Some(first) = folded.first() else {
            continue;
        };
        // Agreement is the fold precondition, and a disagreeing section is
        // silent — that is `POST-CAMPAIGN-FOLD`'s own wording ("a section
        // whose units agree collapses to one unit marker … mixed sections
        // stay fact-marked"). The task's own §6 sketch described a *mixed*
        // section and so could never fire; the executor read the rule over
        // the fixture, which was the right call.
        //
        // Reviewer's ruling on the residual (2026-07-25): what survives the
        // precondition — unanimous units under a section marker carrying a
        // different pair — is exactly what `EXPLICIT-BEATS` calls "a
        // divergence [that] is information, not noise". A document cannot
        // tell a lying fold from a deliberate explicit marker, so this
        // reports at **warning** severity and never fails a gate. It becomes
        // fatal where the distinction exists: Phase F's folder, which knows
        // it is asserting a fold, runs this as its pre-flight.
        // <!-- REVIEW: if the owner rules that a section marker diverging
        // from unanimous children is an error rather than information,
        // promote the adapter's fold class back to Error. -->
        //
        // spec://vibevm/modules/vibe-progress/PROP-043#rollup
        let pair = (first.stage, first.state);
        if folded.iter().any(|m| (m.stage, m.state) != pair) {
            continue;
        }
        if let Some((m, lost)) = folded
            .iter()
            .find_map(|m| loss_of(section, m).map(|lost| (*m, lost)))
        {
            out.push(FoldIssue {
                section: section_name(u),
                line: section.line,
                unit: unit_name(doc, m),
                unit_line: m.line,
                lost,
            });
        }
    }
    out
}

/// The section-level marker each heading unit carries, by unit index. A
/// standalone marker sits directly under its heading, so its owner is the
/// last unit to open at or before it.
fn section_markers(doc: &ParsedDoc) -> Vec<Option<&Marker>> {
    let mut out: Vec<Option<&Marker>> = vec![None; doc.units.len()];
    for m in doc
        .markers
        .iter()
        .filter(|m| m.granularity == Granularity::Section)
    {
        if let Some(i) = owner_index(doc, m.line)
            && out[i].is_none()
        {
            out[i] = Some(m);
        }
    }
    out
}

/// Index of the heading unit a line falls directly under.
fn owner_index(doc: &ParsedDoc, line: usize) -> Option<usize> {
    doc.units.iter().rposition(|u| u.line_start <= line)
}

/// The units a section marker stands over: every unit marker inside the
/// section, plus — for a subsection carrying its own marker — that
/// marker instead of the subsection's interior. An explicit marker beats
/// both directions (§3.10), so a marked subsection folds as one unit;
/// an unmarked one is transparent and its units float up.
fn folded_units<'a>(
    doc: &'a ParsedDoc,
    idx: usize,
    sections: &[Option<&'a Marker>],
) -> Vec<&'a Marker> {
    let u = &doc.units[idx];
    let mut opaque: Vec<(usize, usize)> = Vec::new();
    let mut out: Vec<&Marker> = Vec::new();
    for (j, d) in doc.units.iter().enumerate().skip(idx + 1) {
        if d.line_start <= u.line_start || d.line_start > u.line_end {
            continue;
        }
        if opaque
            .iter()
            .any(|&(s, e)| d.line_start > s && d.line_start <= e)
        {
            continue; // already spoken for by an outer marked subsection
        }
        if let Some(m) = sections[j] {
            opaque.push((d.line_start, d.line_end));
            out.push(m);
        }
    }
    for m in &doc.markers {
        let mine = m.line > u.line_start
            && m.line <= u.line_end
            && !opaque.iter().any(|&(s, e)| m.line > s && m.line <= e);
        let unit_grain = matches!(
            m.granularity,
            Granularity::Paragraph | Granularity::Item | Granularity::Cell
        );
        if mine && unit_grain {
            out.push(m);
        }
    }
    out.sort_by_key(|m| m.line);
    out
}

/// What the section marker fails to carry from one folded unit.
fn loss_of(section: &Marker, unit: &Marker) -> Option<FoldLoss> {
    if (unit.stage, unit.state) != (section.stage, section.state) {
        return Some(FoldLoss::State);
    }
    if unit.action.is_some() && unit.action != section.action {
        return Some(FoldLoss::Action);
    }
    if unit.actionstage.is_some() && unit.actionstage != section.actionstage {
        return Some(FoldLoss::Actionstage);
    }
    let carried = audience_of(section);
    if !audience_of(unit).iter().all(|a| carried.contains(a)) {
        return Some(FoldLoss::Audience);
    }
    if unit.r#ref.is_some() && unit.r#ref != section.r#ref {
        return Some(FoldLoss::Ref);
    }
    if unit.comment.is_some() && unit.comment != section.comment {
        return Some(FoldLoss::Comment);
    }
    None
}

/// A marker's effective audience — an empty list means `dev` (§3.6), so
/// the test compares what markers mean, not what they spell.
fn audience_of(m: &Marker) -> Vec<Audience> {
    if m.audience.is_empty() {
        vec![Audience::Dev]
    } else {
        m.audience.clone()
    }
}

fn section_name(u: &Unit) -> String {
    u.anchor.clone().unwrap_or_else(|| u.heading.clone())
}

/// The name of the unit a marker belongs to: a subsection's anchor for a
/// section marker, else the `##<ID>` of the fact it sits in.
fn unit_name(doc: &ParsedDoc, m: &Marker) -> String {
    if m.granularity == Granularity::Section
        && let Some(j) = owner_index(doc, m.line)
    {
        return section_name(&doc.units[j]);
    }
    for b in &doc.blocks {
        if m.line < b.line_start || m.line > b.line_end {
            continue;
        }
        if let Some(f) = b.facts.iter().rev().find(|f| f.line <= m.line)
            && let Some(id) = &f.id
        {
            return id.clone();
        }
    }
    format!("line {}", m.line)
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

    /// A tombstone does not govern the file it sits in — the live unit
    /// beside it does (DRIFT-028 §4.1). Driven through the parser rather
    /// than against `rollup_key` directly, so the new vocabulary value is
    /// shown to reach the fold from the markup a document actually writes.
    #[test]
    fn a_void_unit_does_not_govern_the_document() {
        let text = "\
# T {#t}

##a Split in two; kept only so the name is not reused. @spec/void

##b The heir, still being written. @impl/plan
";
        let r = rollup_doc(&parse_document("x.md", text));
        assert_eq!(r.computed, Some((Stage::Impl, State::Plan)));

        // And a document whose every unit is void is itself void, which
        // falls out of the same rule rather than being special-cased.
        let all_void = "\
# T {#t}

##a Cancelled with no replacement. @spec/void

##b Split into heirs kept elsewhere. @doc/void
";
        let r = rollup_doc(&parse_document("y.md", all_void));
        assert_eq!(r.computed.map(|(_, s)| s), Some(State::Void));
    }

    /// Three agreeing units under a section marker that says exactly what
    /// they say: the fold keeps everything.
    #[test]
    fn fold_lossless_is_silent() {
        let text = "\
# T {#t}

## S {#s}

<status stage=\"impl\" state=\"done\"/>

##a First fact. @impl/done

##b Second fact. @impl/done

##c Third fact. @impl/done
";
        assert_eq!(fold_check(&parse_document("x.md", text)), Vec::new());
    }

    /// The section marker claims `impl/done` over a unit that stands at
    /// `spec/done` — folding it would silence the difference.
    #[test]
    fn fold_losing_state_is_caught() {
        let text = "\
# T {#t}

## S {#s}

<status stage=\"impl\" state=\"done\"/>

##a First fact. @spec/done
";
        let issues = fold_check(&parse_document("x.md", text));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].section, "s");
        assert_eq!(issues[0].unit, "a");
        assert_eq!(issues[0].lost, FoldLoss::State);
    }

    /// The units agree, so the section folds them — but one carries a
    /// verdict the section marker does not.
    #[test]
    fn fold_losing_action_is_caught() {
        let text = "\
# T {#t}

## S {#s}

<status stage=\"impl\" state=\"done\"/>

##a First fact. <status stage=\"impl\" state=\"done\" action=\"drift\"/>

##b Second fact. @impl/done
";
        let issues = fold_check(&parse_document("x.md", text));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].unit, "a");
        assert_eq!(issues[0].lost, FoldLoss::Action);
    }

    /// Mixed stages are not a fold at all: the check never demands that
    /// anyone fold, so a mixed section stays fact-marked and silent.
    #[test]
    fn disagreeing_section_is_not_a_fold() {
        let text = "\
# T {#t}

## S {#s}

<status stage=\"impl\" state=\"done\"/>

##a First fact. @impl/done

##b Second fact. @spec/done

##c Third fact. @test/work
";
        assert_eq!(fold_check(&parse_document("x.md", text)), Vec::new());
    }

    /// A marked subsection folds as one unit of its parent (its own
    /// marker is the parent's unit), and is tested on its own interior.
    #[test]
    fn nested_section_folds_transitively() {
        let text = "\
# T {#t}

## S {#s}

<status stage=\"impl\" state=\"done\"/>

##a First fact. @impl/done

### N {#n}

<status stage=\"impl\" state=\"done\"/>

##b Inner fact. @spec/work
";
        let issues = fold_check(&parse_document("x.md", text));
        // The outer section sees N's marker (impl/done), not `b` — so only
        // the inner fold is lossy.
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].section, "n");
        assert_eq!(issues[0].unit, "b");
        assert_eq!(issues[0].lost, FoldLoss::State);
    }

    /// The live shape of the corpus: a section marker carrying a comment
    /// its units do not. Nothing is lost by folding — the loss test is
    /// one-directional (what the units carried must survive).
    #[test]
    fn section_only_attributes_are_not_a_loss() {
        let text = "\
# T {#t}

## S {#s}

<status stage=\"spec\" state=\"work\" comment=\"three questions still open\"/>

##a First question. @spec/work

##b Second question. @spec/work
";
        assert_eq!(fold_check(&parse_document("x.md", text)), Vec::new());
    }
}
