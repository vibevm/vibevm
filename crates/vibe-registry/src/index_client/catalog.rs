//! The catalog files an index publishes — `repomd.json` and
//! `primary.jsonl` (PROP-005 §2.4).
//!
//! The two enumeration surfaces above this one answer about ONE package:
//! `by-name/<name>.json` for the version selector, `/v1/packages` for
//! search. A consumer that wants the whole catalog — the documentation
//! site, which asks «what does this registry publish now» and compares
//! the answer with what it last rendered (PROP-057
//! `##SITE-SOURCE-REGISTRY`) — has no way to ask that a package at a
//! time, and every static mirror already serves both files.
//!
//! ## Why the reading of `primary.jsonl` lives here and not in `vibe-index`
//!
//! The SHAPE has one home and this module does not touch it: a line is
//! read into the generated [`VersionEntry`], the same type
//! `vibe_index::index::primary::parse` reads it into, minted from
//! `formats/vocabularies.json`. What differs is the TRANSPORT — that one
//! reads a file beside itself, this one reads an HTTP body — and the
//! separation is not a preference: `vibe-index` links axum and tokio, and
//! the documentation pipeline that consumes this is linked by every
//! `vibe doc` check and by the site generator (PROP-057
//! `##PIPE-CRATES` keeps the server out of the pipeline by name). So the
//! envelope is read twice and the record is read once, which is the split
//! that cannot drift: a change to what a record CONTAINS lands in one
//! generated type and reaches both readers.
//!
//! ## What it deliberately does not do
//!
//! It does not fetch `primary.jsonl.gz`. The gzip sibling is a bandwidth
//! envelope for the same bytes, and reading it would put a DEFLATE
//! backend into this crate's dependency graph for a catalog measured in
//! kilobytes — the same graph whose backend choice was already shown to
//! move bytes under feature unification. A site that outgrows the plain
//! file is where that price becomes worth paying.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout");

use std::time::Duration;

use vibe_wire::generated::index::e1::repomd::Repomd;
use vibe_wire::generated::shared::VersionEntry;

use super::{FETCH_TIMEOUT_SECS, IndexClient, IndexError};

/// The catalog manifest's file name, as PROP-005 §2.4 spells it.
pub const REPOMD: &str = "repomd.json";

/// The per-version catalog's file name.
pub const PRIMARY: &str = "primary.jsonl";

impl IndexClient {
    /// Fetch `repomd.json` — what the index says its files are right now.
    ///
    /// The manifest is written LAST on every batch update, so a reader
    /// that took it first and then chased the hashes in it sees a
    /// consistent set. That is the whole reason a site asks for it before
    /// the catalog: the pair «this is the catalog I read, and this is the
    /// manifest that described it» is what makes a queue reproducible.
    ///
    /// `Ok(None)` when the file is absent (404) — a base that answers
    /// nothing here is not an index, which is a state and not an error.
    ///
    /// ```no_run
    /// let client = vibe_registry::index_client::IndexClient::at("https://example.invalid/index");
    /// if let Some(repomd) = client.repomd().unwrap() {
    ///     assert!(repomd.version_count >= repomd.package_count);
    /// }
    /// ```
    pub fn repomd(&self) -> Result<Option<Repomd>, IndexError> {
        let url = format!("{}/{REPOMD}", self.file_base());
        let Some(body) = self.catalog_file(&url)? else {
            return Ok(None);
        };
        let parsed: Repomd = serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
            url,
            message: e.to_string(),
        })?;
        Ok(Some(parsed))
    }

    /// Fetch `primary.jsonl` — every version this index currently lists,
    /// one record per line.
    ///
    /// «Currently» is the whole contract. A version number may be
    /// published many times, and the catalog holds only the last
    /// publication of each: there is no history here to read, and a
    /// consumer that wants to know whether something moved compares the
    /// `content_hash` it last saw with the one it sees now (PROP-057
    /// `##SITE-VERSION-SHOWS-CURRENT`).
    ///
    /// A malformed line names its own number, because a catalog of
    /// thousands of records is not debuggable by «somewhere in this file».
    ///
    /// ```no_run
    /// let client = vibe_registry::index_client::IndexClient::at("https://example.invalid/index");
    /// for entry in client.primary().unwrap().unwrap_or_default() {
    ///     println!("{}/{}@{} {}", entry.group, entry.name, entry.version, entry.content_hash);
    /// }
    /// ```
    pub fn primary(&self) -> Result<Option<Vec<VersionEntry>>, IndexError> {
        let url = format!("{}/{PRIMARY}", self.file_base());
        let Some(body) = self.catalog_file(&url)? else {
            return Ok(None);
        };
        Ok(Some(parse_primary(&url, &body)?))
    }

    /// GET one catalog file, with the fetch timeout and this client's
    /// auth plan. `Ok(None)` on 404, which is «this index does not carry
    /// that file», never «the read failed».
    fn catalog_file(&self, url: &str) -> Result<Option<Vec<u8>>, IndexError> {
        let client = IndexClient::build_client(
            Duration::from_secs(FETCH_TIMEOUT_SECS),
            self.auth(),
            self.file_base(),
        )
        .map_err(|e| IndexError::Http {
            url: url.to_string(),
            message: e.to_string(),
        })?;
        let resp = client.get(url).send().map_err(|e| IndexError::Http {
            url: url.to_string(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(self.classify_failure(url.to_string(), status.as_u16()));
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.to_string(),
            message: e.to_string(),
        })?;
        Ok(Some(body.to_vec()))
    }
}

/// Read the JSON Lines envelope: one [`VersionEntry`] per non-blank line.
///
/// Public to the crate so a test can hand it bytes without a server; the
/// envelope is the only thing this function knows, and the record it
/// produces is the generated type and nobody's second opinion.
pub(crate) fn parse_primary(url: &str, body: &[u8]) -> Result<Vec<VersionEntry>, IndexError> {
    let text = std::str::from_utf8(body).map_err(|e| IndexError::Malformed {
        url: url.to_string(),
        message: format!("{PRIMARY} is not valid UTF-8: {e}"),
    })?;
    let mut out = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: VersionEntry =
            serde_json::from_str(line).map_err(|e| IndexError::Malformed {
                url: url.to_string(),
                message: format!("{PRIMARY} line {} is malformed: {e}", number + 1),
            })?;
        out.push(entry);
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
