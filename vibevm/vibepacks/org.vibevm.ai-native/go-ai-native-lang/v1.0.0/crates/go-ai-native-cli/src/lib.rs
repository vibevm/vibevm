//! `go-ai-native` — the Go umbrella tool, full subcommand parity with
//! `rust-ai-native` / `typescript-ai-native`: init / floor / conform /
//! specmap / trace / test-gate / tripwire / health / fast-loop /
//! codemod.
//!
//! The engines are the shared neutral crates (conform-core,
//! specmap-core — testgate and tripwire evaluation are language-free);
//! what is Go-shaped here is the tooling surface: the seven-step floor
//! over the go toolchain, the `go test -json` parser feeding the
//! xfail-strict gate, the health collector over go-extract facts
//! (including the package-grain Example-coverage join), the per-cell
//! fast loop over per-package `go test`, and the add-cell codemod.

specmark::scope!("spec://go-ai-native-lang/go/GUIDE-AI-NATIVE-GO#wiring");

pub const DEFAULT_TESTS_BASELINE: &str = "discipline/registry/tests-baseline.json";
pub const DEFAULT_DEBT_REGISTRY: &str = "discipline/registry/debt.json";
pub const DEFAULT_INTENT_REGISTRY: &str = "discipline/registry/intent.json";
pub const DEFAULT_HEALTH_OUT: &str = "discipline/health/latest-go.json";

mod codemod;
mod fast_loop;
mod floor;
mod gotest;
mod health;
mod init;
mod test_gate;
mod tools;
mod trace;
mod tripwire;

pub use codemod::run_codemod_add_cell;
pub use fast_loop::run_fast_loop;
pub use floor::{FloorOptions, run_floor};
pub use gotest::parse_gotest_json;
pub use health::run_health;
pub use init::{InitOptions, run_init};
pub use test_gate::run_test_gate;
pub use trace::run_trace_explain;
pub use tripwire::run_tripwire;
