//! `vibe prefs ui` — launch the interactive settings TUI (PROP-041). The
//! command surface over the [`super::tui`] module: loads the three on-disk
//! layers plus the shared application schema ([`super::load`] builds it at the
//! one registration point the tree TUI's settings cell owns), resolves
//! the snapshot, determines whether there's an active project (L2) on disk, and
//! hands all three to [`super::tui::run`].
//!
//! Launches only when the session is attended (`--unattended` or a non-tty
//! refuses, mirroring `vibe tree`'s gate). The TUI owns no preference logic
//! (PROP-041 §1 `#surface-not-engine`); it reads `ResolvedPrefs` + the schema.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-041#overview");

use std::path::Path;

use anyhow::Result;
use vibe_settings::resolver;

use crate::cli::PrefsPathArgs;
use crate::commands::prefs::tui;
use crate::commands::prefs::tui::state::PrefsCtx;
use crate::commands::prefs::{Loaded, load, resolve_repo, warn_load_warnings};
use crate::output;

/// Run `vibe prefs ui` — load + resolve + launch the settings TUI.
pub fn run(ctx: &output::Context, args: PrefsPathArgs) -> Result<()> {
    // `--unattended` refuses to open any interactive wizard (the global flag's
    // contract); a non-tty cannot drive the TUI either. Both fall back to a
    // one-line notice.
    if ctx.is_unattended() || !console::user_attended() {
        if ctx.is_quiet() {
            ctx.summary("prefs ui skipped (non-interactive)");
        } else {
            eprintln!("vibe prefs ui requires an interactive terminal");
        }
        return Ok(());
    }

    let repo = resolve_repo(&args.path);
    // `load` builds the schema at the one registration point the `vibe tree`
    // TUI's settings cell owns, so the built-in `vibe.tree.*` defaults
    // materialise in the resolved snapshot (the origin hint + the theme both
    // read through them).
    let Loaded {
        raw,
        schema,
        warnings,
        ..
    } = load(&repo)?;
    warn_load_warnings(&warnings);

    let prefs = resolver::resolve(raw, &schema, toml::Table::new(), toml::Table::new());

    // An active project = the repo carries a `.vibe/` dir (an L2 context). A
    // no-project session (PROP-041 §3 #tree-context) shows only L1 pages.
    let has_project = has_project_root(&repo);

    tui::run(prefs, schema, PrefsCtx::new(has_project))
}

/// Whether `repo` is an active project root (carries a `.vibe/` directory). A
/// no-project session hides project-scoped pages (PROP-041 §3 #tree-context).
fn has_project_root(repo: &Path) -> bool {
    repo.join(".vibe").exists()
}
