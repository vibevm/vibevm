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

    // The interactive rat-salsa TUI launches when the session is attended and
    // `--plain` was not passed (PROP-036 §2.11). `--plain` and a non-tty fall
    // through to the plain ASCII renderer; `--json` returned above. Neither
    // `--json` nor `--plain` ever enters interactive mode (§2.1).
    if console::user_attended() && !args.plain {
        return tui::run(tree);
    }
    print!("{}", plain::render(&tree));
    Ok(())
}
