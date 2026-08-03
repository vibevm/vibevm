//! Materialization-mode policy the install pipeline consults — currently the
//! destructive-operation guard (PROP-022 §2.6).
//!
//! The *placement* of each mode (`snapshot` full copy, `hardlink`, the future
//! `in-place` clone) lives in [`crate::vibedeps`]; this cell owns only the
//! **decision** of whether a destructive operation may proceed on a slot of a
//! given mode. It is a pure function over the mode and the caller's
//! interactivity, so the CLI's uninstall / `reinstall --force` paths can be
//! unit-tested without a terminal.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#destructive-guard");

use specmark::spec;
use vibe_core::manifest::Materialization;

/// What a caller must do before a **destructive** operation — `uninstall`,
/// `reinstall --force`, a version switch that re-clones, or any slot removal —
/// may proceed on a slot of a given materialization mode (PROP-022 §2.6).
///
/// A `snapshot` / `hardlink` slot is vendored and cheap to reconstruct
/// (offline, from the project's own git), so its removal is unguarded. An
/// `in-place` slot is a git-native, **non-vendored** working tree whose only
/// restoration is a network re-clone — possibly a multi-hour download — so its
/// destruction must never happen silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestructiveGuard {
    /// Not guarded — proceed under the caller's normal confirmation flow.
    /// Either the mode is not `in-place`, or the operator gave an explicit
    /// non-interactive opt-in (`--force` / `--assume-yes` / `--unattended`).
    Proceed,
    /// `in-place` slot in an interactive session with no prior opt-in — the
    /// caller **must** obtain an explicit `y/n` it cannot bypass (a `--json`
    /// or otherwise non-interactive auto-yes is not allowed to stand in).
    ConfirmInteractively,
    /// `in-place` slot, non-interactive, no explicit opt-in — the caller must
    /// **abort** rather than silently delete an expensive resource.
    Abort,
}

/// Decide whether a destructive operation may proceed on a slot of the given
/// materialization `mode` (PROP-022 §2.6).
///
/// - **non-`in-place`** (`snapshot` / `hardlink`) → [`DestructiveGuard::Proceed`]:
///   vendored, offline-reproducible, no special guard.
/// - **`in-place` + `opted_in`** (an explicit `--force` / `--assume-yes` /
///   `--unattended`) → [`DestructiveGuard::Proceed`]: the operator deliberately
///   chose the removal.
/// - **`in-place` + interactive, no opt-in** → [`DestructiveGuard::ConfirmInteractively`]:
///   ask a `y/n` that no output mode may auto-answer.
/// - **`in-place` + non-interactive, no opt-in** → [`DestructiveGuard::Abort`].
///
/// Hooks and their `git clean -dfx` reset are **not** destructive in this
/// sense (PROP-022 §2.6) — they are routine, trusted lifecycle, so they never
/// reach this guard.
///
/// ```
/// use vibe_core::manifest::Materialization;
/// use vibe_workspace::materialization::{DestructiveGuard, guard_destructive};
///
/// // A vendored snapshot slot is cheap to rebuild — never guarded.
/// assert_eq!(
///     guard_destructive(Materialization::Snapshot, false, false),
///     DestructiveGuard::Proceed,
/// );
/// // An in-place slot in a non-interactive run with no opt-in aborts.
/// assert_eq!(
///     guard_destructive(Materialization::InPlace, false, false),
///     DestructiveGuard::Abort,
/// );
/// // …but an explicit --force / --assume-yes lets it through.
/// assert_eq!(
///     guard_destructive(Materialization::InPlace, false, true),
///     DestructiveGuard::Proceed,
/// );
/// // Interactive with no prior opt-in must ask a mandatory y/n.
/// assert_eq!(
///     guard_destructive(Materialization::InPlace, true, false),
///     DestructiveGuard::ConfirmInteractively,
/// );
/// ```
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#destructive-guard",
    r = 1
)]
pub fn guard_destructive(
    mode: Materialization,
    interactive: bool,
    opted_in: bool,
) -> DestructiveGuard {
    if !mode.is_in_place() || opted_in {
        DestructiveGuard::Proceed
    } else if interactive {
        DestructiveGuard::ConfirmInteractively
    } else {
        DestructiveGuard::Abort
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#destructive-guard",
        r = 1
    )]
    fn snapshot_and_hardlink_are_never_guarded() {
        // Vendored modes reconstruct from the project's own git, so removing
        // their slot carries no special risk in any context.
        for mode in [Materialization::Snapshot, Materialization::Hardlink] {
            for interactive in [false, true] {
                for opted_in in [false, true] {
                    assert_eq!(
                        guard_destructive(mode, interactive, opted_in),
                        DestructiveGuard::Proceed,
                        "{mode:?} interactive={interactive} opted_in={opted_in}",
                    );
                }
            }
        }
    }

    #[test]
    #[verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#destructive-guard",
        r = 1
    )]
    fn in_place_aborts_only_when_non_interactive_and_not_opted_in() {
        // The one case the spec forbids: silent deletion of an in-place slot.
        assert_eq!(
            guard_destructive(Materialization::InPlace, false, false),
            DestructiveGuard::Abort,
        );
    }

    #[test]
    #[verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#destructive-guard",
        r = 1
    )]
    fn in_place_opt_in_proceeds_in_any_session() {
        // An explicit --force / --assume-yes / --unattended is the spec's
        // sanctioned non-interactive opt-in — it wins regardless of TTY.
        assert_eq!(
            guard_destructive(Materialization::InPlace, false, true),
            DestructiveGuard::Proceed,
        );
        assert_eq!(
            guard_destructive(Materialization::InPlace, true, true),
            DestructiveGuard::Proceed,
        );
    }

    #[test]
    #[verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#destructive-guard",
        r = 1
    )]
    fn in_place_interactive_without_opt_in_must_ask() {
        assert_eq!(
            guard_destructive(Materialization::InPlace, true, false),
            DestructiveGuard::ConfirmInteractively,
        );
    }
}
