//! The layers rows come from, and how each source is turned into rows
//! (`PROP-066#layers`).
//!
//! Four layers, lowest first: remap packages accepted in the user home,
//! remap packages a project requires, the local rows a developer wrote
//! themselves, and the invocation. Only the last two produce rows today —
//! the package variants exist because the precedence rule has to rank them
//! and because a row's provenance must be able to say "a package", not
//! merely "a file".
//!
//! Three kinds of source feed the local layer, and the third is the reason
//! nothing a user already wrote stops working: PROP-002's `[[mirror]]` and
//! `[[override]]` tables are read as rows here
//! ([`rows_from_mirrors`], [`rows_from_overrides`]) rather than kept as a
//! second, parallel redirection mechanism.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#layers");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::manifest::{MirrorSection, OverrideSection, RegistrySection};

use super::row::{
    Provenance, RemapFileDecl, RemapKey, RemapParseError, RemapRow, RemapTarget, RowDecl,
    UnmatchedPolicy,
};

/// The four layers of `PROP-066#layers`, lowest precedence first.
///
/// ```
/// use vibe_core::remap::Layer;
///
/// // The invocation is the highest layer; an accepted user package the lowest.
/// assert!(Layer::Invocation.rank() > Layer::Local.rank());
/// assert!(Layer::Local.rank() > Layer::ProjectPackage.rank());
/// assert!(Layer::ProjectPackage.rank() > Layer::UserPackage.rank());
/// assert_eq!(Layer::Local.label(), "local");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// A remap package the user accepted in the user home. No source
    /// produces these rows yet — consent is R-2.
    UserPackage,
    /// A remap package the project requires in `vibe.toml`. No source
    /// produces these rows yet — consent is R-2.
    ProjectPackage,
    /// Rows the user wrote on this machine: `~/.vibe/settings.toml`,
    /// `.vibe/settings.local.toml`, and the PROP-002 `[[mirror]]` /
    /// `[[override]]` tables.
    Local,
    /// `--remap <file>` (repeatable) and `VIBE_REMAP=<file>[,<file>]`.
    Invocation,
}

impl Layer {
    /// The precedence rank — higher wins among equally specific rows
    /// (`PROP-066#precedence`).
    pub fn rank(self) -> u8 {
        match self {
            Layer::UserPackage => 0,
            Layer::ProjectPackage => 1,
            Layer::Local => 2,
            Layer::Invocation => 3,
        }
    }

    /// The short name every human view and `--json` document prints.
    pub fn label(self) -> &'static str {
        match self {
            Layer::UserPackage => "user-package",
            Layer::ProjectPackage => "project-package",
            Layer::Local => "local",
            Layer::Invocation => "invocation",
        }
    }
}

/// One source's contribution: its rows in file order and the `[policy]`
/// it declared.
///
/// ```
/// use vibe_core::remap::{Layer, LoadedSource, UnmatchedPolicy};
///
/// let empty = LoadedSource::empty(Layer::Local, "~/.vibe/settings.toml");
/// assert!(empty.rows.is_empty());
/// assert_eq!(empty.policy, UnmatchedPolicy::Allow);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSource {
    /// The layer every row of this source belongs to.
    pub layer: Layer,
    /// The file path or package key the rows came from.
    pub source: String,
    /// The rows, in the order the source declared them.
    pub rows: Vec<RemapRow>,
    /// The source's `[policy] unmatched`.
    pub policy: UnmatchedPolicy,
}

impl LoadedSource {
    /// A source that declared nothing — the shape a missing file takes,
    /// because a missing remap file is an absent layer, not an error.
    pub fn empty(layer: Layer, source: impl Into<String>) -> Self {
        LoadedSource {
            layer,
            source: source.into(),
            rows: Vec::new(),
            policy: UnmatchedPolicy::Allow,
        }
    }
}

/// Read a standalone remap file — what `--remap <file>` and `VIBE_REMAP`
/// name. A missing file is an error here, unlike a settings layer: the
/// user pointed at it by name on this invocation, so silence would hide a
/// typo behind a run that fetched from the public hosts anyway.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
pub fn rows_from_file(path: &Path, layer: Layer) -> Result<LoadedSource, RemapParseError> {
    let shown = path.display().to_string();
    let body = std::fs::read_to_string(path).map_err(|source| RemapParseError::Io {
        path: shown.clone(),
        source,
    })?;
    let decl: RemapFileDecl = toml::from_str(&body).map_err(|source| RemapParseError::Parse {
        path: shown.clone(),
        source,
    })?;
    rows_from_decl(decl, layer, shown)
}

