//! The `by-name/<name>.json` reads (PROP-008 §2.8) and the check that a
//! copy served for one is the copy `repomd.json` names.
//!
//! Why the check exists: a static index sits behind caches, and a cache
//! may hold an older `by-name` file for minutes after a push while
//! `repomd.json` — written last on every update — already names the new
//! state. On 2026-10-31, right after `vim@1.1.2` was published, a fresh
//! home read the old `by-name/vim.json` and chose `1.1.1`, a version this
//! build refuses. The manifest already carries the size and `sha256` of
//! every `by-name` file it vouches for, so the client compares the bytes
//! it received with that entry and, on a mismatch, answers «no index
//! answer» — which sends the selector to the package's own tags, the path
//! every index-less registry already takes.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http");

use semver::Version;
use sha2::{Digest, Sha256};
use vibe_core::Group;
use vibe_wire::generated::index::e1::repomd::RepomdFileEntry;

use super::wire::NameEntryView;
use super::{IndexClient, IndexError, IndexVersion};
use crate::short_name::{AliasTarget, NameLookup};

impl IndexClient {
    /// Fetch the `by-name/<name>.json` candidate set and return the
    /// versions of the `(group, name)` package in ascending semver
    /// order — the capability-declaration half of each record dropped.
    /// The version selector must not use this (it would be
    /// capability-blind, B-080); it asks
    /// [`IndexClient::list_version_entries`] instead. Kept for the
    /// enumeration surfaces that genuinely want every version listed.
    ///
    /// Returns `Ok(None)` when the file is absent (404) **or** the
    /// candidate set carries no package for `group` — both mean
    /// "fall through to `git ls-remote`". `Ok(Some(versions))` on a
    /// hit; `Err(...)` for any other failure.
    pub fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Option<Vec<Version>>, IndexError> {
        Ok(self
            .list_version_entries(group, name)?
            .map(|entries| entries.into_iter().map(|e| e.version).collect()))
    }

    /// [`IndexClient::list_versions`] without the blindness: each
    /// returned [`IndexVersion`] carries the `must_understand`
    /// declarations its record states on the wire (PROP-044 §4.5), so
    /// the caller can skip versions this reader must quarantine.
    /// Ordering and the `Ok(None)` fall-through contract are exactly
    /// `list_versions`'s.
    ///
    /// The `by-name/` layer is keyed by bare `name` and holds the whole
    /// candidate set — every group that publishes a package of that
    /// name (PROP-008 §2.8). The lookup selects the candidate whose
    /// `group` matches the requested `(group, name)` identity.
    ///
    /// A file `repomd.json` does not vouch for is answered `Ok(None)` as
    /// well (see [`IndexClient::by_name_is_current`]): the selector then
    /// enumerates the package's tags, so a cache that still serves an
    /// older copy cannot make it choose an older version.
    pub fn list_version_entries(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Option<Vec<IndexVersion>>, IndexError> {
        let url = format!("{}/by-name/{}.json", self.file_base, name);
        let client = self.file_client().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let resp = self.get(client, client.get(&url), &url)?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(self.classify_failure(url, status.as_u16()));
        }
        let body = resp
            .bytes()
            .map_err(|error| Self::body_error(&url, error))?;
        if !self.by_name_is_current(name, &body)? {
            return Ok(None);
        }
        let parsed: NameEntryView =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        let Some(pkg) = parsed.packages.into_iter().find(|p| &p.group == group) else {
            return Ok(None);
        };
        let mut versions: Vec<IndexVersion> = pkg.versions.into_iter().map(Into::into).collect();
        versions.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(Some(versions))
    }

    /// Fetch the `by-name/<name>.json` candidate set and return every
    /// `group` that publishes a package of this bare name (PROP-008
    /// §2.8). This is the primitive index-backed short-name resolution
    /// (PROP-008 §2.6) walks: one GET per registry enumerates the
    /// `(*, name)` candidates, so a collision (PROP-008 §2.7) — two
    /// groups under one bare name — is visible at once.
    ///
    /// `Ok(vec![])` when the file is absent (404) — the name is simply
    /// not carried by this index. `Err(...)` for any other failure;
    /// the caller decides whether to treat it as fatal or skip the
    /// registry. Groups are returned in on-disk order; de-duplication
    /// and sorting are the caller's job (it unions across registries).
    pub fn name_candidates(&self, name: &str) -> Result<Vec<Group>, IndexError> {
        Ok(self.name_lookup(name)?.groups)
    }

