//! Rendering one queued version — and never stopping for it
//! (PROP-057 `##SITE-RENDER-IDEMPOTENT`, `##LEVEL-ZERO`).
//!
//! Every pair goes through the same three steps: its bytes are brought
//! onto disk, a documentation package is composed from them
//! ([`super::level0`]), and that package is built in the three
//! projections the site serves. The build is the one `vibe doc build`
//! already is — there is no second renderer for the site — so a page a
//! reader opens on the web and a page an agent reads locally come out of
//! one function.
//!
//! ## A package that will not render becomes a page
//!
//! «A render error is shown as the version's page.» Not a failed build,
//! not a gap in the site, and above all not a stop: a registry of
//! hundreds of packages will always hold one whose manifest is broken,
//! and a builder that refused to finish because of it would publish
//! nothing at all. So the reason is composed into a card and a page at
//! the same address, the row in the state file records the failure, and
//! the next run tries again.
//!
//! ## Where the trees go, and why they stay
//!
//! One directory per coordinate and version, one subdirectory per
//! projection, under the builder's own `.vibe-site/trees`. They stay
//! between runs on purpose: the site's static build reads EVERY
//! edition's tree, so a run that rebuilt only what moved would still
//! need the others, and re-rendering them to hand them over would make
//! the incremental queue pointless.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-RENDER-IDEMPOTENT");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::feed::{Origin, Pair};
use super::level0;
use crate::build::{self, Format};
use crate::citations::SpecSources;
use crate::error::{DocError, Result};
use crate::manifest;

/// The three projections a site serves, in the order they are written.
///
/// One `vibe doc build` writes ONE projection, so an edition is three
/// runs into three directories that share a coordinate — which is what
/// the static build's copying step is already written to read.
pub const FORMATS: &[Format] = &[Format::Html, Format::Md, Format::Xml];

/// What the composition root answers about one pair, because the library
/// resolves nothing ambient of its own.
///
/// Two questions, one seam. Warming is the install stack's job and the
/// pipeline must not link it (`##PIPE-CRATES`); generating a `derived`
/// block means RUNNING a product, and which product — and whether it may
/// be run at all for a package the site merely publishes — is a decision
/// above this library, not inside it.
///
/// The canonical implementation lives in the composition root, warms
/// through `vibe cache add`'s own walk, and runs a product only for the
/// host's own packages. An implementation that does neither is legal and
/// useful — it is what a build over a checkout alone needs:
///
/// ```
/// use std::collections::BTreeMap;
/// use std::path::{Path, PathBuf};
/// use vibe_doc::site::{Pair, Prepare};
///
/// /// Everything is already on disk, and no product is run: the blocks
/// /// render as the marked gaps they are.
/// struct OnDisk {
///     root: PathBuf,
/// }
///
/// impl Prepare for OnDisk {
///     fn warm(&self, pair: &Pair) -> vibe_doc::error::Result<PathBuf> {
///         Ok(self.root.join(pair.coordinate()))
///     }
///
///     fn derived(&self, _pair: &Pair, _package: &Path) -> BTreeMap<String, String> {
///         BTreeMap::new()
///     }
/// }
///
/// let prepare = OnDisk { root: PathBuf::from("/srv/packages") };
/// let pair = Pair {
///     source: "vibespecs".into(),
///     group: "org.example".into(),
///     name: "wal".into(),
///     version: "1.0.0".into(),
///     content_hash: "sha256:aa".into(),
///     origin: vibe_doc::site::Origin::Registry,
///     entry: None,
/// };
/// assert_eq!(
///     prepare.warm(&pair).unwrap(),
///     Path::new("/srv/packages").join("org.example/wal"),
/// );
/// ```
pub trait Prepare {
    /// Bring the pair's bytes onto disk and answer with the directory
    /// that holds them. Warming a `doc` package warms its subjects too,
    /// so its `spec://` citations resolve without the network
    /// (`##REL-WARMUP-CLOSURE`).
    fn warm(&self, pair: &Pair) -> Result<PathBuf>;

    /// The generated text of each `derived` block, by kind and
    /// reference. An empty map is legal and honest: the blocks render as
    /// the marked gaps they are, which is what a build from a warmed
    /// store on a machine that compiled nothing already does.
    fn derived(&self, pair: &Pair, package_dir: &Path) -> BTreeMap<String, String>;
}

/// Everything one pair's render needs that the pair cannot say.
pub struct Options<'a> {
    /// The path the site is served under.
    pub base: &'a str,
    /// The world a `rule` citation resolves against.
    pub sources: &'a SpecSources,
    /// Where the composed packages and the built trees are kept.
    pub work: &'a Path,
    /// The instant the manifest carries. Supplied, never read from a
    /// clock here: the same sources must render to the same bytes.
    pub rendered_at: DateTime<Utc>,
    /// What the rest of the catalog says about the package being
    /// rendered — folded once for the whole site, handed in per pair.
    pub related: &'a level0::Related,
}