/// Validate already-parsed `[[remap]]` rows — the rows a site profile
/// carries (`PROP-019#site-profile`) — and tag them with their provenance,
/// exactly as a remap file's rows are.
///
/// ```
/// use vibe_core::remap::{rows_from_decls, Layer, RowDecl};
///
/// let row: RowDecl = toml::from_str(
///     "prefix = 'https://example.invalid/'\nurl = 'https://mirror.corp.example/'",
/// ).unwrap();
/// let loaded = rows_from_decls(vec![row], Layer::Local, "site-profile.toml").unwrap();
/// assert_eq!(loaded.rows.len(), 1);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
pub fn rows_from_decls(
    rows: Vec<RowDecl>,
    layer: Layer,
    source: impl Into<String>,
) -> Result<LoadedSource, RemapParseError> {
    rows_from_decl(
        RemapFileDecl {
            remap: rows,
            policy: Default::default(),
        },
        layer,
        source.into(),
    )
}

/// Read the `[[remap]]` rows and `[policy]` table out of an already
/// parsed TOML document — a `settings.toml` or `settings.local.toml` that
/// the PROP-040 loader read for its own reasons. A file with no `[[remap]]`
/// table contributes nothing.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
pub fn rows_from_settings(
    table: &toml::Table,
    layer: Layer,
    source: impl Into<String>,
) -> Result<LoadedSource, RemapParseError> {
    let shown = source.into();
    if !table.contains_key("remap") && !table.contains_key("policy") {
        return Ok(LoadedSource::empty(layer, shown));
    }
    // Re-serialising the two tables we want keeps the typed parse in one
    // place (`RemapFileDecl`) instead of hand-walking `toml::Value`, and
    // it is the same bytes either way: a settings file's `[[remap]]`
    // array is exactly a remap file's.
    let mut narrowed = toml::Table::new();
    for key in ["remap", "policy"] {
        if let Some(value) = table.get(key) {
            narrowed.insert(key.to_owned(), value.clone());
        }
    }
    let decl: RemapFileDecl =
        toml::Value::Table(narrowed)
            .try_into()
            .map_err(|source| RemapParseError::Parse {
                path: shown.clone(),
                source,
            })?;
    rows_from_decl(decl, layer, shown)
}

/// Validate every row of a parsed file and tag it with its provenance.
fn rows_from_decl(
    decl: RemapFileDecl,
    layer: Layer,
    source: String,
) -> Result<LoadedSource, RemapParseError> {
    let mut rows = Vec::with_capacity(decl.remap.len());
    for (index, row) in decl.remap.into_iter().enumerate() {
        rows.push(row.into_row(Provenance::new(layer, source.clone(), index))?);
    }
    Ok(LoadedSource {
        layer,
        source,
        rows,
        policy: decl.policy.unmatched,
    })
}

