//! The lifecycle world `vibe update` runs its slot rows over.
//!
//! The run METADATA is not built here any more: it belongs to the command's
//! one prepared epoch ([`super::prepare`]), which selects exactly one identity
//! and hands the same value to the trace owner, the slot lifecycle and every
//! continuation. A helper that selected an identity of its own — as this cell
//! used to — was a second selector: it ran later, could allocate a second run
//! directory, and had no way to know the effective trace bit the command had
//! already committed to.

use vibe_lifecycle::process::StreamMode;

use crate::output;

pub(super) fn stream_mode(ctx: &output::Context) -> StreamMode {
    if ctx.is_json() {
        StreamMode::Capture
    } else if ctx.suppresses_output() {
        StreamMode::Null
    } else {
        StreamMode::Inherit
    }
}
