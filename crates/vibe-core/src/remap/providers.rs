//! The derived provider map of one home (`PROP-066#providers`).
//!
//! `<home>/remap/providers.json` answers one question: for every active
//! row, which installed packages offer it. It is **derived** — from the
//! installed remap packages and the acceptance ledger — and rebuilt
//! whenever either changes, so it is never the sole home of a fact. What
//! it buys is the question uninstall has to ask: "does any other accepted
//! package still offer this row?" Answering that by walking every package
//! tree at uninstall time would work too; keeping the answer here is what
//! lets `remap list` show a shared row differently from a row one package
//! brought, which is the display `PROP-066#providers` asks for.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#providers");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::consent::{ConsentError, ConsentLedger, remap_dir};
use super::layers::Layer;
use super::package::{RemapPackage, RowIdentity};

/// The document protocol of `providers.json`.
pub const PROVIDERS_PROTOCOL: &str = "vibe-remap-providers/1";

/// `<home>/remap/providers.json`.
pub fn providers_path(home: &Path) -> PathBuf {
    remap_dir(home).join("providers.json")
}

/// One row and every package that offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#providers")]
pub struct ProvidedRow {
    /// Key, target and scope — the row's identity (`PROP-066#providers`).
    pub row: RowIdentity,
    /// `<key>@<version>` of every package that offers it, sorted.
    pub providers: Vec<String>,
    /// Of those, the ones whose acceptance currently applies.
    pub accepted_providers: Vec<String>,
}

impl ProvidedRow {
    /// Whether any accepted package still offers this row.
    pub fn is_live(&self) -> bool {
        !self.accepted_providers.is_empty()
    }
}

/// The map of one home.
///
/// ```
/// use vibe_core::remap::{ConsentLedger, ProviderMap, RemapPackage, RemapSection, RowDecl};
///
/// let row: RowDecl = toml::from_str("registry = \"r\"\nurl = \"https://nexus/r\"").unwrap();
/// let package = |key: &str| RemapPackage {
///     key: key.into(),
///     version: "1.0.0".into(),
///     content_hash: format!("sha256:{key}"),
///     section: RemapSection {
///         description: None,
///         priority: 0,
///         rows: vec![row.clone()],
///     },
///     policy: Default::default(),
/// };
/// let packages = [package("org.corp/a"), package("org.corp/b")];
/// let mut ledger = ConsentLedger::default();
/// ledger.accept("org.corp/a", "sha256:org.corp/a", "2026-11-01T00:00:00Z");
///
/// let map = ProviderMap::derive(&packages, &ledger);
/// assert_eq!(map.rows.len(), 1, "one row, two providers");
/// assert_eq!(map.rows[0].providers.len(), 2);
/// // Only the accepted one makes the row live.
/// assert_eq!(map.rows[0].accepted_providers, ["org.corp/a@1.0.0"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#providers")]
pub struct ProviderMap {
    /// `vibe-remap-providers/1`.
    pub protocol: String,
    /// Every row any installed package offers, sorted by identity.
    #[serde(default)]
    pub rows: Vec<ProvidedRow>,
}

impl Default for ProviderMap {
    fn default() -> Self {
        ProviderMap {
            protocol: PROVIDERS_PROTOCOL.to_owned(),
            rows: Vec::new(),
        }
    }
}

impl ProviderMap {
    /// Derive the map from the installed packages and the ledger.
    ///
    /// A malformed row in a package is skipped rather than raised: the map
    /// is a view, and `vibe check` is where a package's own rows are
    /// judged. Refusing to build the map would make an unrelated
    /// uninstall impossible.
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#providers")]
    pub fn derive(packages: &[RemapPackage], ledger: &ConsentLedger) -> Self {
        let mut rows: Vec<ProvidedRow> = Vec::new();
        for package in packages {
            let label = package.label();
            let applies = ledger.state(&package.key, &package.content_hash).applies();
            let Ok(loaded) = package.loaded(Layer::UserPackage) else {
                continue;
            };
            for row in &loaded.rows {
                let identity = RowIdentity::of(row);
                let found = rows.iter().position(|found| found.row == identity);
                let position = match found {
                    Some(position) => position,
                    None => {
                        rows.push(ProvidedRow {
                            row: identity,
                            providers: Vec::new(),
                            accepted_providers: Vec::new(),
                        });
                        rows.len() - 1
                    }
                };
                let entry = &mut rows[position];
                if !entry.providers.contains(&label) {
                    entry.providers.push(label.clone());
                }
                if applies && !entry.accepted_providers.contains(&label) {
                    entry.accepted_providers.push(label.clone());
                }
            }
        }
        for entry in &mut rows {
            entry.providers.sort();
            entry.accepted_providers.sort();
        }
        rows.sort_by(|left, right| left.row.cmp(&right.row));
        ProviderMap {
            protocol: PROVIDERS_PROTOCOL.to_owned(),
            rows,
        }
    }