/// Read PROP-002's `[[mirror]]` entries as rows (`PROP-066#layers`).
///
/// A mirror naming one registry becomes `registry = <of>` → `url`. The
/// wildcard `of = "*"` cannot become a `registry` row — it names no
/// registry — so it becomes one `prefix` row per declared registry URL,
/// which is what "any registry" meant all along and what makes a wildcard
/// mirror visible in `remap list` as the set of hosts it actually
/// rewrites.
///
/// Mirror `priority` orders the chain within the layer: PROP-002 tries the
/// lower number first, and `PROP-066#precedence` breaks a tie by the later
/// row, so the rows come out in descending priority and the chain's head
/// stays the winner.
///
/// ```
/// use vibe_core::manifest::MirrorSection;
/// use vibe_core::remap::{rows_from_mirrors, Layer};
///
/// let mirrors = [MirrorSection {
///     publication: Default::default(),
///     of: "vibespecs".into(),
///     url: "https://nexus.corp.example/vibespecs".into(),
///     priority: 0,
/// }];
/// let rows = rows_from_mirrors(&mirrors, &[], Layer::Local, "~/.vibe/registry.toml");
/// assert_eq!(rows.len(), 1);
/// assert_eq!(rows[0].key.kind(), "registry");
/// assert_eq!(rows[0].key.what(), "vibespecs");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
pub fn rows_from_mirrors(
    mirrors: &[MirrorSection],
    registries: &[RegistrySection],
    layer: Layer,
    source: impl Into<String>,
) -> Vec<RemapRow> {
    let source = source.into();
    let mut ordered: Vec<&MirrorSection> = mirrors.iter().collect();
    // Later row wins a tie, so the highest `priority` number is emitted
    // first and the chain's head (priority 0) emitted last.
    ordered.sort_by_key(|mirror| std::cmp::Reverse(mirror.priority));
    let mut rows = Vec::new();
    for mirror in ordered {
        let reason = Some(format!(
            "`[[mirror]]` of `{}` (priority {})",
            mirror.of, mirror.priority
        ));
        if mirror.of == "*" {
            for registry in registries {
                rows.push(row(
                    RemapKey::Prefix(registry.url.clone()),
                    RemapTarget::Url(mirror.url.clone()),
                    reason.clone(),
                    Provenance::new(layer, source.clone(), rows.len()).catalog(mirror.publication),
                ));
            }
            continue;
        }
        rows.push(row(
            RemapKey::Registry(mirror.of.clone()),
            RemapTarget::Url(mirror.url.clone()),
            reason,
            Provenance::new(layer, source.clone(), rows.len()).catalog(mirror.publication),
        ));
    }
    rows
}

/// Read PROP-002's `[[override]]` entries as rows (`PROP-066#layers`):
/// `package = <pkgref>` → `url = <source_url>`, with the override's `ref`
/// carried as the row's target detail so the pinned revision survives the
/// translation.
///
/// ```
/// use vibe_core::manifest::OverrideSection;
/// use vibe_core::remap::{rows_from_overrides, Layer};
///
/// let overrides = [OverrideSection {
///     pkgref: "org.example/wal".into(),
///     source_url: "https://github.com/me/fork".into(),
///     r#ref: Some("v2".into()),
///     reason: Some("pinning a patched fork".into()),
/// }];
/// let rows = rows_from_overrides(&overrides, Layer::Local, "vibe.toml");
/// assert_eq!(rows[0].key.kind(), "package");
/// assert_eq!(rows[0].target_ref.as_deref(), Some("v2"));
/// assert_eq!(rows[0].reason.as_deref(), Some("pinning a patched fork"));
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
pub fn rows_from_overrides(
    overrides: &[OverrideSection],
    layer: Layer,
    source: impl Into<String>,
) -> Vec<RemapRow> {
    let source = source.into();
    overrides
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let mut built = row(
                RemapKey::Package(entry.pkgref.clone()),
                RemapTarget::Url(entry.source_url.clone()),
                entry
                    .reason
                    .clone()
                    .or_else(|| Some("`[[override]]` source pin".to_owned())),
                Provenance::new(layer, source.clone(), index)
                    .catalog(crate::RoutePublication::Local),
            );
            built.target_ref = entry.r#ref.clone();
            built
        })
        .collect()
}

/// The source label the PROP-002 registry tables load under, in a CLI
/// Locator and in every catalog that composes its tables onto one
/// ([`crate::remap::Locator::with_registry_tables`]).
pub const REGISTRY_TABLES_SOURCE: &str = "[[mirror]] / [[override]]";

