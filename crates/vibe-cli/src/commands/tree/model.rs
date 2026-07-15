//! The serde model for `vibe tree --json` (PROP-036 §2.7).
//!
//! These `#[derive(Serialize)]` types mirror the shipped JSON Schema at
//! `crates/vibe-cli/resources/package-tree.schema.v1.json` field-for-field.
//! Optional fields that the reference instance emits as `null` are modelled
//! as `Option<T>` **without** `skip_serializing_if`, so the wire form always
//! carries the key (matching the schema's `["…", null]` unions). Fields whose
//! schema `enum` has no `null` member (`load.origin`, `source.kind`) are
//! non-nullable — a value is always computed, or the key is omitted.
//!
//! Display state (TUI mode, ordering, tab, selection) is deliberately absent:
//! it is TUI-only and never part of the machine surface (PROP-036 §2.7).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#json");

use serde::Serialize;

/// The one and only schema version this producer emits (PROP-036 §2.7).
pub const SCHEMA_VERSION: u32 = 1;

/// The host project's `spec://` authority (PROP-035 §6) — the root project is
/// addressed as `vibevm`.
pub const HOST_NAMESPACE: &str = "vibevm";

/// The root document — one object, validated against the v1 schema.
#[derive(Debug, Serialize)]
pub struct PackageTree {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
    pub project: Project,
    pub roots: Vec<String>,
    pub packages: Vec<Package>,
    pub boot: Boot,
    pub in_place_specs: Vec<InPlaceSpec>,
    pub diagnostics: Vec<Diagnostic>,
}

/// `project` — the analysed project's context.
#[derive(Debug, Serialize)]
pub struct Project {
    pub root: String,
    pub name: Option<String>,
    pub is_workspace: bool,
    pub host_namespace: String,
}

/// One resolved package (one object per lock entry — the unique set; the
/// tree edges live in [`Package::dependencies`], PROP-036 §2.12).
#[derive(Debug, Serialize)]
pub struct Package {
    pub id: String,
    pub group: String,
    pub name: String,
    pub kind: String,
    pub version: String,
    pub content_hash: Option<String>,
    pub source: Option<Source>,
    pub load: Load,
    pub condition: Condition,
    pub dependencies: Vec<String>,
}

/// `source` — where the bytes came from on this install (informational).
#[derive(Debug, Serialize)]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<SourceKind>,
    pub url: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub commit: Option<String>,
}

/// `load` — the *effective* boot lane a package lands in (PROP-036 §2.3–§2.5).
#[derive(Debug, Serialize)]
pub struct Load {
    #[serde(rename = "type")]
    pub load_type: LoadType,
    pub transitive: bool,
    pub declared: Option<DeclaredLink>,
    pub origin: LoadOrigin,
    pub in_static_md: bool,
    pub in_index_md: bool,
    pub boot_path: Option<String>,
}

/// `condition` — the boot entry's `when` gate (PROP-036 §2.5).
#[derive(Debug, Serialize)]
pub struct Condition {
    pub present: bool,
    pub raw: Option<String>,
    pub kind: Option<ConditionKind>,
    pub value: Option<String>,
}

impl Condition {
    /// The unconditional condition — no `when` gate.
    pub fn absent() -> Self {
        Condition {
            present: false,
            raw: None,
            kind: None,
            value: None,
        }
    }
}

/// `boot` — the two committed boot lanes and their sizes.
#[derive(Debug, Serialize)]
pub struct Boot {
    pub static_md: Option<StaticLane>,
    pub index_md: IndexLane,
}

/// `boot.static_md` — the decompiled `STATIC.md` lane (PROP-036 §2.8).
#[derive(Debug, Serialize)]
pub struct StaticLane {
    pub present: bool,
    pub path: String,
    pub bytes: u64,
    pub lines: u64,
    pub contributions: Vec<StaticContribution>,
}

/// One `<!-- vibe:static … -->` region of `STATIC.md`.
#[derive(Debug, Serialize)]
pub struct StaticContribution {
    pub order: u64,
    pub origin: String,
    pub source_path: String,
    pub bytes: u64,
    pub lines: u64,
    pub embeds: Vec<EmbedSpan>,
}

/// A nested `<!-- embed: … -->` span attributed inside a static region.
#[derive(Debug, Serialize)]
pub struct EmbedSpan {
    pub address: String,
    pub start_line: u64,
    pub end_line: u64,
}

/// `boot.index_md` — the `INDEX.md` dynamic lane.
#[derive(Debug, Serialize)]
pub struct IndexLane {
    pub present: bool,
    pub path: String,
    pub static_pointer: Option<String>,
    pub entries: Vec<IndexEntry>,
}

/// One `[[entry]]` of `INDEX.md`. `kind` is the read-timing axis
/// (`when`-gated ⇒ `dynamic`), NOT the package load type (PROP-036 §2.7).
#[derive(Debug, Serialize)]
pub struct IndexEntry {
    pub order: u64,
    pub path: String,
    pub kind: IndexKind,
    pub when: Option<String>,
}

/// One collected in-place boot-lane spec marker (PROP-036 §2.9).
#[derive(Debug, Serialize)]
pub struct InPlaceSpec {
    pub carrier: Carrier,
    pub address: String,
    pub file: String,
    pub line: u64,
    pub resolved: bool,
    pub target_package: Option<String>,
}

/// One non-fatal diagnostic (PROP-036 §2.10).
// Wired in Phase 1 (the `diagnostics` array is always `[]`); populated in
// Phase 4 when the stale-artifacts / root-drift checks land.
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub locator: Option<String>,
}

/// Effective lane a package's boot snippet actually built into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadType {
    Static,
    Dynamic,
    None,
}

/// Consumer-declared link for a direct root edge (`null` = not declared).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeclaredLink {
    Static,
    Dynamic,
    StaticTransitive,
}

/// Why `load.type` has its value (PROP-036 §2.3–§2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoadOrigin {
    Declared,
    Suggested,
    Default,
    StaticTransitive,
    WhenForced,
    None,
}

/// The kind of `when` condition (v1: only `os`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionKind {
    Os,
}

/// `INDEX.md` read-timing kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexKind {
    Static,
    Dynamic,
}

/// Resolution path that produced a lockfile entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Registry,
    Git,
    Override,
    Path,
    Embedded,
}

/// The carrier of an in-place spec marker (PROP-036 §2.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Carrier {
    #[serde(rename = "@spec")]
    AtSpec,
    #[serde(rename = "#use")]
    Use,
    #[serde(rename = "#embed")]
    Embed,
    #[serde(rename = "#source")]
    Source,
}

/// A diagnostic severity.
// Phase 1 wires the `diagnostics` field (always `[]`); the variants are
// constructed when the diagnostics themselves land in Phase 4 (PROP-036 §2.10:
// stale-artifacts, lock↔toml root-drift).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Error,
}
