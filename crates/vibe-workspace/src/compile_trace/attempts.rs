//! The attempt grammar — how one artifact base identity compiles repeatedly
//! inside one adopted run.
//!
//! A scope base (`node:members/alpha`, `unit:org.x/y`) may compile more than
//! once in one lifecycle run: a post-install park/resume regenerates the same
//! node, a scoped update re-emits the same unit. The trace epoch forbids
//! redeclaring a scope id that reached a terminal word, so the integration
//! layer needs id OCCURRENCES: the same base, one id per attempt.
//!
//! The spelling is closed and minted in exactly one place — here:
//!
//! ```text
//! <base>::attempt:<positive-decimal>
//! ```
//!
//! Three laws keep it honest:
//!
//! * **Canonical numbers only.** `attempt:1`, `attempt:2` — never `attempt:0`
//!   (attempts are positive), never `attempt:01` (no leading zeros), never
//!   `attempt:1x`. A non-canonical spelling is not authority: the allocator
//!   ignores it rather than guessing which number it meant, because the only
//!   ids it may act on are ids this grammar minted.
//! * **Exhaustion is explicit.** The counter is a `u32` and the allocator
//!   refuses past [`ATTEMPT_CEILING`] with a dedicated error — it never
//!   saturates onto the last id (which would collide with a real occurrence)
//!   and never wraps to 1 (which would redeclare a terminal scope).
//! * **No arbitrary parsing.** Nothing here reads a user or display string
//!   for authority. The prefix compared against is always the caller's base
//!   descriptor id, and the suffix is always this module's own separator and
//!   grammar.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use vibe_wire::generated::compiler_trace_index::e1::index::{Scope, ScopeStatus};

use super::{ScopeDescriptor, TraceError, bounded};

/// The separator between a base identity and its attempt number. Chosen so
/// that a scope id remains one `scalar-gates`-clean string (no CR/LF/NUL) and
/// so no plausible package name, member path, or run id can contain it by
/// accident — `::attempt:` is not a substring any layer of this stack
/// generates on its own.
pub(super) const SEPARATOR: &str = "::attempt:";

/// The last attempt number this grammar can address. `u32::MAX`, because the
/// epoch's own counters are `u32` and an attempt id is only ever compared and
/// stored as a string — but the ceiling is NAMED so the refusal above it is a
/// law, not an arithmetic accident.
pub(super) const ATTEMPT_CEILING: u32 = u32::MAX;

/// The id of attempt `attempt` under `base`. Only called with a number this
/// grammar produced or validated; callers never hand-render the spelling.
pub(super) fn attempt_id(base: &str, attempt: u32) -> String {
    let mut id = String::with_capacity(base.len() + SEPARATOR.len() + 10);
    id.push_str(base);
    id.push_str(SEPARATOR);
    id.push_str(&attempt.to_string());
    id
}

/// The attempt number `id` carries under `base`, or `None` when `id` is not
/// this grammar's spelling — a different base, the bare base itself, or a
/// non-canonical number. THE ONLY parser of attempt authority.
pub(super) fn parse_attempt(id: &str, base: &str) -> Option<u32> {
    let suffix = id.strip_prefix(base)?.strip_prefix(SEPARATOR)?;
    if suffix.len() > 1 && suffix.starts_with('0') {
        return None; // `01` is not this grammar's spelling of `1`
    }
    if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Attempts are POSITIVE: `attempt:0` is not a number this grammar mints.
    suffix
        .parse()
        .ok()
        .filter(|attempt| *attempt >= FIRST_ATTEMPT)
}

/// The attempt number after `last`, or `None` at the ceiling. The one place
/// the explicit-exhaustion law lives, so no caller can advance past
/// [`ATTEMPT_CEILING`] by its own arithmetic.
pub(super) const fn next_attempt(last: u32) -> Option<u32> {
    if last == ATTEMPT_CEILING {
        None
    } else {
        Some(last + 1)
    }
}

/// The first attempt a base that never compiled in this run spends.
pub(super) const FIRST_ATTEMPT: u32 = 1;

