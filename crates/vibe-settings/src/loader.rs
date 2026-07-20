//! The three settings loaders + the path classifier (PROP-040 §3 `#locations`,
//! §9 `#path-classifier`).
//!
//! - [`Layer`] — the L1/L2/L3 tag plus its conventional file name and the
//!   role-marker header written into each file (§3 `#role-marker`).
//! - [`classify`] / [`classify_with_home`] — mechanically determine a file's
//!   layer from its path alone (§9 `#path-classifier`): a file named
//!   `settings.local.toml` is L3 *because of its name*, never because its
//!   author remembered to mark it.
//! - [`load_layer`] — read + parse one layer; a **missing** file is an empty
//!   table, never an error (§3 `#missing-is-default`).
//! - [`load_all`] / [`LayeredRaw`] — load the three file layers into one raw
//!   container the resolver (phase 2.3) deep-merges.
//!
//! Frontend-agnostic (PROP-040 §1 `#frontend-agnostic`): only `std`, `toml`
//! parse, and a read-only `HOME` lookup for the L1 root — zero rendering deps.
//!
//! Spec: [PROP-040 §3, §9](../../../../spec/modules/vibe-settings/PROP-040-settings.md#locations).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#locations");

use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::SettingsError;

/// Basename of the shared settings file (L1 and L2 share it; told apart by
/// location — see [`classify`]).
const SETTINGS_FILE: &str = "settings.toml";
/// Basename of the user-project file — L3 *because of this name* (§9
/// `#path-classifier`).
const LOCAL_SETTINGS_FILE: &str = "settings.local.toml";
/// The directory, sibling of `cache/`, that holds the settings files (§3
/// `#dotvibe-not-cache`).
const DOT_VIBE: &str = ".vibe";

/// The three file layers, lowest-to-highest precedence (PROP-040 §2
/// `#three-levels`, §3 `#file-layout`).
///
/// A file's layer is **not** declared by its author — it is mechanically
/// classified from the file's path by [`classify`] (§9 `#path-classifier`).
/// This type carries only the conventional metadata (file name, role-marker
/// header) for each layer.
///
/// ```
/// use vibe_settings::loader::Layer;
///
/// // L3 is L3 because of its file name — the path classifier makes that
/// // mechanical, not a matter of author discipline.
/// assert_eq!(Layer::L3.file_name(), "settings.local.toml");
/// assert_eq!(Layer::L2.file_name(), "settings.toml");
/// // L1 and L2 share the basename; location tells them apart.
/// assert_eq!(Layer::L1.file_name(), Layer::L2.file_name());
/// // Role-marker headers cite each layer's place in the precedence law.
/// assert!(Layer::L2.role_marker().contains("repo-shared"));
/// assert_eq!(Layer::L2.to_string(), "L2");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// User-machine global defaults — `~/.vibe/settings.toml` (not committed).
    L1,
    /// Repo-shared — `<repo>/.vibe/settings.toml` (committed).
    L2,
    /// User-project fine-tuning — `<repo>/.vibe/settings.local.toml` (gitignored).
    L3,
}

