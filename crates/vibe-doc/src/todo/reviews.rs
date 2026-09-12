//! `maintenance/reviews.toml` — when each page was last READ ALOUD, by
//! whom, and when the last full reconciliation was (PROP-057
//! `##OBS-MAINTENANCE-TOOLS`, `MAINTENANCE.md` §2.2, §6).
//!
//! Authored data, never a generated record. A machine can tell that a
//! citation resolves and an example still prints what the page says; only
//! a person can tell that the prose around them still reads. So the one
//! thing this file holds is the fact that somebody sat down with a page
//! on a day — not what the page was checked against, not a revision, not
//! a hash. A page with no row has never been read aloud, and the queue
//! reports exactly that rather than an age it does not have.
//!
//! The order of the rows is also the rota: the «page of the week» is the
//! next one along.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

use std::path::Path;

use chrono::NaiveDate;
use vibe_wire::generated::doc_reviews::DocReviews;

use super::history;
use crate::error::{DocError, Result};
use crate::pages::PageSet;

/// Where the file sits inside a package.
pub const REVIEWS: &str = "maintenance/reviews.toml";

/// The schema version a file written today carries.
pub const SCHEMA_VERSION: u32 = 1;

/// One page's standing: when it was read, by whom, and how often it has
/// been edited since.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Age {
    pub page: String,
    /// The date of the last reading, absent when there has never been
    /// one.
    pub read: Option<NaiveDate>,
    pub by: String,
    /// Commits touching the page since that date, when the history is
    /// readable. `None` is «not asked» and is silent by the norm's own
    /// instruction — a tree published without its history is a legitimate
    /// place to run this from.
    pub edits: Option<usize>,
}

impl Age {
    /// Days since the reading, or `None` when there has not been one.
    pub fn days(&self, today: NaiveDate) -> Option<i64> {
        Some((today - self.read?).num_days())
    }
}

/// Read the file, or `None` when the package keeps none.
///
/// ```
/// let tmp = tempfile::tempdir().unwrap();
/// assert!(vibe_doc::todo::reviews::read(tmp.path()).unwrap().is_none());
/// ```
pub fn read(package_dir: &Path) -> Result<Option<DocReviews>> {
    let path = package_dir.join(REVIEWS);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    // `toml::from_str`, not `str::parse`: the latter reads a single TOML
    // VALUE, and this is a document.
    let reviews: DocReviews = toml::from_str(&text).map_err(|e| DocError::Todo {
        message: format!("`{}` does not parse: {e}", path.display()),
    })?;
    Ok(Some(reviews))
}

/// Every page of the package with its standing, in the file's own order
/// first — that order is the reading rota — and the pages nobody has ever
/// read after them, in page order.
pub fn ages(
    reviews: &DocReviews,
    set: &PageSet,
    repo_root: Option<&Path>,
    package_dir: &Path,
) -> Vec<Age> {
    let mut out: Vec<Age> = Vec::new();
    for row in &reviews.page {
        let read = date(&row.read);
        let edits = match (repo_root, read) {
            (Some(root), Some(since)) => {
                history::edits_since(root, &page_path(package_dir, &row.path), since)
            }
            _ => None,
        };
        out.push(Age {
            page: row.path.clone(),
            read,
            by: row.by.clone(),
            edits,
        });
    }
    for page in &set.pages {
        if !out.iter().any(|age| age.page == page.rel) {
            out.push(Age {
                page: page.rel.clone(),
                read: None,
                by: String::new(),
                edits: None,
            });
        }
    }
    out
}

/// Days since the last full reconciliation, when one is recorded.
pub fn days_since_reconcile(reviews: &DocReviews, today: NaiveDate) -> Option<u32> {
    let since = date(reviews.reconciled.as_deref()?)?;
    Some((today - since).num_days().max(0) as u32)
}

/// The path on disk a row's page address names.
fn page_path(package_dir: &Path, rel: &str) -> std::path::PathBuf {
    let mut path = package_dir.join(crate::pages::SPEC_ROOT);
    for segment in rel.split('/') {
        path.push(segment);
    }
    path
}

/// `YYYY-MM-DD`, or nothing.
///
/// A date the file spells wrong reads as no date, and the page then
/// reports as never read. That is the safe direction: an unreadable date
/// must not become a young page.
fn date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests;
