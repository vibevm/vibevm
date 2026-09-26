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

use vibe_specdoc::doc::{Block, DerivedKind};

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

impl ExampleBody {
    /// The body an AUTHORED example lends, and `None` for every other
    /// block — a borrowed `example ref` included, because it holds an
    /// address and no body of its own.
    ///
    /// This is the ONE conversion from the pivot's block to this bundle,
    /// and it lives here because both ends of a borrowing depend on it
    /// agreeing with itself: a backend renders an authored example
    /// through it, and [`crate::translations::borrowed`] fills the
    /// bundle a borrowing page resolves against with it. Two spellings
    /// would part the day the genre grew a sixth field, and then one
    /// page would show a `stderr` the other dropped.
    ///
    /// ```
    /// use vibe_doc::content::ExampleBody;
    /// use vibe_specdoc::doc::Block;
    ///
    /// let doc = vibe_specdoc::from_xml_with(
    ///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
    ///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
    ///        <title id=\"root\">Hello</title>\n\
    ///        <example id=\"version\" fixture=\"none\">\n\
    ///          <run>vibe --version</run>\n\
    ///          <expect>vibe 1.0.0</expect>\n\
    ///        </example>\n\
    ///      </spec>\n",
    ///     vibe_specdoc::Vocabulary::Doc,
    /// )
    /// .unwrap();
    ///
    /// let body = ExampleBody::of(&doc.preamble[0].block).expect("the page authors one");
    /// assert_eq!(body.run, "vibe --version");
    /// assert_eq!(body.expect, "vibe 1.0.0");
    ///
    /// // A reference lends nothing: it names an example, it is not one.
    /// let borrowed = Block::ExampleRef {
    ///     id: "version".to_owned(),
    /// };
    /// assert!(ExampleBody::of(&borrowed).is_none());
    /// ```
    pub fn of(block: &Block) -> Option<ExampleBody> {
        match block {
            Block::Example {
                lang,
                exit,
                run,
                expect,
                stderr,
                ..
            } => Some(ExampleBody {
                lang: lang.clone(),
                exit: *exit,
                run: run.clone(),
                expect: expect.clone(),
                stderr: stderr.clone(),
            }),
            _ => None,
        }
    }
}

/// Everything the backends need that is not in the document.
#[derive(Debug, Clone, Default)]
pub struct Content {
    /// Cited rules by address, as [`crate::citations::resolve_rules`]
    /// produced them.
    pub rules: BTreeMap<String, RuleText>,
    /// Generated `derived` text by `<kind>:<reference>`.
    pub derived: BTreeMap<String, String>,
    /// Example bodies for a translation's `example ref`, by the
    /// BORROWING page's address in its own package (a [`Page::rel`], e.g.
    /// `start/what-vibevm-is.xml`) and then by example id.
    ///
    /// Two levels rather than one, because an example id is unique on its
    /// PAGE and nowhere wider: the runner addresses an example as
    /// `page#id`, the mirror check compares a reference against the
    /// same-path source page, and an English manual reuses an id like
    /// `tree` across half a dozen chapters. A map keyed by bare id would
    /// hand one page the command another page authored, and report no gap
    /// while doing it.
    ///
    /// The page address is the borrowing page's own and not the source's,
    /// which costs nothing and says something: a translation mirrors its
    /// source file for file (`##LOC-MIRROR`), so the two addresses are
    /// equal by law, and keying on the borrower's means a renderer needs
    /// to know only the page it is rendering.
    ///
    /// [`Page::rel`]: crate::pages::Page::rel
    pub examples: BTreeMap<String, BTreeMap<String, ExampleBody>>,
    /// The base a citation's link is built on. Empty means «no links»:
    /// the island then carries the address in `data-uri` and no `href`,
    /// which is what a projection with nowhere to point should do.
    pub base: String,
    /// The language the EDITION being rendered is written in — the
    /// package's `[i18n].canonical` (PROP-057 `##LOC-LANGUAGE-FIELD`).
    ///
    /// It is here beside the base because it is a property of the RENDER
    /// and not of the document: one package is one language, and what a
    /// page needs to know is which edition it is a page of. The island
    /// spends it on the one sentence it writes in its own voice rather
    /// than quoting — the line a folded rule shows when the rule's own
    /// words cannot describe it (`##READER-RULE-FOLDED`). Empty means the
    /// project default; [`Content::edition_lang`] is where that is
    /// decided.
    pub lang: String,
}

impl Content {
    /// An empty bundle on the site's own base — the shape a caller uses
    /// when it wants the structure of a page and none of the fetched
    /// text.
    pub fn new() -> Content {
        Content {
            base: SITE_BASE.to_owned(),
            lang: vibe_core::manifest::i18n::DEFAULT_CANONICAL_LANGUAGE.to_owned(),
            ..Content::default()
        }
    }

    /// The same bundle served from another base.
    #[must_use]
    pub fn with_base(mut self, base: impl Into<String>) -> Content {
        self.base = base.into();
        self
    }

    /// The same bundle rendering another edition's language.
    #[must_use]
    pub fn with_lang(mut self, lang: impl Into<String>) -> Content {
        self.lang = lang.into();
        self
    }

    /// The edition's language, with the project default standing in for a
    /// bundle that named none.
    ///
    /// A default rather than a refusal: an unnamed language is the common
    /// case in a caller that wants the shape of a page and none of its
    /// fetched text, and a renderer's job is to render (see this module's
    /// head).
    ///
    /// ```
    /// use vibe_doc::content::Content;
    ///
    /// assert_eq!(Content::new().edition_lang(), "en");
    /// assert_eq!(Content::new().with_lang("ru").edition_lang(), "ru");
    /// assert_eq!(Content::default().edition_lang(), "en");
    /// ```
    pub fn edition_lang(&self) -> &str {
        match self.lang.is_empty() {
            true => vibe_core::manifest::i18n::DEFAULT_CANONICAL_LANGUAGE,
            false => &self.lang,
        }
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

    /// The body one borrowed example resolves to, when this build read the
    /// source page it is borrowed from.
    ///
    /// `page` is the BORROWING page's address in its own package. It is a
    /// parameter rather than an ambient lookup for the reason the two
    /// levels of [`Content::examples`] exist: an id alone does not name an
    /// example, and a function that accepted one would be a function that
    /// guessed.
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use vibe_doc::content::{Content, ExampleBody};
    ///
    /// let body = ExampleBody {
    ///     run: "vibe --version".to_owned(),
    ///     expect: "vibe 1.0.0".to_owned(),
    ///     ..ExampleBody::default()
    /// };
    /// let content = Content {
    ///     examples: BTreeMap::from([(
    ///         "start/what-vibevm-is.xml".to_owned(),
    ///         BTreeMap::from([("version".to_owned(), body)]),
    ///     )]),
    ///     ..Content::new()
    /// };
    ///
    /// assert_eq!(
    ///     content
    ///         .example("start/what-vibevm-is.xml", "version")
    ///         .map(|b| b.run.as_str()),
    ///     Some("vibe --version")
    /// );
    /// // The same id on another page is another example, or none at all.
    /// assert!(content.example("start/first-project.xml", "version").is_none());
    /// ```
    pub fn example(&self, page: &str, id: &str) -> Option<&ExampleBody> {
        self.examples.get(page)?.get(id)
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
