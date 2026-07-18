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
mod model;
mod plain;
// `pub(crate)` so the `vibe prefs` settings TUI (PROP-041) composes the same
// PROP-037 `ui::` component library + `theme::Theme` without duplicating them —
// the component library is the reuse unit (PROP-041 §1 #built-on-tree-tui).
pub(crate) mod tui;

use anyhow::Result;
use serde_json::Value;

use crate::cli::TreeArgs;
use crate::output;

/// Run `vibe tree` — build the model, then dispatch to the requested surface.
pub fn run(ctx: &output::Context, args: TreeArgs) -> Result<()> {
    let root = super::resolve_project_root(&args.path)?;
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
    if args.terminal && !args.plain {
        return open_in_vibeterm(&root);
    }

    // The interactive rat-salsa TUI launches when the session is attended and
    // `--plain` was not passed (PROP-036 §2.11). `--plain` and a non-tty fall
    // through to the plain ASCII renderer; `--json` returned above. Neither
    // `--json` nor `--plain` ever enters interactive mode (§2.1).
    if console::user_attended() && !args.plain {
        return match resolve_launch_mode(&args) {
            tui::settings::LaunchMode::Vibeterm => open_in_vibeterm(&root),
            tui::settings::LaunchMode::Console => tui::run(tree),
        };
    }
    print!("{}", plain::render(&tree));
    Ok(())
}

/// Resolve where `vibe tree` opens: an explicit `-c`/`-t` flag wins; else the
/// persisted `vibe.tree.launch-mode` setting, defaulting to `console`
/// (TERMINAL-AIUI §6.2 — never force the desktop app on a fresh user).
fn resolve_launch_mode(args: &TreeArgs) -> tui::settings::LaunchMode {
    use tui::settings::LaunchMode;
    if args.terminal {
        LaunchMode::Vibeterm
    } else if args.console {
        LaunchMode::Console
    } else {
        let settings = tui::settings::TreeSettings::new();
        settings.launch_mode(&settings.load())
    }
}

/// Open the tree in vibeterm: launch it running `vibe tree --path <root> -c`, so
/// the child renders the console TUI *inside* vibeterm (the `-c` prevents `-t`
/// recursion). The running binary is quoted, since an installed path may contain
/// spaces (a `<root>` with spaces is a known edge until vibeterm's `splitCommand`
/// tokenises quoted arguments).
fn open_in_vibeterm(root: &std::path::Path) -> Result<()> {
    let exe = std::env::current_exe()?;
    let exec = format!(
        "{} tree --path {} -c",
        super::term::quote_exe(&exe.to_string_lossy()),
        root.display(),
    );
    // The tree carries the `vibetree` window icon so it matches VibeTree.exe
    // (PROP-043 #icon / VIBE-LAUNCHERS D8).
    super::term::launch_vibeterm(&exec, None, None, Some("vibetree"))
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
