//! The blank line between blocks (PROP-067 `#block`, `#list`): exactly one
//! between two blocks, none above the first, none between the rows of one
//! list.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#block");

#[cfg(not(test))]
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// What was last written on a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A block, a rule, a question or anything else that stands alone.
    Block,
    /// A row of the named list: rows of one list touch.
    Row(&'static str),
}

/// The stream a block goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stream {
    Out,
    Err,
}

static ONE_SCREEN: AtomicBool = AtomicBool::new(false);

/// The record of what was last written, one per stream slot. In the
/// binary it is shared by every thread, because the waiting reporter of
/// the plain renderer writes from a thread of its own and still owes the
/// blank line. Under `cfg(test)` it is per thread instead: the unit tests
/// run in parallel threads, and a test that asserts an exact capture would
/// otherwise see the blank line another test's block left behind.
#[cfg(not(test))]
static LAST: Mutex<[Option<Kind>; 2]> = Mutex::new([None, None]);

#[cfg(test)]
thread_local! {
    static LAST_OF_THREAD: std::cell::RefCell<[Option<Kind>; 2]> =
        const { std::cell::RefCell::new([None, None]) };
}

#[cfg(not(test))]
fn with_last<R>(f: impl FnOnce(&mut [Option<Kind>; 2]) -> R) -> R {
    match LAST.lock() {
        Ok(mut last) => f(&mut last),
        Err(_) => f(&mut [None, None]),
    }
}

#[cfg(test)]
fn with_last<R>(f: impl FnOnce(&mut [Option<Kind>; 2]) -> R) -> R {
    LAST_OF_THREAD.with(|last| f(&mut last.borrow_mut()))
}

pub(super) fn set_one_screen(one: bool) {
    ONE_SCREEN.store(one, Ordering::Relaxed);
}

/// Both streams on one screen share one record, so a block on stdout after
/// one on stderr still gets its blank line; a redirected stream keeps its
/// own, so a captured stdout never starts with one.
fn slot(stream: Stream) -> usize {
    match (ONE_SCREEN.load(Ordering::Relaxed), stream) {
        (true, _) | (false, Stream::Err) => 0,
        (false, Stream::Out) => 1,
    }
}

/// The separator to write before something of `kind` on `stream`, and the
/// record that it was written: `"\n"` between blocks, `""` above the first
/// and between the rows of one list.
///
/// ```text
/// before(Err, Row("steps")); before(Err, Row("steps")); before(Err, Block)
///   ->  ""                    ""                         "\n"
/// ```
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
pub(crate) fn before(stream: Stream, kind: Kind) -> &'static str {
    with_last(|last| {
        let slot = &mut last[slot(stream)];
        let separator = match (*slot, kind) {
            (None, _) => "",
            (Some(Kind::Row(previous)), Kind::Row(current)) if previous == current => "",
            _ => "\n",
        };
        *slot = Some(kind);
        separator
    })
}

/// Record that something of `kind` was written without asking for a
/// separator — a line another library drew, such as a prompt's answer.
pub(crate) fn mark(stream: Stream, kind: Kind) {
    with_last(|last| last[slot(stream)] = Some(kind));
}
