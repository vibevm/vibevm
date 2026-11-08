//! One `[[remap]]` row and the per-file `[policy]` table
//! (`PROP-066#row`, `PROP-066#policy`).
//!
//! A row says three things and nothing else: **what** it matches (exactly
//! one key field), **with** what (exactly one target field), and **where**
//! it applies (an optional scope). The TOML surface is deliberately flat —
//! a reader sees `sha256 = …` next to `path = …` and needs no nesting to
//! understand the pair — which means "exactly one of each" cannot be a
//! type invariant of the wire form. [`RowDecl`] is that wire form;
//! [`RowDecl::into_row`] is where the pair becomes a [`RemapRow`] or a
//! parse error naming the offending row.
//!
//! Nothing here reads a file, resolves anything, or knows about layers:
//! `layers.rs` builds rows from sources and `locator.rs` chooses among
//! them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#row");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::layers::Layer;

/// What a row matches — exactly one of the four keys of `PROP-066#row`.
///
/// The variants are ordered by the specificity the precedence rule gives
/// them, most specific first, so [`RemapKey::specificity`] is the
/// discriminant's own order rather than a second table that could drift
/// from it.
///
/// ```
/// use vibe_core::remap::RemapKey;
///
/// let digest = RemapKey::Sha256("ab".repeat(32));
/// let prefix = RemapKey::Prefix("https://github.com/".into());
/// // sha256 is the most specific key, prefix the least.
/// assert!(digest.specificity() > prefix.specificity());
/// assert_eq!(digest.kind(), "sha256");
/// assert_eq!(prefix.what(), "https://github.com/");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemapKey {
    /// `sha256 = "<hex>"` — any artifact declared with that digest,
    /// whatever its URL. Stored lowercased.
    Sha256(String),
    /// `package = "<group>/<name>[@<req>]"` — the registry package, its
    /// index entries and its embedded sources.
    Package(String),
    /// `registry = "<name>"` — every URL a declared registry resolves to.
    Registry(String),
    /// `prefix = "<url prefix>"` — every URL starting with it; the host
    /// rewrite.
    Prefix(String),
}

impl RemapKey {
    /// The precedence rank of this key kind: `sha256` 3, `package` 2,
    /// `registry` 1, `prefix` 0 (`PROP-066#precedence`).
    pub fn specificity(&self) -> u8 {
        match self {
            RemapKey::Sha256(_) => 3,
            RemapKey::Package(_) => 2,
            RemapKey::Registry(_) => 1,
            RemapKey::Prefix(_) => 0,
        }
    }

    /// The key's TOML field name, as `remap list` prints it.
    pub fn kind(&self) -> &'static str {
        match self {
            RemapKey::Sha256(_) => "sha256",
            RemapKey::Package(_) => "package",
            RemapKey::Registry(_) => "registry",
            RemapKey::Prefix(_) => "prefix",
        }
    }

    /// The matched value — the hex digest, the pkgref, the registry name
    /// or the URL prefix.
    pub fn what(&self) -> &str {
        match self {
            RemapKey::Sha256(value)
            | RemapKey::Package(value)
            | RemapKey::Registry(value)
            | RemapKey::Prefix(value) => value,
        }
    }
}

/// What a row redirects to — exactly one of the three target fields of
/// `PROP-066#row`.
///
/// ```
/// use vibe_core::remap::RemapTarget;
/// use std::path::PathBuf;
///
/// let url = RemapTarget::Url("https://nexus.corp.example/vibe".into());
/// assert_eq!(url.display(), "https://nexus.corp.example/vibe");
/// assert!(!url.is_deny());
/// assert!(RemapTarget::Deny.is_deny());
/// assert_eq!(RemapTarget::Path(PathBuf::from("/srv/a")).display(), "/srv/a");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemapTarget {
    /// `url = "…"` — an `https://`, `http://`, `file://` or git URL. For a
    /// `prefix` key the matched prefix is replaced by it; for every other
    /// key it is the whole new location.
    Url(String),
    /// `path = "…"` — a local file, or a directory searched as
    /// `<path>/<sha256>/<filename>` the way the application download cache
    /// is laid out.
    Path(PathBuf),
    /// `deny = true` — the fetch is refused before any connection, naming
    /// the row.
    Deny,
}

impl RemapTarget {
    /// The target as a human and a `--json` document show it; `deny` for
    /// the refusal.
    pub fn display(&self) -> String {
        match self {
            RemapTarget::Url(url) => url.clone(),
            RemapTarget::Path(path) => path.display().to_string(),
            RemapTarget::Deny => "deny".to_owned(),
        }
    }

    /// Whether this target refuses the fetch rather than relocating it.
    pub fn is_deny(&self) -> bool {
        matches!(self, RemapTarget::Deny)
    }
}

