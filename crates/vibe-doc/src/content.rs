//! What a backend needs BESIDE the document (PROP-057 `##PIPE-LIBRARY`).
//!
//! Three of the documentation genre's blocks hold an address instead of a
//! text, on purpose: a `rule` names a fact in a specification, a `derived`
//! names a generator, and a translation's `example ref` names an example
//! on the source page. None of the three keeps a copy, because a stored
//! copy is correct on the day it is pasted and wrong on every day after,
//! and nothing in the repository can tell the two days apart.
//!
//! So the text arrives at render time, and this is the bundle it arrives
//! in. A backend renders the document and reads what it needs from here;
//! an entry that is missing is rendered as an honest gap — marked
//! `data-unresolved` — and never as silence. Which missing entries are a
//! BUILD FAILURE is a different question, answered by the checks
//! (`vibe doc check --citations`, `--derived`), not by a renderer: a
//! renderer that aborts tells an author about one defect per run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use std::collections::BTreeMap;

use vibe_specdoc::doc::DerivedKind;

use crate::citations::RuleText;

/// The site path documentation is mounted under
/// (PROP-057 `##SITE-MOUNT`). It is the default base of every citation
/// link the island carries, and a caller serving the pages elsewhere —
/// the local reader on `127.0.0.1`, a webview with its own base — passes
/// its own.
pub const SITE_BASE: &str = "/doc/";

/// The version segment an address without a pin resolves to
/// (`##SITE-MOUNT`).
pub const LATEST: &str = "latest";

/// One example's body, as a page carries it. A translation's
/// `example ref` borrows exactly this from the source page rather than
/// restating a command in another language — a command has no
/// translation (PROP-045 `##ROW-DOCVOCAB-EXAMPLE-REF`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExampleBody {
    pub lang: Option<String>,
    pub exit: Option<i32>,
    pub run: String,
    pub expect: String,
    pub stderr: Option<String>,
}

/// Everything the backends need that is not in the document.
#[derive(Debug, Clone, Default)]
pub struct Content {
    /// Cited rules by address, as [`crate::citations::resolve_rules`]
    /// produced them.
    pub rules: BTreeMap<String, RuleText>,
    /// Generated `derived` text by `<kind>:<reference>`.
    pub derived: BTreeMap<String, String>,
    /// Example bodies by id, for a translation's `example ref`.
    pub examples: BTreeMap<String, ExampleBody>,
    /// The base a citation's link is built on. Empty means «no links»:
    /// the island then carries the address in `data-uri` and no `href`,
    /// which is what a projection with nowhere to point should do.
    pub base: String,
}

impl Content {
    /// An empty bundle on the site's own base — the shape a caller uses
    /// when it wants the structure of a page and none of the fetched
    /// text.
    pub fn new() -> Content {
        Content {
            base: SITE_BASE.to_owned(),
            ..Content::default()
        }
    }

    /// The same bundle served from another base.
    #[must_use]
    pub fn with_base(mut self, base: impl Into<String>) -> Content {
        self.base = base.into();
        self
    }

    /// The key a `derived` block is stored under.
    pub fn derived_key(kind: DerivedKind, reference: &str) -> String {
        format!("{}:{reference}", kind.as_str())
    }

    /// The generated text of one `derived` block, when this build has it.
    pub fn derived_text(&self, kind: DerivedKind, reference: &str) -> Option<&str> {
        self.derived
            .get(&Content::derived_key(kind, reference))
            .map(String::as_str)
    }

    /// The link a citation points at, by the deterministic address map of
    /// PROP-057 `##SITE-MOUNT`:
    ///
    /// ```text
    /// spec://<group>/<name>@<version>/<document>#<anchor>
    ///   → <base><group>/<name>/<version>/<document>/#<anchor>
    /// spec://<group>/<name>/<document>#<anchor>
    ///   → <base><group>/<name>/latest/<document>/#<anchor>
    /// ```
    ///
    /// The map needs no index, which is why it can live in a pure
    /// function a backend calls while it renders.
    ///
    /// ```
    /// use vibe_doc::content::Content;
    ///
    /// let c = Content::new();
    /// assert_eq!(
    ///     c.link("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY").as_deref(),
    ///     Some("/doc/org.vibevm.core/vibevm/latest/common/PROP-057/#PIPE-LIBRARY")
    /// );
    /// assert_eq!(
    ///     c.link("spec://org.demo/lib@2.1.0/guide#X").as_deref(),
    ///     Some("/doc/org.demo/lib/2.1.0/guide/#X")
    /// );
    /// // With no base there is nowhere to point, and the island says so
    /// // by carrying the address alone.
    /// assert_eq!(Content::default().link("spec://org.demo/lib/guide#X"), None);
    /// ```
    pub fn link(&self, uri: &str) -> Option<String> {
        if self.base.is_empty() {
            return None;
        }
        let address = vibe_spec::SpecAddress::parse(uri).ok()?;
        let (group, name, version) = match &address.authority {
            vibe_spec::Authority::Package {
                group,
                name,
                version,
            } => (
                group.clone(),
                name.clone(),
                version.clone().unwrap_or_else(|| LATEST.to_owned()),
            ),
            // An undotted authority names no package, so it addresses no
            // page either — the citation keeps its `data-uri` and gets no
            // link rather than a link to somewhere invented.
            vibe_spec::Authority::Host(_) => return None,
        };
        let anchor = if address.anchor.is_empty() {
            String::new()
        } else {
            format!("#{}", address.anchor.join("."))
        };
        Some(format!(
            "{}{group}/{name}/{version}/{}/{anchor}",
            self.base, address.doc_path
        ))
    }
}

#[cfg(test)]
mod tests;
