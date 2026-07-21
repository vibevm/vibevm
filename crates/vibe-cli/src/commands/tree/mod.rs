//! `vibe tree` — the deterministic spec-tree analyzer (PROP-036).
//!
//! Read-only: joins the lockfile graph, the committed boot artifacts, and the
//! node manifests into a model that annotates every package with its
//! *effective* boot load type and the flags that explain it (`T`ransitive /
//! `C`ondition / `S`TATIC.md). Three surfaces: `--json` (the machine model,
//! §2.7), a plain ASCII tree (non-tty / `--plain`), and — Phase 2 — the
//! interactive TUI. This module owns the command flow; the engine lives in
//! [`build`], the serde model in [`model`], the artifact decompilers in
//! [`artifacts`], and the renderer in [`plain`].

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#command");

mod artifacts;
mod build;
mod diagnostics;
mod host;
mod model;
mod picker;
mod plain;
// `pub(crate)` so the `vibe prefs` settings TUI (PROP-041) composes the same
// PROP-037 `ui::` component library + `theme::Theme` without duplicating them —
// the component library is the reuse unit (PROP-041 §1 #built-on-tree-tui).
pub(crate) mod tui;

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use vibe_settings::resolver::ResolvedPrefs;

use crate::cli::TreeArgs;
use crate::output;

use tui::settings::TreeSettings;

/// Run `vibe tree` — resolve the project, build the model, then dispatch to the
/// requested surface.
pub fn run(ctx: &output::Context, args: TreeArgs) -> Result<()> {
    let settings = TreeSettings::new();
    let prefs = settings.load();

    // `--json` is a pure scripting surface: resolve strictly from the given path
    // — no remembered-project fallback, no picker, no recording, so a script is
    // never silently redirected to a different project. The human surfaces get
    // the VibeTree "works from anywhere" resolution (cwd → last → picker).
    let root = if ctx.is_json() {
        super::resolve_project_root(&args.path)?
    } else {
        match resolve_root(&args, &settings, &prefs)? {
            Some(root) => root,
            // The picker was cancelled — a clean no-op exit, so a GUI launcher
            // shows no error dialog.
            None => return Ok(()),
        }
    };
    let tree = build::build_tree(&root)?;

    if ctx.is_json() {
        // The schema-valid model is flattened under the CLI envelope
        // convention (`{"ok": true, "command": "tree", …}`, PROP-036 §2.7).
        // The `ok`/`command` envelope keys are the only additions; the rest
        // is the schema document verbatim.
        let mut payload = serde_json::json!({ "ok": true, "command": "tree" });
        if let (Value::Object(map), Ok(Value::Object(model))) =
            (&mut payload, serde_json::to_value(&tree))
        {
            for (key, value) in model {
                map.insert(key, value);
            }
        }
        ctx.emit_json(&payload)?;
        return Ok(());
    }

    // An explicit `-t`/`--terminal` is a deliberate request for the vibeterm
    // desktop app and overrides tty detection — a GUI launcher / Start-menu
    // shortcut (e.g. VibeTree.exe) has no console but still means it. `--json`
    // (returned above) and `--plain` (an explicit ASCII request) still win;
    // without `-t` a non-tty falls through to ASCII as before, so a pipe is
    // never surprised by a spawned window (TERMINAL-AIUI §6.2).
    //
    // Already inside vibeterm, on a real tty? Don't open a *second* window —
    // upgrade the current terminal in place: render the console TUI here,
    // swapping vibeterm's icon to vibetree for the session (PROP-042 §5.1).
    // The `user_attended` guard matters: `-t` with no tty (a GUI
    // double-click, or a `| pipe`) must NOT try to run the interactive TUI —
    // it would block on a non-tty — so it falls back to spawning the app.
    if args.terminal && !args.plain {
        if host::in_vibeterm() && console::user_attended() {
            return host::run_upgraded(|| tui::run(tree));
        }
        return open_in_vibeterm(&root);
    }

    // The interactive rat-salsa TUI launches when the session is attended and
    // `--plain` was not passed (PROP-036 §2.11). `--plain` and a non-tty fall
    // through to the plain ASCII renderer; `--json` returned above. Neither
    // `--json` nor `--plain` ever enters interactive mode (§2.1). Vibeterm launch
    // mode opens the desktop app — unless we are already in vibeterm, where it
    // (and plain console mode) run the TUI in place with the vibetree icon.
    if console::user_attended() && !args.plain {
        return match resolve_launch_mode(&args, &settings, &prefs) {
            tui::settings::LaunchMode::Vibeterm if !host::in_vibeterm() => open_in_vibeterm(&root),
            _ => host::run_upgraded(|| tui::run(tree)),
        };
    }
    print!("{}", plain::render(&tree));
    Ok(())
}