/// Where a row applies — `scope = { package = "…" [, step = "ID"] }`.
///
/// ```
/// use vibe_core::remap::RemapScope;
///
/// let whole: RemapScope = toml::from_str("package = \"org.example/tool\"").unwrap();
/// assert_eq!(whole.specificity(), 1);
/// let one_step: RemapScope =
///     toml::from_str("package = \"org.example/tool\"\nstep = \"fetch\"").unwrap();
/// assert_eq!(one_step.specificity(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemapScope {
    /// The application package whose installation the row is limited to.
    pub package: String,
    /// One step of that package, by `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
}

impl RemapScope {
    /// How narrow this scope is: 1 for a package, 2 for one of its steps.
    /// An unscoped row is 0 (`PROP-066#precedence`).
    pub fn specificity(&self) -> u8 {
        if self.step.is_some() { 2 } else { 1 }
    }
}

/// Where a row came from: which layer, which file or package, and its
/// position in that source (`PROP-066#precedence` breaks a tie by the
/// later row within one layer, so the index is part of the row's identity,
/// not a display detail).
///
/// ```
/// use vibe_core::remap::{Layer, Provenance};
///
/// let from = Provenance::new(Layer::Local, "~/.vibe/settings.toml", 0);
/// assert_eq!(from.layer, Layer::Local);
/// assert_eq!(from.index, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Runtime report visibility; omitted from existing wire documents.
    #[serde(skip)]
    pub publication: crate::RoutePublication,
    /// Catalog-generated mirror/override rows cannot retarget a held source.
    #[serde(skip)]
    pub catalog: bool,
    /// The layer of `PROP-066#layers` this row belongs to.
    pub layer: Layer,
    /// The file that declared it, or the package that offers it.
    pub source: String,
    /// Zero-based position within that source.
    pub index: usize,
}

impl Provenance {
    pub fn catalog(mut self, publication: crate::RoutePublication) -> Self {
        self.catalog = true;
        self.publication = publication;
        self
    }
    /// Build a provenance from its three parts.
    pub fn new(layer: Layer, source: impl Into<String>, index: usize) -> Self {
        Provenance {
            publication: crate::RoutePublication::Local,
            catalog: false,
            layer,
            source: source.into(),
            index,
        }
    }
}

/// One resolved `[[remap]]` row: the three parts of `PROP-066#row` plus
/// the provenance the precedence rule and every display need.
///
/// ```
/// use vibe_core::remap::{Layer, Provenance, RemapRow, RemapTarget, RowDecl};
///
/// let decl: RowDecl = toml::from_str(
///     "sha256 = \"ab\"\npath = \"/srv/artifacts\"\nreason = \"air-gapped\"",
/// )
/// .unwrap();
/// let row: RemapRow = decl
///     .clone()
///     .into_row(Provenance::new(Layer::Local, "~/.vibe/settings.toml", 0))
///     .unwrap();
/// assert_eq!(row.key.kind(), "sha256");
/// assert!(matches!(row.target, RemapTarget::Path(_)));
/// assert_eq!(row.reason.as_deref(), Some("air-gapped"));
/// // The row round-trips back to the flat TOML form it was read from.
/// assert_eq!(row.to_decl(), decl);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemapRow {
    /// What this row matches.
    pub key: RemapKey,
    /// What it redirects to, or its refusal.
    pub target: RemapTarget,
    /// The installation, or the one step, it is limited to.
    pub scope: Option<RemapScope>,
    /// Why — shown by `explain` and in refusals.
    pub reason: Option<String>,
    /// Which layer, file and position produced it.
    pub from: Provenance,
    /// `from = "<package>@<version>"` — the remap package a kept row
    /// outlived (`PROP-066#uninstall`). Written into the user's local
    /// layer when an uninstall copies an orphaned row there, so `list`
    /// still says where the row came from after the package is gone.
    pub from_package: Option<String>,
    /// The git ref a `[[override]]` pinned alongside its `source_url`
    /// (`PROP-066#layers` reads an override as a `package` → `url` row and
    /// carries its `ref` here). No TOML `[[remap]]` row sets it: the key
    /// is a target detail of the PROP-002 tables, kept beside the target
    /// rather than folded into the URL so the canonical spelling of both
    /// stays recoverable.
    pub target_ref: Option<String>,
    /// The one kind of artifact this row applies to, when it is not every
    /// kind. Only the built-in single-file rows set it (`RawFile`): they
    /// stand in for PROP-002's HTTPS single-file read, and a `git clone` of
    /// the same host must not be sent to that endpoint. No TOML row sets
    /// it, so it is not part of the written form.
    #[serde(skip)]
    pub only: Option<super::ArtifactKind>,
    /// How a `path` target is searched (`PROP-066#drops`). Every row
    /// written in TOML is [`PathLookup::Mirror`]; only the lowering of an
    /// `[artifacts] packages` drop produces [`PathLookup::PackageDrop`].
    #[serde(skip)]
    pub lookup: PathLookup,
}