impl Layer {
    /// The short tag used in diagnostics and `--show-origins`, e.g. `"L2"`.
    pub const fn label(self) -> &'static str {
        match self {
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
        }
    }

    /// The conventional file name for the layer. L1 and L2 share
    /// `settings.toml` (they are told apart by location, not name — see
    /// [`classify`]); L3 is `settings.local.toml`.
    pub const fn file_name(self) -> &'static str {
        match self {
            Layer::L1 | Layer::L2 => SETTINGS_FILE,
            Layer::L3 => LOCAL_SETTINGS_FILE,
        }
    }

    /// The role-marker header comment written at the top of a layer file so a
    /// reader never confuses layers (PROP-040 §3 `#role-marker`). The L2 wording
    /// follows the spec's example verbatim; L1/L3 are the analogous statement of
    /// the same precedence law (§2 `#precedence-law`).
    pub fn role_marker(self) -> &'static str {
        match self {
            Layer::L1 => {
                "# L1 — user-machine (global). Not committed. Overridden by L2, L3, and CLI/env."
            }
            Layer::L2 => {
                "# L2 — repo-shared (committed). Overrides L1; overridden by L3 and CLI/env."
            }
            Layer::L3 => {
                "# L3 — user-project (gitignored). Overrides L1 and L2; overridden by CLI/env."
            }
        }
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Mechanically determine a file's layer from its path (PROP-040 §9
/// `#path-classifier`).
///
/// The layer is a function of the **path**, overriding any in-file declaration
/// (the IntelliJ `getEffectiveRoamingType` lesson, clean-room): a file named
/// `settings.local.toml` is L3 *because of its name*; `settings.toml` under the
/// user's `~/.vibe/` is L1; `settings.toml` under any other `.vibe/` is L2.
///
/// [`classify`] reads `$VIBE_SETTINGS` (else `HOME`/`USERPROFILE`) only to
/// locate the user-machine root (PROP-040 §3 fixes L1 at `~/.vibe/`, or
/// wherever `$VIBE_SETTINGS` points); the deterministic core is
/// [`classify_with_settings_dir`], which tests drive without env.
/// Unrecognised basenames fall back to [`Layer::L2`] (the repo-shared
/// default) so the classifier is total and never panics.
///
/// ```
/// use vibe_settings::loader::{Layer, classify_with_home};
/// use std::path::Path;
///
/// // Name-based: any settings.local.toml is L3, anywhere.
/// assert_eq!(
///     classify_with_home(Path::new("/repo/.vibe/settings.local.toml"), None),
///     Layer::L3,
/// );
/// // Location-based: a repo's shared file is L2 (no home here).
/// assert_eq!(
///     classify_with_home(Path::new("/srv/repo/.vibe/settings.toml"), None),
///     Layer::L2,
/// );
/// // And the same basename under the user's home is L1.
/// assert_eq!(
///     classify_with_home(
///         Path::new("/home/u/.vibe/settings.toml"),
///         Some(Path::new("/home/u")),
///     ),
///     Layer::L1,
/// );
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#path-classifier")]
pub fn classify(path: &Path) -> Layer {
    classify_with_settings_dir(path, resolved_settings_dir().as_deref())
}

/// The pure core of [`classify`], parameterised by the resolved L1 settings
/// directory so tests (and callers that already resolved it) can classify
/// deterministically without touching the process environment. This is the
/// function the path-classifier REQ (§9 `#path-classifier`) is verified
/// against: a `settings.toml` whose parent is exactly `settings_dir` is L1;
/// any other `settings.toml` is L2; `settings.local.toml` is L3 by name.
///
/// ```
/// use vibe_settings::loader::{Layer, classify_with_settings_dir};
/// use std::path::Path;
///
/// // L1 is the file directly under the resolved settings dir — which, under
/// // `$VIBE_SETTINGS`, need not be named `.vibe` at all.
/// assert_eq!(
///     classify_with_settings_dir(Path::new("/opt/vibe/settings.toml"), Some(Path::new("/opt/vibe"))),
///     Layer::L1,
/// );
/// // The same basename elsewhere is the repo-shared L2.
/// assert_eq!(
///     classify_with_settings_dir(
///         Path::new("/srv/repo/.vibe/settings.toml"),
///         Some(Path::new("/opt/vibe")),
///     ),
///     Layer::L2,
/// );
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#path-classifier")]
pub fn classify_with_settings_dir(path: &Path, settings_dir: Option<&Path>) -> Layer {
    let basename = path.file_name().and_then(|s| s.to_str());
    match basename {
        // L3 by name — `settings.local.toml` anywhere is the user-project layer.
        Some(name) if name == LOCAL_SETTINGS_FILE => Layer::L3,
        // `settings.toml`: L1 iff it sits directly in the resolved settings
        // dir, else L2.
        Some(name) if name == SETTINGS_FILE => {
            if is_l1(path, settings_dir) {
                Layer::L1
            } else {
                Layer::L2
            }
        }
        // Unrecognised basename: the layer is indeterminate — fall back to the
        // repo-shared default so the classifier stays total.
        _ => Layer::L2,
    }
}

