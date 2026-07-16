//! `vibe prefs <subcommand>` — the CLI surface for application/user preferences
//! (PROP-040 §8 `#prefs-command`).
//!
//! Decision D3 (SETTINGS-SYSTEM-IMPL-PLAN-v0.1 §6): the *logic* (get/set/list/
//! check/migrate/show-origins over the resolver) lives in the frontend-agnostic
//! `vibe-settings::cli` cell; this module is the *surface* — clap dispatch,
//! L1/L2/L3 path resolution, layer loading, output formatting, and the disk
//! persist that `set`/`migrate` perform. Persistence is the enriched
//! `vibe-settings::persist` cell (phase 2.7): diff-from-default strips
//! default-valued keys (§6 `#diff-from-default`), `toml_edit` preserves an
//! operator's comments + role-marker header (§3 `#role-marker`), and the write
//! is atomic (sibling `.tmp` + rename).
//!
//! L1 is the user's `~/.vibe/settings.toml` (via `dirs::home_dir`); L2/L3 sit
//! under `<repo>/.vibe/`. A malformed layer is a non-fatal warning (PROP-040 §3
//! `#missing-is-default`), surfaced to stderr — the command continues with that
//! layer treated as absent.
//!
//! Spec: [PROP-040 §8](../../../../spec/modules/vibe-settings/PROP-040-settings.md#prefs-command).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#prefs-command");

mod check;
mod get;
mod list;
mod migrate;
mod origins;
mod set;
mod tui;
mod ui;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_settings::loader::{Layer, LayeredRaw, load_layer};
use vibe_settings::schema::Schema;

use crate::cli::{PrefsArgs, PrefsSubcommand};
use crate::output;

pub fn run(ctx: &output::Context, args: PrefsArgs) -> Result<()> {
    match args.command {
        PrefsSubcommand::Get(a) => get::run(ctx, a),
        PrefsSubcommand::Set(a) => set::run(ctx, a),
        PrefsSubcommand::List(a) => list::run(ctx, a),
        PrefsSubcommand::Check(a) => check::run(ctx, a),
        PrefsSubcommand::Migrate(a) => migrate::run(ctx, a),
        PrefsSubcommand::ShowOrigins(a) => origins::run(ctx, a),
        PrefsSubcommand::Ui(a) => ui::run(ctx, a),
    }
}

// ── shared helpers (pub(super) for the subcommand siblings) ─────────────────

/// The three on-disk layer paths for one repo root.
pub(super) struct LayerPaths {
    pub l1: PathBuf,
    pub l2: PathBuf,
    pub l3: PathBuf,
}

impl LayerPaths {
    /// Borrow the path for a given layer.
    pub(super) fn for_layer(&self, layer: Layer) -> &Path {
        match layer {
            Layer::L1 => &self.l1,
            Layer::L2 => &self.l2,
            Layer::L3 => &self.l3,
        }
    }
}

/// Everything a subcommand needs: the resolved layer paths, the loaded raw
/// layers, the schema, and any non-fatal load warnings (propagated to output).
pub(super) struct Loaded {
    pub paths: LayerPaths,
    pub raw: LayeredRaw,
    pub schema: Schema,
    pub warnings: Vec<String>,
}

