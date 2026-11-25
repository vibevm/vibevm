//! The gated-or-exempt coverage invariant, generalised across languages
//! (B-034). One core rejects six violation classes, parameterised by
//! the unit noun (crate / package / cell) and the section name so each
//! language's error strings speak its own word and name the NEW keys
//! (`[rust] gated`, never the retired spelling). Per-language unit
//! enumerators walk the configured roots the way the scanner does, so
//! the classified set matches the scanned corpus; the three
//! `validate_*_against_tree` wrappers are thin shapes over the core
//! (the Rust one is behaviourally stable). The vacuous-green and
//! empty-scope helpers hand the drivers (W2) the announce strings Rust
//! already prints.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#facts");

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, bail};
use walkdir::WalkDir;

use super::{Config, ExemptEntry, GoConfig, RustConfig, TsConfig};
use crate::facts::SourceFacts;
use crate::store::{GO_SKIP_DIRS, keep_walk_entry};

/// The gated-or-exempt coverage invariant, parameterised by the unit
/// noun and the section name. Six refusal classes, each message in the
/// language's own unit word and naming the new keys (`[<section>] gated`,
/// `[[<section>.exempt]]` — never the retired spelling):
///
/// 1. a duplicate unit in `gated`,
/// 2. a duplicate unit in `exempt`,
/// 3. a unit listed in both,
/// 4. an `exempt` entry without a recorded reason,
/// 5. an on-disk unit neither gated nor exempt,
/// 6. a listed unit with no matching on-disk directory (a ghost/typo).
///
/// `on_disk` is the set of units the language's enumerator found under
/// the configured roots. `literals` are extra names a non-enumerated
/// root resolves to (Rust's literal `[rust]` roots, via
/// [`crate::store::crate_dir_name`]) — they satisfy the ghost-entry
/// check without entering the unclassified-on-disk check, so a literal
/// root like `.` is never forced into `gated`/`exempt`. Empty for Go
/// and TS, whose enumerators already cover every unit.
pub(crate) fn validate_units(
    gated: &[String],
    exempt: &[ExemptEntry],
    on_disk: &BTreeSet<String>,
    literals: &BTreeSet<String>,
    noun: &str,
    section: &str,
) -> Result<()> {
    let gated_set: BTreeSet<&str> = gated.iter().map(|s| s.as_str()).collect();
    let exempt_set: BTreeSet<&str> = exempt.iter().map(|e| e.unit.as_str()).collect();

    if gated_set.len() != gated.len() {
        bail!("conform.toml: `[{section}]` `gated` carries a duplicate {noun}");
    }
    if exempt_set.len() != exempt.len() {
        bail!("conform.toml: `[[{section}.exempt]]` carries a duplicate {noun}");
    }
    let both: Vec<&str> = gated_set.intersection(&exempt_set).copied().collect();
    if !both.is_empty() {
        bail!("conform.toml: {noun}s both gated and exempt: {both:?}");
    }
    for e in exempt {
        if e.reason.trim().is_empty() {
            bail!(
                "conform.toml: `{}` is exempt without a recorded reason — the one \
                 thing the exemption table exists to forbid",
                e.unit
            );
        }
    }
    for u in on_disk {
        if !gated_set.contains(u.as_str()) && !exempt_set.contains(u.as_str()) {
            bail!("conform.toml: {noun} `{u}` is neither gated nor exempt — classify it");
        }
    }
    for u in gated_set.union(&exempt_set) {
        if !on_disk.contains(*u) && !literals.contains(*u) {
            bail!("conform.toml: `{u}` is listed but no {noun} directory matches it — typo?");
        }
    }
    Ok(())
}

