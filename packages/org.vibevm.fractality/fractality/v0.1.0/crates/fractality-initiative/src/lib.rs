//! The initiative engine (Campaign 2, plan D1/D5/D6/D7).
//!
//! Pure policy and rendering over mission-control facts: the engine
//! takes DTOs the bus already serves (I3 — no shadow accounting) plus
//! explicit timestamps, and returns strings and decisions. No I/O, no
//! clock reads, no state of its own — everything is a function, so
//! every behavior is a plain unit test. The CLI verbs and the harness
//! hooks are thin shells around this crate.

pub mod scoreboard;

pub use scoreboard::{month_web_calls, render_board, render_line};
