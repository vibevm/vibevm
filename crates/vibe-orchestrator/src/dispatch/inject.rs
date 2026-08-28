//! A deterministic failure injection for the post-row boundary.
//!
//! The rows this dispatcher measures are only carried outward if the real
//! refresh-and-carry seam runs, and the ONLY failures that exercise it are the
//! generic ones — a state write, a checkpoint, a park reconciliation — which no
//! fixture can provoke on demand. A unit test that hand-built a populated
//! vector would stay green after that seam was deleted, which is exactly the
//! mutation this exists to kill.
//!
//! So: arm a fault after N real reports, run the REAL producer, and require
//! those N rows to come back on the carried draft. The switch is thread-local
//! (tests run in parallel), reads no environment, is disarmed by a guard that
//! runs on panic as well as on return, and does not exist outside `cfg(test)`.

use std::cell::Cell;

thread_local! {
    static FAIL_AFTER: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Disarms on drop — including while unwinding, so a failed assertion
/// cannot leak the fault into the next test on this thread.
pub(crate) struct Armed;

impl Drop for Armed {
    fn drop(&mut self) {
        FAIL_AFTER.with(|slot| slot.set(None));
    }
}

#[must_use = "the guard disarms the injection when it drops"]
pub(crate) fn fail_after(rows: usize) -> Armed {
    FAIL_AFTER.with(|slot| slot.set(Some(rows)));
    Armed
}

pub(crate) fn armed_at(pushed: usize) -> bool {
    FAIL_AFTER
        .with(Cell::get)
        .is_some_and(|after| pushed >= after)
}