/// Decide which attempt number the next occurrence of `base` spends, given
/// every scope this run has already declared — the whole hosted-resume law in
/// one pure decision:
///
/// * the latest attempt still `pending` is REACQUIRED exactly — after a crash
///   the interrupted occurrence continues, it does not fork — and ONLY under
///   the same identity: a pending attempt wearing a different descriptor is a
///   conflict, never silently redefined;
/// * a latest attempt at a terminal word deterministically mints the next
///   positive number **under the current descriptor**, refusing out loud at
///   the ceiling. Identity is deliberately not compared there: a base survives
///   a display-label or version change inside one adopted run, and a terminal
///   occurrence is history — the new attempt records what compiles NOW;
/// * a base with no attempt yet starts at [`FIRST_ATTEMPT`].
///
/// Only the LATEST attempt is consulted, because it is the only one an
/// allocation can continue: every earlier attempt of a series is either
/// terminal or was already superseded.
pub(super) fn allocate(scopes: &[Scope], base: &ScopeDescriptor) -> Result<u32, TraceError> {
    let mut latest: Option<(u32, &Scope)> = None;
    for scope in scopes {
        let Some(attempt) = parse_attempt(&scope.id, &base.id) else {
            continue;
        };
        if latest.is_none_or(|(seen, _)| attempt > seen) {
            latest = Some((attempt, scope));
        }
    }
    match latest {
        Some((attempt, scope)) if scope.status == ScopeStatus::Pending => {
            if base.matches(scope) {
                Ok(attempt)
            } else {
                Err(TraceError::ScopeConflict {
                    id: bounded::preview(&scope.id),
                })
            }
        }
        Some((attempt, _)) => next_attempt(attempt).ok_or_else(|| TraceError::AttemptExhausted {
            base: bounded::preview(&base.id),
        }),
        None => Ok(FIRST_ATTEMPT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The spelling law, in both directions, including that the number is
    /// canonical: this is the grammar, not a convention.
    #[test]
    fn the_grammar_round_trips_exactly() {
        assert_eq!(attempt_id("node:.", FIRST_ATTEMPT), "node:.::attempt:1");
        assert_eq!(attempt_id("unit:org.x/y", 42), "unit:org.x/y::attempt:42");
        assert_eq!(
            attempt_id("node:.", ATTEMPT_CEILING),
            format!("node:.::attempt:{ATTEMPT_CEILING}")
        );
        assert_eq!(parse_attempt("node:.::attempt:1", "node:."), Some(1));
        assert_eq!(
            parse_attempt("unit:org.x/y::attempt:42", "unit:org.x/y"),
            Some(42)
        );
        assert_eq!(
            parse_attempt(&attempt_id("node:.", 7), "node:."),
            Some(7),
            "what this grammar mints, it reads back"
        );
    }

    /// A different base, the bare base, an empty suffix and any non-canonical
    /// number are ALL non-authority: the parser answers `None`, never a guess.
    #[test]
    fn nothing_but_this_grammar_s_own_spelling_is_authority() {
        let base = "unit:org.x/y";
        for id in [
            "unit:org.x/y",
            "unit:org.x/y::attempt:",
            "unit:org.x/y::attempt:0",
            "unit:org.x/y::attempt:01",
            "unit:org.x/y::attempt:1x",
            "unit:org.x/y::attempt:+1",
            "unit:org.x/y::attempt: 1",
            "node:org.x/y::attempt:1",
            "unit:org.x/y::attempt:1::attempt:1",
        ] {
            assert_eq!(parse_attempt(id, base), None, "`{id}` is not authority");
        }
        // A base that is a strict prefix of another base still cannot steal
        // its attempts: the separator pins the boundary.
        assert_eq!(parse_attempt("unit:org.x/y2::attempt:1", base), None);
    }

    /// The explicit ceiling helper: the last representable attempt advances to
    /// nothing, never wraps, never saturates — and exhaustion is absorbing.
    #[test]
    fn the_ceiling_helper_refuses_past_the_last_attempt() {
        assert_eq!(next_attempt(1), Some(2));
        assert_eq!(next_attempt(41), Some(42));
        assert_eq!(next_attempt(ATTEMPT_CEILING - 1), Some(ATTEMPT_CEILING));
        assert_eq!(
            next_attempt(ATTEMPT_CEILING),
            None,
            "exhaustion is explicit"
        );
        assert_eq!(next_attempt(0), Some(FIRST_ATTEMPT));
    }
}
