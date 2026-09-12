//! # vibe-doc — the documentation pipeline (PROP-057 §10)
//!
//! Everything with content lives here, and every surface above — the CLI
//! `vibe doc …`, the MCP tools, the local HTTP reader — is a thin
//! projection of it (PROP-057 `##PIPE-LIBRARY`,
//! `##INV-LOGIC-IN-THE-LIBRARY`). The crate opens with the two halves the
//! documentation cannot lie without:
//!
//! * [`examples`] — the example runner. Every `<example>` on every page
//!   runs against the built binary in a fresh sandbox and its output is
//!   compared EXACTLY after the normalisation its fixture declares. There
//!   are no match templates: a line the product does not promise to keep
//!   stable is closed by a named `replace` rule that a reviewer can see
//!   (`##PIPE-EXAMPLE-RUNNER`). This is what makes
//!   `##INV-EXAMPLES-RUN` — «every example runs» — a fact rather than a
//!   hope.
//! * [`derived`] — the `derived` generators. Command help, schema field
//!   tables and manifest fields are produced at build time and never
//!   committed as page text, because stored derived text is how
//!   documentation starts lying (`##PIPE-DERIVED`,
//!   `##INV-DERIVED-NEVER-HAND-KEPT`).
//!
//! * [`citations`] — the `rule` resolver. A page names an address and the
//!   pipeline puts the fact's CURRENT text on it at every render, so a
//!   quoted rule has no stored copy that could go stale. The check asks
//!   one question — does the anchor exist — and the `documents` edge it
//!   mints carries no pin, so a citation can never go «suspect»
//!   (`##OBS-RULE-EDGE-UNPINNED`).
//!
//! Below all three sits [`pages`], the one place that reads a
//! documentation package: the pivot's documentation vocabulary is a
//! parameter of the READER, and choosing it from the package's kind is
//! precisely the job the pivot refuses to do for separability (PROP-045
//! `##DOC-VOCAB-BY-KIND`).
//!
//! ## What this crate will not do
//!
//! It runs no shell. The runner dispatches a closed, named set of
//! programs, so a command a page shows is a command a reader can type;
//! a page that needs something else is a defect of the page, reported by
//! name. It writes nothing outside its sandbox — the real `~/.vibe` and
//! the source tree are hashed before and after every run, and a change is
//! fatal. And it never loosens a comparison to turn a check green: the
//! normalisation is declared, or the product is fixed.
//!
//! ```
//! use std::fs;
//! let tmp = tempfile::tempdir().unwrap();
//! let dir = tmp.path().join("vibevm/vibespecs");
//! fs::create_dir_all(&dir).unwrap();
//! fs::write(
//!     dir.join("page.xml"),
//!     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
//!      <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
//!        <title id=\"root\">Page</title>\n  \
//!        <derived kind=\"cli-help\" ref=\"vibe list --help\"/>\n\
//!      </spec>\n",
//! )
//! .unwrap();
//!
//! // One walk, one vocabulary decision, both halves of the result.
//! let set = vibe_doc::pages::read_package(tmp.path()).unwrap();
//! assert_eq!(set.total(), 1);
//! assert!(set.unreadable.is_empty());
//! ```

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

pub mod citations;
pub mod derived;
pub mod error;
pub mod examples;
pub mod pages;

pub use error::{DocError, Result};
