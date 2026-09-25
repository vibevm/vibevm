//! The bodies a translation's pages borrow from the source they mirror
//! (PROP-057 `##LOC-EXAMPLE-REF`, `##LOC-MIRROR`, PROP-045
//! `##ROW-DOCVOCAB-EXAMPLE-REF-MD`).
//!
//! A translation must not author examples. A command has no translation
//! and its output is checked ONCE, on the source, so an adaptation writes
//! `<example ref="<id>"/>` and the pipeline fills it with «the same
//! fences, copied from the source at projection time». This module is
//! that copying: it reads the source package a translation declares and
//! hands back the example bodies its pages author, ready for
//! [`crate::content::Content::examples`].
//!
//! ## By page, then by id
//!
//! An example id is unique on its PAGE and nowhere wider. The runner
//! addresses an example as `page#id`, the mirror check compares a
//! reference against the SAME-PATH source page, and a manual reuses an id
//! across chapters as a matter of course. So the answer is keyed by page
//! address and then by id, and a caller resolving a reference must say
//! which page it is rendering. A map keyed by bare id would hand one page
//! the command another page authored and report no gap while doing it.
//!
//! The address a body is stored under is the SOURCE page's, which is also
//! the borrowing page's: a mirror is file for file (`##LOC-MIRROR`), so
//! the two are equal by law, and a renderer needs to know only its own.
//!
//! ## Within one page, the first of a repeated id wins
//!
//! An example's id is a fact anchor, and the pivot refuses a page that
//! defines one twice — so a page this reads can carry no collision, and
//! the rule below is defence in depth rather than a live case. It is
//! stated anyway, because «which example does this id name» must have one
//! answer whatever reaches the walk: the FIRST in document order, by the
//! same walk the mirror check collects a page's ids with.
//!
//! ## Both functions are infallible, on purpose
//!
//! A package that adapts nothing, a `[translates]` that names no
//! `<group>/<name>`, a source no source holds, a source page that is
//! missing or will not parse — each yields no bodies for the pages it
//! affects, and the references then render as the marked gaps they are.
//! That is the renderer's contract, stated in [`crate::content`]: a
//! backend never aborts over a text it could not fetch, because a run
//! that aborts reports one defect per run. WHICH gap is a failure is the
//! checks' question — `vibe doc check --translations` is where an
//! unreachable source and an unknown example id are errors, with the
//! words an author can act on.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOC-EXAMPLE-REF");

use std::collections::BTreeMap;
use std::path::{Component, Path};

use vibe_specdoc::doc::{Block, SpecDoc};

use crate::citations::sources::SpecSources;
use crate::content::ExampleBody;
use crate::manifest;
use crate::pages;

/// Every example the source of the translation at `package_dir` authors,
/// by page address and then by id.
///
/// `Block::Example` only: a reference on the source page carries no body,
/// so there is nothing there to lend on. A source page that authors no
/// example contributes no entry, which keeps the bundle about examples
/// rather than about pages.
///
/// The whole source package is read, because a build renders every page
/// of the translation and would otherwise open the source once per page.
/// The per-request counterpart is [`borrowed_by`].
///
/// ```
/// use vibe_doc::citations::SpecSources;
/// use vibe_doc::translations::borrowed;
///
/// let tmp = tempfile::tempdir().unwrap();
/// let source = tmp.path().join("source");
/// let adaptation = tmp.path().join("adaptation");
/// std::fs::create_dir_all(source.join("vibevm/vibespecs/guide")).unwrap();
/// std::fs::create_dir_all(&adaptation).unwrap();
/// std::fs::write(
///     source.join("vibevm/vibespecs/guide/first.xml"),
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
///        <title id=\"root\">First</title>\n\
///        <example id=\"demo\" fixture=\"none\">\n\
///          <run>vibe --version</run>\n\
///          <expect>vibe 1.0.0</expect>\n\
///        </example>\n\
///      </spec>\n",
/// )
/// .unwrap();
/// std::fs::write(
///     adaptation.join("vibe.toml"),
///     "[translates]\npackage = \"com.example.docs/pair\"\nversion = \"^0.1\"\n",
/// )
/// .unwrap();
///
/// let world = SpecSources::for_checkout(&source, Some("com.example.docs"), "pair");
/// let found = borrowed(&adaptation, &world);
/// assert_eq!(found["guide/first.xml"]["demo"].run, "vibe --version");
///
/// // A world that reaches no source lends nothing, and says so by being
/// // empty rather than by refusing.
/// assert!(borrowed(&adaptation, &SpecSources::new()).is_empty());
/// ```
pub fn borrowed(
    package_dir: &Path,
    sources: &SpecSources,
) -> BTreeMap<String, BTreeMap<String, ExampleBody>> {
    let Ok(Some(adapted)) = super::adapted(package_dir, sources) else {
        return BTreeMap::new();
    };
    let Ok(set) = pages::read_package(&adapted.instance.root) else {
        return BTreeMap::new();
    };
    set.pages
        .iter()
        .map(|page| (page.rel.clone(), authored(&page.doc)))
        .filter(|(_, bodies)| !bodies.is_empty())
        .collect()
}