/// Resolve which project the tree opens — the VibeTree "works from anywhere"
/// order (VIBE-LAUNCHERS):
///
/// 1. the given path (cwd by default, or an explicit `--path`) — recorded as the
///    last project on success;
/// 2. else, if no explicit `--path` was given, the remembered last project;
/// 3. else, a `-t` (VibeTree / GUI) launch opens a native folder picker.
///
/// `Ok(None)` means the picker was cancelled — a clean no-op. A console launch
/// with neither a cwd project nor a memory returns the original `vibe init`
/// guidance. An explicit `--path` that is not a project is a hard error (never
/// silently redirected).
fn resolve_root(
    args: &TreeArgs,
    settings: &TreeSettings,
    prefs: &ResolvedPrefs,
) -> Result<Option<PathBuf>> {
    let cwd_err = match super::resolve_project_root(&args.path) {
        Ok(root) => {
            record_last_project(settings, prefs, &root);
            return Ok(Some(root));
        }
        Err(err) => {
            // An explicit `--path` that fails is a hard error; only the default
            // (`.`, i.e. cwd) falls back to the remembered project / picker.
            if args.path.as_path() != Path::new(".") {
                return Err(err);
            }
            err
        }
    };

    // The remembered project (recorded on a previous open), if still valid.
    if let Some(last) = settings.last_project(prefs)
        && let Ok(root) = super::resolve_project_root(&last)
    {
        return Ok(Some(root));
    }

    // A GUI/terminal launch (VibeTree) with nothing to show opens a picker; a
    // cancel is a clean no-op (`Ok(None)`).
    if args.terminal {
        return match picker::pick_project_folder() {
            Some(dir) => {
                let root = super::resolve_project_root(&dir).map_err(|_| {
                    anyhow!(
                        "`{}` is not a vibe project (no `vibe.toml`) — nothing to show",
                        dir.display()
                    )
                })?;
                record_last_project(settings, prefs, &root);
                Ok(Some(root))
            }
            None => Ok(None),
        };
    }

    // A console launch with no project and no memory: the original guidance
    // (names the cwd and points at `vibe init`).
    Err(cwd_err)
}

/// Record `root` as the last-opened project (VIBE-LAUNCHERS), skipping the write
/// when it is unchanged so a repeat launch from the same project does not rewrite
/// the settings file.
fn record_last_project(settings: &TreeSettings, prefs: &ResolvedPrefs, root: &Path) {
    if settings.last_project(prefs).as_deref() == Some(root) {
        return;
    }
    settings.set(
        tui::settings::KEY_LAST_PROJECT,
        toml::Value::String(root.to_string_lossy().into_owned()),
    );
}

/// Resolve where `vibe tree` opens: an explicit `-c`/`-t` flag wins; else the
/// persisted `vibe.tree.launch-mode` setting, defaulting to `console`
/// (TERMINAL-AIUI §6.2 — never force the desktop app on a fresh user).
fn resolve_launch_mode(
    args: &TreeArgs,
    settings: &TreeSettings,
    prefs: &ResolvedPrefs,
) -> tui::settings::LaunchMode {
    use tui::settings::LaunchMode;
    if args.terminal {
        LaunchMode::Vibeterm
    } else if args.console {
        LaunchMode::Console
    } else {
        settings.launch_mode(prefs)
    }
}

/// Open the tree in a desktop terminal: launch vibeframe running
/// `vibe tree --path <root> -c`, so the child renders the console TUI *inside*
/// the terminal (the `-c` prevents `-t` recursion). The running binary is
/// quoted, since an installed path may contain spaces (a `<root>` with spaces
/// is a known edge until the terminal's `splitCommand` tokenises quoted
/// arguments).
///
/// When vibeframe is not resolvable (no `$VIBEVM_VIBEFRAME`, no packaged
/// `<instance>/vibeframe/`, no `vibeframe` on `PATH`), the tree runs **in
/// place** in the current terminal via `vibe tree --path <root> -c` as a
/// subprocess — the extracted-product fallback (a box without the terminal
/// product installed still gets a working tree, just no desktop window).
fn open_in_vibeterm(root: &std::path::Path) -> Result<()> {
    let exe = std::env::current_exe()?;
    let exec = format!(
        "{} tree --path {} -c",
        super::term::quote_exe(&exe.to_string_lossy()),
        root.display(),
    );
    // The tree carries the `vibetree` window icon so it matches VibeTree.exe
    // (PROP-043 #icon / VIBE-LAUNCHERS D8).
    let exe_for_in_place = exe.clone();
    let root_for_in_place = root.to_path_buf();
    super::term::launch_vibeterm_or_in_place(&exec, Some("vibetree"), "vibeframe", || {
        run_tree_in_place(&exe_for_in_place, &root_for_in_place)
    })
}

/// Run `vibe tree --path <root> -c` synchronously in THIS terminal, inheriting
/// stdio so the TUI renders right here. The in-place fallback for when no
/// desktop terminal product (vibeframe / vibeterm) is resolvable.
fn run_tree_in_place(exe: &std::path::Path, root: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new(exe)
        .arg("tree")
        .arg("--path")
        .arg(root)
        .arg("-c")
        .status()?;
    if status.success() {
        Ok(())
    } else {
        bail!("in-place `vibe tree` exited with {:?}", status.code());
    }
}

/// Render the tree TUI headlessly to a snapshot string — the AIUI render plane
/// (PROP-042 §1/§4). Resolves + builds the model like [`run`], then projects one
/// frame (driven by the `send` key script) at `cols×rows` to `text` (or `cells`).
pub(crate) fn snapshot(
    path: &std::path::Path,
    cols: u16,
    rows: u16,
    send: &str,
    cells: bool,
) -> Result<String> {
    let root = super::resolve_project_root(path)?;
    let tree = build::build_tree(&root)?;
    tui::snapshot_headless(tree, cols, rows, send, cells)
}

/// Project the tree TUI state headlessly — the AIUI model plane (PROP-042 §4
/// `state`). Resolves + builds the model like [`snapshot`], drives the `send`
/// key script, and projects the resulting state to a
/// [`tui::model_view::TreeModelView`] for flow/state assertions.
pub(crate) fn state(path: &std::path::Path, send: &str) -> Result<tui::model_view::TreeModelView> {
    let root = super::resolve_project_root(path)?;
    let tree = build::build_tree(&root)?;
    tui::state_headless(tree, send)
}
