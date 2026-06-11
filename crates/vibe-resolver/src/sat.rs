//! `Sat` — the `sat` DepSolver cell.
//!
//! Scaffolded by `cargo xtask codemod add-cell`; the seam
//! implementation is the author's next edit. The `#[cell]`
//! manifest and the REQ edge are present from birth so the
//! selection registry and the specmap see the cell
//! immediately.

use specmark::{cell, spec};

#[cell(seam = "DepSolver", variant = "sat")]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade")]
pub struct Sat;
