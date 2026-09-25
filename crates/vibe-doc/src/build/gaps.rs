//! How many blocks a build left as marked gaps
//! (PROP-057 `##PIPE-LIBRARY`).
//!
//! Three of the documentation genre's blocks hold an address instead of a
//! text — a `rule`, a `derived`, a translation's `example ref` — and a
//! build that could not fetch one renders it as a marked gap rather than
//! as silence or as a refusal ([`crate::content`]). That is the right
//! behaviour, and it had one cost: nothing ever said HOW MANY. A run could
//! leave sixty-two example frames empty across thirty-four pages, print a
//! clean summary and pass every check — which is how B-176 stayed unseen
//! for as long as it did.
//!
//! So a build counts them and says the number out loud. The number is a
//! MEASUREMENT and never a verdict: which gaps are failures is the checks'
//! question (`vibe doc check --citations`, `--derived`, `--translations`),
//! a renderer does not abort over a text it could not fetch, and the site
//! renders every version whatever happens (`##SITE-RENDER-IDEMPOTENT`).
//! No exit code reads what is counted here.
//!
//! The island's `data-unresolved` attribute is the definition rather than
//! a second opinion: this counts exactly the blocks that carry it, so a
//! number in a summary can be checked against the page a reader has open.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use vibe_specdoc::doc::Block;

use crate::content::Content;
use crate::manifest;
use crate::numbering::expand_derived;
use crate::pages::Page;

/// The marked gaps of one page, or of a whole build, by kind.
///
/// Three counters rather than one total, because the three kinds are
/// gated by three different checks and closed by three different pieces of
/// work: an unresolved `rule` is a citation whose subject is not in this
/// build's world, an unresolved `derived` is a generator that did not run,
/// and an unresolved `example` is a translation whose source page was not
/// reached. A single number would send all three to the same place to be
/// looked into, and only one of them would be there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Unresolved {
    /// `example ref` blocks whose borrowed body this build did not have.
    pub examples: usize,
    /// `rule` blocks whose cited text this build did not resolve.
    pub rules: usize,
    /// `derived` blocks whose generated text this build did not carry.
    pub derived: usize,
}

impl Unresolved {
    /// Every marked gap, whatever its kind.
    pub fn total(&self) -> usize {
        self.examples + self.rules + self.derived
    }

    /// Nothing was left as a gap.
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// The human form: the total always, and the breakdown only when
    /// there is something to break down.
    ///
    /// A build with nothing to report says so in the shortest sentence
    /// that is still a sentence — `0 unresolved block(s)`. A summary line
    /// that spelled three zeroes every time would teach a reader to skip
    /// the place the number appears, which is the whole cost of adding it.
    ///
    /// ```
    /// use vibe_doc::build::Unresolved;
    ///
    /// assert_eq!(Unresolved::default().render(), "0 unresolved block(s)");
    /// assert_eq!(
    ///     Unresolved {
    ///         examples: 62,
    ///         rules: 1,
    ///         derived: 0,
    ///     }
    ///     .render(),
    ///     "63 unresolved block(s) (62 example, 1 rule, 0 derived)"
    /// );
    /// ```
    pub fn render(&self) -> String {
        let total = self.total();
        if self.is_empty() {
            return format!("{total} unresolved block(s)");
        }
        format!(
            "{total} unresolved block(s) ({} example, {} rule, {} derived)",
            self.examples, self.rules, self.derived
        )
    }
}

/// Summing the pages of a build, which is the only arithmetic a census
/// needs: a build is the fold of its pages, and a page is counted once.
impl std::ops::AddAssign for Unresolved {
    fn add_assign(&mut self, other: Unresolved) {
        self.examples += other.examples;
        self.rules += other.rules;
        self.derived += other.derived;
    }
}

/// Count one page's marked gaps, exactly as [`super::render_page`] leaves
/// them.
///
/// The page is expanded first, by the same [`expand_derived`] the render
/// runs, because a `derived` block whose text this build HAS stops being a
/// `derived` block: it becomes the fence its generator built. Counting
/// before expansion would report every generated block on every page as a
/// gap, and a census that is loudest when nothing is wrong is a census
/// nobody reads twice. What survives expansion is what the island marks.
///
/// The walk is [`manifest::page::walk`], the one this crate already asks
/// «every block of this document» with, and it needs no recursion of its
/// own: no block of the genre nests another, so a gap is always at the top
/// of a block list.
///
/// The count describes the BUNDLE, not a projection. The XML projection
/// keeps a citation's address by its own contract and marks nothing at
/// all, so a census read out of its bytes would call a build clean whose
/// every rule resolved to nothing. A block whose text this build could not
/// fetch is a gap in all three projections, the one that was going to
/// print the address anyway included.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_doc::{build, content::Content, pages::Page};
///
/// let doc = vibe_specdoc::from_xml_with(
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
///        <title id=\"root\">Page</title>\n\
///        <what-it-quotes title=\"What it quotes\">\n\
///          <rule ref=\"spec://com.example/subject/common/PROP-001#A-RULE\"/>\n\
///          <derived kind=\"cli-help\" ref=\"vibe list --help\"/>\n\
///          <example ref=\"version\"/>\n\
///        </what-it-quotes>\n\
///      </spec>\n",
///     vibe_specdoc::Vocabulary::Doc,
/// )
/// .unwrap();
/// let page = Page {
///     rel: "guide/page.xml".to_owned(),
///     path: PathBuf::from("guide/page.xml"),
///     doc,
/// };
///
/// // This build fetched nothing, so all three blocks are marked gaps —
/// // and the island says the same thing in its own way.
/// let gaps = build::unresolved(&page, &Content::new());
/// assert_eq!(
///     gaps.render(),
///     "3 unresolved block(s) (1 example, 1 rule, 1 derived)"
/// );
/// assert_eq!(
///     gaps.total(),
///     build::render_page(&page, &Content::new(), build::Format::Html)
///         .matches("data-unresolved=\"true\"")
///         .count()
/// );
/// ```
pub fn unresolved(page: &Page, content: &Content) -> Unresolved {
    let expanded = expand_derived(&page.doc, content);
    let mut out = Unresolved::default();
    manifest::page::walk(&expanded, &mut |block| match block {
        // The island's own three predicates, unchanged: a borrowed example
        // is resolved against the page being rendered and never against
        // the id alone, a citation against the bundle's rules, and a
        // `derived` block that survived expansion is one whose generator
        // did not run.
        Block::ExampleRef { id } => {
            if content.example(&page.rel, id).is_none() {
                out.examples += 1;
            }
        }
        Block::Rule { uri, .. } => {
            if !content.rules.contains_key(uri) {
                out.rules += 1;
            }
        }
        Block::Derived { .. } => out.derived += 1,
        _ => {}
    });
    out
}
