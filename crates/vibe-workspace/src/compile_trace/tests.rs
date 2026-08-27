//! Reds for the durable trace run, split by the law each group holds.
//!
//! Every one of them drives the REAL built-in compiler schedule through
//! `compile_artifact_traced`, reads the result back off disk through the
//! generated type, and holds it to the epoch's own relational validator. No
//! test asserts an elapsed value and no production path here reads a clock:
//! instants are arguments.

mod attempts;
mod concurrency;
mod hardening;
mod lifecycle;
mod names;
mod publication;
mod retention;
mod serialized;
mod support;
