//! The markup vocabulary and the marker model.
//!
//! Closed vocabularies per PROP-043 §3.2 — any value outside these enums
//! is a validation error, never a silent pass-through.
//!
//! This cell holds the vocabularies themselves — the enums a marker is
//! written from and the [`Marker`] that carries them. Three things read
//! those vocabularies rather than declaring them, and each has its own
//! cell beside this one: the terminal-artifact set
//! ([`artifacts`]), the worst-of rollup order ([`rollup_order`]) and the
//! typo hint a rejected token gets ([`typo_hints`]). Their public names
//! are re-exported here, so `model::ArtifactKind`, `model::rollup_key`
//! and `model::nearest` are the paths they have always been.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#attributes");

use serde::{Deserialize, Serialize};
use std::fmt;

mod artifacts;
mod rollup_order;
mod typo_hints;

pub use artifacts::{ArtifactKind, ArtifactRequirements};
pub use rollup_order::rollup_key;
pub use typo_hints::nearest;

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
}
