//! `reviews.toml` — when a human last read a page aloud against the
//! product (PROP-057 `##OBS-MAINTENANCE-TOOLS`, the vision's D-26).
//!
//! This is the only staleness signal the project keeps, and it is
//! deliberately a human one. There is no revision, no content hash and no
//! drift counter anywhere in the documentation pipeline, because those
//! need a history the product does not keep (`##OBS-VERSION-CONTRACT`).
//! What is left is a person saying «I read this page on this day», which
//! is worth more than any of them: whether the prose around a citation
//! still means what it did is exactly the question a machine cannot ask
//! (`##OBS-PROSE-STALENESS-IS-HUMAN`).
//!
//! The file is authored by hand, so it holds a calendar date and not an
//! instant:
//!
//! ```toml
//! [pages]
//! "model/lock-and-store.xml" = 2026-09-12
//! "start/install-vibe.xml" = { read = 2026-09-12, by = "Oleg" }
//! ```
//!
//! The wire form is the project's one date spelling — RFC 3339, the
//! `timestamp` vocabulary — so the date is lifted to midnight UTC on the
//! way out, the same lift the REVIEW-marker check already makes
//! (`crates/vibe-check/src/checks/review_aging.rs`). A day is what the
//! author wrote down; the hour is not information anyone put there.
//!
//! An absent file is not an error. A package that has never been read
//! aloud has no dates, and refusing to build its manifest over that would
//! make the maintenance loop a precondition of publishing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, TimeZone, Utc};

use crate::error::{DocError, Result};

/// The read-aloud dates of one package, by page address.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reviews {
    dates: BTreeMap<String, DateTime<Utc>>,
}

impl Reviews {
    /// No page has been read aloud — the state of a package that has not
    /// been through a full reconciliation yet.
    pub fn none() -> Reviews {
        Reviews::default()
    }

    /// Read `reviews.toml`, or return the empty set when the package
    /// carries none.
    ///
    /// The document is read as TOML data. Only two shapes of value are
    /// understood — a bare date, and a table whose `read` key holds one —
    /// and anything else is ignored rather than refused, because this
    /// file belongs to the maintenance loop and will grow columns (who
    /// read it, in which loop) that a manifest has no business knowing
    /// about.
    ///
    /// ```
    /// use vibe_doc::manifest::Reviews;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("reviews.toml");
    /// std::fs::write(
    ///     &path,
    ///     "[pages]\n\"model/versions.xml\" = 2026-09-12\n",
    /// )
    /// .unwrap();
    ///
    /// let reviews = Reviews::read(&path).unwrap();
    /// assert!(reviews.read_aloud("model/versions.xml").is_some());
    /// assert!(reviews.read_aloud("model/registries.xml").is_none());
    /// ```
    pub fn read(path: impl AsRef<Path>) -> Result<Reviews> {
        let path: PathBuf = path.as_ref().to_path_buf();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Reviews::none()),
            Err(e) => return Err(DocError::io("reading", &path, e)),
        };
        let parsed: toml::Value = toml::from_str(&text).map_err(|e| {
            DocError::manifest(&path, format!("does not parse as `reviews.toml`: {e}"))
        })?;
        let mut dates: BTreeMap<String, DateTime<Utc>> = BTreeMap::new();
        let Some(pages) = parsed.get("pages").and_then(toml::Value::as_table) else {
            return Ok(Reviews { dates });
        };
        for (page, value) in pages {
            let read = match value {
                toml::Value::Datetime(_) | toml::Value::String(_) => Some(value),
                other => other.get("read"),
            };
            if let Some(when) = read.and_then(as_utc_midnight) {
                dates.insert(page.replace('\\', "/"), when);
            }
        }
        Ok(Reviews { dates })
    }

    /// When the page at this address was last read aloud.
    pub fn read_aloud(&self, page: &str) -> Option<DateTime<Utc>> {
        self.dates.get(page).copied()
    }

    /// How many pages carry a date.
    pub fn len(&self) -> usize {
        self.dates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dates.is_empty()
    }
}

/// A TOML date (or a string spelling one) as midnight UTC of that day.
///
/// TOML's own date type and a plain string are both accepted: an author
/// writing `2026-09-12` and an author writing `"2026-09-12"` mean the same
/// day, and a reader that took only one of them would be a trap in a
/// hand-written file.
fn as_utc_midnight(value: &toml::Value) -> Option<DateTime<Utc>> {
    let text = match value {
        toml::Value::Datetime(d) => d.to_string(),
        toml::Value::String(s) => s.clone(),
        _ => return None,
    };
    let day = text.get(..10)?;
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    Some(Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0)?))
}

#[cfg(test)]
mod tests;
