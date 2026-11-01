//! Remap packages and the offer they make (`PROP-066#package`).
//!
//! A remap package is an ordinary vibepack that happens to carry
//! `[[remap]]` rows: versioned, content-hashed, published to any registry,
//! installed into a user home or required by a project. What makes it
//! different from a settings file is that its rows are an **offer**, never
//! an obligation — a package author must not be able to redirect another
//! person's downloads by being installed — so nothing here produces rows
//! of the Locator until the acceptance ledger of
//! [`super::consent`] says the user agreed.
//!
//! The TOML surface deserves a word. `PROP-066#package` asks for a
//! `[remap] description` beside `[[remap]]` rows, and TOML cannot carry
//! both spellings of one key in one document: `[remap]` and `[[remap]]`
//! are a redefinition error. So the key is read in either of two shapes —
//! the bare array of rows, or a table carrying `description`, `priority`
//! and its rows under `[[remap.rows]]` — and [`RemapSection`] is the one
//! place that difference is resolved.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#package");

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::layers::{Layer, LoadedSource};
use super::row::{
    PolicySection, Provenance, RemapParseError, RemapRow, RemapScope, RemapTarget, RowDecl,
    UnmatchedPolicy,
};

/// The `remap` key of a manifest, in either of the two shapes TOML allows
/// for it.
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// // The bare array: `[[remap]]` rows and nothing else.
/// let rows = Manifest::parse_str(
///     "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\
///      [[remap]]\nregistry = \"r\"\nurl = \"https://nexus/r\"\n",
/// )
/// .unwrap();
/// assert_eq!(rows.remap.rows.len(), 1);
/// assert!(rows.remap.description.is_none());
///
/// // The table: a description and a priority above the same rows.
/// let described = Manifest::parse_str(
///     "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\
///      [remap]\ndescription = \"the corp network\"\npriority = 10\n\
///      [[remap.rows]]\nregistry = \"r\"\nurl = \"https://nexus/r\"\n",
/// )
/// .unwrap();
/// assert_eq!(described.remap.description.as_deref(), Some("the corp network"));
/// assert_eq!(described.remap.priority, 10);
/// assert_eq!(described.remap.rows.len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemapSection {
    /// `[remap] description` — what network this offer is for, who
    /// maintains it, where to ask. Heads every question and listing about
    /// the package (`PROP-066#row-offer-description`).
    pub description: Option<String>,
    /// `[remap] priority` — higher wins among accepted packages
    /// (`PROP-066#package`).
    pub priority: i64,
    /// The rows, in declaration order.
    pub rows: Vec<RowDecl>,
}

impl RemapSection {
    /// `true` when the section carries nothing at all — the shape a
    /// manifest that never mentions remaps takes, so the serializer skips
    /// the key entirely.
    pub fn is_empty(&self) -> bool {
        self.description.is_none() && self.priority == 0 && self.rows.is_empty()
    }
}

/// The wire form of [`RemapSection`]: the array spelling or the table
/// spelling, told apart by shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#package")]
pub enum RemapSectionWire {
    /// `[[remap]]` — rows only.
    Rows(Vec<RowDecl>),
    /// `[remap]` with `description` / `priority` and `[[remap.rows]]`.
    Described(DescribedRemapWire),
}

/// The table spelling of the `remap` key.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#row-offer-description")]
pub struct DescribedRemapWire {
    /// What the offer is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Higher wins among accepted packages.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub priority: i64,
    /// `[[remap.rows]]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<RowDecl>,
}

fn is_zero(value: &i64) -> bool {
    *value == 0
}

impl From<RemapSectionWire> for RemapSection {
    fn from(wire: RemapSectionWire) -> Self {
        match wire {
            RemapSectionWire::Rows(rows) => RemapSection {
                description: None,
                priority: 0,
                rows,
            },
            RemapSectionWire::Described(table) => RemapSection {
                description: table.description,
                priority: table.priority,
                rows: table.rows,
            },
        }
    }
}

impl From<RemapSection> for RemapSectionWire {
    fn from(section: RemapSection) -> Self {
        // The array spelling is what a package with no description wrote
        // and what it must read back as; only a described offer needs the
        // table, and a round-trip that changed the spelling would make
        // `vibe check` disagree with the file on disk.
        if section.description.is_none() && section.priority == 0 {
            return RemapSectionWire::Rows(section.rows);
        }
        RemapSectionWire::Described(DescribedRemapWire {
            description: section.description,
            priority: section.priority,
            rows: section.rows,
        })
    }
}

