//! One page, by address, for a machine (PROP-057 `##OBS-AUDIENCE-AGENT`,
//! the vision's §7.3 agent endpoints).
//!
//! The web serves a page at `/doc/<group>/<name>/<version>/<document>/`
//! and its projections beside it as files; an agent reading over MCP has
//! no HTTP at all, and asks for the same page by the `spec://` address it
//! would cite. That is the whole of this module: one address in, one
//! projection out, resolved through the same four sources a citation
//! resolves through, so «which copy did I read» has an answer.
//!
//! ## Why no HTML
//!
//! The island is for a reader with a browser. An agent paying by the
//! token wants either the Markdown — every cited rule's CURRENT text
//! substituted in, so the page is self-contained — or the dialect XML,
//! which keeps the addresses so the agent resolves them itself and
//! spends nothing on rules it will not read.
//!
//! ## Why a language is a fallback and not a switch
//!
//! One documentation package is one language (`##LOC-LANGUAGE-FIELD`).
//! So `lang` cannot select inside a package: it names the adaptation
//! published as `<name>-<lang>` (`##LOC-OFFICIAL-TRANSLATION`), and when
//! that package is not on this machine the source answers and says which
//! language it is. The site does the same thing for the same reason
//! (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`): a reader who asked for
//! Russian and got a missing page is worse served than one who got
//! English and was told so.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-AUDIENCE-AGENT");

use std::collections::BTreeMap;

use vibe_spec::{Authority, SpecAddress};

use crate::build::{self, Format};
use crate::citations::{self, Source, SpecSources};
use crate::content::Content;
use crate::error::{DocError, Result};
use crate::pages::{Page, PageSet};

/// One page, fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedPage {
    /// The address that was resolved — the caller's, or the adaptation's
    /// when a language moved it.
    pub address: String,
    /// The language the returned page is actually written in.
    pub lang: String,
    /// Which of the four sources answered.
    pub source: Source,
    /// The projection.
    pub text: String,
}

/// Read the page `address` names, in `format`.
///
/// `lang`, when given, asks for the adaptation first and falls back to
/// the address as written.
///
/// ```no_run
/// use vibe_doc::agent::read_page;
/// use vibe_doc::build::Format;
/// use vibe_doc::citations::SpecSources;
///
/// let world = SpecSources::for_checkout("/repo", Some("org.vibevm.core"), "vibevm");
/// let page = read_page(
///     "spec://org.vibevm.core/vibevm-docs/model/boot-lane",
///     Format::Md,
///     None,
///     &world,
/// )?;
/// assert!(page.text.contains("[p01]"));
/// # Ok::<(), vibe_doc::DocError>(())
/// ```
pub fn read_page(
    address: &str,
    format: Format,
    lang: Option<&str>,
    sources: &SpecSources,
) -> Result<FetchedPage> {
    let asked = parse(address)?;
    // The adaptation first when a language was asked for. A miss is not
    // a failure: it is the state the site calls a fallback, and the
    // answer says which language came back.
    if let Some(lang) = lang
        && let Some(adapted) = adaptation_of(address, lang)
        && let Ok(parsed) = parse(&adapted)
        && let Ok(page) = fetch(&adapted, &parsed, format, sources)
    {
        return Ok(page);
    }
    fetch(address, &asked, format, sources)
}

/// Parse an address and drop its anchor: this returns a whole page, and
/// a fragment that looked like a selector would be a promise nothing
/// here keeps.
fn parse(address: &str) -> Result<SpecAddress> {
    SpecAddress::parse(address).map_err(|e| DocError::Citation {
        page: String::new(),
        line: 0,
        uri: address.to_owned(),
        message: e.to_string(),
    })
}

