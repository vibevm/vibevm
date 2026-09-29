//! The need-gate (D-C3-1): the one auditable decision that turns a task
//! into a verdict — `inline | route | fold-local | spawn | escalate` —
//! with a journaled reason. This module is the pure decision *procedure*
//! (plan §10.3, fixed order, first match wins); the *policy* it reads
//! (depth caps, window margins, capability classes) is tabular data in
//! the `delegation-rules` package, passed in as [`GateInputs`]. Keeping
//! the procedure pure makes every verdict a unit test, and keeps the
//! boss/MC call site a thin wrapper that gathers the signals and records
//! the result.
//!
//! Why a gate at all: the evidence (RD-1, RD-2, RD-6) says routing must be
//! auditable data, not prompt-embedded vibes — and that wrapping a
//! natively-capable model in machinery makes it *worse*, so "does this fit
//! as-is?" is asked before "how do I decompose it?".

use serde::{Deserialize, Serialize};

specmark::scope!("spec://fractality/PROP-001#model");

/// The five need-gate verdicts (plan §10.2 glossary). Serialized
/// kebab-case so `fold-local` round-trips as the journal reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    /// The boss does it itself. No worker, no packet.
    Inline,
    /// ONE worker call with the task as-is — no decomposition, no child
    /// tree (Fugu-standard's whole business; the cheap default for
    /// single-skill tasks that fit the worker's window).
    Route,
    /// A bounded sub-session in the boss's OWN context space; returns a
    /// summary, no pod, no isolation.
    FoldLocal,
    /// Child packet(s) through MC, each with its own budgets, env, and run
    /// identity — the only verdict that creates a run tree.
    Spawn,
    /// Return the task UP, annotated `escalated(reason, needs)`. A
    /// first-class outcome, not a failure; the top of every chain is the
    /// human.
    Escalate,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Inline => "inline",
            Verdict::Route => "route",
            Verdict::FoldLocal => "fold-local",
            Verdict::Spawn => "spawn",
            Verdict::Escalate => "escalate",
        }
    }
}

/// A verdict plus its one-line journaled reason (D-C3-1: every decision is
/// recorded verbatim, never inferred later). Serialize-only: the reason is
/// a `&'static str` literal from the procedure, which journals (Serialize)
/// but cannot be borrowed back as `'static` on read — a journal reader
/// uses a `String`-carrying DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub verdict: Verdict,
    pub reason: &'static str,
}

impl Decision {
    fn new(verdict: Verdict, reason: &'static str) -> Self {
        Self { verdict, reason }
    }
}

/// The signals the decision procedure evaluates (plan §10.3). The boss/MC
/// gathers these — most from the task and the candidate worker's profile,
/// the caps from `delegation-rules`. Booleans keep the *procedure* pure
/// and total; how each signal is measured is the caller's (and the
/// policy table's) concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateInputs {
    /// An O(1) lookup / single fact.
    pub o1_lookup: bool,
    /// The boss lacks a tool the (O(1)) task needs — routes instead of
    /// doing it inline.
    pub needs_absent_tool: bool,
    /// Task + its context fit the candidate worker's window with ≥30%
    /// margin (RD-2's window-fit guard).
    pub fits_window: bool,
    /// The task draws on a single skill (do not decompose what fits).
    pub single_skill: bool,
    /// Cross-chunk dependence dominates: the answer needs global reasoning
    /// over the whole context and chunking destroys it (Silo regime).
    pub cross_chunk_dominant: bool,
    /// A largest-window profile is available to route a Silo task to,
    /// instead of escalating.
    pub large_window_available: bool,
    /// Decomposable into sub-results that compose (mechanically or via one
    /// merge node).
    pub decomposable: bool,
    /// The caller's current nesting depth.
    pub depth: u32,
    /// The depth cap from `delegation-rules` (0 = unlimited; default
    /// policy is 1 — depth 2 only behind the experimental flag).
    pub max_depth: u32,
    /// Whether the caller's capability class may open a child tree at all
    /// (the routing policy's `max_depth > 0`). The weak class carries a
    /// policy cap of `0`, which on the *routing* axis means "no spawning";
    /// but on `max_depth` `0` means "unlimited", so a no-spawn class must
    /// be gated HERE, not through `max_depth`. Separating the two keeps
    /// `decide` total: a no-spawn class folds its decomposable work
    /// instead of opening a tree (resolves the D-C3-8 overload).
    pub can_spawn: bool,
}