impl<'de> Deserialize<'de> for RemapSection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        RemapSectionWire::deserialize(deserializer).map(Into::into)
    }
}

impl Serialize for RemapSection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        RemapSectionWire::from(self.clone()).serialize(serializer)
    }
}

/// One installed remap package: its identity, the offer it carries, and
/// the content hash the acceptance is bound to (`PROP-066#consent`).
///
/// ```
/// use vibe_core::remap::{RemapPackage, RemapSection, RowDecl};
///
/// let row: RowDecl = toml::from_str("registry = \"r\"\nurl = \"https://nexus/r\"").unwrap();
/// let package = RemapPackage {
///     key: "org.corp/intranet".into(),
///     version: "1.0.0".into(),
///     content_hash: "sha256:ab".into(),
///     section: RemapSection {
///         description: Some("the corp network".into()),
///         priority: 0,
///         rows: vec![row],
///     },
///     policy: Default::default(),
/// };
/// assert_eq!(package.label(), "org.corp/intranet@1.0.0");
/// // Its rows become a layer's source once the user accepted them.
/// let loaded = package.loaded(vibe_core::remap::Layer::UserPackage).unwrap();
/// assert_eq!(loaded.rows.len(), 1);
/// assert_eq!(loaded.source, "org.corp/intranet@1.0.0");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemapPackage {
    /// `<group>/<name>` — the key the ledger and every verb name it by.
    pub key: String,
    /// The installed version.
    pub version: String,
    /// `sha256:<hex>` over the package tree. A new hash is a new offer.
    pub content_hash: String,
    /// The offer: description, priority and rows.
    pub section: RemapSection,
    /// The `[policy]` the package declared.
    pub policy: PolicySection,
}

impl RemapPackage {
    /// `<key>@<version>` — how every row's provenance, question and
    /// listing names the package.
    pub fn label(&self) -> String {
        format!("{}@{}", self.key, self.version)
    }

    /// The package's rows as one source of `layer`, with its `[policy]`.
    ///
    /// The error names the offending row the way a remap file's does: a
    /// malformed row in a package is the package author's mistake, and the
    /// reader needs its position to report it.
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#package")]
    pub fn loaded(&self, layer: Layer) -> Result<LoadedSource, RemapParseError> {
        let source = self.label();
        let mut rows = Vec::with_capacity(self.section.rows.len());
        for (index, row) in self.section.rows.iter().enumerate() {
            rows.push(
                row.clone()
                    .into_row(Provenance::new(layer, source.clone(), index))?,
            );
        }
        Ok(LoadedSource {
            layer,
            source,
            rows,
            policy: self.policy.unmatched,
        })
    }
}

/// Where a row came from in terms a provider map can compare: the key, the
/// target and the scope, and nothing else (`PROP-066#providers`).
///
/// Two rows are the same row when those three agree — the reason, the
/// package and the layer are not part of the identity, because a row two
/// accepted packages both offer is one row with two providers.
///
/// ```
/// use vibe_core::remap::{Layer, Provenance, RowDecl, RowIdentity};
///
/// let one: RowDecl = toml::from_str("registry = \"r\"\nurl = \"https://nexus/r\"").unwrap();
/// let two: RowDecl = toml::from_str(
///     "registry = \"r\"\nurl = \"https://nexus/r\"\nreason = \"said twice\"",
/// )
/// .unwrap();
/// let left = RowIdentity::of(&one.into_row(Provenance::new(Layer::UserPackage, "a", 0)).unwrap());
/// let right = RowIdentity::of(&two.into_row(Provenance::new(Layer::UserPackage, "b", 7)).unwrap());
/// // Same key, target and scope — the differing reason and provenance do
/// // not make it a second row.
/// assert_eq!(left, right);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RowIdentity {
    /// The key kind, as `remap list` prints it.
    pub kind: String,
    /// The matched value.
    pub what: String,
    /// The target, as it is displayed (`deny` for a refusal).
    pub to: String,
    /// `scope.package`, when the row is scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_package: Option<String>,
    /// `scope.step`, when the scope names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_step: Option<String>,
}

