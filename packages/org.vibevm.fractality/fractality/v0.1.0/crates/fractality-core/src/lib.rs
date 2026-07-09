//! Core model of the fractality agent operating system.
//!
//! Everything that must mean the same thing to the mission-control daemon,
//! the pod supervisor, the CLI, and worker backends lives here: identifiers,
//! the run-state machine, the task packet (the universal seam, plan D7),
//! journal events and their replay fold (D9), API DTOs for the client and
//! pod legs of the mission-control bus (D10), claim-check file references
//! and node/scope identity (D19), and the `WorkerBackend` trait.
//!
//! This crate is deliberately dependency-light (serde + toml + ulid +
//! camino); anything that talks to the OS or the network belongs to the
//! daemon, pod, or client crates.
//!
//! Canonical spec: `fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`
//! (Decisions D1–D19) and `spec/PROP-001-foundation.md` (invariants I1–I7).

pub mod api;
pub mod error;
pub mod fileref;
pub mod ids;
pub mod journal;
pub mod node;
pub mod packet;
pub mod run;
pub mod time;
pub mod worker;

pub use error::CoreError;
pub use ids::{PodId, RunId, ScopeId};
pub use packet::{Packet, WorkspaceMode};
pub use run::{KillReason, RunRecord, RunState, UsageTotals};
pub use worker::{RunContext, WorkerBackend, WorkerSpec};