/// `spec://<group>/<name>/…` → `spec://<group>/<name>-<lang>/…`.
///
/// Spelled on the ADDRESS rather than on the parsed value because the
/// naming convention is about the coordinate's text, and rebuilding an
/// address from its parts would have to reproduce every optional piece
/// the grammar allows.
fn adaptation_of(address: &str, lang: &str) -> Option<String> {
    let rest = address.strip_prefix("spec://")?;
    let (group, after) = rest.split_once('/')?;
    let (name, tail) = after.split_once('/')?;
    // A name already carrying the tag is the adaptation; asking for it
    // again would name `<name>-ru-ru`.
    if name.ends_with(&format!("-{lang}")) {
        return None;
    }
    // The version pin travels with the name, and the tag goes before it.
    let (name, version) = match name.split_once('@') {
        Some((name, version)) => (name, format!("@{version}")),
        None => (name, String::new()),
    };
    Some(format!("spec://{group}/{name}-{lang}{version}/{tail}"))
}

/// Locate one page and project it.
fn fetch(
    address: &str,
    parsed: &SpecAddress,
    format: Format,
    sources: &SpecSources,
) -> Result<FetchedPage> {
    let located = sources
        .locate(parsed)
        .map_err(|message| DocError::Citation {
            page: String::new(),
            line: 0,
            uri: address.to_owned(),
            message,
        })?;
    let raw = std::fs::read_to_string(&located.path)
        .map_err(|e| DocError::io("reading", &located.path, e))?;
    let doc =
        vibe_specdoc::from_xml_with(&raw, vibe_specdoc::doc::Vocabulary::Doc).map_err(|e| {
            DocError::Page {
                path: located.path.clone(),
                message: e.to_string(),
            }
        })?;
    // The address names a document, the package holds a file: the
    // extension is the pivot's serialisation and not part of the address
    // (PROP-052's layout owns it), so it is put back here.
    let rel = match parsed.doc_path.ends_with(".xml") {
        true => parsed.doc_path.clone(),
        false => format!("{}.xml", parsed.doc_path),
    };
    // When the page belongs to a translation, the examples it borrows come
    // from the source page at the SAME address. The package root is found
    // the way `locate` found the file — through the chain, by the
    // authority's coordinate — so the two cannot disagree about which
    // instance answered. One page is read, not the source package.
    let examples = borrowed_by(&rel, parsed, sources);
    let page = Page {
        rel: rel.clone(),
        path: located.path.clone(),
        doc,
    };

    // Only this page's citations are resolved. A manual cites hundreds of
    // rules, and resolving the corpus to answer for one page would make
    // the cheap question expensive.
    let one = PageSet {
        pages: vec![page.clone()],
        unreadable: Vec::new(),
    };
    let content = Content {
        rules: citations::resolve_rules(&one, sources),
        derived: BTreeMap::new(),
        examples: BTreeMap::from([(rel, examples)]),
        base: crate::content::SITE_BASE.to_owned(),
        // The language `locate` answered with: the edition this page
        // belongs to is the instance that held it, and a second read of
        // its manifest could only disagree (`##LOC-LANGUAGE-FIELD`).
        lang: located.lang.clone(),
    };
    Ok(FetchedPage {
        address: address.to_owned(),
        lang: located.lang,
        source: located.source,
        text: build::render_page(&page, &content, format),
    })
}

/// The example bodies the page at `rel` borrows, or none at all.
///
/// An undotted authority names no package, so it names no root to read a
/// source from, and an address the chain cannot place lends nothing —
/// both answer with an empty map rather than a refusal, because a
/// reference this run could not fill renders as the marked gap it is
/// ([`crate::translations::borrowed_by`]).
fn borrowed_by(
    rel: &str,
    parsed: &SpecAddress,
    sources: &SpecSources,
) -> BTreeMap<String, crate::content::ExampleBody> {
    let Authority::Package { group, name, .. } = &parsed.authority else {
        return BTreeMap::new();
    };
    sources
        .instance_of(group, name)
        .map(|instance| crate::translations::borrowed_by(&instance.root, rel, sources))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
