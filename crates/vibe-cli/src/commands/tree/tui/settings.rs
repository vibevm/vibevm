//! The vibe-settings bridge for the `vibe tree` TUI (PROP-037 §9, PROP-040).
//!
//! Owns the one place the TUI touches the settings system: the `vibe.tree.*`
//! schema (palette, tier, mode, sort, shape, static-first), the launch-time
//! [`load`](TreeSettings::load), the [`theme`](TreeSettings::theme) + UI-state
//! snapshot built from the resolved prefs, and the atomic
//! [`set`](TreeSettings::set) that persists a single key when the F2/F3 menus
//! change it. Every key is application/user preference (NOT `vibe.toml`),
//! declared up front in a [`Schema`] so typos surface and the surface is
//! introspectable (PROP-040 §6 `#schema-first`).
//!
//! Scope (PROP-040 §7): palette + tier + mode + sort + shape + static-first are
//! all [`Scope::User`] — they roam across the L1/L2/L3 layers; the TUI persists
//! them to L1 (`~/.vibe/settings.toml`) by default.
//!
//! Frontend-agnostic reads + a thin write seam: a corrupt or missing settings
//! file is swallowed and warned via `tracing` (PROP-037 §9 "missing/corrupt →
//! defaults"), never a hard error — the TUI always launches with *some* theme.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#settings");

use std::path::PathBuf;

use vibe_settings::loader::{Layer, LayeredRaw, load_all};
use vibe_settings::persist::{diff_from_default, write_layer};
use vibe_settings::resolver::{self, ResolvedPrefs};
use vibe_settings::schema::{KeyMeta, KeyType, Schema, Scope};

use super::shape::TreeShape;
use super::state::{DisplayMode, Ordering};
use super::theme::{PaletteName, Theme, Tier, detect_tier};

// ── key paths (PROP-037 §9) ─────────────────────────────────────────────────

/// The dotted path of the palette key.
pub const KEY_PALETTE: &str = "vibe.tree.palette";
/// The dotted path of the optional tier-override key.
pub const KEY_TIER: &str = "vibe.tree.tier";
/// The dotted path of the display-mode key.
pub const KEY_MODE: &str = "vibe.tree.mode";
/// The dotted path of the row-ordering key.
pub const KEY_SORT: &str = "vibe.tree.sort";
/// The dotted path of the tree-shape key.
pub const KEY_SHAPE: &str = "vibe.tree.shape";
/// The dotted path of the static-first block-order key.
pub const KEY_STATIC_FIRST: &str = "vibe.tree.static-first";
/// The dotted path of the launch-mode key (TERMINAL-AIUI §6.2): where a bare
/// `vibe tree` opens.
pub const KEY_LAUNCH_MODE: &str = "vibe.tree.launch-mode";
/// The dotted path of the last-opened-project key (VIBE-LAUNCHERS): the project
/// root recorded on every successful open, so a context-free VibeTree launch
/// (double-clicked from `~/opt/bin`, no project in cwd) reopens it.
pub const KEY_LAST_PROJECT: &str = "vibe.tree.last-project";

/// The conventional L1 file: `<home>/.vibe/settings.toml`.
const DOT_VIBE: &str = ".vibe";
/// The shared settings basename (L1 and L2 share it; told apart by location).
const SETTINGS_TOML: &str = "settings.toml";

/// The palette key's built-in default — Rosé Pine, the canonical-locked look.
pub const DEFAULT_PALETTE: &str = "rose-pine";

// ── string ↔ enum mappings (the wire format for each key) ───────────────────

/// Parse a palette label into a [`PaletteName`]. Accepts the [`PaletteName::label`]
/// spellings (`rose-pine`, `catppuccin-mocha`, …); an unknown value falls back
/// to the default (Rosé Pine) — a corrupt settings file never breaks the launch.
#[must_use]
pub fn parse_palette(value: Option<&str>) -> PaletteName {
    match value {
        Some(s) => match s {
            "rose-pine" => PaletteName::RosePine,
            "catppuccin-mocha" => PaletteName::Mocha,
            "catppuccin-macchiato" => PaletteName::Macchiato,
            "catppuccin-frappe" => PaletteName::Frappe,
            "catppuccin-latte" => PaletteName::Latte,
            _ => PaletteName::RosePine,
        },
        None => PaletteName::RosePine,
    }
}

/// The display-mode label (the `vibe.tree.mode` wire value).
#[must_use]
pub(super) fn mode_label(mode: DisplayMode) -> &'static str {
    match mode {
        DisplayMode::All => "all",
        DisplayMode::SubTables => "sub-tables",
        DisplayMode::Tabs => "tabs",
    }
}

