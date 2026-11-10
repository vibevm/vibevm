//! The failure law of one `slot:` contribution (PROP-065
//! `#ordinary-failure`): how a failed one is recorded, whether the point's
//! sequence goes on, and the retry the message names.
//!
//! **Pure, and separate from the engine that runs them.** The decision is a
//! function of three facts the caller already holds — was a stop requested,
//! is this the soft post-install case PROP-020 `#FAIL-POST-FLAGS` admits,
//! and is the contribution `optional` — so the law is tested as a table
//! instead of being inferred from three failing installs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-failure");

use specmark::spec;
use vibe_lifecycle::SlotTarget;

/// How one failed contribution is recorded, and what happens next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FailureOutcome {
    /// The outcome status the row carries.
    pub(super) status: &'static str,
    /// Whether the package is installed and **flagged** — true of every
    /// failure the run goes on past, because such a failure must never read
    /// as a silent success.
    pub(super) flagged: bool,
    /// Whether this ends the point's sequence for this package.
    pub(super) stops: bool,
}

/// Classify one failure.
///
/// Three rules, in this order, and the order is the law:
///
/// 1. **A requested stop is an interrupt, not a failure.** The outcome is
///    recorded `interrupted`, nothing after it runs, and an `optional`
///    contribution does not excuse continuing: `#ordinary-failure` says no
///    contribution is marked `done` by an interrupted exit and nothing
///    after it runs, which an optional "carry on" would quietly break. A
///    `pre-install` interrupt therefore rolls the slot back exactly as a
///    failure does, because it stops the pass the same way.
/// 2. **The soft post-install case keeps PROP-020 `#FAIL-POST-FLAGS`.** The
///    package is already installed and *is* the thing that would have to be
///    undone, so it stays installed and flagged.
/// 3. **An `optional` contribution is recorded `failed` and the sequence
///    continues** (`#failure`'s rule for a step, which `#ordinary-failure`
///    adopts verbatim); anything else ends the point's sequence for that
///    package, which for `pre-install` is PROP-020 `#FAIL-PRE-ABORTS`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-failure")]
pub(super) fn failure_outcome(cancelled: bool, soft_post: bool, optional: bool) -> FailureOutcome {
    if cancelled {
        return FailureOutcome {
            status: "interrupted",
            flagged: false,
            stops: true,
        };
    }
    if soft_post || optional {
        return FailureOutcome {
            status: "fail",
            flagged: true,
            stops: false,
        };
    }
    FailureOutcome {
        status: "fail",
        flagged: false,
        stops: true,
    }
}

/// The retry sentence every failed `slot:` contribution carries.
///
/// `#ordinary-failure` requires the message to name the exact command, and
/// requires it to be a **rerun** rather than a resume: there is no
/// `--resume` and no `--from-step` on this plane, because both need a kept
/// work directory keyed to an installation key an ordinary package has not
/// got. `vibe reinstall <group>/<name>` restores the recorded payload,
/// which is what makes PROP-020 `#RESET-THEN-RERUN`'s payload diff
/// non-empty — the condition under which contributions rerun at all.
///
/// A row with no slot target is a host-declared contribution, which no
/// reinstall of a dependency re-runs, so it is given no command to retry
/// with rather than a wrong one.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-failure")]
pub(super) fn retry_line(target: Option<&SlotTarget>) -> String {
    target.map_or_else(String::new, |target| {
        format!(
            "; the retry is a rerun and not a resume — `vibe reinstall {}/{}` (this plane has no \
             `--resume` and no `--from-step`)",
            target.group, target.name
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> SlotTarget {
        SlotTarget {
            group: "org.example".into(),
            name: "hooked".into(),
            version: "0.1.0".into(),
            kind: "tool".into(),
            root: "/slot".into(),
        }
    }

    /// The whole table, so the three rules are read as one decision.
    #[test]
    fn the_failure_table_is_the_law_of_ordinary_failure() {
        // A requested stop wins over everything, optional included.
        for optional in [false, true] {
            for soft_post in [false, true] {
                let outcome = failure_outcome(true, soft_post, optional);
                assert_eq!(outcome.status, "interrupted");
                assert!(outcome.stops, "nothing after an interrupt runs");
                assert!(!outcome.flagged, "an interrupted run flags nothing");
            }
        }
        // The soft post-install case: installed and flagged, never silent.
        let soft = failure_outcome(false, true, false);
        assert_eq!(
            (soft.status, soft.flagged, soft.stops),
            ("fail", true, false)
        );
        // An optional contribution is recorded and the sequence continues.
        let optional = failure_outcome(false, false, true);
        assert_eq!(
            (optional.status, optional.flagged, optional.stops),
            ("fail", true, false)
        );
        // Anything else ends this point's sequence for this package.
        let hard = failure_outcome(false, false, false);
        assert_eq!(
            (hard.status, hard.flagged, hard.stops),
            ("fail", false, true)
        );
    }

    /// The exact command `#ordinary-failure` requires, and no command at all
    /// where a reinstall would not rerun the row.
    #[test]
    fn the_retry_names_vibe_reinstall_group_slash_name() {
        let line = retry_line(Some(&target()));
        assert!(
            line.contains("`vibe reinstall org.example/hooked`"),
            "{line}"
        );
        assert!(line.contains("rerun"), "and says it is a rerun: {line}");
        assert!(retry_line(None).is_empty());
    }
}
