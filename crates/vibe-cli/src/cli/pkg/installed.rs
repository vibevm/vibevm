//! Argument structs for the two verbs that act on what is already
//! installed — `vibe reinstall` and `vibe uninstall`. Split from `pkg.rs`
//! for the file-length budget; `pkg` re-exports both, so `crate::cli::X`
//! paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct ReinstallArgs {
    /// Any directory inside the workspace. Discovery bubbles up to the
    /// absolute workspace root; `vibe reinstall` regenerates the boot
    /// artifacts of every node. Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Re-fetch every locked package's content from its source
    /// repository — at the version `vibe.lock` pins, never re-resolving
    /// — bypassing the local cache and overwriting the current
    /// `vibedeps/` files. The escape hatch for a corrupted or
    /// hand-edited `vibedeps/` subtree. Without this flag,
    /// `vibe reinstall` only recomputes the boot artifacts from the
    /// materialised tree already on disk — no fetch, no network — which
    /// is the fix for a stale or wrongly-generated `INDEX.md`.
    #[arg(long)]
    pub force: bool,

    /// Skip the interactive confirmation prompt.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,

    /// PROP-054 `##OBS-TRACE`: record every compiler pass this invocation runs
    /// into `.vibe/trace/<run-id>/`. The flag is one half of the activation —
    /// the selected project's own `[compile] trace = true` is the other, and
    /// either alone enables it. Neither can switch the other off.
    ///
    /// `--force` is a MATERIALISATION force and unrelated: it re-fetches from
    /// source, while this only asks the compiler to narrate itself.
    #[arg(long)]
    pub trace_compile: bool,
}

#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    /// Direct package reference; removes dependencies no longer needed by the workspace.
    /// The version selector is ignored on uninstall. A bare `<name>` is
    /// resolved against this project's `vibe.lock` — or, with `-g`, this
    /// home's application index — and never against a registry's issued
    /// short names, because the verb acts on what is already installed
    /// (PROP-008 §2.6).
    pub package: String,

    /// Uninstall one user application without mutating a project.
    #[arg(short = 'g', long = "global")]
    pub global: bool,

    /// Operate on the personal specification project rather than an application or MCP store.
    #[arg(long, requires = "global", conflicts_with = "path")]
    pub user_project: bool,

    /// Target agent registration for a global MCP package; `all` removes all owned registrations.
    #[arg(long, requires = "global")]
    pub agent: Option<String>,

    /// Ask the selected application installer to force its uninstall operation.
    #[arg(long)]
    pub force: bool,

    /// Also remove the applications that require this one at runtime, after
    /// reviewing them with --plan-only (PROP-065 `#uninstall-tree`).
    #[arg(long)]
    pub cascade: bool,

    /// Also remove the auto-installed applications nothing requires any more.
    #[arg(long)]
    pub autoremove: bool,

    /// Print the application removal plan and change nothing.
    #[arg(long)]
    pub plan_only: bool,

    /// Directory of the project (defaults to current).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Skip the interactive confirmation prompt.
    #[arg(long, alias = "yes")]
    pub assume_yes: bool,

    /// PROP-066 `#uninstall`: when the package being removed offers
    /// accepted remap rows no other accepted package offers, copy those
    /// rows into this user home's own `[[remap]]` table — with
    /// `from = "<package>@<version>"` and the original reason — so the
    /// machine keeps fetching from the same places. Answers the question
    /// without a terminal; mutually exclusive with `--drop-remap`.
    #[arg(long, conflicts_with = "drop_remap")]
    pub keep_remap: bool,

    /// PROP-066 `#uninstall`: drop the orphaned remap rows with the
    /// package. Answers the question without a terminal.
    #[arg(long)]
    pub drop_remap: bool,
}