/// Enumerate the Rust crate units the invariant classifies: each
/// subdirectory a `<dir>/*` root expands to that carries a `Cargo.toml`
/// — exactly as [`validate_against_tree`](Config::validate_against_tree)
/// always has. A literal root (e.g. `.` for a single-crate layout) is
/// NOT a glob unit; it is resolved through
/// [`crate::store::crate_dir_name`] for the ghost-entry check only (see
/// [`rust_literal_roots`]), so a literal-root crate is never forced into
/// `gated`/`exempt` by the unclassified check (a bare `roots = ["."]`
/// defaults clean).
pub fn rust_units(root: &Path, cfg: &RustConfig) -> BTreeSet<String> {
    let mut units = BTreeSet::new();
    for entry in &cfg.roots {
        if let Some(parent) = entry.strip_suffix("/*")
            && let Ok(rd) = std::fs::read_dir(root.join(parent))
        {
            for e in rd.filter_map(Result::ok) {
                let kept = e
                    .file_name()
                    .to_str()
                    .is_none_or(|name| !cfg.skip_dirs.iter().any(|skip| skip == name));
                if kept && e.path().is_dir() && e.path().join("Cargo.toml").exists() {
                    units.insert(e.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }
    units
}

/// The names a literal (non-glob) `[rust]` root resolves to through
/// [`crate::store::crate_dir_name`] — so `.` names the project directory
/// and a `tooling` root names itself. These satisfy the ghost-entry
/// check (a gated/exempt name that is a literal root is not a phantom)
/// without entering the unclassified-on-disk check.
pub(crate) fn rust_literal_roots(root: &Path, cfg: &RustConfig) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in &cfg.roots {
        if entry.strip_suffix("/*").is_none()
            && let Some(name) = crate::store::crate_dir_name(&root.join(entry))
        {
            out.insert(name);
        }
    }
    out
}

/// Enumerate the Go package units under the configured roots. A unit is
/// any directory that immediately contains at least one `.go` file the
/// scanner would keep (the `GO_SKIP_DIRS` trees and `exclude_substrings`
/// matches are dropped, so the unit set matches the scanned corpus),
/// keyed by its forward-slash path **relative to the scan root** (a
/// package at the root itself keys as `.`).
pub fn go_units(root: &Path, cfg: &GoConfig) -> BTreeSet<String> {
    let mut units = BTreeSet::new();
    for root_entry in &cfg.roots {
        let base = root.join(root_entry);
        if !base.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&base)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                // depth 0 is the scan root itself — a literal `.` root must
                // not be eaten by the hidden-dir filter (parity with
                // `go_sources`).
                e.depth() == 0 || keep_walk_entry(e, GO_SKIP_DIRS, &cfg.skip_dirs, true)
            })
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_dir() {
                continue;
            }
            let dir = entry.path();
            if !dir_has_scanned_go(root, dir, &cfg.exclude_substrings) {
                continue;
            }
            let rel = dir.strip_prefix(&base).unwrap_or(dir);
            let key = rel.to_string_lossy().replace('\\', "/");
            units.insert(if key.is_empty() { ".".to_string() } else { key });
        }
    }
    units
}

/// Enumerate the TypeScript cell units: the immediate subdirectories of
/// `cells_dir`, named by the subdirectory. `cells_dir = None` (or a
/// missing directory) yields the empty set — a project with no cell
/// isolation configured has no cells to classify, vacuously green by
/// the discipline's own boundary.
pub fn ts_units(root: &Path, cfg: &TsConfig) -> BTreeSet<String> {
    let mut units = BTreeSet::new();
    let Some(cells) = &cfg.cells_dir else {
        return units;
    };
    let base = root.join(cells);
    let Ok(rd) = std::fs::read_dir(&base) else {
        return units;
    };
    for entry in rd.filter_map(Result::ok) {
        if !matches!(entry.file_type(), Ok(ft) if ft.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if cfg.skip_dirs.iter().any(|skip| skip == &name) {
            continue;
        }
        let dir_rel = format!("{cells}/{name}");
        if cfg
            .exclude_substrings
            .iter()
            .any(|s| dir_rel.contains(s.as_str()))
        {
            continue;
        }
        units.insert(name);
    }
    units
}

/// `dir` contains at least one `.go` file the scanner would keep: a real
/// file (not `_test.go`-excluded — the extractor stamps those `in_test`
/// and they count), whose repo-relative path matches no
/// `exclude_substrings` entry.
fn dir_has_scanned_go(root: &Path, dir: &Path, exclude: &[String]) -> bool {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return false;
    };
    rd.filter_map(Result::ok).any(|f| {
        if !matches!(f.file_type(), Ok(ft) if ft.is_file()) {
            return false;
        }
        if f.path().extension().and_then(|e| e.to_str()) != Some("go") {
            return false;
        }
        let frel = f
            .path()
            .strip_prefix(root)
            .unwrap_or(&f.path())
            .to_string_lossy()
            .replace('\\', "/");
        !exclude.iter().any(|s| frel.contains(s.as_str()))
    })
}

impl Config {
    /// The Rust gated-or-exempt tree invariant — every crate under the
    /// `[rust]` glob roots is classified exactly once (behaviourally
    /// stable; the noun is now `crate` and the keys are `[rust] gated` /
    /// `[[rust.exempt]]`).
    pub fn validate_against_tree(&self, root: &Path) -> Result<()> {
        let on_disk = rust_units(root, &self.rust);
        let literals = rust_literal_roots(root, &self.rust);
        validate_units(
            &self.rust.gated,
            &self.rust.exempt,
            &on_disk,
            &literals,
            "crate",
            "rust",
        )
    }

    /// The Go gated-or-exempt tree invariant — every package under the
    /// `[go]` roots is classified exactly once (noun `package`).
    pub fn validate_go_against_tree(&self, root: &Path) -> Result<()> {
        let on_disk = go_units(root, &self.go);
        validate_units(
            &self.go.gated,
            &self.go.exempt,
            &on_disk,
            &BTreeSet::new(),
            "package",
            "go",
        )
    }

    /// The TypeScript gated-or-exempt tree invariant — every cell under
    /// `[typescript] cells_dir` is classified exactly once (noun `cell`).
    pub fn validate_typescript_against_tree(&self, root: &Path) -> Result<()> {
        let on_disk = ts_units(root, &self.typescript);
        validate_units(
            &self.typescript.gated,
            &self.typescript.exempt,
            &on_disk,
            &BTreeSet::new(),
            "cell",
            "typescript",
        )
    }