/// A journaled need-gate decision (D-C3-8): the verdict, its reason, the
/// candidate worker's capability class, and the signals that produced it
/// — owned and round-trippable, unlike [`Decision`] whose `reason` is a
/// `&'static str`. One row per gate invocation; the soft-label table (per
/// worker-class × task shape, §10.2) is a replay-and-aggregate over these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub verdict: Verdict,
    pub reason: String,
    /// The candidate worker's capability class this verdict was made for.
    pub class: crate::routing::CapabilityClass,
    /// The gate inputs, so the table can slice by task shape.
    pub inputs: GateInputs,
}

impl DecisionRecord {
    /// Builds a record from a live [`Decision`]; the `&'static` reason is
    /// copied into an owned `String` for the journal.
    pub fn from_decision(
        decision: &Decision,
        class: crate::routing::CapabilityClass,
        inputs: GateInputs,
    ) -> Self {
        Self {
            verdict: decision.verdict,
            reason: decision.reason.to_owned(),
            class,
            inputs,
        }
    }
}

/// One decisions-journal line: a timestamp plus the record. Rides the
/// sibling-stem machinery (`open_stem`/`replay_stem`) like the session
/// journal — its own file, no fold into run state (D-C3-8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionEnvelope {
    pub ts_ms: u64,
    #[serde(flatten)]
    pub record: DecisionRecord,
}

impl DecisionEnvelope {
    /// Wraps a record with the current wall clock.
    pub fn now(record: DecisionRecord) -> Self {
        Self {
            ts_ms: crate::time::now_ms(),
            record,
        }
    }
}