/// How the bytes behind a `path` target are found (`PROP-066#drops`).
///
/// ```
/// use vibe_core::remap::PathLookup;
///
/// assert_eq!(PathLookup::default(), PathLookup::Mirror);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathLookup {
    /// The directory mirrors the declared URL: a `prefix` key's remainder
    /// is joined onto it, and `<path>/<sha256>/<filename>` is searched too.
    #[default]
    Mirror,
    /// An `[artifacts] packages` drop, searched by the package convention
    /// of [`crate::remap::drop::package_lookup`].
    PackageDrop,
}

impl RemapRow {
    /// The flat TOML form of this row, without its provenance — the shape
    /// `[[remap]]` is written in, and the half of a row that round-trips.
    pub fn to_decl(&self) -> RowDecl {
        let mut decl = RowDecl {
            scope: self.scope.clone(),
            reason: self.reason.clone(),
            from: self.from_package.clone(),
            ..RowDecl::default()
        };
        match &self.key {
            RemapKey::Sha256(value) => decl.sha256 = Some(value.clone()),
            RemapKey::Package(value) => decl.package = Some(value.clone()),
            RemapKey::Registry(value) => decl.registry = Some(value.clone()),
            RemapKey::Prefix(value) => decl.prefix = Some(value.clone()),
        }
        match &self.target {
            RemapTarget::Url(url) => decl.url = Some(url.clone()),
            RemapTarget::Path(path) => decl.path = Some(path.clone()),
            RemapTarget::Deny => decl.deny = Some(true),
        }
        decl
    }
}

/// The flat `[[remap]]` table as it is written: every key field and every
/// target field optional, because "exactly one of each" is a property of
/// the pair and is checked by [`RowDecl::into_row`], which can name the
/// row it rejects.
///
/// ```
/// use vibe_core::remap::{Layer, Provenance, RowDecl};
///
/// // Exactly one key and one target: accepted.
/// let good: RowDecl =
///     toml::from_str("prefix = \"https://github.com/\"\nurl = \"https://mirror/\"").unwrap();
/// assert!(good.into_row(Provenance::new(Layer::Invocation, "--remap", 0)).is_ok());
///
/// // Two keys: a parse error that names the row's position.
/// let two: RowDecl =
///     toml::from_str("prefix = \"https://a/\"\nregistry = \"r\"\nurl = \"https://b/\"").unwrap();
/// let error = two
///     .into_row(Provenance::new(Layer::Invocation, "--remap", 1))
///     .unwrap_err();
/// assert!(error.to_string().contains("row 2"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowDecl {
    /// `sha256 = "<hex>"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// `package = "<group>/<name>[@<req>]"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// `registry = "<name>"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// `prefix = "<url prefix>"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// `url = "…"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// `path = "…"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// `deny = true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deny: Option<bool>,
    /// `scope = { package = "…", step = "…" }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<RemapScope>,
    /// `reason = "…"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// `from = "<package>@<version>"` — present on a row an uninstall
    /// copied out of a remap package into the local layer
    /// (`PROP-066#uninstall`). Read here as well as written, because a
    /// settings file carrying one has to keep parsing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

impl RowDecl {
    /// Validate the "exactly one key, exactly one target" rule of
    /// `PROP-066#row` and attach the provenance. The error names the row
    /// by its one-based position in its source, because that is what a
    /// reader has in front of them in the file.
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#row")]
    pub fn into_row(self, from: Provenance) -> Result<RemapRow, RemapParseError> {
        let keys: Vec<RemapKey> = [
            self.sha256
                .as_ref()
                .map(|v| RemapKey::Sha256(v.to_ascii_lowercase())),
            self.package.as_ref().map(|v| RemapKey::Package(v.clone())),
            self.registry
                .as_ref()
                .map(|v| RemapKey::Registry(v.clone())),
            self.prefix.as_ref().map(|v| RemapKey::Prefix(v.clone())),
        ]
        .into_iter()
        .flatten()
        .collect();
        let targets: Vec<RemapTarget> = [
            self.url.as_ref().map(|v| RemapTarget::Url(v.clone())),
            self.path.as_ref().map(|v| RemapTarget::Path(v.clone())),
            self.deny.unwrap_or(false).then_some(RemapTarget::Deny),
        ]
        .into_iter()
        .flatten()
        .collect();
        let named = |part: &'static str, found: usize| RemapParseError::RowShape {
            file: from.source.clone(),
            row: from.index + 1,
            part,
            found,
        };
        if keys.len() != 1 {
            return Err(named("key", keys.len()));
        }
        if targets.len() != 1 {
            return Err(named("target", targets.len()));
        }
        let mut keys = keys;
        let mut targets = targets;
        Ok(RemapRow {
            key: keys.remove(0),
            target: targets.remove(0),
            scope: self.scope,
            reason: self.reason,
            from_package: self.from,
            from,
            target_ref: None,
            only: None,
            lookup: PathLookup::Mirror,
        })
    }
}

