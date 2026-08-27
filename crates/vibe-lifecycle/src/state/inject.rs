//! Injection points for durable state-write faults, so the transaction REDs
//! have deterministic counterexamples instead of ones that depend on
//! filesystem permissions. Compiled out entirely outside tests, and they read
//! no environment: the canonical file's OLD bytes must stay readable, which
//! deleting or chmod-ing it would not preserve.
//!
//! Two seams, one per transaction stage:
//!
//! - `fail_state_writes` fails a publication BEFORE anything is attempted —
//!   the `BeforePublication` branch. The disk is untouched.
//! - `fail_state_publication_possibly` fails a publication AS IF the rename
//!   had been attempted and then failed — the `PossiblyPublished` recovery
//!   window. It performs no write either, so the bytes the recovery then
//!   re-reads are exactly the bytes the test arranged: the prior bytes, a
//!   third state, or an unsafe shape.

use std::cell::RefCell;

/// One armed synthetic fault plus the concurrent-writer plant that fires with
/// it: the reason rendered into the diagnostic, and the closure that stands
/// in for the third writer inside the publication window.
type ArmedPlant = (String, Box<dyn FnOnce()>);

thread_local! {
    static ARMED: RefCell<Option<String>> = const { RefCell::new(None) };
    static ARMED_POSSIBLY: RefCell<Option<String>> = const { RefCell::new(None) };
    static ARMED_POSSIBLY_PLANT: RefCell<Option<ArmedPlant>> = const { RefCell::new(None) };
}

/// Make every subsequent state publication on THIS thread fail before it
/// begins. Pass `None` to disarm.
pub(crate) fn fail_state_writes(reason: Option<&str>) {
    ARMED.with(|armed| *armed.borrow_mut() = reason.map(str::to_string));
}

pub(super) fn armed() -> Option<String> {
    ARMED.with(|armed| armed.borrow().clone())
}

/// Make every subsequent state publication on THIS thread fail as if its
/// rename had been attempted — the post-publication recovery window —
/// without touching the disk itself, so the test controls exactly which
/// bytes the one bounded re-read will find. The fault stays armed until
/// disarmed, exactly like `fail_state_writes`. Pass `None` to disarm.
pub(crate) fn fail_state_publication_possibly(reason: Option<&str>) {
    ARMED_POSSIBLY.with(|armed| *armed.borrow_mut() = reason.map(str::to_string));
}

pub(super) fn armed_possibly() -> Option<String> {
    ARMED_POSSIBLY.with(|armed| armed.borrow().clone())
}

/// The one-shot twin of `fail_state_publication_possibly` for the outcome no
/// single-threaded sequence can arrange: a `plant` closure runs in the
/// publication window — after the prior was read, before the recovery's
/// re-read — so a test can stand in for the concurrent writer that puts a
/// THIRD state on disk while the publication is in flight. The fault fires
/// once and disarms itself.
pub(crate) fn fail_state_publication_possibly_planting(reason: &str, plant: Box<dyn FnOnce()>) {
    ARMED_POSSIBLY_PLANT.with(|armed| {
        *armed.borrow_mut() = Some((reason.to_string(), plant));
    });
}

pub(super) fn armed_possibly_plant() -> Option<ArmedPlant> {
    ARMED_POSSIBLY_PLANT.with(|armed| armed.borrow_mut().take())
}