/// Parse a display-mode label; an unknown/absent value falls back to `All`.
#[must_use]
fn parse_mode(value: Option<&str>) -> DisplayMode {
    match value {
        Some("sub-tables") => DisplayMode::SubTables,
        Some("tabs") => DisplayMode::Tabs,
        _ => DisplayMode::All,
    }
}

/// The ordering label (the `vibe.tree.sort` wire value).
#[must_use]
pub(super) fn sort_label(order: Ordering) -> &'static str {
    match order {
        Ordering::Topological => "topological",
        Ordering::Alphabetical => "alphabetical",
    }
}

/// Parse an ordering label; an unknown/absent value falls back to `Topological`.
#[must_use]
fn parse_sort(value: Option<&str>) -> Ordering {
    match value {
        Some("alphabetical") => Ordering::Alphabetical,
        _ => Ordering::Topological,
    }
}

/// The shape label (the `vibe.tree.shape` wire value).
#[must_use]
pub(super) fn shape_label(shape: TreeShape) -> &'static str {
    match shape {
        TreeShape::MembersAsRoots => "members-as-roots",
        TreeShape::LoadTypeForest => "load-type-forest",
        TreeShape::PrunedTree => "pruned-tree",
    }
}

/// Parse a shape label; an unknown/absent value falls back to the default shape.
#[must_use]
fn parse_shape(value: Option<&str>) -> TreeShape {
    match value {
        Some("load-type-forest") => TreeShape::LoadTypeForest,
        Some("pruned-tree") => TreeShape::PrunedTree,
        _ => TreeShape::MembersAsRoots,
    }
}

/// Where a bare `vibe tree` opens (TERMINAL-AIUI §6.2): the in-terminal console
/// TUI, or the vibeterm desktop terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    /// The in-terminal console TUI (today's behaviour; the clean-install default).
    Console,
    /// Open in the vibeterm desktop terminal.
    Vibeterm,
}

/// The launch-mode label (the `vibe.tree.launch-mode` wire value).
#[must_use]
pub(super) fn launch_mode_label(mode: LaunchMode) -> &'static str {
    match mode {
        LaunchMode::Console => "console",
        LaunchMode::Vibeterm => "vibeterm",
    }
}

/// Parse a launch-mode label; an unknown/absent value falls back to `Console`
/// — the clean-install default never forces the desktop app on a fresh user.
#[must_use]
fn parse_launch_mode(value: Option<&str>) -> LaunchMode {
    match value {
        Some("vibeterm") => LaunchMode::Vibeterm,
        _ => LaunchMode::Console,
    }
}

// ── the resolved UI snapshot ────────────────────────────────────────────────

/// The TUI's view of its persisted state — the `vibe.tree.*` leaves projected
/// onto the typed enums the model carries. Built once from a [`ResolvedPrefs`]
/// on launch; every field has a sensible default so a missing settings file
/// yields the pre-settings look.
#[derive(Debug, Clone)]
pub struct TreePrefs {
    /// The active palette name.
    #[allow(dead_code)] // introspection: carried for a future settings UI / `vibe prefs show`.
    pub palette: PaletteName,
    /// The rendering tier — an explicit override, or `None` to auto-detect.
    #[allow(dead_code)] // introspection: carried for a future settings UI / `vibe prefs show`.
    pub tier_override: Option<Tier>,
    /// The display mode (`x` / F3).
    pub mode: DisplayMode,
    /// The row ordering (`n` / F2 sort).
    pub sort: Ordering,
    /// The tree shape (F2 shape group).
    pub shape: TreeShape,
    /// Whether `static` sorts before `dynamic` (F2 block-order / `t`).
    pub static_first: bool,
}

impl Default for TreePrefs {
    /// The built-in defaults: Rosé Pine, auto-detect tier, `all` mode,
    /// topological sort, members-as-roots shape, static-first.
    fn default() -> Self {
        Self {
            palette: PaletteName::RosePine,
            tier_override: None,
            mode: DisplayMode::All,
            sort: Ordering::Topological,
            shape: TreeShape::MembersAsRoots,
            static_first: true,
        }
    }
}

// ── TreeSettings — the schema + paths + load/save cell ──────────────────────

/// The settings cell the TUI carries: the declared `vibe.tree.*` [`Schema`],
/// the three layer paths, and the load/save operations over them.
///
/// Production builds with [`TreeSettings::new`] locate L1 at `~/.vibe/` and
/// L2/L3 at `<cwd>/.vibe/`; tests build one with [`TreeSettings::with_paths`]
/// against a tempdir so they never touch the operator's real settings.
pub struct TreeSettings {
    schema: Schema,
    l1: PathBuf,
    l2: PathBuf,
    l3: PathBuf,
}

