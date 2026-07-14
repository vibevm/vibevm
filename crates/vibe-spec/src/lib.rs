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
//! The crate now carries the full router — the address grammar, the document
//! IR, and file resolution — plus the directive scanner (§7). The compilation
//! pipeline (§8) and link tables (§10) build on top of it next.

mod address;
mod directives;
mod doctree;
mod resolver;

pub use address::{Authority, SpecAddress, SpecAddressError};
pub use directives::{Directive, DirectiveError, DirectiveKind, Directives, InPlaceUse};
pub use doctree::{DocTree, Node, NodeId};
pub use resolver::{FileResolver, ResolveError};
