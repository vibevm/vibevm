//! Reds for the command-level trace owner, split by the law each group holds.
//!
//! Every instant is injected and every clock counts its own calls, so "the
//! disabled path never asked what time it is" is an assertion rather than a
//! hope. Nothing here measures a duration.

mod bounds;
mod funnel;
mod session;
mod support;