/// The same for ONE page, by its address inside the package.
///
/// Only `<source root>/vibevm/vibespecs/<page>` is read, so a reader
/// answering one request does not pay for the whole source package — a
/// manual has hundreds of pages and a view shows one.
///
/// `page` must be a page address: a `/`-separated path of ordinary names,
/// as a [`crate::pages::Page::rel`] is. Anything else — empty, absolute,
/// carrying a `.`, a `..`, a drive letter or a backslash — yields no
/// bodies rather than a path. The reader above already refuses such an
/// address; this refuses it again, because the one thing that must never
/// happen here is a caller's string becoming a read outside the source
/// tree.
///
/// ```
/// use vibe_doc::citations::SpecSources;
/// use vibe_doc::translations::borrowed_by;
///
/// let tmp = tempfile::tempdir().unwrap();
/// let source = tmp.path().join("source");
/// let adaptation = tmp.path().join("adaptation");
/// std::fs::create_dir_all(source.join("vibevm/vibespecs/guide")).unwrap();
/// std::fs::create_dir_all(&adaptation).unwrap();
/// std::fs::write(
///     source.join("vibevm/vibespecs/guide/first.xml"),
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
///        <title id=\"root\">First</title>\n\
///        <example id=\"demo\" fixture=\"none\">\n\
///          <run>vibe --version</run>\n\
///          <expect>vibe 1.0.0</expect>\n\
///        </example>\n\
///      </spec>\n",
/// )
/// .unwrap();
/// std::fs::write(
///     adaptation.join("vibe.toml"),
///     "[translates]\npackage = \"com.example.docs/pair\"\nversion = \"^0.1\"\n",
/// )
/// .unwrap();
///
/// let world = SpecSources::for_checkout(&source, Some("com.example.docs"), "pair");
/// let page = borrowed_by(&adaptation, "guide/first.xml", &world);
/// assert_eq!(page["demo"].expect, "vibe 1.0.0");
///
/// // A page the source does not carry, and an address that is not a page
/// // address, both lend nothing.
/// assert!(borrowed_by(&adaptation, "guide/second.xml", &world).is_empty());
/// assert!(borrowed_by(&adaptation, "../vibe.toml", &world).is_empty());
/// ```
pub fn borrowed_by(
    package_dir: &Path,
    page: &str,
    sources: &SpecSources,
) -> BTreeMap<String, ExampleBody> {
    if !is_page_address(page) {
        return BTreeMap::new();
    }
    let Ok(Some(adapted)) = super::adapted(package_dir, sources) else {
        return BTreeMap::new();
    };
    let path = adapted.instance.root.join(pages::SPEC_ROOT).join(page);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let Ok(doc) = vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc) else {
        return BTreeMap::new();
    };
    authored(&doc)
}

/// The examples one page authors, by id, first in document order winning.
///
/// The walk is `manifest::page::walk` — the one the mirror check collects
/// a page's example ids with — so «the examples of a page» means the same
/// thing to the check that reports an unknown reference and to the reader
/// that resolves one.
///
/// Visible to the module above so its tests can hand it a document with a
/// repeated id: the pivot refuses such a page, so the tie-break cannot be
/// reached through a file and is proved on the walk instead.
pub(super) fn authored(doc: &SpecDoc) -> BTreeMap<String, ExampleBody> {
    let mut out: BTreeMap<String, ExampleBody> = BTreeMap::new();
    manifest::page::walk(doc, &mut |block| {
        if let Block::Example { id, .. } = block
            && let Some(body) = ExampleBody::of(block)
        {
            out.entry(id.clone()).or_insert(body);
        }
    });
    out
}

/// Whether `page` is a page address and nothing else.
///
/// Read through [`Component`] rather than by string surgery: a root, a
/// drive prefix, a `.` and a `..` are all «not an ordinary name» to the
/// path model, and one test for all four cannot be the one that was
/// forgotten. The backslash is checked separately because it is a
/// separator on one platform and an ordinary character on the other, and
/// this must refuse it on both.
fn is_page_address(page: &str) -> bool {
    !page.is_empty()
        && !page.contains('\\')
        && Path::new(page)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
