//! R3.4 traced-install reds: ONE borrowed trace run carried through package
//! units first and every node after, per the workspace substrate atom.
//!
//! Every test here injects the recorder and its timestamps directly — no CLI,
//! no lifecycle state, no report surface. Nothing asserts a measured
//! duration, and every index is read back off disk through the generated type
//! under the epoch's own relational validator.
//!
//! Split by the question each file answers, so no authored file approaches the
//! length budget:
//!
//! * [`support`] — the fixtures and on-disk readers every red shares;
//! * [`one_run`] — one run really does cover both scope kinds, in an order
//!   nothing hash-shaped can permute;
//! * [`attempts`] — occurrences across a run: fresh skips, reacquired pending
//!   work, and the fresh observation that refuses;
//! * [`refusals`] — everything that must NOT produce a recorded compile: a
//!   failing compiler, a pre-boot park, an empty lane, the untraced wrappers,
//!   and a planted declaration fault.

// This file is itself loaded through a `#[path]` module declaration, so its
// children are spelled explicitly rather than inherited from a directory.
#[path = "tests_trace/support.rs"]
mod support;

#[path = "tests_trace/attempts.rs"]
mod attempts;
#[path = "tests_trace/one_run.rs"]
mod one_run;
#[path = "tests_trace/refusals.rs"]
mod refusals;
