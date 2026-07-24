//! The markup vocabulary and the marker model.
//!
//! Closed vocabularies per PROP-043 §3.2 — any value outside these enums
//! is a validation error, never a silent pass-through.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#attributes");

use serde::{Deserialize, Serialize};
use std::fmt;

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
/// first): `hold < plan < work < done`.
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
    pub const ALL: [State; 4] = [State::Plan, State::Work, State::Done, State::Hold];

    pub fn as_str(self) -> &'static str {
        match self {
            State::Plan => "plan",
            State::Work => "work",
            State::Done => "done",
            State::Hold => "hold",
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
    pub const ALL: [Audience; 3] = [Audience::User, Audience::Author, Audience::Dev];

    pub fn as_str(self) -> &'static str {
        match self {
            Audience::User => "user",
            Audience::Author => "author",
            Audience::Dev => "dev",
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

/// The fixed sort key for worst-of rollup (PROP-043 §3.10):
/// `unknown < idea < spec < impl < test < doc < freeze`, and within a
/// stage the `State` completeness order. Lower = less advanced = "worse".
pub fn rollup_key(stage: Stage, state: State) -> (u8, u8) {
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

    #[test]
    fn rollup_order_matches_prop_043() {
        // unknown is the floor; freeze/done is the ceiling.
        assert!(rollup_key(Stage::Unknown, State::Done) < rollup_key(Stage::Idea, State::Hold));
        assert!(rollup_key(Stage::Idea, State::Done) < rollup_key(Stage::Spec, State::Hold));
        assert!(rollup_key(Stage::Impl, State::Work) < rollup_key(Stage::Impl, State::Done));
        assert!(rollup_key(Stage::Doc, State::Done) < rollup_key(Stage::Freeze, State::Plan));
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