/// Canonicalise the repo root (stripping the Windows `\\?\` verbatim prefix so
/// display is clean). Falls back to the raw path when canonicalisation fails.
fn resolve_repo(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .map(|p| crate::commands::init::strip_unc_public(p).to_path_buf())
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Resolve the L1/L2/L3 file paths for `repo_root`. L1 needs the user's home;
/// an unresolvable home is a hard error (a clearer failure than silently
/// skipping the user-machine layer).
pub(super) fn layer_paths(repo_root: &Path) -> Result<LayerPaths> {
    let home = dirs::home_dir().context(
        "could not resolve the user home for L1 (`~/.vibe/settings.toml`); \
         set HOME (Unix/Git Bash) or USERPROFILE (Windows)",
    )?;
    let dot_vibe = repo_root.join(".vibe");
    Ok(LayerPaths {
        l1: home.join(".vibe").join("settings.toml"),
        l2: dot_vibe.join("settings.toml"),
        l3: dot_vibe.join("settings.local.toml"),
    })
}

/// Load the three layers + build the schema. Each layer is loaded independently
/// so one malformed file is a non-fatal warning (PROP-040 §3
/// `#missing-is-default`), not a command abort. The schema is empty in phase
/// 2.6 — populating it (the `tree.*` TUI keys, etc.) is the consumer's job
/// (meta-plan D5; the TUI lands in Step 3).
pub(super) fn load(repo_root: &Path) -> Result<Loaded> {
    let paths = layer_paths(repo_root)?;
    let mut warnings = Vec::new();
    let l1 = load_or_warn(&paths.l1, Layer::L1, &mut warnings);
    let l2 = load_or_warn(&paths.l2, Layer::L2, &mut warnings);
    let l3 = load_or_warn(&paths.l3, Layer::L3, &mut warnings);
    Ok(Loaded {
        paths,
        raw: LayeredRaw { l1, l2, l3 },
        schema: Schema::new(),
        warnings,
    })
}

/// Load one layer, mapping a parse/I/O failure to an empty table + a warning
/// that cites the REQ (the layer is treated as absent, §3 `#missing-is-default`).
fn load_or_warn(path: &Path, layer: Layer, warnings: &mut Vec<String>) -> toml::Table {
    match load_layer(path) {
        Ok(table) => table,
        Err(err) => {
            warnings.push(format!("{layer} `{}`: {err}", path.display()));
            toml::Table::new()
        }
    }
}

/// Print non-fatal load warnings to stderr (side-channel in both human and JSON
/// modes — JSON payloads stay focused on the outcome).
pub(super) fn warn_load_warnings(warnings: &[String]) {
    for w in warnings {
        eprintln!("warning: {w}");
    }
}

/// Parse a `--layer` string (`L1`/`L2`/`L3`, case-insensitive).
pub(super) fn parse_layer(s: &str) -> Result<Layer> {
    match s.trim().to_ascii_uppercase().as_str() {
        "L1" => Ok(Layer::L1),
        "L2" => Ok(Layer::L2),
        "L3" => Ok(Layer::L3),
        other => bail!("unknown layer `{other}` (expected L1, L2, or L3)"),
    }
}

/// Coerce a CLI value string to a typed TOML value. Tries strict TOML first
/// (`true`→bool, `123`→int, `3.14`→float, `"x"`→string, `[1,2]`→array); an
/// unquoted bareword that is not a valid TOML scalar (e.g. `rosé-pine`) falls
/// back to a plain string. This keeps `vibe prefs set tree.palette rosé-pine`
/// ergonomic without forcing the user to quote every value.
pub(super) fn parse_value(s: &str) -> Result<toml::Value> {
    let wrapped = format!("__v = {s}\n");
    if let Ok(table) = toml::from_str::<toml::Table>(&wrapped)
        && let Some(v) = table.get("__v")
    {
        return Ok(v.clone());
    }
    Ok(toml::Value::String(s.to_owned()))
}

/// Persist a layer table to disk via the enriched `vibe-settings::persist`
/// cell: strip default-valued keys (§6 `#diff-from-default`), preserve an
/// existing file's comments + role-marker header (§3 `#role-marker`), and
/// install atomically (sibling `.tmp` + rename). The diff is a no-op while the
/// schema is empty (phase 2.6 state) — unknown keys pass through unchanged — so
/// `set`/`migrate` are safe before the TUI populates the schema.
pub(super) fn persist_layer(
    path: &Path,
    table: &toml::Table,
    layer: Layer,
    schema: &Schema,
) -> Result<()> {
    let diffed = vibe_settings::persist::diff_from_default(table, schema);
    vibe_settings::persist::write_layer(path, &diffed, layer)
        .with_context(|| format!("persisting {layer} to `{}`", path.display()))?;
    Ok(())
}

/// Render a TOML value for human display — scalars render as TOML (`"x"`, `123`,
/// `true`), arrays as `[1, 2]`, tables as their TOML body. `toml::to_string`
/// requires a table at the root, so a bare scalar/array is wrapped in a one-key
/// table and the `__v = ` prefix stripped; anything that still fails to
/// serialise falls back to its debug form.
pub(super) fn display_value(v: &toml::Value) -> String {
    if let toml::Value::Table(t) = v {
        return toml::to_string(t)
            .map(|s| s.trim().to_owned())
            .unwrap_or_else(|_| format!("{v:?}"));
    }
    let mut wrapper = toml::Table::new();
    wrapper.insert("__v".to_owned(), v.clone());
    toml::to_string(&wrapper)
        .ok()
        .and_then(|s| {
            s.trim()
                .strip_prefix("__v = ")
                .map(|rest| rest.trim().to_owned())
        })
        .unwrap_or_else(|| format!("{v:?}"))
}

/// Convert a TOML value to a JSON value for `--json` envelopes.
pub(super) fn value_to_json(v: &toml::Value) -> Result<serde_json::Value> {
    Ok(serde_json::to_value(v)?)
}
