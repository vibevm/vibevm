//! The global event sequence, with exhaustion said out loud.
//!
//! `sequence-density` makes `events[i].sequence == i`, so the next sequence a
//! run may spend is exactly its event count — and the epoch stores it in a
//! `uint32`. A run at that ceiling has no next number, and the two obvious
//! ways to write it down are both wrong:
//!
//! * `unwrap_or(u32::MAX)` on a restored count silently maps *unrepresentable*
//!   onto the last representable value, so a reopened run hands out a sequence
//!   an existing event already spent;
//! * refusing at `u32::MAX` throws away a number the validator accepts —
//!   `dense_sequence` narrows a position with `try_from` and does not advance
//!   past it, so the final representable sequence is legal *once*.
//!
//! So the state is a two-arm value rather than a number with a sentinel. The
//! last sequence is usable exactly once and the very next transition is
//! [`NextSequence::Exhausted`], from which nothing is ever handed out again.
//!
//! Invocation ordinals are deliberately NOT modelled this way: the validator's
//! `invocation-key` law advances its own counter with a checked add on the
//! same event, so an event *carrying* `u32::MAX` is refused outright. That
//! stricter law stays a plain `checked_add` in the caller — the asymmetry is
//! real, and flattening it would make one of the two counters lie.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

/// The next global sequence this run may spend, or the fact that there is
/// none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NextSequence {
    Next(u32),
    Exhausted,
}

impl NextSequence {
    /// Restore from an adopted index's event count.
    ///
    /// A count the epoch's `uint32` cannot address is `Exhausted`, never
    /// `Next(u32::MAX)`: an index that long has already spent every number,
    /// and mapping it back onto the last one would reissue it.
    pub(super) fn restored_from(events: usize) -> Self {
        match u32::try_from(events) {
            Ok(next) => Self::Next(next),
            Err(_) => Self::Exhausted,
        }
    }

    /// The number to spend, or `None` when there is none left.
    pub(super) const fn value(self) -> Option<u32> {
        match self {
            Self::Next(value) => Some(value),
            Self::Exhausted => None,
        }
    }

    /// The state after the current number has been committed to the index.
    pub(super) const fn advanced(self) -> Self {
        match self {
            Self::Next(value) => match value.checked_add(1) {
                Some(next) => Self::Next(next),
                None => Self::Exhausted,
            },
            Self::Exhausted => Self::Exhausted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ordinary_count_restores_to_the_next_number() {
        assert_eq!(NextSequence::restored_from(0), NextSequence::Next(0));
        assert_eq!(NextSequence::restored_from(7), NextSequence::Next(7));
        assert_eq!(NextSequence::restored_from(0).value(), Some(0));
    }

    /// The ceiling, exercised at the helper rather than by allocating four
    /// billion events: the last representable sequence is handed out ONCE and
    /// the state then has nothing left.
    #[test]
    fn the_last_sequence_is_spent_once_and_never_reissued() {
        let last = NextSequence::Next(u32::MAX);
        assert_eq!(last.value(), Some(u32::MAX));
        let after = last.advanced();
        assert_eq!(after, NextSequence::Exhausted);
        assert_eq!(after.value(), None, "no number survives exhaustion");
        assert_eq!(
            after.advanced(),
            NextSequence::Exhausted,
            "and exhaustion is absorbing",
        );
    }

    /// An event count past the `uint32` ceiling is exhaustion, NOT the last
    /// representable number. This is the exact regression the two-arm type
    /// exists to make unwritable.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn an_unrepresentable_count_never_folds_back_onto_the_last_number() {
        assert_eq!(
            NextSequence::restored_from(u32::MAX as usize),
            NextSequence::Next(u32::MAX),
            "exactly at the ceiling the last number is still owed",
        );
        for events in [u32::MAX as usize + 1, u32::MAX as usize + 9_999] {
            assert_eq!(
                NextSequence::restored_from(events),
                NextSequence::Exhausted,
                "{events} events have already spent every number",
            );
        }
    }
}