/// The fixed-order decision procedure (plan §10.3). Evaluate top-down;
/// first match wins. Every arm returns the verdict AND the reason to
/// journal — no silent verdicts.
pub fn decide(i: &GateInputs) -> Decision {
    // 1. O(1) lookup / single fact → inline (route only if it needs a
    //    tool the boss lacks). Never spawn for these.
    if i.o1_lookup {
        return if i.needs_absent_tool {
            Decision::new(
                Verdict::Route,
                "O(1) lookup needs a tool the boss lacks — route, never spawn",
            )
        } else {
            Decision::new(
                Verdict::Inline,
                "O(1) lookup / single fact — the boss does it inline",
            )
        };
    }
    // 2. Fits the worker window with margin AND single-skill → route. Do
    //    NOT decompose what fits — wrapping natively-capable models makes
    //    them worse (RD-2, three independent sources).
    if i.fits_window && i.single_skill {
        return Decision::new(
            Verdict::Route,
            "fits the worker window (>=30% margin), single-skill — route as-is, do not decompose",
        );
    }
    // 3. Cross-chunk dependence dominates → escalate, or route to the
    //    largest-window profile if one exists. Never fan out (Silo
    //    theorem: fan-out saturates below optimal regardless of quality).
    if i.cross_chunk_dominant {
        return if i.large_window_available {
            Decision::new(
                Verdict::Route,
                "cross-chunk (Silo): route to the largest-window profile, never fan out",
            )
        } else {
            Decision::new(
                Verdict::Escalate,
                "cross-chunk dependence dominates (Silo theorem) — escalate up, do not fan out",
            )
        };
    }
    // 4. Decomposable with composable sub-results → spawn, under the depth
    //    cap. At the cap, force-execute in-context (fold-local) rather
    //    than opening a deeper tree (D-C3-3 at-cap boundary).
    if i.decomposable {
        // A class that may not spawn (routing policy cap 0 → can_spawn
        // false) never opens a tree: its decomposable work folds into the
        // caller. This is where the weak class is kept off spawning roots
        // (D-C3-10); `max_depth` cannot express it, since `0` there means
        // unlimited, not "no spawning".
        if !i.can_spawn {
            return Decision::new(
                Verdict::FoldLocal,
                "decomposable but this class may not spawn a tree — fold into the caller",
            );
        }
        let at_cap = i.max_depth != 0 && i.depth >= i.max_depth;
        return if at_cap {
            Decision::new(
                Verdict::FoldLocal,
                "decomposable but at the depth cap — force-execute locally, do not spawn deeper",
            )
        } else {
            Decision::new(
                Verdict::Spawn,
                "decomposable into composable sub-results — spawn under the depth cap",
            )
        };
    }
    // 5. Otherwise (context-heavy, sequential, no isolation need) →
    //    fold-local: a bounded sub-session, no pod.
    Decision::new(
        Verdict::FoldLocal,
        "context-heavy, sequential, no isolation need — fold locally",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A baseline that reaches the final fold-local arm; each test flips
    /// exactly the signals its branch needs.
    fn base() -> GateInputs {
        GateInputs {
            o1_lookup: false,
            needs_absent_tool: false,
            fits_window: false,
            single_skill: false,
            cross_chunk_dominant: false,
            large_window_available: false,
            decomposable: false,
            depth: 0,
            max_depth: 1,
            can_spawn: true,
        }
    }

    #[test]
    fn o1_lookup_is_inline_unless_a_tool_is_missing() {
        let i = GateInputs {
            o1_lookup: true,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::Inline);

        let i = GateInputs {
            o1_lookup: true,
            needs_absent_tool: true,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::Route);
    }

    #[test]
    fn fits_window_single_skill_routes_not_spawns() {
        let i = GateInputs {
            fits_window: true,
            single_skill: true,
            // Even if it looks decomposable, fitting wins first (do not
            // decompose what fits).
            decomposable: true,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::Route);
    }

    #[test]
    fn silo_escalates_or_routes_to_the_biggest_window() {
        let i = GateInputs {
            cross_chunk_dominant: true,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::Escalate);

        let i = GateInputs {
            cross_chunk_dominant: true,
            large_window_available: true,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::Route);
    }

    #[test]
    fn decomposable_spawns_under_the_cap_and_folds_at_it() {
        let i = GateInputs {
            decomposable: true,
            depth: 0,
            max_depth: 1,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::Spawn);

        // At the cap: force-execute locally, never spawn deeper.
        let i = GateInputs {
            decomposable: true,
            depth: 1,
            max_depth: 1,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::FoldLocal);

        // max_depth = 0 means unlimited: spawn regardless of depth.
        let i = GateInputs {
            decomposable: true,
            depth: 9,
            max_depth: 0,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::Spawn);
    }

    /// A class that may not spawn (`can_spawn = false` — the weak class's
    /// routing cap of 0) folds its decomposable work rather than opening a
    /// tree. The `max_depth = 0` overload ("unlimited" on that axis) must
    /// NOT win here: the `can_spawn` gate keeps the weak class off
    /// spawning roots (D-C3-8 overload resolution).
    #[test]
    fn a_no_spawn_class_folds_instead_of_spawning() {
        let i = GateInputs {
            decomposable: true,
            can_spawn: false,
            max_depth: 0,
            depth: 0,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::FoldLocal);
    }

    #[test]
    fn everything_else_folds_local() {
        assert_eq!(decide(&base()).verdict, Verdict::FoldLocal);
    }

    #[test]
    fn verdicts_serialize_kebab_case() {
        let json = serde_json::to_string(&Verdict::FoldLocal).expect("serializes");
        assert_eq!(json, "\"fold-local\"");
    }

    /// First match wins: an O(1) task that also fits the window is inline,
    /// not route — the order in §10.3 is load-bearing.
    #[test]
    fn first_match_wins_in_declared_order() {
        let i = GateInputs {
            o1_lookup: true,
            fits_window: true,
            single_skill: true,
            ..base()
        };
        assert_eq!(decide(&i).verdict, Verdict::Inline);
    }

    /// A decision journals as one flat line: `ts_ms` plus the verdict,
    /// reason, class, and inputs, round-tripping intact (D-C3-8).
    #[test]
    fn decision_record_round_trips_as_a_flat_line() {
        let inputs = GateInputs {
            decomposable: true,
            can_spawn: true,
            ..base()
        };
        let decision = decide(&inputs);
        assert_eq!(decision.verdict, Verdict::Spawn);
        let record = DecisionRecord::from_decision(
            &decision,
            crate::routing::CapabilityClass::Medium,
            inputs,
        );
        let env = DecisionEnvelope {
            ts_ms: 42,
            record: record.clone(),
        };
        let json = serde_json::to_string(&env).expect("serializes");
        assert!(json.contains("\"ts_ms\":42"), "{json}");
        assert!(json.contains("\"verdict\":\"spawn\""), "{json}");
        assert!(json.contains("\"class\":\"medium\""), "{json}");
        let back: DecisionEnvelope = serde_json::from_str(&json).expect("parses");
        assert_eq!(back.ts_ms, 42);
        assert_eq!(back.record, record);
    }
}