impl Default for TreeSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSettings {
    /// Build the schema + locate the conventional layer paths from the process
    /// environment: L1 at `~/.vibe/settings.toml`, L2/L3 at `<cwd>/.vibe/`.
    /// Used by the launch path ([`super::run`]); tests use
    /// [`TreeSettings::with_paths`] against a tempdir.
    #[must_use]
    pub fn new() -> Self {
        let l1 = home_dot_vibe().join(SETTINGS_TOML);
        let cwd_l2 = repo_dot_vibe().join(SETTINGS_TOML);
        let cwd_l3 = repo_dot_vibe().join("settings.local.toml");
        Self::with_paths(l1, cwd_l2, cwd_l3)
    }

    /// Build the schema with explicit layer paths — the test entry point.
    #[must_use]
    pub fn with_paths(l1: PathBuf, l2: PathBuf, l3: PathBuf) -> Self {
        Self {
            schema: build_schema(),
            l1,
            l2,
            l3,
        }
    }

    /// The declared `vibe.tree.*` schema (introspection for the AIUI / `vibe prefs`).
    #[must_use]
    #[allow(dead_code)] // introspection: read by tests + a future `vibe prefs` surface.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// The conventional file path for a given layer (PROP-040 §3). Used by the
    /// `vibe prefs` settings form to write a key to the chosen write-layer
    /// (PROP-041 §4 `#write-layer-choice`) — the form mirrors this cell's
    /// persist path (load → set-dotted → diff → atomic-write) against whichever
    /// layer the user picked.
    #[must_use]
    pub fn layer_path(&self, layer: Layer) -> &std::path::Path {
        match layer {
            Layer::L1 => &self.l1,
            Layer::L2 => &self.l2,
            Layer::L3 => &self.l3,
        }
    }

    /// Load + resolve every layer into one immutable snapshot (PROP-040 §5).
    /// A missing/unreadable file is swallowed — [`load_all`] treats a missing
    /// file as an empty table; a parse error is logged and treated as empty so
    /// the TUI always launches.
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#settings")]
    pub fn load(&self) -> ResolvedPrefs {
        let raw = match load_all(&self.l1, &self.l2, &self.l3) {
            Ok(raw) => raw,
            Err(err) => {
                tracing::warn!(
                    %err,
                    l1 = %self.l1.display(),
                    "vibe.tree settings: a layer failed to load — using built-in defaults"
                );
                LayeredRaw::default()
            }
        };
        resolver::resolve(raw, &self.schema, toml::Table::new(), toml::Table::new())
    }

    /// Build the active [`Theme`] from the resolved prefs: the palette + tier
    /// (explicit override, else auto-detected from `COLORTERM`/`TERM`).
    #[must_use]
    pub fn theme(&self, prefs: &ResolvedPrefs) -> Theme {
        let palette = parse_palette(prefs.get(KEY_PALETTE).and_then(toml::Value::as_str));
        let tier = prefs
            .get(KEY_TIER)
            .and_then(toml::Value::as_integer)
            .and_then(tier_from_int)
            .unwrap_or_else(detect_env_tier);
        Theme::new(palette, tier)
    }

    /// Project the resolved prefs onto the typed UI snapshot ([`TreePrefs`]).
    #[must_use]
    pub fn snapshot(&self, prefs: &ResolvedPrefs) -> TreePrefs {
        TreePrefs {
            palette: parse_palette(prefs.get(KEY_PALETTE).and_then(toml::Value::as_str)),
            tier_override: prefs
                .get(KEY_TIER)
                .and_then(toml::Value::as_integer)
                .and_then(tier_from_int),
            mode: parse_mode(prefs.get(KEY_MODE).and_then(toml::Value::as_str)),
            sort: parse_sort(prefs.get(KEY_SORT).and_then(toml::Value::as_str)),
            shape: parse_shape(prefs.get(KEY_SHAPE).and_then(toml::Value::as_str)),
            static_first: prefs
                .get(KEY_STATIC_FIRST)
                .and_then(toml::Value::as_bool)
                .unwrap_or(true),
        }
    }

    /// The persisted launch mode for a bare `vibe tree` (TERMINAL-AIUI §6.2): the
    /// `vibe.tree.launch-mode` setting, defaulting to `console` (never force the
    /// desktop app on a fresh user).
    #[must_use]
    pub fn launch_mode(&self, prefs: &ResolvedPrefs) -> LaunchMode {
        parse_launch_mode(prefs.get(KEY_LAUNCH_MODE).and_then(toml::Value::as_str))
    }

