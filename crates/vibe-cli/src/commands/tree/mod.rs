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
mod model;
mod plain;

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

    // Phase 1 renders the plain ASCII tree on every non-JSON surface.
    //
    // Phase 2 seam: the interactive rat-salsa TUI launches when the session
    // is attended and `--plain` was not passed; `--plain` and a non-tty keep
    // the plain renderer below (PROP-036 §2.11 fallback). The decision is
    // wired now so Phase 2 is a one-line branch here.
    let _would_launch_tui = console::user_attended() && !args.plain;
    print!("{}", plain::render(&tree));
    Ok(())
}
