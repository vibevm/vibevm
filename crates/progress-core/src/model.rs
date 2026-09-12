//! The markup vocabulary and the marker model.
//!
//! Closed vocabularies per PROP-043 §3.2 — any value outside these enums
//! is a validation error, never a silent pass-through.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#attributes");

use serde::{Deserialize, Serialize};
use specmark::spec;
use std::fmt;

/// One artifact kind whose presence contributes to closing a fact.
///
/// Declaration order is the canonical wire and rendering order.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactKind {
    Specification,
    Implementation,
    Verification,
    Documentation,
    Decision,
    Research,
    Plan,
    Disposition,
    External,
}

impl ArtifactKind {
    pub const ALL: [ArtifactKind; 9] = [
        ArtifactKind::Specification,
        ArtifactKind::Implementation,
        ArtifactKind::Verification,
        ArtifactKind::Documentation,
        ArtifactKind::Decision,
        ArtifactKind::Research,
        ArtifactKind::Plan,
        ArtifactKind::Disposition,
        ArtifactKind::External,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactKind::Specification => "specification",
            ArtifactKind::Implementation => "implementation",
            ArtifactKind::Verification => "verification",
            ArtifactKind::Documentation => "documentation",
            ArtifactKind::Decision => "decision",
            ArtifactKind::Research => "research",
            ArtifactKind::Plan => "plan",
            ArtifactKind::Disposition => "disposition",
            ArtifactKind::External => "external",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

impl fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A non-empty, duplicate-free artifact set stored in canonical order.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRequirements(Vec<ArtifactKind>);

impl ArtifactRequirements {
    pub fn parse_csv(value: &str) -> Result<Self, String> {
        if value.is_empty() {
            return Err("requirements list is empty".into());
        }
        let mut kinds = Vec::new();
        for member in value.split(',') {
            if member.is_empty() {
                return Err("requirements list contains an empty member".into());
            }
            let Some(kind) = ArtifactKind::parse(member) else {
                return Err(format!("unknown required artifact kind `{member}`"));
            };
            if kinds.contains(&kind) {
                return Err(format!("duplicate required artifact kind `{member}`"));
            }
            kinds.push(kind);
        }
        kinds.sort();
        Ok(Self(kinds))
    }

    pub fn iter(&self) -> impl Iterator<Item = ArtifactKind> + '_ {
        self.0.iter().copied()
    }

    pub fn contains(&self, kind: ArtifactKind) -> bool {
        self.0.contains(&kind)
    }

    pub fn to_csv(&self) -> String {
        self.iter()
            .map(ArtifactKind::as_str)
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl Serialize for ArtifactRequirements {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ArtifactRequirements {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = Vec::<ArtifactKind>::deserialize(deserializer)?;
        if raw.is_empty() {
            return Err(serde::de::Error::custom(
                "artifact requirements must not be empty",
            ));
        }
        let csv = raw
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(",");
        Self::parse_csv(&csv).map_err(serde::de::Error::custom)
    }
}

/// Where a unit of text stands in its development cycle (PROP-043 §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// Outside the order: "looked at, not understood" — compares below
    /// everything in rollup (PROP-043 §3.3).
    Unknown,
    Idea,
    Spec,
    Impl,
    Test,
    Doc,
    Freeze,
}

/// Work state at the current stage (PROP-043 §3.4).
///
/// The derived `Ord` is the rollup completeness order (least-advanced
/// first): `hold < plan < work < done < void`.
// REVIEW: PROP-043 §3.10 fixes the stage order for worst-of rollup but is
// silent on the state tiebreak within one stage. Conservative reading
// implemented here: hold (parked) is the least advanced. Surface for owner
// confirmation; амендировать §3.10 одной строкой при подтверждении.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Hold,
    Plan,
    Work,
    Done,
    /// The unit no longer asserts anything — `void` as in a **void
    /// contract**: without effect. Not the programming sense (still
    /// works, discouraged); this unit does not operate at all. It was
    /// either split into heirs and left as a pointer to them, or
    /// cancelled with no replacement; the text survives only so the name
    /// is not reused and inbound links do not break.
    ///
    /// Declared last so the derived `Ord` agrees with [`rollup_key`]:
    /// void is the *most* advanced value in the completeness order,
    /// which is how a tombstone stops counting as work.
    Void,
}

/// The verdict attribute: what is to be done (PROP-043 §3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Continue,
    Drift,
    Rework,
    Remove,
}