    /// The last project opened in `vibe tree` (VIBE-LAUNCHERS): recorded on every
    /// open, so a context-free VibeTree launch reopens it. Absent (`None`) until
    /// the first open, or if the stored value is blank.
    #[must_use]
    pub fn last_project(&self, prefs: &ResolvedPrefs) -> Option<PathBuf> {
        prefs
            .get(KEY_LAST_PROJECT)
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
    }

    /// Persist a single key to the L1 layer atomically (PROP-037 §9, PROP-040
    /// §6 `#diff-from-default`): load the current L1 table (preserving the
    /// other keys), set the one dotted path, diff against the schema defaults,
    /// and install via a sibling `.tmp` + rename. A key set to its default is
    /// dropped from the file by the diff.
    pub fn set(&self, path: &str, value: toml::Value) {
        if let Err(err) = self.try_set(path, value) {
            tracing::warn!(
                %err,
                key = path,
                l1 = %self.l1.display(),
                "vibe.tree settings: failed to persist a key — the change is live for this session only"
            );
        }
    }

    /// The fallible core of [`Self::set`], separated so the error can be
    /// logged precisely at the call site rather than swallowed here.
    fn try_set(&self, path: &str, value: toml::Value) -> Result<(), SetError> {
        let mut table = vibe_settings::loader::load_layer(&self.l1)
            .map_err(|err| SetError::Load(err.to_string()))?;
        set_dotted(&mut table, path, value);
        let diffed = diff_from_default(&table, &self.schema);
        write_layer(&self.l1, &diffed, Layer::L1).map_err(|err| SetError::Write(err.to_string()))
    }
}

/// Why a [`TreeSettings::set`] failed — a load or a write; the inner message
/// carries the underlying typed error's `Display` (kept as a string so this
/// stays a thin, dependency-light enum).
#[derive(Debug)]
enum SetError {
    Load(String),
    Write(String),
}

impl std::fmt::Display for SetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetError::Load(msg) => write!(f, "load L1 failed: {msg}"),
            SetError::Write(msg) => write!(f, "write L1 failed: {msg}"),
        }
    }
}

impl std::error::Error for SetError {}

// ── the schema (one registration point — PROP-040 §6 #schema-first) ─────────

/// Declare the `vibe.tree.*` keys: paths, types, built-in defaults, and
/// metadata. Each key is [`Scope::User`] (roams L1→L2→L3); the palette + tier
/// Declare the `vibe.tree.*` keys: paths, types, built-in defaults, and
/// metadata. Each key is [`Scope::User`] (roams L1→L2→L3); the palette + tier
/// take effect on restart, the rest live.
///
/// The `.expect()` calls are on `KeyMeta::new` (which validates a non-empty
/// description) and `Schema::register` (which rejects duplicates). Both are
/// unreachable here: every description is a non-empty string literal and every
/// path is a unique `const`. This is the single schema-registration point, so
/// the invariant is asserted once rather than threaded fallibly through every
/// caller.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-gate: the eight vibe.tree.* KeyMeta are built from static \
              non-empty literal descriptions and unique dotted paths — KeyMeta::new \
              and Schema::register cannot fail on these inputs; asserting the \
              invariant once at this single registration point is clearer than \
              propagating an unreachable Result through every caller"
)]
fn build_schema() -> Schema {
    let mut schema = Schema::new();
    schema
        .register(
            KeyMeta::new(
                KEY_PALETTE,
                KeyType::String,
                Scope::User,
                "the Vibe Tree colour palette (rose-pine, catppuccin-mocha, …)",
            )
            .expect("non-empty palette doc")
            .with_default(toml::Value::String(DEFAULT_PALETTE.into())),
        )
        .expect("unique palette key");
    schema
        .register(
            KeyMeta::new(
                KEY_TIER,
                KeyType::Int,
                Scope::User,
                "rendering-tier override (0=mono, 1=16-colour, 2=256, 3=truecolour); \
                 absent = auto-detect from COLORTERM/TERM",
            )
            .expect("non-empty tier doc"),
        )
        .expect("unique tier key");
    schema
        .register(
            KeyMeta::new(
                KEY_MODE,
                KeyType::String,
                Scope::User,
                "the Vibe Tree display mode (all, sub-tables, tabs)",
            )
            .expect("non-empty mode doc")
            .with_default(toml::Value::String(mode_label(DisplayMode::All).into())),
        )
        .expect("unique mode key");
    schema
        .register(
            KeyMeta::new(
                KEY_SORT,
                KeyType::String,
                Scope::User,
                "the Vibe Tree row ordering (topological, alphabetical)",
            )
            .expect("non-empty sort doc")
            .with_default(toml::Value::String(
                sort_label(Ordering::Topological).into(),
            )),
        )
        .expect("unique sort key");
    schema
        .register(
            KeyMeta::new(
                KEY_SHAPE,
                KeyType::String,
                Scope::User,
                "the Vibe Tree forest shape (members-as-roots, load-type-forest, pruned-tree)",
            )
            .expect("non-empty shape doc")
            .with_default(toml::Value::String(
                shape_label(TreeShape::MembersAsRoots).into(),
            )),
        )
        .expect("unique shape key");
    schema
        .register(
            KeyMeta::new(
                KEY_STATIC_FIRST,
                KeyType::Bool,
                Scope::User,
                "in the partitioned modes, whether `static` sorts before `dynamic`",
            )
            .expect("non-empty static-first doc")
            .with_default(toml::Value::Boolean(true)),
        )
        .expect("unique static-first key");
    schema
        .register(
            KeyMeta::new(
                KEY_LAUNCH_MODE,
                KeyType::String,
                Scope::User,
                "where a bare `vibe tree` opens (console = the in-terminal TUI, vibeterm = the desktop terminal)",
            )
            .expect("non-empty launch-mode doc")
            .with_default(toml::Value::String(
                launch_mode_label(LaunchMode::Console).into(),
            )),
        )
        .expect("unique launch-mode key");
    schema
        .register(
            KeyMeta::new(
                KEY_LAST_PROJECT,
                KeyType::String,
                Scope::User,
                "the last project opened in `vibe tree`; recorded on open so a context-free VibeTree launch reopens it (no default — absent until the first open)",
            )
            .expect("non-empty last-project doc"),
        )
        .expect("unique last-project key");
    schema
}