/// What one pair's render produced.
#[derive(Debug, Clone)]
pub struct Rendered {
    /// The directories the static build is handed, one per projection.
    pub trees: Vec<PathBuf>,
    /// How many files were written across all three.
    pub files: usize,
    /// New or changed files physically written across the three projections.
    pub written: usize,
    /// Byte-identical files retained from the prior render.
    pub reused: usize,
    /// The reason the package did not render, when it did not. The
    /// address still carries a page — that is what `##SITE-RENDER-IDEMPOTENT`
    /// asks for — and the state file records the failure so the next run
    /// tries again.
    pub failed: Option<String>,
    /// The blocks the pages were rendered with as marked gaps
    /// ([`build::Unresolved`]), counted once for the pair rather than once
    /// per projection: the three projections come out of ONE bundle, so
    /// three counts would be one fact said three times.
    ///
    /// A refused version carries its own count like any other, which is
    /// zero — the page a refusal shows quotes nothing and borrows nothing.
    pub unresolved: build::Unresolved,
    /// Things worth saying out loud about this package.
    pub notes: Vec<String>,
}

impl Rendered {
    /// Did it render?
    pub fn ok(&self) -> bool {
        self.failed.is_none()
    }
}

/// Render one pair, whatever happens.
///
/// This function does not return an error for a package: it returns a
/// [`Rendered`] that either carries three trees or carries the reason
/// and three trees holding the «render failed» page. The only way out
/// with an error is a failure of the OUTPUT — a directory that cannot be
/// written — because that is not about the package at all.
pub fn render(pair: &Pair, prepare: &dyn Prepare, options: &Options) -> Result<Rendered> {
    let mut notes = Vec::new();
    match attempt(pair, prepare, options, &mut notes) {
        Ok((trees, files, written, reused, unresolved)) => Ok(Rendered {
            trees,
            files,
            written,
            reused,
            failed: None,
            unresolved,
            notes,
        }),
        Err(failure) => {
            let reason = failure.to_string();
            let (trees, files, written, reused, unresolved) = refused(pair, &reason, options)?;
            Ok(Rendered {
                trees,
                files,
                written,
                reused,
                failed: Some(reason),
                unresolved,
                notes,
            })
        }
    }
}

/// The happy path, whose every failure becomes a page.
fn attempt(
    pair: &Pair,
    prepare: &dyn Prepare,
    options: &Options,
    notes: &mut Vec<String>,
) -> Result<(Vec<PathBuf>, usize, usize, usize, build::Unresolved)> {
    let source = match &pair.origin {
        // The host's bytes are already on disk — that IS the host
        // channel (`##SITE-HOST-CHECKOUT`), and fetching them would
        // mean publishing the host to a registry for the sake of the
        // site.
        Origin::HostProject { root } => root.clone(),
        Origin::HostPackage { dir } => dir.clone(),
        Origin::Registry => prepare.warm(pair)?,
    };
    let related = options.related.clone().adapting(mirror(&source, options));
    let composed = level0::compose(&source, &composed_dir(options.work, pair), &related)?;
    notes.extend(composed.notes.iter().cloned());
    let derived = prepare.derived(pair, &composed.dir);
    build_projections(pair, &composed.dir, derived, options)
}

/// Measure an adaptation against the documentation it adapts, when this
/// package is one.
///
/// Asked of the package's own bytes and of the world the citations
/// resolve in — the same `##LOC-MIRROR` check `vibe doc check
/// --translations` runs, and the same answer, because the site must not
/// have a second opinion about whether a translation mirrors.
///
/// A package that adapts nothing is not asked, and a check that cannot
/// reach the source answers nothing rather than a number: «the source is
/// not here» and «the source and this agree» must not print the same
/// line.
fn mirror(source: &Path, options: &Options) -> Option<level0::Adaptation> {
    let report = crate::translations::check(source, options.sources).ok()?;
    let adapts = report.adapts?;
    report.source?;
    Some(level0::Adaptation {
        source: adapts,
        pages: report.pages,
        divergences: report.problems.len(),
    })
}