    /// [`IndexClient::name_candidates`] with the registry's own alias
    /// for the spelling, from the SAME single GET (PROP-008 §2.6).
    ///
    /// The alias rides beside the candidate list in the one file
    /// short-name resolution already fetches, so offering short
    /// spellings costs a resolving client no extra round trip — which
    /// is the constraint the whole `by-name` layout exists to satisfy.
    pub fn name_lookup(&self, name: &str) -> Result<NameLookup, IndexError> {
        let url = format!("{}/by-name/{}.json", self.file_base, name);
        let client = self.file_client().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let resp = self.get(client, client.get(&url), &url)?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(NameLookup::default());
        }
        if !status.is_success() {
            return Err(self.classify_failure(url, status.as_u16()));
        }
        let body = resp
            .bytes()
            .map_err(|error| Self::body_error(&url, error))?;
        let parsed: NameEntryView =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        Ok(NameLookup {
            groups: parsed.packages.into_iter().map(|p| p.group).collect(),
            alias: parsed.alias.map(|a| AliasTarget {
                group: a.group,
                name: a.name,
            }),
        })
    }

    /// Whether `body`, served for `by-name/<name>.json`, is the file
    /// `repomd.json` vouches for.
    ///
    /// The manifest of a writer that lists `by-name` files one by one
    /// either names this file — and then its `sha256` must match — or
    /// does not, and then the file is one the manifest no longer carries
    /// (a removed name a cache still serves). A manifest that lists no
    /// `by-name` file at all comes from a writer older than the per-file
    /// entries; there is nothing to check against, and the file is
    /// trusted as before. So is the case where the manifest cannot be
    /// read for ordinary transport/format reasons. A contact refusal remains
    /// a typed error, including when replayed from this invocation's cache.
    pub fn by_name_is_current(&self, name: &str, body: &[u8]) -> Result<bool, IndexError> {
        let Some(repomd) = self
            .repomd
            .get_or_init(|| match self.repomd() {
                Ok(value) => Ok(value.map(Box::new)),
                Err(IndexError::ContactRefused(refusal)) => Err(refusal),
                Err(_) => Ok(None),
            })
            .as_ref()
            .map_err(|refusal| IndexError::ContactRefused(refusal.clone()))?
        else {
            return Ok(true);
        };
        let key = format!("by-name/{name}.json");
        let verdict = match repomd.files.get(&key) {
            Some(RepomdFileEntry::File(file)) => {
                let received = format!("sha256:{:x}", Sha256::digest(body));
                if received == file.sha256 {
                    return Ok(true);
                }
                format!(
                    "its sha256 is {received}, repomd.json names {}",
                    file.sha256
                )
            }
            Some(RepomdFileEntry::Directory(_)) => return Ok(true),
            None if repomd.files.keys().any(|k| k.starts_with("by-name/")) => {
                "repomd.json does not list it".to_string()
            }
            None => return Ok(true),
        };
        tracing::debug!(
            target: "vibe_registry::index_client",
            base = %self.file_base(),
            file = %key,
            "the served `by-name` file is not the one repomd.json vouches for ({verdict}); \
             enumerating the package's tags instead"
        );
        Ok(false)
    }
}

impl IndexClient {
    /// The registry's whole alias table — the authored
    /// `aliases.toml` at the index root (PROP-008 §2.6).
    ///
    /// It is an INPUT to the projection rather than one of its outputs,
    /// so `repomd.json` does not vouch for it and the freshness check
    /// above does not apply. That is the honest shape: the manifest is
    /// the manifest of what the writer writes, and this is what the
    /// writer was told. The one consumer is `vibe registry mirror`,
    /// which carries the table across and whose own merge rule decides
    /// what to do with each row; a RESOLVING client never fetches this
    /// — it reads the alias off the `by-name` file it already gets.
    pub fn alias_table(&self) -> Result<Vec<(String, AliasTarget)>, IndexError> {
        let url = format!("{}/aliases.toml", self.file_base);
        let client = self.file_client().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let resp = self.get(client, client.get(&url), &url)?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(Vec::new());
        }
        if !status.is_success() {
            return Err(self.classify_failure(url, status.as_u16()));
        }
        let body = resp
            .bytes()
            .map_err(|error| Self::body_error(&url, error))?;
        // A table this reader cannot parse is answered as "no spellings"
        // rather than as a failure: the file belongs to another registry,
        // the only caller is a mirror run copying what it can, and a
        // stranger's malformed table must not be able to stop it. The
        // same reader tolerance `vibe_index::aliases::attach` applies to
        // a single bad row, one level up.
        let text = String::from_utf8_lossy(&body);
        Ok(parse_alias_table(&text).unwrap_or_else(|message| {
            tracing::warn!(
                target: "vibe_registry::index_client",
                %url,
                %message,
                "the registry's alias table could not be read; carrying no spellings from it"
            );
            Vec::new()
        }))
    }
}

/// The narrow reader for the alias table — the same shape this crate
/// uses for `by-name`: it reads a registry's files, it does not build
/// them, so the rows it cannot use it simply does not return.
pub(crate) fn parse_alias_table(text: &str) -> Result<Vec<(String, AliasTarget)>, String> {
    #[derive(serde::Deserialize)]
    struct Table {
        #[serde(default)]
        aliases: std::collections::BTreeMap<String, String>,
    }
    let table: Table = toml::from_str(text).map_err(|e| e.to_string())?;
    Ok(table
        .aliases
        .into_iter()
        .filter_map(|(alias, spelling)| {
            let (group, name) = spelling.split_once('/')?;
            Some((
                alias,
                AliasTarget {
                    group: Group::parse(group).ok()?,
                    name: name.to_owned(),
                },
            ))
        })
        .collect())
}