/// For whom a promise must eventually be documented (PROP-043 §3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Audience {
    User,
    Author,
    Dev,
    /// A session reading on a user's behalf — the boot snippet, a skill's
    /// instructions, the machine-facing corpus. Admitted 2026-09-11 by the
    /// vocabulary's own amendment-only law (PROP-043 `##AUDIENCE-VALUES`,
    /// on the ruling of PROP-057 `##OBS-AUDIENCE-AGENT`).
    ///
    /// The grammar knows nothing of lanes: that text for this audience
    /// obeys a token budget, carries no narration and never enters a boot
    /// prefix is PROP-057's rule about *where such text may appear*, not a
    /// property this value carries. Here it is one more name a promise can
    /// be owed to.
    Agent,
}

/// Syntactic form a marker was written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkerForm {
    /// `<status …/>`
    Point,
    /// `<status …>text</status>`
    Wrapper,
    /// `@stage` / `@stage/state`
    Shorthand,
}

/// What a marker governs, decided purely by position (PROP-043 §3.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    Document,
    Section,
    Paragraph,
    /// A list item unit (fact amendment, §3.8 item 4).
    Item,
    /// A table body cell unit (fact amendment, §3.8 item 5).
    Cell,
    Fragment,
}

/// One parsed `<status>` marker (or shorthand equivalent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub stage: Stage,
    pub state: State,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actionstage: Option<Stage>,
    /// Empty vec ⇒ default `dev` (PROP-043 §3.6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audience: Vec<Audience>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    pub form: MarkerForm,
    pub granularity: Granularity,
    /// 1-based source line of the marker's first character.
    pub line: usize,
}

