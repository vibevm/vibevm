//! The reading order of a corpus: stable text before text that moves
//! (PROP-048 `##THE-LAYER-LAW`, PROP-057 `##SEO-LLMS-FILES`).
//!
//! The layer law is the project's global idea, not a rule of any one
//! mechanism: everything loaded is one monotone gradient of mutation
//! frequency, the rarest-changing text reads first, because a change at
//! depth N re-prices every byte after it. `##SEO-LLMS-FILES` applies it to
//! the machine corpus by name — «the full corpus ordered by the layer law
//! (stable before mutable)» — and this module is where the order is
//! decided.
//!
//! ## The signal, and why it is not a list of section names
//!
//! PROP-048's own open direction (`##DIR-CACHE-AWARE-ORDERING`) says the
//! ordering «needs a stability signal per package (version age, or a
//! declared tier) before it can be built». A documentation package has a
//! better signal than either, and it is already written on every page:
//! **where the page's text comes from**.
//!
//! A page whose blocks are prose and quoted rules changes when somebody
//! decides to change it. A page whose blocks are `derived` references is
//! regenerated from the product, so it moves whenever a flag moves — ten
//! times a day, by the owner's own account of how this project releases.
//! An `example` sits between: its command is prose, its golden output is
//! the product's. So the rank of a page is the share of its blocks whose
//! text is the product's — `derived`, `example` and a translation's
//! borrowed `example ref` — and the order is that rank ascending.
//!
//! A quoted `rule` deliberately does NOT count as mutable. Specifications
//! change by amendment and tombstone, which is the slow end of the
//! gradient, and a page dense with citations is the most stable kind of
//! page there is.
//!
//! This beats a hard-coded list of section names for one reason that
//! matters: a list would be the vibevm manual's information architecture
//! written into a library that every other documentation package in the
//! world also has to use. The signal is computed from the page, so a
//! package nobody here has seen gets a sensible order for free.
//!
//! Ties — and in a prose-only manual most pages tie at zero — break on
//! the page's address, so the order is total and two runs over an
//! unchanged package produce the same bytes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-048#THE-LAYER-LAW");

use std::collections::BTreeMap;

use vibe_specdoc::doc::{Block, SpecDoc};
use vibe_wire::generated::doc_manifest::DocPage;

use crate::pages::Page;

/// How much of a page's text the product owns: the count of blocks
/// generated from or checked against the product, and the page's total
/// block count.
///
/// ```
/// use vibe_doc::manifest::layer::mutability;
///
/// let doc = vibe_specdoc::from_xml_with(
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
///        <title id=\"root\">T</title>\n\
///        <p>prose</p>\n\
///        <derived kind=\"cli-help\" ref=\"vibe list --help\"/>\n\
///      </spec>\n",
///     vibe_specdoc::Vocabulary::Doc,
/// )
/// .unwrap();
///
/// assert_eq!(mutability(&doc), (1, 2));
/// ```
pub fn mutability(doc: &SpecDoc) -> (usize, usize) {
    let mut from_product = 0usize;
    let mut total = 0usize;
    super::page::walk(doc, &mut |block| {
        total += 1;
        if matches!(
            block,
            Block::Derived { .. } | Block::Example { .. } | Block::ExampleRef { .. }
        ) {
            from_product += 1;
        }
    });
    (from_product, total)
}

/// Sort the manifest's page rows into layer order, using the documents
/// the rows were built from.
///
/// The rows and the pages are matched by address rather than by index, so
/// a caller that filtered or reordered either list still gets the right
/// answer; a row whose page is not in the list sorts as pure prose, which
/// is the most stable rank there is.
pub fn order(rows: &mut [DocPage], pages: &[Page]) {
    let ranks: BTreeMap<&str, (usize, usize)> = pages
        .iter()
        .map(|p| (p.rel.as_str(), mutability(&p.doc)))
        .collect();
    rows.sort_by(|a, b| {
        let left = ranks.get(a.path.as_str()).copied().unwrap_or((0, 0));
        let right = ranks.get(b.path.as_str()).copied().unwrap_or((0, 0));
        compare(left, right).then_with(|| a.path.cmp(&b.path))
    });
}

/// Compare two shares without leaving the integers: `a.0 / a.1` against
/// `b.0 / b.1` is `a.0 * b.1` against `b.0 * a.1`, and a page with no
/// blocks at all is pure stability rather than a division by zero.
fn compare(a: (usize, usize), b: (usize, usize)) -> std::cmp::Ordering {
    match (a.1, b.1) {
        (0, 0) => std::cmp::Ordering::Equal,
        (0, _) => std::cmp::Ordering::Less,
        (_, 0) => std::cmp::Ordering::Greater,
        _ => (a.0 * b.1).cmp(&(b.0 * a.1)),
    }
}

#[cfg(test)]
mod tests;
