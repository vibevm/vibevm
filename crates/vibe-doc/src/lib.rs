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
//! * [`manifest`] — the page manifest, and with it the officiality a
//!   documentation holds over its subjects. It is computed at every
//!   build from the convergence of two edges and stored as a flag
//!   nowhere, because a flag is a second source of truth that vanishes
//!   in local mode (`##REL-NO-OFFICIAL-FLAG`). Everything machine-facing
//!   — the navigation, the `llms` files, `/doc/manifest.json` — is one
//!   fold over it rather than a second walk of the package.
//!
//! * [`coverage`] — the gate. A spec fact marked `actionstage="doc"`
//!   with an audience is a PROMISE that the documentation will tell that
//!   audience this rule; the gate asks, per audience, whether a page for
//!   those people cites it. It answers «is everything told», which is
//!   the question a table of contents cannot answer and a navigation
//!   must never be built from (`##OBS-COVERAGE-GATE`).
//!
//! * [`build`] — one walk, every artefact. The page in the projection
//!   the caller asked for, the manifest, the four `llms` tiers and the
//!   card's pictures, all folded out of ONE page set at the addresses
//!   `##SITE-MOUNT` fixes — so the site and the local reader write the
//!   same shapes from the same function.
//!
//! * [`agent`] — one page, by address, for a machine. The web serves a
//!   page at a path and its projections beside it; an agent reading over
//!   MCP has no HTTP at all and asks by the `spec://` address it would
//!   cite, through the same four sources (`##OBS-AUDIENCE-AGENT`).
//!
//! * [`media`] — the card's pictures. What an author ships is judged
//!   from its first bytes and published under a name taken from its
//!   content; what an author ships NOTHING for is computed from the hash
//!   of the coordinate and the glyph of the kind, so a package looks the
//!   same on the site and in the local reader and no file is stored for
//!   either (`##CARD-PLACEHOLDERS-GENERATED`, `##CARD-SITE-COPIES`).
//!
//! * [`llms`] — the corpus as a machine reads it: the index, the whole
//!   text, and two tiers cut to a token budget, all in the layer law's
//!   order — stable text before text that moves with the product
//!   (`##SEO-LLMS-FILES`, PROP-048 `##THE-LAYER-LAW`).
//!
//! * [`translations`] — the mirror check. An adaptation carries the same
//!   pages, the same anchors and the same blocks as the documentation it
//!   adapts, and borrows every example rather than authoring one. That
//!   is what lets the language selector keep a reader's place and a
//!   citation mean one thing in every language (`##LOC-MIRROR`). It asks
//!   about structure and nothing else: whether the prose still says what
//!   the source says is a human's question at a full reconciliation.
//!
//! * [`html`] — the island. A page's content as finished HTML, with no
//!   script, no style and no page furniture: the public site's shell and
//!   the local reader receive the same bytes, which is what keeps the two
//!   renders from drifting apart (`##PIPE-SHELL-PARSES-NOTHING`). What a
//!   backend needs beside the document — the cited rules, the generated
//!   `derived` text, a translation's borrowed examples — travels in
//!   [`content::Content`].
//!
//! * [`style`] — the mechanical half of the style law. Tics from a list
//!   the package ships per language, sentence and paragraph length by
//!   block kind, a glossary term used before anybody introduced it, three
//!   terms in one sentence, a heading the law forbids by name. It reports
//!   and never edits: prose is written by a person, and a linter that
//!   could rewrite a sentence is a linter authors write FOR
//!   (`##STYLE-LINT`).
//!
//! Below all four sits [`pages`], the one place that reads a
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

pub mod agent;
pub mod build;
pub mod citations;
pub mod content;
pub mod coverage;
pub mod derived;
pub mod error;
pub mod examples;
pub mod html;
pub mod llms;
pub mod manifest;
pub mod md;
pub mod media;
pub mod numbering;
pub mod pages;
pub mod style;
pub mod translations;
pub mod xml;

pub use error::{DocError, Result};