/// `[policy] unmatched = "allow" | "deny"` — what happens to a fetch no
/// row matched (`PROP-066#policy`).
///
/// ```
/// use vibe_core::remap::UnmatchedPolicy;
///
/// // Allow is the default: a remap file that says nothing changes nothing.
/// assert_eq!(UnmatchedPolicy::default(), UnmatchedPolicy::Allow);
/// // The strictest policy of all layers applies, so `deny` absorbs `allow`.
/// assert_eq!(
///     UnmatchedPolicy::Allow.strictest(UnmatchedPolicy::Deny),
///     UnmatchedPolicy::Deny,
/// );
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnmatchedPolicy {
    /// An unmatched fetch goes to its declared origin. Default.
    #[default]
    Allow,
    /// An unmatched fetch is refused before any connection.
    Deny,
}

impl UnmatchedPolicy {
    /// The stricter of two policies — the rule for combining layers
    /// (`PROP-066#policy`).
    pub fn strictest(self, other: UnmatchedPolicy) -> UnmatchedPolicy {
        match (self, other) {
            (UnmatchedPolicy::Deny, _) | (_, UnmatchedPolicy::Deny) => UnmatchedPolicy::Deny,
            _ => UnmatchedPolicy::Allow,
        }
    }
}

/// The `[policy]` table of one remap file.
///
/// ```
/// use vibe_core::remap::{PolicySection, UnmatchedPolicy};
///
/// let p: PolicySection = toml::from_str("unmatched = \"deny\"").unwrap();
/// assert_eq!(p.unmatched, UnmatchedPolicy::Deny);
/// assert!(!p.is_default());
/// assert_eq!(PolicySection::default().unmatched, UnmatchedPolicy::Allow);
/// assert!(PolicySection::default().is_default());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicySection {
    /// What to do with a fetch no row matched.
    #[serde(default)]
    pub unmatched: UnmatchedPolicy,
}

impl PolicySection {
    /// `true` when the table says nothing — lets a serializer skip it on a
    /// manifest that declared no policy at all.
    pub fn is_default(&self) -> bool {
        self.unmatched == UnmatchedPolicy::Allow
    }
}

/// A whole remap file: `[[remap]]` rows and an optional `[policy]` table.
/// Unknown top-level keys are accepted so a file written for a later
/// revision of `PROP-066` — a `[remap] description`, the consent fields of
/// R-2 — still parses here instead of failing a run that only needed the
/// rows.
///
/// ```
/// use vibe_core::remap::RemapFileDecl;
///
/// let file: RemapFileDecl = toml::from_str(
///     "[policy]\nunmatched = \"deny\"\n\
///      [[remap]]\nregistry = \"vibespecs\"\nurl = \"https://nexus.corp.example/vibespecs\"\n",
/// )
/// .unwrap();
/// assert_eq!(file.remap.len(), 1);
/// assert!(matches!(file.policy.unmatched, vibe_core::remap::UnmatchedPolicy::Deny));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemapFileDecl {
    /// The `[[remap]]` rows, in file order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remap: Vec<RowDecl>,
    /// The file's `[policy]` table.
    #[serde(default)]
    pub policy: PolicySection,
}

/// Why a remap source could not be read (`PROP-066#row`).
///
/// ```
/// use vibe_core::remap::RemapParseError;
///
/// let e = RemapParseError::RowShape {
///     file: "~/.vibe/settings.toml".into(),
///     row: 3,
///     part: "target",
///     found: 0,
/// };
/// assert!(e.to_string().contains("row 3"));
/// assert!(e.to_string().contains("PROP-066#row"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#row")]
pub enum RemapParseError {
    #[error(
        "`{file}` row {row} declares {found} {part} fields, not exactly one \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#row; \
          fix: give the row exactly one of `sha256`/`package`/`registry`/`prefix` \
          and exactly one of `url`/`path`/`deny`)"
    )]
    RowShape {
        file: String,
        row: usize,
        part: &'static str,
        found: usize,
    },
    #[error(
        "could not read the remap file `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#layers; \
          fix: check the path and its permissions, or drop the `--remap` / \
          `VIBE_REMAP` entry naming it)"
    )]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "the remap file `{path}` is malformed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#row; \
          fix: repair the TOML at the reported location)"
    )]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}