    /// Rust gated crates the scan attributed NO sources to — each names
    /// a gate that would pass by vacuity (nothing scanned means nothing
    /// findable), the silent failure mode of a mis-shaped `roots` list.
    /// Behaviourally stable: a gated crate with zero scanned
    /// `crate_name`s is returned; an empty return means every gated
    /// crate contributed at least one scanned file.
    ///
    /// ```
    /// use core_ai_native_conform::{Config, SourceFacts};
    ///
    /// let cfg: Config = toml::from_str("[rust]\ngated = [\"app\"]\n").unwrap();
    /// let nothing: Vec<SourceFacts> = Vec::new();
    /// assert_eq!(cfg.vacuously_gated(&nothing), vec!["app".to_string()]);
    ///
    /// let scanned = vec![SourceFacts {
    ///     file: "crates/app/src/lib.rs".into(),
    ///     crate_name: "app".into(),
    ///     facts: vec![],
    /// }];
    /// assert!(cfg.vacuously_gated(&scanned).is_empty());
    /// ```
    pub fn vacuously_gated(&self, facts: &[SourceFacts]) -> Vec<String> {
        let scanned: BTreeSet<&str> = facts.iter().map(|f| f.crate_name.as_str()).collect();
        vacuously_gated_units(&self.rust.gated, &scanned)
    }
}

/// The shared core: gated units absent from the scanned set, in
/// declaration order. `scanned` carries the units the scan attributed
/// files to (Rust derives them from `SourceFacts::crate_name`; Go and
/// TS pass the enumerator's output).
fn vacuously_gated_units(gated: &[String], scanned: &BTreeSet<&str>) -> Vec<String> {
    gated
        .iter()
        .filter(|g| !scanned.contains(g.as_str()))
        .cloned()
        .collect()
}

/// Go packages gated but attributed no scanned unit. Pass the output of
/// [`go_units`] as `scanned` so the unit spelling matches the
/// enumerator's (root-relative path), not the scanner's `root_name`.
pub fn go_vacuously_gated(gated: &[String], scanned: &BTreeSet<String>) -> Vec<String> {
    let scanned: BTreeSet<&str> = scanned.iter().map(|s| s.as_str()).collect();
    vacuously_gated_units(gated, &scanned)
}

/// TS cells gated but attributed no scanned unit. Pass the output of
/// [`ts_units`] as `scanned`.
pub fn ts_vacuously_gated(gated: &[String], scanned: &BTreeSet<String>) -> Vec<String> {
    let scanned: BTreeSet<&str> = scanned.iter().map(|s| s.as_str()).collect();
    vacuously_gated_units(gated, &scanned)
}

// The empty-scope warning (the sharper guard the E8 census showed
// missing for Go/TS): a present config whose language scope enumerated
// zero units warns loudly instead of passing silently. Each helper
// returns a one-string warning when the scope is non-empty but the
// units are not, and an empty Vec otherwise (an absent scope is a valid
// "this language is not in the project", not a warning).

/// Rust: non-empty `[rust]` roots that resolved to zero crates. A
/// literal root (`roots = ["."]`) is a crate even though the glob
/// enumerator does not list it, so the check runs on units ∪ literals —
/// otherwise every single-crate layout would warn spuriously.
pub fn rust_scope_warnings(root: &Path, cfg: &RustConfig) -> Vec<String> {
    let units = rust_units(root, cfg);
    let literals = rust_literal_roots(root, cfg);
    if !cfg.roots.is_empty() && units.is_empty() && literals.is_empty() {
        vec![
            "conform.toml: `[rust]` roots are configured but resolved to zero crates — \
             an empty Rust scope passes silently; point `roots` at the source tree"
                .to_string(),
        ]
    } else {
        Vec::new()
    }
}

/// Go: non-empty `[go]` roots that enumerated zero packages.
pub fn go_scope_warnings(units: &BTreeSet<String>, cfg: &GoConfig) -> Vec<String> {
    if !cfg.roots.is_empty() && units.is_empty() {
        vec![
            "conform.toml: `[go]` roots are configured but enumerated zero packages — \
             an empty Go scope passes silently; point `roots` at the source tree or leave it absent"
                .to_string(),
        ]
    } else {
        Vec::new()
    }
}

/// TypeScript: a configured `cells_dir` that enumerated zero cells.
pub fn ts_scope_warnings(units: &BTreeSet<String>, cfg: &TsConfig) -> Vec<String> {
    if cfg.cells_dir.is_some() && units.is_empty() {
        vec![
            "conform.toml: `[typescript]` cells_dir is configured but enumerated zero cells — \
             an empty TypeScript scope passes silently; point `cells_dir` at the cells tree or leave it absent"
                .to_string(),
        ]
    } else {
        Vec::new()
    }
}
