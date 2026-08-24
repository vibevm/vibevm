//! `conform.toml` — the project's conform policy, lifted out of
//! compile-time constants so the checker runs on *any* project, not
//! only the one it was built in (PROP-024 §2.2; ENGINE-CONFORM §2).
//!
//! The driver (or the `conform` binary) loads this once at startup and
//! constructs the scan + the rule set from it; nothing about the policy
//! is hardcoded in the engine.
//!
//! **The v2 surface (B-029 + B-034, design `gate-parity-config.md`).**
//! The policy is symmetric per language: the root table carries only
//! the genuinely cross-language budget (`max_file_lines`); every
//! language owns a section of one uniform shape (`roots`, `skip_dirs`,
//! `exclude_substrings`, `gated`, `[[<lang>.exempt]] {unit, reason}`,
//! plus its language-specific extras). The retired flat root keys die
//! loudly — declared as tombstone fields whose presence is a targeted
//! error naming the move (the `LegacyHostAuthority` house pattern),
//! not serde's generic unknown-field message.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#facts");

mod coverage;
mod tombstones;

#[cfg(test)]
mod tests;

pub use coverage::{
    go_scope_warnings, go_units, go_vacuously_gated, rust_scope_warnings, rust_units,
    ts_scope_warnings, ts_units, ts_vacuously_gated,
};

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use toml::Value;