/// Backward-compatible shim: classify against the user's home `.vibe/`.
/// Equivalent to [`classify_with_settings_dir`] with `<home>/.vibe` as the
/// L1 directory (the pre-`$VIBE_SETTINGS` fixed location, PROP-040 §3).
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#path-classifier")]
pub fn classify_with_home(path: &Path, home: Option<&Path>) -> Layer {
    let dir = home.map(|h| h.join(DOT_VIBE));
    classify_with_settings_dir(path, dir.as_deref())
}

/// Read and parse one layer file as TOML (PROP-040 §3 `#missing-is-default`).
///
/// A **missing** file is `Ok(empty table)` — never an error; the layer is
/// treated as absent. A present-but-unreadable file is [`SettingsError::Io`];
/// a present-but-malformed file is [`SettingsError::Parse`] (a non-fatal
/// diagnostic the caller surfaces, then treats the layer as absent). The
/// returned error's layer tag is filled mechanically by [`classify`] — the
/// caller does not declare it.
///
/// ```
/// use vibe_settings::loader::load_layer;
/// use std::path::Path;
///
/// // A missing file is an empty layer — built-in defaults win.
/// let table = load_layer(Path::new("/no/such/vibe-settings-doctest.toml")).unwrap();
/// assert!(table.is_empty());
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#missing-is-default")]
pub fn load_layer(path: &Path) -> Result<toml::Table, SettingsError> {
    // Tag the layer mechanically up front (§9); the same tag covers both error
    // arms so the diagnostic names which layer failed without caller input.
    let layer = classify(path);
    if !path.exists() {
        // §3 #missing-is-default: absence is the default, not an error.
        return Ok(toml::Table::new());
    }
    let body = std::fs::read_to_string(path).map_err(|source| SettingsError::Io {
        layer,
        path: path.to_path_buf(),
        source,
    })?;
    let table: toml::Table = toml::from_str(&body).map_err(|source| SettingsError::Parse {
        layer,
        path: path.to_path_buf(),
        source,
    })?;
    Ok(table)
}

/// The three raw TOML tables loaded from disk, before the resolver (phase 2.3)
/// deep-merges them. Each field is an empty table when the file is absent
/// (PROP-040 §3 `#missing-is-default`); nothing here interprets or validates
/// keys — that is the schema cell's job (phase 2.4).
#[derive(Debug, Clone, Default)]
pub struct LayeredRaw {
    /// L1 — user-machine (`~/.vibe/settings.toml`).
    pub l1: toml::Table,
    /// L2 — repo-shared (`<repo>/.vibe/settings.toml`).
    pub l2: toml::Table,
    /// L3 — user-project (`<repo>/.vibe/settings.local.toml`).
    pub l3: toml::Table,
}

impl LayeredRaw {
    /// An empty triple (all three layers absent).
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the table for a given layer.
    pub fn layer(&self, which: Layer) -> &toml::Table {
        match which {
            Layer::L1 => &self.l1,
            Layer::L2 => &self.l2,
            Layer::L3 => &self.l3,
        }
    }
}

/// Load all three file layers (PROP-040 §3 `#locations`). Each layer is
/// independent: a missing file is an empty table. The first unreadable or
/// unparseable file short-circuits as [`SettingsError`]; a later phase may
/// aggregate diagnostics across layers.
///
/// ```
/// use vibe_settings::loader::load_all;
/// use std::path::Path;
///
/// // All three absent → an empty triple, not an error (defaults win).
/// let raw = load_all(
///     Path::new("/no/such/vibe-settings-l1.toml"),
///     Path::new("/no/such/vibe-settings-l2.toml"),
///     Path::new("/no/such/vibe-settings-l3.toml"),
/// )
/// .unwrap();
/// assert!(raw.l1.is_empty() && raw.l2.is_empty() && raw.l3.is_empty());
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#locations")]
pub fn load_all(l1: &Path, l2: &Path, l3: &Path) -> Result<LayeredRaw, SettingsError> {
    Ok(LayeredRaw {
        l1: load_layer(l1)?,
        l2: load_layer(l2)?,
        l3: load_layer(l3)?,
    })
}

// ─── path-classifier helpers (private; deterministic, no env) ─────────────────