/// PROP-002's `[[mirror]]` and `[[override]]` tables as the one local-layer
/// source the Locator holds for them (`PROP-066#layers`): the mirrors
/// first, then the overrides, numbered as one sequence so the layer's
/// "later row" tie-break sees one source rather than two starting at zero.
///
/// This is the single owner of that lowering: the composition root builds
/// the invocation's Locator with it, and a registry catalog that reads its
/// tables on demand composes them onto the active Locator with it, so a
/// `[[mirror]]` means the same thing on every command.
///
/// ```
/// use vibe_core::manifest::{MirrorSection, OverrideSection};
/// use vibe_core::remap::{rows_from_registry_tables, Layer};
///
/// let mirrors = [MirrorSection {
///     publication: Default::default(),
///     of: "vibespecs".into(),
///     url: "https://nexus.corp.example/vibespecs".into(),
///     priority: 0,
/// }];
/// let overrides = [OverrideSection {
///     pkgref: "org.example/wal".into(),
///     source_url: "https://github.com/me/fork".into(),
///     r#ref: None,
///     reason: None,
/// }];
/// let rows = rows_from_registry_tables(&[], &mirrors, &overrides, Layer::Local);
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[0].key.kind(), "registry");
/// assert_eq!(rows[1].key.kind(), "package");
/// assert_eq!(rows[1].from.index, 1);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
pub fn rows_from_registry_tables(
    registries: &[RegistrySection],
    mirrors: &[MirrorSection],
    overrides: &[OverrideSection],
    layer: Layer,
) -> Vec<RemapRow> {
    let mut rows = rows_from_mirrors(mirrors, registries, layer, "[[mirror]]");
    let offset = rows.len();
    for (position, mut row) in rows_from_overrides(overrides, layer, "[[override]]")
        .into_iter()
        .enumerate()
    {
        row.from.index = offset + position;
        rows.push(row);
    }
    rows
}

/// The rows the raw-file fast path reads from by default
/// (`PROP-066#locator`): the hardcoded host table of PROP-002's HTTPS
/// single-file read becomes ordinary `prefix` rows of the lowest layer, so
/// `remap list` shows them and any higher row replaces them.
///
/// The pairs are `(repository host, single-file base)` as the fast path
/// knows them; the caller passes its own table rather than this crate
/// knowing the hosts.
///
/// ```
/// use vibe_core::remap::{default_raw_file_rows, Layer};
///
/// let rows = default_raw_file_rows(&[("github.com", "https://raw.githubusercontent.com")]);
/// assert_eq!(rows.len(), 1);
/// assert_eq!(rows[0].from.layer, Layer::UserPackage);
/// assert_eq!(rows[0].key.what(), "https://github.com/");
/// // Both sides end in `/`, so a prefix replacement joins cleanly.
/// assert_eq!(rows[0].target.display(), "https://raw.githubusercontent.com/");
/// // Only single-file reads take the endpoint; a clone keeps its host.
/// assert_eq!(rows[0].only, Some(vibe_core::remap::ArtifactKind::RawFile));
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#locator")]
pub fn default_raw_file_rows(hosts: &[(&str, &str)]) -> Vec<RemapRow> {
    hosts
        .iter()
        .enumerate()
        .map(|(index, (host, base))| {
            let mut built = row(
                RemapKey::Prefix(format!("https://{host}/")),
                // Prefix replacement is literal, so the key and the
                // target have to agree about the separator: `.../` for
                // `https://<host>/` leaves `<rest>` joined by exactly one
                // slash. Without it the two would concatenate into
                // `https://raw.githubusercontent.comvibespecs/index`.
                RemapTarget::Url(format!("{}/", base.trim_end_matches('/'))),
                Some(format!(
                    "built-in single-file endpoint for `{host}` (single-file reads only)"
                )),
                // The built-in table is the weakest thing in the room: any
                // row a user, a project or the invocation declares outranks
                // it, which is exactly what makes the hosts replaceable.
                Provenance::new(Layer::UserPackage, "built-in", index),
            );
            // The endpoint serves one file at a time; a clone of the same
            // host goes to the host itself (a `git clone` of
            // `raw.githubusercontent.com` is a 404).
            built.only = Some(super::ArtifactKind::RawFile);
            built
        })
        .collect()
}

/// Split a `VIBE_REMAP=<file>[,<file>]` value into the files it names,
/// dropping empty entries so a trailing comma is not a missing file.
///
/// ```
/// use vibe_core::remap::env_remap_files;
///
/// let files = env_remap_files("a.toml,,b.toml");
/// assert_eq!(files.len(), 2);
/// assert!(files[1].ends_with("b.toml"));
/// assert!(env_remap_files("  ").is_empty());
/// ```
pub fn env_remap_files(raw: &str) -> Vec<PathBuf> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Build a row with no scope — the shape every derived (non-TOML) source
/// produces.
fn row(key: RemapKey, target: RemapTarget, reason: Option<String>, from: Provenance) -> RemapRow {
    RemapRow {
        key,
        target,
        scope: None,
        reason,
        from_package: None,
        from,
        target_ref: None,
        only: None,
        lookup: super::row::PathLookup::Mirror,
    }
}
