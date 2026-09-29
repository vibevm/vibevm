//! `rust-ai-native-cli` — the umbrella library behind the `rust-ai-native`
//! binary: bootstrap (`init`), the portable verification floor (`floor`),
//! and the sweep/brownfield drivers (`test-gate`, `tripwire`, `trace`,
//! `health`, `fast-loop`, `codemod`) over the engines the sibling crates
//! ship (`rust-ai-native-conform`, `rust-ai-native-specmap`).
//!
//! Everything is `root: &Path`-parameterised — no dev-repo assumption
//! anywhere: the same functions run from the shipped binary in a consumer
//! project and from a project-local wrapper (a dev repo's task runner).
//! Default artefact paths follow the shipped convention
//! (`discipline/registry/*`, `discipline/health/latest.json`); every one is
//! overridable by flag.

pub mod codemod;
pub mod fast_loop;
pub mod floor;
pub mod health;
pub mod init;
pub mod ledger;
pub mod test_gate;
pub mod trace;
pub mod tripwire;

/// The shipped default path of the xfail-strict test baseline
/// (BROWNFIELD §3–4).
pub const DEFAULT_TESTS_BASELINE: &str = "discipline/registry/tests-baseline.json";
/// The shipped default path of the debt registry (BROWNFIELD §3).
pub const DEFAULT_DEBT_REGISTRY: &str = "discipline/registry/debt.json";
/// The shipped default path of the intent registry (BROWNFIELD §3).
pub const DEFAULT_INTENT_REGISTRY: &str = "discipline/registry/intent.json";
/// The shipped default path of the health snapshot (Sweep Playbook §2).
pub const DEFAULT_HEALTH_OUT: &str = "discipline/health/latest.json";
/// The conform ratchet baseline, at the project root next to conform.toml.
pub const DEFAULT_CONFORM_BASELINE: &str = "conform-baseline.json";

pub use codemod::run_codemod_add_cell;
pub use fast_loop::run_fast_loop;
pub use floor::{FloorOptions, run_floor};
pub use health::run_health;
pub use init::{InitOptions, run_init};
pub use ledger::run_ledger_render;
pub use test_gate::run_test_gate;
pub use trace::run_trace_explain;
pub use tripwire::run_tripwire;
