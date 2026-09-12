//! The worst-of rollup order (PROP-043 §3.10).
//!
//! The one place that decides which of two `(stage, state)` pairs is
//! less advanced, and the one place `void` is given its position outside
//! that scheme.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#attributes");

use super::{Stage, State};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollup_order_matches_prop_043() {
        // unknown is the floor; freeze/done is the ceiling.
        assert!(rollup_key(Stage::Unknown, State::Done) < rollup_key(Stage::Idea, State::Hold));
        assert!(rollup_key(Stage::Idea, State::Done) < rollup_key(Stage::Spec, State::Hold));
        assert!(rollup_key(Stage::Impl, State::Work) < rollup_key(Stage::Impl, State::Done));
        assert!(rollup_key(Stage::Doc, State::Done) < rollup_key(Stage::Freeze, State::Plan));
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
}
