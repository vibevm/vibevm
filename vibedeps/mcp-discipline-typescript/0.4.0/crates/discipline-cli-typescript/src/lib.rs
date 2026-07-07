//! `discipline-typescript` — the TypeScript umbrella tool, full
//! subcommand parity with `discipline-rust` (DEFERRALS-CLOSEOUT
//! Phase 4, owner quality bar): init / floor / conform / specmap /
//! trace / test-gate / tripwire / health / fast-loop / codemod.
//!
//! The engines are the shared neutral crates (conform-core,
//! specmap-core — testgate and tripwire evaluation are language-free);
//! what is TypeScript-shaped here is the tooling surface: the
//! seven-step floor over the npm toolchain, the TAP parser feeding the
//! xfail-strict gate, the health collector over ts-tsc facts, the
//! per-cell fast loop over `node --test`, and the add-cell codemod.

pub const DEFAULT_TESTS_BASELINE: &str = "discipline/registry/tests-baseline.json";
pub const DEFAULT_DEBT_REGISTRY: &str = "discipline/registry/debt.json";
pub const DEFAULT_INTENT_REGISTRY: &str = "discipline/registry/intent.json";
pub const DEFAULT_HEALTH_OUT: &str = "discipline/health/latest-typescript.json";

mod codemod;
mod fast_loop;
mod floor;
mod health;
mod init;
mod tap;
mod test_gate;
mod tools;
mod trace;
mod tripwire;

pub use codemod::run_codemod_add_cell;
pub use fast_loop::run_fast_loop;
pub use floor::{FloorOptions, run_floor};
pub use health::run_health;
pub use init::{InitOptions, run_init};
pub use tap::parse_tap_output;
pub use test_gate::run_test_gate;
pub use trace::run_trace_explain;
pub use tripwire::run_tripwire;