/// The conform policy for one project: the cross-language budget at the
/// root, plus a per-language section each. Loaded from a `conform.toml`
/// at the project root; every field defaults, so a minimal file works
/// and an absent file yields a usable default.
///
/// The nine `Option<Value>` fields are **loud tombstones** — the retired
/// flat root keys (B-029). Their presence parses (they are known fields,
/// not `deny_unknown_fields` fodder) but [`Config::load`] rejects any
/// that is set with a targeted move hint; serde's generic unknown-field
/// error still covers every other stray key.
///
/// ```
/// let cfg: core_ai_native_conform::Config = toml::from_str(
///     "[rust]\n\
///      roots = [\"crates/*\"]\n\
///      gated = [\"app\"]\n\
///      registry_file = \"crates/app/src/registry.rs\"\n\
///      registry_gated_crate = \"app\"\n",
/// )
/// .unwrap();
/// assert_eq!(cfg.max_file_lines, 600);
/// assert_eq!(cfg.invariant_comment_min_file_lines, 120);
/// assert_eq!(cfg.invariant_comment_markers.len(), 5);
/// assert_eq!(cfg.rust.gated, vec!["app".to_string()]);
/// assert_eq!(cfg.rust.registry_gated_crate.as_deref(), Some("app"));
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    // --- loud tombstones: the retired flat root keys (B-029 / B-031 house
    // pattern). `Option<Value>` so *any* presence — list, scalar, or
    // table-array — is caught; the value itself is irrelevant, only that
    // the key survived the migration. ---
    /// Tombstone — moved to `[rust] roots`.
    pub roots: Option<Value>,
    /// Tombstone — moved to `[rust] exclude_substrings`.
    pub exclude_substrings: Option<Value>,
    /// Tombstone — renamed `gated` under `[rust]`.
    pub gated_crates: Option<Value>,
    /// Tombstone — moved under `[rust] gated_pub_doctest`.
    pub gated_pub_doctest: Option<Value>,
    /// Tombstone — moved under `[rust] audit_crates`.
    pub audit_crates: Option<Value>,
    /// Tombstone — moved under `[rust] env_roots`.
    pub env_roots: Option<Value>,
    /// Tombstone — moved under `[rust] registry_file`.
    pub registry_file: Option<Value>,
    /// Tombstone — moved under `[rust] registry_gated_crate`.
    pub registry_gated_crate: Option<Value>,
    /// Tombstone — moved to `[[rust.exempt]]` (field `crate` → `unit`).
    pub exempt: Option<Value>,

    // --- the live surface ---
    /// The per-file line budget (`file-length`); read by every frontend.
    pub max_file_lines: u32,
    /// The invariant-marker vocabulary for `invariant-comment-position`
    /// — the labeled comment tags (normalized, as written) that mark an
    /// invariant worth surfacing. A root key (beside `max_file_lines`)
    /// because the vocabulary is language-neutral, not per-language
    /// policy (design `new-rule-classes.md` §2). Empty disables the rule.
    ///
    /// **A marker is a labeled tag, not a word from prose** (B-036). Every
    /// entry carries a trailing `:` — the colon is the mark of labeling,
    /// what tells a reader "this line declares an invariant" rather than
    /// "this sentence uses a forceful word". A bare `NEVER` mid-sentence
    /// («a body is NEVER re-qualified whole») is emphasis, not an
    /// invariant, and the bare-word vocabulary caught it as a false
    /// positive — the same class of error as the `violates REQ` vs bare
    /// `REQ` lesson. So the dictionary is the five colon-bearing tags
    /// only; the rule still re-checks membership, so a stale cached bare
    /// marker can never red a frozen baseline.
    ///
    /// `SAFETY:` is excluded for a different reason: it IS a labeled tag,
    /// but by Rust convention (and clippy's `undocumented_unsafe_blocks`)
    /// a `SAFETY:` comment is the *block-local justification of an
    /// `unsafe` block* and must sit directly beside it — moving it to a
    /// file's edge to satisfy a position rule would destroy its meaning.
    /// It is not a file-level invariant, so it has no place in a rule
    /// about file-level position. (A real `// SAFETY:` at its `unsafe`
    /// block is therefore correctly left alone by this rule.)
    pub invariant_comment_markers: Vec<String>,
    /// The minimum file length below which the `invariant-comment-position`
    /// rule is silent — on a short file «thirds» are meaningless, so no
    /// comment is ever «buried in the middle». A root key, same reason as
    /// the vocabulary; defaults to 120 lines.
    pub invariant_comment_min_file_lines: u32,
    /// Where a flora step deposits foreign-linter SARIF reports for the
    /// gate to read back in (B-026). Repo-relative paths: each is a report
    /// file read directly, or a directory walked for `*.sarif` / `*.json`.
    ///
    /// A root key — beside `max_file_lines` and the invariant vocabulary —
    /// because SARIF ingest is a CROSS-LANGUAGE engine mechanism, not a
    /// per-language policy: one flora deposit point holds every linter's
    /// report (clippy, eslint, golangci-lint together), and each report's
    /// `runs[].tool.driver.name` says which tool it is, so the engine never
    /// needs to know which language a report is about. That is the v2
    /// logic — the root carries the language-neutral, the per-language
    /// sections own the homogeneous `roots`/`gated`/`exempt`/… shape — and
    /// a report directory is language-neutral plumbing, not a per-language
    /// slot (which would triplicate the key and break the one-place deposit
    /// model). Empty (the default) means no reports are read — the norm
    /// today, since no project deposits them yet; absence is never an error.
    pub sarif_reports: Vec<String>,
    /// The Rust half of the policy (`[rust]`).
    pub rust: RustConfig,
    /// The TypeScript half of the policy (`[typescript]`), consumed by
    /// `typescript-ai-native-conform` (the `ts-tsc` frontend). Absent for
    /// Rust-only projects; the Rust rules never read it.
    pub typescript: TsConfig,
    /// The Go half of the policy (`[go]`), consumed by
    /// `go-ai-native-conform` (the `go-extract` frontend). Absent for
    /// projects without Go; no other rules read it.
    pub go: GoConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            roots: None,
            exclude_substrings: None,
            gated_crates: None,
            gated_pub_doctest: None,
            audit_crates: None,
            env_roots: None,
            registry_file: None,
            registry_gated_crate: None,
            exempt: None,
            max_file_lines: 600,
            invariant_comment_markers: vec![
                "INVARIANT:".into(),
                "WARNING:".into(),
                "PANICS:".into(),
                "MUST:".into(),
                "NEVER:".into(),
            ],
            invariant_comment_min_file_lines: 120,
            sarif_reports: Vec::new(),
            rust: RustConfig::default(),
            typescript: TsConfig::default(),
            go: GoConfig::default(),
        }
    }
}

