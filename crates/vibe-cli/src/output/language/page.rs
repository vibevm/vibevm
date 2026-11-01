//! The blank line between blocks (PROP-067 `#block`, `#list`): exactly one
//! between two blocks, none above the first, none between the rows of one
//! list.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#block");

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
static LAST: Mutex<[Option<Kind>; 2]> = Mutex::new([None, None]);

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
    let Ok(mut last) = LAST.lock() else {
        return "";
    };
    let slot = &mut last[slot(stream)];
    let separator = match (*slot, kind) {
        (None, _) => "",
        (Some(Kind::Row(previous)), Kind::Row(current)) if previous == current => "",
        _ => "\n",
    };
    *slot = Some(kind);
    separator
}

/// Record that something of `kind` was written without asking for a
/// separator — a line another library drew, such as a prompt's answer.
pub(crate) fn mark(stream: Stream, kind: Kind) {
    if let Ok(mut last) = LAST.lock() {
        last[slot(stream)] = Some(kind);
    }
}
