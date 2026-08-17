//! `vibe-wire` — generated Rust types for vibevm wire contracts.
//!
//! Every type under [`generated`] is **machine-generated** from a JTD
//! schema in [`schemas/`](../../../schemas/) at the repo root. Source of truth lives
//! there; this crate carries the codegen output after our transformation
//! layer in `xtask/src/codegen/postproc.rs` (boxing union arms, renaming
//! field identifiers to snake_case while dropping the identity renames,
//! rewriting map fields to canonically ordered `BTreeMap`s, collapsing
//! optional collections per the schema's `x-empty`, lifting the `Box`
//! off optional scalars and structures per the schema's `x-default`,
//! stamping `#[serde(deny_unknown_fields)]` on the structs of formats
//! the registry marks `foreign_parsers = "none"`, binding the domain
//! Rust types the schema's `x-rust-type` names — an alias's right side
//! or a type's name, whichever the definition's form makes it, together
//! with the import items such a substitution leaves with no user —
//! opening vocabularies per the schema's `x-vocabulary`) — the files are
//! still never hand-edited. `cargo
//! xtask codegen` regenerates; `cargo xtask check-codegen` asserts no
//! drift (CI runs the latter). Per PROP-000 §16, JTD + codegen is the
//! standing pattern for wire contracts in this project.
//!
//! One more layer phase runs after those passes, and it explains the
//! `pub use` lines the generated files carry: the vocabulary fragments
//! a schema pulls in (`metadata.x-vocabularies`, home
//! `formats/vocabularies.json`) are emitted ONCE, into
//! [`generated::shared`], and every schema module that pulls a fragment
//! re-exports its type instead of redeclaring it — byte-checked against
//! the shared block, swapped in place by
//! `xtask/src/codegen/shared_module.rs`. A `pub use` inside a generated
//! file is therefore machine-written like everything around it, not a
//! hand edit; one type per name (`a::VersionEntry` and
//! `b::VersionEntry` were distinct types before this phase, identical
//! bytes and all).
//!
//! See [`tools/jtd-codegen/README.md`](../../../tools/jtd-codegen/README.md) for the
//! generator install procedure and pinned version.
//!
//! Migration of existing hand-written `Serialize` structs to
//! JTD-derived types lands incrementally — `vibe init --json` was the
//! first consumer.
//!
//! # Every generated type here is permissive, and that is not a decision
//!
//! None of these types carries `#[serde(deny_unknown_fields)]`: an
//! unexpected field on the wire is silently ignored. The hand-written
//! types in this project do the opposite — strictness is the house style
//! there, in roughly 63 places — so one class of type follows one policy
//! and the other class follows the reverse, and until 2026-08-06 nobody
//! had chosen either.
//!
//! It is written down because the state is easy to misread as
//! deliberate. It is not: **the generator cannot emit it.** Measured
//! 2026-08-06 — no key in any of our schemas controls it; JTD's own
//! `additionalProperties` works the *opposite* way (setting it OPENS the
//! form); and it is validation semantics rather than a promise about
//! generated Rust. Of the workarounds, a wrapper type does not work
//! (strictness is consumed where fields are declared) and a separate
//! `impl` file cannot work at all (it is a container attribute the derive
//! consumes at the definition site). Only post-processing the generator's
//! output could.
//!
//! There is a real argument for keeping it permissive, and the owner's
//! ruling of 2026-08-06 rests on it: an index record is read from a
//! foreign registry, possibly written by a newer tool, and strictness
//! there means a new field breaks old clients. For a format that arrives
//! from outside, permissiveness is forward compatibility. The point of
//! this note is that softness becomes a deliberate choice rather than an
//! accident of tooling.
//!
//! The mechanism the note said was missing now exists: the codegen
//! post-processing stamps `#[serde(deny_unknown_fields)]` itself, and
//! whether it does is not a per-type choice but the format's registry
//! record — `foreign_parsers = "none"` in `formats/REGISTRY.toml` takes
//! the attribute on every generated struct of that format's output,
//! every other role keeps the permissive reading byte for byte. On
//! today's registry no format with a built schema carries the `none`
//! role, so the generated tree above is still entirely permissive —
//! now as the registry's verdict rather than the generator's limit.

#![forbid(unsafe_code)]

/// Generated wire types. Populated by `cargo xtask codegen` from
/// `*.jtd.json` schemas under `schemas/` at the repo root. Each
/// submodule corresponds to one schema; the top-level
/// `generated/mod.rs` is itself synthesised by the xtask and lists the
/// submodules in alphabetical order.
pub mod generated;