/// The `[rust]` policy table — the uniform per-language shape (B-029
/// ruling: Rust's gate unit is the **crate**) plus Rust's own extras,
/// lifted out of the retired flat root keys. Defaults are exactly
/// today's root-table values (census Q1), only relocated.
///
/// ```
/// let cfg: core_ai_native_conform::Config = toml::from_str(
///     "[rust]\nroots = [\"crates/*\"]\ngated = [\"app\"]\n",
/// )
/// .unwrap();
/// assert_eq!(cfg.rust.roots, vec!["crates/*".to_string()]);
/// assert_eq!(cfg.rust.gated, vec!["app".to_string()]);
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RustConfig {
    /// Source roots to scan. A `<dir>/*` entry scans each subdirectory
    /// of `<dir>` as one crate; any other entry is a literal crate dir.
    pub roots: Vec<String>,
    /// Project-specific directory names the source walk never descends
    /// into. These exact names supplement the engine's ecosystem-wide
    /// built-ins; consumer layout names belong here, not in the engine.
    pub skip_dirs: Vec<String>,
    /// A source file whose repo-relative path contains any of these
    /// substrings is skipped (generated code, vendored trees).
    pub exclude_substrings: Vec<String>,
    /// Crates the Class-F/G gates apply to — the unit list (Rust's unit
    /// is the crate).
    pub gated: Vec<String>,
    /// Crates deliberately *outside* `gated`, each paired with the
    /// reason it has not (yet) flipped — a silent exemption reads as a
    /// bug, a recorded one as a decision.
    pub exempt: Vec<ExemptEntry>,
    /// Crates whose whole public *type* surface is gated for doctests
    /// (the wider `pub-doctest` lens).
    pub gated_pub_doctest: Vec<String>,
    /// Designated audit crates — exempt wholesale from the unsafe and
    /// ambient-env gates (they own the unsafety behind a safe API).
    pub audit_crates: Vec<String>,
    /// Repo-relative files where reading the ambient environment is
    /// sanctioned (the composition / config-resolution roots).
    pub env_roots: Vec<String>,
    /// The one legal cell-construction site (R-001 flag-sites). `None`
    /// disables R-001 — a project without the cell idiom omits it.
    pub registry_file: Option<String>,
    /// The crate R-001 gates; meaningful only with `registry_file`.
    pub registry_gated_crate: Option<String>,
    /// Floor steps this project explicitly disables, each with a
    /// recorded reason — the Rust twin of the Go/TypeScript
    /// `floor_disable` slot (B-049), the same `{step, reason}` shape and
    /// the same posture (printed on every run). The Rust floor reads it
    /// in its own lane (`rust-ai-native-cli/src/floor.rs`, next slice);
    /// here the field is added so the engine already parses
    /// `[[rust.floor_disable]]`.
    pub floor_disable: Vec<FloorDisable>,
}

impl Default for RustConfig {
    fn default() -> Self {
        RustConfig {
            roots: vec!["crates/*".into()],
            skip_dirs: Vec::new(),
            exclude_substrings: vec!["/generated/".into()],
            gated: Vec::new(),
            exempt: Vec::new(),
            gated_pub_doctest: Vec::new(),
            audit_crates: Vec::new(),
            env_roots: Vec::new(),
            registry_file: None,
            registry_gated_crate: None,
            floor_disable: Vec::new(),
        }
    }
}

