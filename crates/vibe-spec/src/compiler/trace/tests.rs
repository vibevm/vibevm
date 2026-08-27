//! Focused reds for the R3.4 observer seam, split by semantic cell.
//!
//! Everything is asserted on VALUES — statuses, shapes, counts, decoded
//! carriers, byte equality — never on rendered log text, and no test looks at
//! an elapsed number: a timing assertion on a real clock is a flake, so only
//! the presence/absence structure the trace epoch legislates is checked.

mod defects;
mod observed;
mod refusals;
mod support;
