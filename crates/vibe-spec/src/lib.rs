//! `vibe-spec` — `spec://` addressing, the document IR, and the deterministic
//! router.
//!
//! This crate implements the resolution layer of PROP-035 (the spec compiler):
//! a `spec://` address is parsed here (§6) and resolved into a node of a
//! document's hierarchical IR (§5). It is a **read-only** consumer of the spec
//! corpus — it never mutates authored files.
//!
//! It deliberately does **not** reuse the vendored `specmark-grammar` parser:
//! that parser rejects both the optional `@version` and the dotted tree-path
//! anchor this grammar introduces, and it is a sync-engines–gated snapshot that
//! must not be edited from the host tree. The flat-anchor kebab rule is
//! reproduced here segment-by-segment so a plain `spec://pkg/doc#anchor` parses
//! byte-identically to the legacy engine.
//!
//! Today the crate carries the address grammar; the document IR and the router
//! land on top of it in following slices.

mod address;
mod doctree;

pub use address::{Authority, SpecAddress, SpecAddressError};
pub use doctree::{DocTree, Node, NodeId};