// ── private helpers ─────────────────────────────────────────────────────────

/// Map a settings integer to a [`Tier`]. Values outside 0..=3 yield `None`
/// (the caller then auto-detects).
fn tier_from_int(n: i64) -> Option<Tier> {
    match n {
        0 => Some(Tier::T0),
        1 => Some(Tier::T1),
        2 => Some(Tier::T2),
        3 => Some(Tier::T3),
        _ => None,
    }
}

/// Auto-detect the tier from `COLORTERM`/`TERM` (PROP-037 §2.2.3).
fn detect_env_tier() -> Tier {
    let colorterm = std::env::var("COLORTERM").ok();
    let term = std::env::var("TERM").ok();
    detect_tier(colorterm.as_deref(), term.as_deref())
}

/// Insert `value` at a dotted `path` in `table`, creating intermediate tables.
pub(crate) fn set_dotted(table: &mut toml::Table, path: &str, value: toml::Value) {
    let mut segments = path.split('.');
    let Some(last) = segments.next_back() else {
        return;
    };
    let mut current = table;
    for seg in segments {
        let entry = current
            .entry(seg.to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        match entry {
            toml::Value::Table(t) => current = t,
            // A prior scalar at this segment blocks descent; replace it with a
            // table so the dotted path lands (a settings file that had
            // `vibe = "x"` then `vibe.tree.mode` is recovered, not panicked).
            other => {
                *other = toml::Value::Table(toml::Table::new());
                // Re-borrow the now-Table entry; the `let-else` is unreachable
                // (the preceding line set it to a Table) but keeps the borrow
                // checker happy without an `expect`.
                let toml::Value::Table(t) = other else {
                    return;
                };
                current = t;
            }
        }
    }
    current.insert(last.to_owned(), value);
}

/// Locate `~/.vibe/` — the L1 root (PROP-040 §3). Reads `HOME` then
/// `USERPROFILE`; falls back to the conventional `.vibe` relative path when
/// neither is set (the classifier still treats it as L1-by-name only when it
/// sits under a known home — this fallback is best-effort for the launch path).
fn home_dot_vibe() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
        return PathBuf::from(home).join(DOT_VIBE);
    }
    if cfg!(windows)
        && let Some(profile) = std::env::var_os("USERPROFILE").filter(|s| !s.is_empty())
    {
        return PathBuf::from(profile).join(DOT_VIBE);
    }
    PathBuf::from(DOT_VIBE)
}

/// Locate `<cwd>/.vibe/` — the L2/L3 root.
fn repo_dot_vibe() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DOT_VIBE)
}

#[cfg(test)]
mod tests;