/// Compose and build the page a refused package shows at its address.
///
/// It goes through the same build as any other package, for the same
/// reason level 0 does: a second way of producing a page would be a
/// second set of rules about what a page is, and the one page nobody
/// looks at until something is wrong is the worst place to keep them.
fn refused(
    pair: &Pair,
    reason: &str,
    options: &Options,
) -> Result<(Vec<PathBuf>, usize, usize, usize, build::Unresolved)> {
    let dir = composed_dir(options.work, pair);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| DocError::io("clearing", &dir, e))?;
    }
    std::fs::create_dir_all(dir.join(crate::pages::SPEC_ROOT))
        .map_err(|e| DocError::io("creating", &dir, e))?;
    std::fs::write(dir.join("vibe.toml"), failed_manifest(pair, reason))
        .map_err(|e| DocError::io("writing", dir.join("vibe.toml"), e))?;
    std::fs::write(
        dir.join(crate::pages::SPEC_ROOT).join(FAILED_PAGE),
        failed_page(pair, reason),
    )
    .map_err(|e| DocError::io("writing", dir.join(FAILED_PAGE), e))?;
    build_projections(pair, &dir, BTreeMap::new(), options)
}

/// Where the «render failed» page is served from.
pub const FAILED_PAGE: &str = "render-failed.xml";

/// Build the composed package in every projection the site serves.
fn build_projections(
    pair: &Pair,
    package_dir: &Path,
    derived: BTreeMap<String, String>,
    options: &Options,
) -> Result<(Vec<PathBuf>, usize, usize, usize, build::Unresolved)> {
    let mut trees = Vec::new();
    let mut files = 0;
    let mut written = 0;
    let mut reused = 0;
    let mut unresolved = build::Unresolved::default();
    for format in FORMATS {
        let tree = tree_dir(options.work, pair, *format);
        let built = build::build(
            package_dir,
            options.sources,
            &build::Options {
                format: *format,
                base: options.base.to_owned(),
                manifest: manifest::Options::at(options.rendered_at),
                derived: derived.clone(),
            },
        )?;
        // The gap census of the pair, taken from one projection and not
        // added up over three: every format here builds from the same
        // package, the same world and the same generated text, so the
        // three answers are one answer — and summing them would treat a
        // page with one unfilled example as a page with three.
        if matches!(format, Format::Html) {
            unresolved = built.unresolved;
        }
        let report = build::write_reconciled(&built, &tree)?;
        files += built.files.len();
        written += report.written + report.removed;
        reused += report.unchanged;
        trees.push(tree);
    }
    Ok((trees, files, written, reused, unresolved))
}

/// Where a pair's composed documentation package is written.
///
/// One directory segment for the whole coordinate, not three nested
/// ones. The build writes `<group>/<name>/<version>/…` inside the tree
/// it is given, so a nested work path would spell the coordinate twice
/// and Windows counts every character of a path that leads to a page.
fn composed_dir(work: &Path, pair: &Pair) -> PathBuf {
    work.join(format!("{}@{}", slot(pair), pair.version))
        .join("package")
}

/// Where one projection of a pair is built.
pub fn tree_dir(work: &Path, pair: &Pair, format: Format) -> PathBuf {
    work.join(format!("{}@{}", slot(pair), pair.version))
        .join(format.as_str())
}

/// The coordinate as one path segment.
fn slot(pair: &Pair) -> String {
    format!("{}.{}", pair.group, pair.name)
}

/// The card of a version that would not render.
fn failed_manifest(pair: &Pair, reason: &str) -> String {
    let abstract_ = format!(
        "This version did not render. The site shows the reason rather than an empty \
         address, and tries again on the next build. The reason: {reason}"
    );
    format!(
        "# Written by `vibe doc build-site` for a version that would not render\n\
         # (PROP-057 `##SITE-RENDER-IDEMPOTENT`).\n\n\
         [package]\n\
         name = {}\n\
         group = {}\n\
         version = {}\n\
         title = {}\n\
         abstract = {}\n",
        toml_string(&pair.name),
        toml_string(&pair.group),
        toml_string(&pair.version),
        toml_string(&format!("{} — did not render", pair.coordinate())),
        toml_string(&abstract_),
    )
}

/// The page a refused version shows.
fn failed_page(pair: &Pair, reason: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
         <title id=\"root\">This version did not render</title>\n  \
         <status stage=\"doc\" state=\"work\" audience=\"user\"/>\n  \
         <p>{} is published, and the site could not turn it into pages. The address \
         is here so that a link to it is not a dead end, and the build tries again \
         every time it runs — nothing has to be done to ask for that.</p>\n  \
         <the-reason title=\"What went wrong\">\n    \
         <p>The renderer reported this, in its own words:</p>\n    \
         <fence>{}</fence>\n  \
         </the-reason>\n\
         </spec>\n",
        escape(&pair.spelled()),
        escape(reason),
    )
}

/// A TOML basic string.
fn toml_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// XML text, escaped. A renderer's message may hold anything at all,
/// including the path of a file with an angle bracket in its name.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests;