    /// The providers of one row, as `remap list` shows them.
    pub fn providers_of(&self, row: &RowIdentity) -> &[String] {
        self.rows
            .iter()
            .find(|found| &found.row == row)
            .map_or(&[], |found| found.providers.as_slice())
    }

    /// The rows `package` offers that **no other accepted package** does,
    /// and the rows that survive because one still does — the two blocks
    /// the uninstall question shows (`PROP-066#uninstall`).
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#uninstall")]
    pub fn split_on_removal(&self, label: &str) -> (Vec<RowIdentity>, Vec<RowIdentity>) {
        let mut orphaned = Vec::new();
        let mut surviving = Vec::new();
        for entry in &self.rows {
            if !entry.accepted_providers.iter().any(|name| name == label) {
                continue;
            }
            if entry
                .accepted_providers
                .iter()
                .any(|name| name.as_str() != label)
            {
                surviving.push(entry.row.clone());
            } else {
                orphaned.push(entry.row.clone());
            }
        }
        (orphaned, surviving)
    }

    /// Write the map under the home's lock. The caller holds
    /// [`super::consent::lock`]; this function only replaces the file.
    pub fn store(&self, home: &Path) -> Result<(), ConsentError> {
        let path = providers_path(home);
        let body =
            serde_json::to_string_pretty(self).map_err(|source| ConsentError::Malformed {
                path: path.display().to_string(),
                source,
            })?;
        super::consent::write_map(&path, format!("{body}\n").as_bytes())
    }

    /// Read the map of `home`; a missing or unreadable file is an empty
    /// map, because the map is derived and the next write replaces it.
    pub fn load(home: &Path) -> Self {
        std::fs::read_to_string(providers_path(home))
            .ok()
            .and_then(|body| serde_json::from_str(&body).ok())
            .unwrap_or_default()
    }
}

/// `<home>/remap/reachability.json` — what the last `remap test` found
/// (`PROP-066#diagnostics-reachability`).
pub fn reachability_path(home: &Path) -> PathBuf {
    remap_dir(home).join("reachability.json")
}

/// The answer one target gave the last time it was probed.
///
/// Every view of a row states it, so a reader judging a redirection knows
/// whether the place it points at was there — but it is a **record of a
/// past probe**, never a live claim: the view says when it was taken, and
/// a target with no record says nothing rather than guessing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#diagnostics-reachability")]
pub struct Reachability {
    /// Target (as displayed) → the last answer.
    #[serde(default)]
    pub targets: std::collections::BTreeMap<String, Probed>,
}

/// One recorded probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#diagnostics-reachability")]
pub struct Probed {
    /// `reachable`, `unreachable`, `denied`, `skipped` — the outcome
    /// vocabulary of `vibe-remap-test/1`.
    pub outcome: String,
    /// When the probe ran, RFC 3339.
    pub when: String,
}

impl Reachability {
    /// Read the record of `home`; a missing or unreadable file is an empty
    /// record, because nothing has been probed yet.
    pub fn load(home: &Path) -> Self {
        std::fs::read_to_string(reachability_path(home))
            .ok()
            .and_then(|body| serde_json::from_str(&body).ok())
            .unwrap_or_default()
    }

    /// Write the record of `home`.
    pub fn store(&self, home: &Path) -> Result<(), ConsentError> {
        let path = reachability_path(home);
        let body =
            serde_json::to_string_pretty(self).map_err(|source| ConsentError::Malformed {
                path: path.display().to_string(),
                source,
            })?;
        super::consent::write_map(&path, format!("{body}\n").as_bytes())
    }

    /// The one-line note a view prints for `target`, when one was taken.
    ///
    /// ```
    /// use vibe_core::remap::{Probed, Reachability};
    ///
    /// let mut record = Reachability::default();
    /// assert!(record.note("https://nexus/vibespecs").is_none());
    /// record.targets.insert(
    ///     "https://nexus/vibespecs".into(),
    ///     Probed { outcome: "reachable".into(), when: "2026-11-01T00:00:00Z".into() },
    /// );
    /// let note = record.note("https://nexus/vibespecs").unwrap();
    /// assert!(note.starts_with("reachable at "));
    /// ```
    pub fn note(&self, target: &str) -> Option<String> {
        let probed = self.targets.get(target)?;
        Some(format!("{} at {}", probed.outcome, probed.when))
    }
}