impl Stage {
    pub const ALL: [Stage; 7] = [
        Stage::Idea,
        Stage::Spec,
        Stage::Impl,
        Stage::Test,
        Stage::Doc,
        Stage::Freeze,
        Stage::Unknown,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Idea => "idea",
            Stage::Spec => "spec",
            Stage::Impl => "impl",
            Stage::Test => "test",
            Stage::Doc => "doc",
            Stage::Freeze => "freeze",
            Stage::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Option<Stage> {
        Stage::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

impl State {
    pub const ALL: [State; 5] = [
        State::Plan,
        State::Work,
        State::Done,
        State::Hold,
        State::Void,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            State::Plan => "plan",
            State::Work => "work",
            State::Done => "done",
            State::Hold => "hold",
            State::Void => "void",
        }
    }

    pub fn parse(s: &str) -> Option<State> {
        State::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

impl Action {
    pub const ALL: [Action; 4] = [
        Action::Continue,
        Action::Drift,
        Action::Rework,
        Action::Remove,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Action::Continue => "continue",
            Action::Drift => "drift",
            Action::Rework => "rework",
            Action::Remove => "remove",
        }
    }

    pub fn parse(s: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

impl Audience {
    pub const ALL: [Audience; 4] = [
        Audience::User,
        Audience::Author,
        Audience::Dev,
        Audience::Agent,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Audience::User => "user",
            Audience::Author => "author",
            Audience::Dev => "dev",
            Audience::Agent => "agent",
        }
    }

    pub fn parse(s: &str) -> Option<Audience> {
        Audience::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Audience {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The key `void` sorts to: above every real `(stage, state)` pair, and
/// the same value whatever stage it is written at.
///
/// A sentinel is only how the invariant is implemented — the property
/// test `void_outranks_every_pair_at_every_stage` is the contract, and it
/// is what a future change has to keep true.
const VOID_KEY: (u8, u8) = (u8::MAX, u8::MAX);

/// The fixed sort key for worst-of rollup (PROP-043 §3.10):
/// `unknown < idea < spec < impl < test < doc < freeze`, and within a
/// stage the `State` completeness order. Lower = less advanced = "worse".
///
/// `void` is the one value outside that scheme: it sorts above every pair
/// **regardless of stage**, so a tombstone never governs a document that
/// still has live units, and a document whose every unit is void is
/// itself void without that case being written down anywhere. This is the
/// pair, not the state, precisely because stage dominates the pair —
/// giving `void` the top state slot within its stage would leave an
/// `@spec/void` still dragging the file down to `spec`.
pub fn rollup_key(stage: Stage, state: State) -> (u8, u8) {
    // Short-circuits before the stage is ever consulted — that is the
    // whole of the rule.
    if state == State::Void {
        return VOID_KEY;
    }
    let s = match stage {
        Stage::Unknown => 0,
        Stage::Idea => 1,
        Stage::Spec => 2,
        Stage::Impl => 3,
        Stage::Test => 4,
        Stage::Doc => 5,
        Stage::Freeze => 6,
    };
    let t = match state {
        State::Hold => 0,
        State::Plan => 1,
        State::Work => 2,
        State::Done => 3,
        // Unreachable — the short-circuit above returned already. Named
        // rather than swept up by a wildcard so that the next value added
        // to the vocabulary breaks this match instead of silently landing
        // on some neighbour's rank.
        State::Void => return VOID_KEY,
    };
    (s, t)
}

/// Nearest legal value for a typo'd token, for `check` hints
/// (PROP-043 §3.2 — "typos like `rewrok` die in CI").
pub fn nearest<'a>(input: &str, legal: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    legal
        .into_iter()
        .map(|cand| (levenshtein(input, cand), cand))
        .filter(|(d, _)| *d <= 3)
        .min_by_key(|(d, _)| *d)
        .map(|(_, cand)| cand)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabularies_round_trip() {
        for s in Stage::ALL {
            assert_eq!(Stage::parse(s.as_str()), Some(s));
        }
        for s in State::ALL {
            assert_eq!(State::parse(s.as_str()), Some(s));
        }
        for a in Action::ALL {
            assert_eq!(Action::parse(a.as_str()), Some(a));
        }
        for a in Audience::ALL {
            assert_eq!(Audience::parse(a.as_str()), Some(a));
        }
    }

    /// The audience vocabulary spelled out, not merely round-tripped: it is
    /// closed and amendment-only (PROP-043 `##VOCAB-AMENDMENT-ONLY`), so a
    /// value that appears here without a line in §3.6 is the defect this
    /// assertion exists to catch. `agent` joined the other three on
    /// 2026-09-11 (PROP-057 `##OBS-AUDIENCE-AGENT`).
    #[test]
    fn the_audience_vocabulary_is_exactly_the_four_amended_names() {
        assert_eq!(
            Audience::ALL.map(Audience::as_str),
            ["user", "author", "dev", "agent"]
        );
        assert_eq!(Audience::parse("agent"), Some(Audience::Agent));
        assert_eq!(Audience::parse("agents"), None);
    }

    #[test]
    fn artifact_requirements_serde_preserves_the_non_empty_canonical_set() {
        let requirements =
            ArtifactRequirements::parse_csv("external,specification,verification").unwrap();
        assert_eq!(
            serde_json::to_string(&requirements).unwrap(),
            r#"["specification","verification","external"]"#
        );
        let decoded: ArtifactRequirements =
            serde_json::from_str(r#"["external","specification"]"#).unwrap();
        assert_eq!(decoded.to_csv(), "specification,external");
        assert!(serde_json::from_str::<ArtifactRequirements>("[]").is_err());
        assert!(serde_json::from_str::<ArtifactRequirements>(r#"["plan","plan"]"#).is_err());
    }

    #[test]
    fn rollup_order_matches_prop_043() {
        // unknown is the floor; freeze/done is the ceiling.
        assert!(rollup_key(Stage::Unknown, State::Done) < rollup_key(Stage::Idea, State::Hold));
        assert!(rollup_key(Stage::Idea, State::Done) < rollup_key(Stage::Spec, State::Hold));
        assert!(rollup_key(Stage::Impl, State::Work) < rollup_key(Stage::Impl, State::Done));
        assert!(rollup_key(Stage::Doc, State::Done) < rollup_key(Stage::Freeze, State::Plan));
    }

    /// The spelling, pinned. `vocabularies_round_trip` cannot do this
    /// job: `parse` is defined *through* `as_str`, so it round-trips any
    /// spelling whatsoever and would bless `voidx` — verified by
    /// deliberately breaking it. The word is the owner's (`void` as in a
    /// void contract) and it is written into documents, so the string
    /// itself is part of the contract on both the display and serde
    /// wires, which are two independent renderings of the same value.
    #[test]
    fn void_is_spelled_void_on_both_wires() {
        assert_eq!(State::Void.as_str(), "void");
        assert_eq!(State::parse("void"), Some(State::Void));
        assert_eq!(serde_json::to_string(&State::Void).unwrap(), "\"void\"");
        assert_eq!(
            serde_json::from_str::<State>("\"void\"").unwrap(),
            State::Void
        );
    }

    /// The `void` contract, stated over the whole `Stage::ALL ×
    /// State::ALL` product rather than as the three examples that
    /// motivated it: every real pair sorts below `void`, and `void` sorts
    /// to one value no matter which stage carries it.
    #[test]
    fn void_outranks_every_pair_at_every_stage() {
        for stage in Stage::ALL {
            for state in State::ALL {
                for at in Stage::ALL {
                    let key = rollup_key(stage, state);
                    let void = rollup_key(at, State::Void);
                    if state == State::Void {
                        assert_eq!(
                            key, void,
                            "`void` must not depend on its stage: {stage}/{state} vs {at}/void"
                        );
                    } else {
                        assert!(key < void, "{stage}/{state} must sort below {at}/void");
                    }
                }
            }
        }
    }

    /// The three worked examples of DRIFT-028 §4.1, by name. "Worst-of"
    /// is `min_by_key(rollup_key)` — the fold `rollup_doc` runs.
    #[test]
    fn worst_of_reads_void_as_no_claim() {
        fn worst(pairs: &[(Stage, State)]) -> (Stage, State) {
            *pairs
                .iter()
                .min_by_key(|(st, s)| rollup_key(*st, *s))
                .expect("at least one marker")
        }
        // worst-of {spec/void, impl/plan} = impl/plan — the live part
        // governs; the tombstone's *stage* no longer drags the document.
        assert_eq!(
            worst(&[(Stage::Spec, State::Void), (Stage::Impl, State::Plan)]),
            (Stage::Impl, State::Plan)
        );
        // worst-of {done, void} = done — real work outranks no claim.
        assert_eq!(
            worst(&[(Stage::Impl, State::Done), (Stage::Impl, State::Void)]),
            (Stage::Impl, State::Done)
        );
        // worst-of {void} = void, and a document whose every unit is void
        // *is* void — the same rule, not a special case.
        assert_eq!(
            worst(&[(Stage::Spec, State::Void)]),
            (Stage::Spec, State::Void)
        );
        assert_eq!(
            worst(&[(Stage::Spec, State::Void), (Stage::Doc, State::Void)]).1,
            State::Void
        );
    }

    #[test]
    fn nearest_catches_the_famous_typo() {
        assert_eq!(
            nearest("rewrok", Action::ALL.iter().map(|a| a.as_str())),
            Some("rework")
        );
        assert_eq!(
            nearest("zzzzzz", Action::ALL.iter().map(|a| a.as_str())),
            None
        );
    }
}