impl RowIdentity {
    /// The identity of one row.
    pub fn of(row: &RemapRow) -> Self {
        RowIdentity {
            kind: row.key.kind().to_owned(),
            what: row.key.what().to_owned(),
            to: row.target.display(),
            scope_package: row.scope.as_ref().map(|scope| scope.package.clone()),
            scope_step: row.scope.as_ref().and_then(|scope| scope.step.clone()),
        }
    }

    /// The row's one-line spelling, for a question or a listing.
    pub fn display(&self) -> String {
        let mut shown = format!("{} {} → {}", self.kind, self.what, self.to);
        if let Some(package) = &self.scope_package {
            shown.push_str(&format!(" (scope {package}"));
            match &self.scope_step {
                Some(step) => shown.push_str(&format!(", step {step})")),
                None => shown.push(')'),
            }
        }
        shown
    }

    /// The scope this identity describes, rebuilt.
    pub fn scope(&self) -> Option<RemapScope> {
        self.scope_package.as_ref().map(|package| RemapScope {
            package: package.clone(),
            step: self.scope_step.clone(),
        })
    }
}

/// A row copied into the user home's local layer when the package that
/// offered it is uninstalled (`PROP-066#uninstall`).
///
/// The copy keeps the original reason and records `from = "<package>@<version>"`,
/// so the machine keeps working and `remap list` still says where the row
/// came from.
///
/// ```
/// use vibe_core::remap::{KeptRow, RowIdentity};
///
/// let kept = KeptRow {
///     identity: RowIdentity {
///         kind: "registry".into(),
///         what: "r".into(),
///         to: "https://nexus/r".into(),
///         scope_package: None,
///         scope_step: None,
///     },
///     reason: Some("the only host the proxy allows".into()),
///     from: "org.corp/intranet@1.0.0".into(),
/// };
/// let toml = kept.to_toml().unwrap();
/// assert!(toml.contains("registry = \"r\""));
/// assert!(toml.contains("from = \"org.corp/intranet@1.0.0\""));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptRow {
    /// The row itself.
    pub identity: RowIdentity,
    /// The reason the package gave.
    pub reason: Option<String>,
    /// `<package>@<version>` — the offer this row outlived.
    pub from: String,
}

impl KeptRow {
    /// The `[[remap]]` table this row is appended to the user settings as.
    ///
    /// `from` is written beside the row rather than folded into `reason`:
    /// `PROP-066#uninstall` asks for both, and a reader of the settings
    /// file has to be able to tell the package's own words from vibe's
    /// note about where they came from.
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#uninstall")]
    pub fn to_toml(&self) -> Result<String, String> {
        let mut out = String::from("\n[[remap]]\n");
        out.push_str(&format!(
            "{} = {}\n",
            self.identity.kind,
            quoted(&self.identity.what)
        ));
        if self.identity.to == RemapTarget::Deny.display() {
            out.push_str("deny = true\n");
        } else if self.identity.to.contains("://") {
            out.push_str(&format!("url = {}\n", quoted(&self.identity.to)));
        } else {
            out.push_str(&format!("path = {}\n", quoted(&self.identity.to)));
        }
        if let Some(scope) = self.identity.scope() {
            match &scope.step {
                Some(step) => out.push_str(&format!(
                    "scope = {{ package = {}, step = {} }}\n",
                    quoted(&scope.package),
                    quoted(step)
                )),
                None => out.push_str(&format!(
                    "scope = {{ package = {} }}\n",
                    quoted(&scope.package)
                )),
            }
        }
        if let Some(reason) = &self.reason {
            out.push_str(&format!("reason = {}\n", quoted(reason)));
        }
        out.push_str(&format!("from = {}\n", quoted(&self.from)));
        Ok(out)
    }
}

/// A TOML basic string. The `from` key a kept row carries is not part of
/// [`RowDecl`]'s `deny_unknown_fields` surface, so the writer is here
/// rather than `toml::to_string` over a type that would refuse to read its
/// own output.
fn quoted(value: &str) -> String {
    let escaped: String = value
        .chars()
        .flat_map(|character| match character {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            other => vec![other],
        })
        .collect();
    format!("\"{escaped}\"")
}

/// The strictest policy of a set of packages — the rule
/// `PROP-066#policy` applies across layers, applied here across the
/// accepted packages of one home.
pub fn strictest_policy(packages: &[RemapPackage]) -> UnmatchedPolicy {
    packages
        .iter()
        .fold(UnmatchedPolicy::Allow, |strictest, package| {
            strictest.strictest(package.policy.unmatched)
        })
}