/// The `[go]` policy table (GUIDE-AI-NATIVE-GO §2, §6, §7). Carries the
/// uniform `gated` / `[[go.exempt]]` slots (Go's gate unit is the
/// **package**, B-029 ruling) alongside its existing scan/cell fields.
///
/// ```
/// let cfg: core_ai_native_conform::Config = toml::from_str(
///     "[go]\nroots = [\".\"]\ncells_dir = \"internal/cells\"\n",
/// )
/// .unwrap();
/// assert_eq!(cfg.go.roots, vec![".".to_string()]);
/// assert_eq!(cfg.go.cells_dir.as_deref(), Some("internal/cells"));
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GoConfig {
    /// Go source roots (flat walk; `<dir>/*` scans subdirs).
    pub roots: Vec<String>,
    /// Project-specific directory names the source walk never descends
    /// into, in addition to Go's ecosystem-wide built-ins.
    pub skip_dirs: Vec<String>,
    /// A `.go` file whose repo-relative path contains any of these
    /// substrings is skipped (fixtures, goldens, vendored trees).
    pub exclude_substrings: Vec<String>,
    /// Packages the gates apply to — the unit list (Go's unit is the
    /// package).
    pub gated: Vec<String>,
    /// Packages deliberately *outside* `gated`, each with a recorded
    /// reason.
    pub exempt: Vec<ExemptEntry>,
    /// The directory whose immediate subdirectories are cells
    /// (`go-cell-isolation`); `None` disables the isolation rule.
    /// Unlike the TS shape there is no seam module INSIDE a cell:
    /// Go cells never import siblings at all — seams live in
    /// `seams_pkg` and the registry is the only cell importer
    /// (GUIDE-AI-NATIVE-GO §2).
    pub cells_dir: Option<String>,
    /// The seams package path (repo-relative) — for the oracle's
    /// `scope` answers and the init generator; carries no rule.
    pub seams_pkg: Option<String>,
    /// The registry package path (repo-relative) — the one legal cell
    /// importer and flag reader, for init/codemod, AND the perimeter the
    /// `go-flag-sites` rule carves out: a cell package imported by any
    /// file outside `cells_dir` other than this package is a selection
    /// flag that leaked past the composition root (GUIDE-AI-NATIVE-GO
    /// §6). The rule mounts only when both `cells_dir` and `registry_pkg`
    /// are set; `None` leaves it off, exactly as Rust's R-001 is off
    /// without `registry_file`.
    pub registry_pkg: Option<String>,
    /// Floor steps this project explicitly disables, each with a
    /// recorded reason — printed on every run, same posture as the
    /// TypeScript table.
    pub floor_disable: Vec<FloorDisable>,
}

impl Default for GoConfig {
    fn default() -> Self {
        GoConfig {
            roots: vec![".".into()],
            skip_dirs: Vec::new(),
            exclude_substrings: vec!["/testdata/".into(), "/vendor/".into(), "/fixtures/".into()],
            gated: Vec::new(),
            exempt: Vec::new(),
            cells_dir: None,
            seams_pkg: None,
            registry_pkg: None,
            floor_disable: Vec::new(),
        }
    }
}

/// The `[typescript]` policy table (GUIDE-AI-NATIVE-TYPESCRIPT §3, §8).
/// Carries the uniform `gated` / `[[typescript.exempt]]` slots (TS's gate
/// unit is the **cell**, B-029 ruling) alongside its existing fields.
///
/// ```
/// let cfg: core_ai_native_conform::Config = toml::from_str(
///     "[typescript]\nroots = [\"src\"]\ncells_dir = \"src/cells\"\n",
/// )
/// .unwrap();
/// assert_eq!(cfg.typescript.roots, vec!["src".to_string()]);
/// assert_eq!(cfg.typescript.seam, "index");
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TsConfig {
    /// TypeScript source roots (flat walk; `<dir>/*` scans subdirs).
    pub roots: Vec<String>,
    /// Project-specific directory names the source walk never descends
    /// into, in addition to TypeScript's ecosystem-wide built-ins.
    pub skip_dirs: Vec<String>,
    /// A `.ts` file whose repo-relative path contains any of these
    /// substrings is skipped (fixtures, generated output).
    pub exclude_substrings: Vec<String>,
    /// Cells the gates apply to — the unit list (TS's unit is the cell).
    pub gated: Vec<String>,
    /// Cells deliberately *outside* `gated`, each with a recorded
    /// reason.
    pub exempt: Vec<ExemptEntry>,
    /// The directory whose immediate subdirectories are cells
    /// (`ts-cell-isolation`); `None` disables the isolation rule.
    pub cells_dir: Option<String>,
    /// The seam module name a sibling cell may be imported through.
    pub seam: String,
    /// The one legal site for environment/config reads
    /// (`process.env` / `import.meta.env`) — the composition root, the
    /// TS twin of Rust's `registry_file` (GUIDE-AI-NATIVE-TYPESCRIPT §7,
    /// B-039). A repo-relative file. The `ts-flag-sites` rule is mounted
    /// ONLY when this is `Some`; `None` leaves the rule off, exactly as
    /// a Rust project without the cell idiom omits `registry_file`.
    pub composition_root: Option<String>,
    /// Floor steps this project explicitly disables, each with a
    /// recorded reason. The floor PRINTS every disablement every run —
    /// the "a defaulted nothing-gated run announces itself" posture
    /// extended to step disablement; absent tooling without an entry
    /// here is a hard step failure, never a silent skip.
    pub floor_disable: Vec<FloorDisable>,
}