/// Whether `path` is `<settings-dir>/<file>` — i.e. its parent is exactly
/// the resolved settings directory. `false` when the settings dir is
/// unknown. This is the L1 test (PROP-040 §9 `#path-classifier`): under
/// `$VIBE_SETTINGS` the L1 dir is arbitrary, so L1 is location-by-parent,
/// not name-by-`.vibe`.
fn is_l1(path: &Path, settings_dir: Option<&Path>) -> bool {
    matches!(
        (path.parent(), settings_dir),
        (Some(parent), Some(dir)) if parent == dir
    )
}

/// The resolved L1 settings directory used to *tag* a classified file:
/// `$VIBE_SETTINGS` (verbatim) else `<home>/.vibe`. Deliberately mirrors
/// the workspace chokepoint `vibe_core::settings::settings_dir`;
/// `vibe-settings` stays `vibe-core`-free (PROP-040 §12 `#crate-boundary`),
/// so this thin resolution is duplicated here rather than imported — keep
/// the two in sync.
fn resolved_settings_dir() -> Option<PathBuf> {
    if let Some(o) = std::env::var_os("VIBE_SETTINGS").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(o));
    }
    Some(home_dir()?.join(DOT_VIBE))
}

/// Best-effort user-home detection — reads `HOME` (Unix / Git Bash) then
/// `USERPROFILE` (Windows). PROP-040 §3 fixes L1 at `~/.vibe/`; this lookup
/// locates that root. Read-only — it mutates no ambient state. Mirrors the
/// established pattern in `vibe-core::user_config`.
fn home_dir() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(h));
    }
    if cfg!(windows)
        && let Some(p) = std::env::var_os("USERPROFILE").filter(|s| !s.is_empty())
    {
        return Some(PathBuf::from(p));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SettingsError;
    use std::fs;
    use tempfile::tempdir;

    // ── path classifier (§9 #path-classifier) ───────────────────────────────

    #[test]
    fn classify_local_is_l3_by_name_anywhere() {
        // Name wins regardless of directory or known home.
        assert_eq!(
            classify_with_home(Path::new("/repo/.vibe/settings.local.toml"), None),
            Layer::L3
        );
        assert_eq!(
            classify_with_home(
                Path::new("/home/u/.vibe/settings.local.toml"),
                Some(Path::new("/home/u")),
            ),
            Layer::L3
        );
        // Even at an unusual location, the name makes it L3.
        assert_eq!(
            classify_with_home(Path::new("/tmp/settings.local.toml"), None),
            Layer::L3
        );
    }

    #[test]
    fn classify_settings_toml_splits_l1_l2_by_home() {
        let home = Path::new("/home/u");
        assert_eq!(
            classify_with_home(Path::new("/home/u/.vibe/settings.toml"), Some(home)),
            Layer::L1
        );
        assert_eq!(
            classify_with_home(Path::new("/srv/repo/.vibe/settings.toml"), Some(home)),
            Layer::L2
        );
        // No home known → a `.vibe/settings.toml` is repo-shared (L2).
        assert_eq!(
            classify_with_home(Path::new("/srv/repo/.vibe/settings.toml"), None),
            Layer::L2
        );
    }

    #[test]
    fn classify_requires_dotvibe_parent_for_l1() {
        let home = Path::new("/home/u");
        // `settings.toml` directly under home (no `.vibe/`) is NOT L1 — the L1
        // location is specifically `~/.vibe/settings.toml` (§3 #file-layout).
        assert_eq!(
            classify_with_home(Path::new("/home/u/settings.toml"), Some(home)),
            Layer::L2
        );
    }

    #[test]
    fn classify_unknown_basename_falls_back_to_l2() {
        assert_eq!(
            classify_with_home(Path::new("/etc/random.toml"), None),
            Layer::L2
        );
        assert_eq!(
            classify_with_home(Path::new("/x/vibe.toml"), None),
            Layer::L2
        );
        assert_eq!(
            classify_with_home(Path::new("no_extension"), None),
            Layer::L2
        );
    }

    #[test]
    fn layer_metadata_is_stable() {
        assert_eq!(Layer::L1.label(), "L1");
        assert_eq!(Layer::L2.label(), "L2");
        assert_eq!(Layer::L3.label(), "L3");
        assert_eq!(Layer::L1.file_name(), "settings.toml");
        assert_eq!(Layer::L2.file_name(), "settings.toml");
        assert_eq!(Layer::L3.file_name(), "settings.local.toml");
        assert!(Layer::L1.role_marker().contains("user-machine"));
        assert!(Layer::L2.role_marker().contains("repo-shared"));
        assert!(Layer::L3.role_marker().contains("user-project"));
        // Display renders the tag.
        assert_eq!(Layer::L3.to_string(), "L3");
    }

    // ── loaders (§3 #missing-is-default, #locations) ────────────────────────

    #[test]
    fn load_layer_missing_file_is_empty_not_error() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("absent.toml");
        let table = load_layer(&p).unwrap();
        assert!(table.is_empty());
    }

    #[test]
    fn load_layer_parses_valid_toml() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("settings.local.toml");
        fs::write(&p, "tree.palette = \"rosé-pine\"\n[node]\nfold = true\n").unwrap();
        let table = load_layer(&p).unwrap();
        assert_eq!(table.len(), 2); // `tree` + `node`
        assert!(table.contains_key("tree"));
        assert!(table.contains_key("node"));
    }

    #[test]
    fn load_layer_malformed_is_parse_error_tagged_with_layer() {
        let dir = tempdir().unwrap();
        // Name makes it L3; the malformed body must surface as Parse(L3).
        let p = dir.path().join("settings.local.toml");
        fs::write(&p, "not = valid = toml\n").unwrap();
        let err = load_layer(&p).unwrap_err();
        assert!(matches!(err, SettingsError::Parse { .. }));
        let msg = err.to_string();
        assert!(msg.contains("L3"), "diagnostic names the layer: {msg}");
        assert!(
            msg.contains("missing-is-default"),
            "diagnostic cites the REQ: {msg}"
        );
    }

    #[test]
    fn load_layer_io_error_is_typed() {
        // A directory is present but not a readable file → Io error (not Parse).
        let dir = tempdir().unwrap();
        let err = load_layer(dir.path()).unwrap_err();
        assert!(matches!(err, SettingsError::Io { .. }));
        let msg = err.to_string();
        assert!(msg.contains("missing-is-default"));
    }

    #[test]
    fn load_all_three_missing_yields_empty_triple() {
        let dir = tempdir().unwrap();
        let raw = load_all(
            &dir.path().join("l1.toml"),
            &dir.path().join("l2.toml"),
            &dir.path().join("l3.toml"),
        )
        .unwrap();
        assert!(raw.l1.is_empty());
        assert!(raw.l2.is_empty());
        assert!(raw.l3.is_empty());
        assert_eq!(raw.layer(Layer::L2).len(), 0);
        // Default-equivalent.
        assert_eq!(raw.l1.len(), LayeredRaw::new().l1.len());
    }

    #[test]
    fn load_all_mixed_present_and_absent() {
        let dir = tempdir().unwrap();
        let l2 = dir.path().join(".vibe").join("settings.toml");
        fs::create_dir_all(l2.parent().unwrap()).unwrap();
        fs::write(&l2, "[tree]\npalette = \"default\"\n").unwrap();

        let raw = load_all(
            &dir.path().join("l1.toml"),             // absent
            &l2,                                     // present
            &dir.path().join("settings.local.toml"), // absent
        )
        .unwrap();

        assert!(raw.l1.is_empty(), "absent L1 → empty");
        assert!(!raw.l2.is_empty(), "present L2 → parsed");
        assert!(raw.l2.contains_key("tree"));
        assert!(raw.l3.is_empty(), "absent L3 → empty");
    }

    #[test]
    fn load_all_short_circuits_on_first_parse_error() {
        let dir = tempdir().unwrap();
        let good = dir.path().join("settings.toml");
        fs::write(&good, "[tree]\npalette = \"x\"\n").unwrap();
        let bad = dir.path().join("settings.local.toml");
        fs::write(&bad, "broken = = =\n").unwrap();

        let err = load_all(&good, &good, &bad).unwrap_err();
        assert!(matches!(err, SettingsError::Parse { .. }));
    }
}
