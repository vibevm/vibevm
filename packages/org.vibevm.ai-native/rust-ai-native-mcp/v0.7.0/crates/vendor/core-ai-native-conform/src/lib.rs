//! `conform-core` — the cross-language conformance engine core
//! (ENGINE-CONFORM v0.1; PLAYBOOK Phase 4).
//!
//! - [`Fact`] — the language-neutral normalized fact model (§3); the
//!   ledger's "facts class" instantiated.
//! - [`Store`] — content-addressed fact cache keyed by
//!   `(file content-hash, producer id+version)` (§3): facts never rot
//!   semantically; a 1-file diff re-extracts 1 file. Lives under
//!   `target/conform/` — derived data with a deterministic producer,
//!   never committed.
//! - [`Rule`] — rules as compiled queries over facts (§4); the
//!   declarative DSL is deliberately deferred (Open Question 2).
//! - [`sarif`] — byte-stable SARIF 2.1.0 rendering (§5): same inputs,
//!   byte-identical output, tested by double-run diff.
//! - [`baseline`] — the ratchet baseline (`conform-baseline.json`):
//!   pre-existing findings frozen per scope, new ones fail the gate,
//!   the file only shrinks.
//!
//! Frontier behaviour (B5, no cliffs): facts are extracted for the
//! whole workspace; **findings** are reported only inside the gate's
//! `--scope`.

mod config;
mod facts;
mod finding;
mod store;

pub mod baseline;
pub mod rules;
pub mod sarif;

pub use config::{Config, ConfigOrigin, ExemptEntry, FloorDisable, GoConfig, TsConfig};
pub use facts::{Fact, Frontend, SourceFacts};
pub use finding::{Finding, Rule, check, count_by_rule};
pub use store::{ExtractionLog, Store, content_hash, sort_source_facts};