impl Default for TsConfig {
    fn default() -> Self {
        TsConfig {
            roots: vec!["src".into()],
            skip_dirs: Vec::new(),
            exclude_substrings: vec!["/fixtures/".into()],
            gated: Vec::new(),
            exempt: Vec::new(),
            cells_dir: None,
            seam: "index".into(),
            composition_root: None,
            floor_disable: Vec::new(),
        }
    }
}

/// One disabled floor step + why (`[[<lang>.floor_disable]]`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorDisable {
    /// The step name (`prettier` / `tsc` / `tests` / `eslint` /
    /// `conform` / `specmap` / `test-gate`).
    pub step: String,
    /// Why it is off — never empty.
    pub reason: String,
}

/// A gate unit held outside `gated`, with the reason it has not flipped
/// — the checklist the remaining conform-adoption phases drain. One
/// shared shape across languages: the `unit` field names the language's
/// own unit (crate / package / cell), so no foreign term lives in any
/// config (B-029).
///
/// ```
/// let e = core_ai_native_conform::ExemptEntry {
///     unit: "vibe-graph".into(),
///     reason: "M0 stub, no code yet".into(),
/// };
/// assert_eq!(e.unit, "vibe-graph");
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExemptEntry {
    /// The gate unit (crate / package / cell — the TOML key is `unit`).
    pub unit: String,
    /// Why it is exempt — never empty.
    pub reason: String,
}

/// Where a [`Config`] came from — a real `conform.toml`, or the built-in
/// default because none exists. The drivers print this so a defaulted
/// (nothing-gated) run can never masquerade as a configured green.
///
/// ```
/// assert_ne!(core_ai_native_conform::ConfigOrigin::Loaded, core_ai_native_conform::ConfigOrigin::Defaulted);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOrigin {
    /// Parsed from the project's `conform.toml`.
    Loaded,
    /// No `conform.toml` at the root — the topology-detected default
    /// (nothing gated, everything advisory) is in force.
    Defaulted,
}

impl Config {
    /// Parse a `conform.toml` from `path`, then reject any retired flat
    /// root key with a targeted move hint (the loud-tombstone gate).
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading conform config {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("parsing conform config {}", path.display()))?;
        tombstones::check(&cfg)?;
        Ok(cfg)
    }

    /// Load the project's `conform.toml`, or fall back to a usable default
    /// when none exists (the doc-promised behaviour): scan roots detected
    /// from the tree's topology — `crates/*` for a workspace layout, `.`
    /// for a single-crate one — with nothing gated. The origin tells the
    /// caller which case it got.
    pub fn load_or_default(root: &Path) -> Result<(Config, ConfigOrigin)> {
        let path = root.join("conform.toml");
        if path.exists() {
            return Ok((Config::load(&path)?, ConfigOrigin::Loaded));
        }
        let mut cfg = Config::default();
        if !root.join("crates").is_dir() {
            cfg.rust.roots = vec![".".into()];
        }
        Ok((cfg, ConfigOrigin::Defaulted))
    }
}
