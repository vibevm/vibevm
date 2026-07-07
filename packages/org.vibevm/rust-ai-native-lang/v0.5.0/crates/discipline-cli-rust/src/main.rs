//! `discipline-rust` — the umbrella AI-Native discipline tool (PROP-024
//! code-bearing packages). Installing the rust-ai-native stack yields this
//! binary, so a consumer gets the whole operating surface — bootstrap,
//! the verification floor, both engines, and the sweep/brownfield tooling —
//! not a description of it. Per-language suffix (`-rust`) mirrors
//! `conform-rust` / `specmap-rust`.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "discipline-rust",
    about = "The AI-Native discipline umbrella tool (Rust stack): init, floor, engines, sweep tooling"
)]
struct Cli {
    /// Project root — where the policies and registries live. Defaults to
    /// the current dir.
    #[arg(long, global = true, default_value = ".")]
    path: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Bootstrap the discipline surface: conform.toml + specmap.toml +
    /// the discipline/registry/ files + [[external_specs]] from vibedeps/.
    Init {
        /// The spec:// namespace for this project's units (default: the
        /// root directory's name).
        #[arg(long)]
        namespace: Option<String>,
        /// Overwrite init-owned files even when they exist.
        #[arg(long)]
        force: bool,
    },
    /// The portable verification floor: fmt → test → clippy → conform →
    /// specmap → test-gate (when a baseline exists).
    Floor {
        /// Run every step even after a failure.
        #[arg(long)]
        keep_going: bool,
        /// Suppress per-step headers.
        #[arg(long)]
        quiet: bool,
        /// Also run the per-cell fast-loop (expensive).
        #[arg(long)]
        fast_loop: bool,
    },
    /// The conform gate (ENGINE-CONFORM) — delegate of `conform-rust`.
    Conform {
        #[command(subcommand)]
        cmd: ConformCmd,
    },
    /// Build (or check) the specmap index — delegate of `specmap-rust`.
    Specmap {
        /// Byte-compare against the committed specmap.json, fail on drift.
        #[arg(long)]
        check: bool,
        /// Orphan-coverage gate only (no committed index needed).
        #[arg(long, conflicts_with = "check")]
        gate: bool,
    },
    /// Traceability queries over the specmap (PROP-014 §2.6).
    Trace {
        #[command(subcommand)]
        cmd: TraceCmd,
    },
    /// Run the workspace tests and diff against the xfail-strict baseline
    /// (BROWNFIELD §4).
    TestGate {
        /// Path to the baseline registry, project-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_TESTS_BASELINE)]
        baseline: String,
    },
    /// List debt entries whose `touch:` tripwires fire on the current
    /// change set (warn-only; BROWNFIELD §3).
    Tripwire {
        /// Diff against this revision (`<base>...HEAD`) instead of the
        /// working-tree change set.
        #[arg(long)]
        base: Option<String>,
        /// Path to the debt registry, project-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_DEBT_REGISTRY)]
        debt: String,
    },
    /// The Discipline health collector (Sweep Playbook §2): advisory
    /// coverage/danger/backlog facts above the binary gates.
    Health {
        /// Where to write the JSON snapshot, project-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_HEALTH_OUT)]
        out: String,
    },
    /// The Class-E fast-loop checker: every cell builds and tests in
    /// isolation inside the per-cell budget.
    FastLoop {
        /// Check a single cell (workspace member name) instead of all.
        #[arg(long)]
        cell: Option<String>,
        /// Per-cell first-signal budget, seconds.
        #[arg(long, default_value_t = 60)]
        budget: u64,
        /// Fail on budget overruns, not only on red tests.
        #[arg(long)]
        enforce_budget: bool,
    },
    /// Scaffolded edit operations (card scaffold-i-codemods).
    Codemod {
        #[command(subcommand)]
        cmd: CodemodCmd,
    },
    /// Human views over the BROWNFIELD registries.
    Ledger {
        #[command(subcommand)]
        cmd: LedgerCmd,
    },
}

#[derive(Subcommand)]
enum LedgerCmd {
    /// Render `discipline/DEBT.md` + `discipline/INTENT.md` from the
    /// debt and intent registries (deterministic; a generated-by
    /// banner names the regen command). `--check` regenerates and
    /// byte-compares, failing on drift.
    Render {
        /// Compare instead of writing; non-zero exit on stale views.
        #[arg(long)]
        check: bool,
        /// The debt registry, project-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_DEBT_REGISTRY)]
        debt: String,
        /// The intent registry, project-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_INTENT_REGISTRY)]
        intent: String,
    },
}

#[derive(Subcommand)]
enum ConformCmd {
    /// Extract facts, run the rules, fail on any new finding past the
    /// baseline.
    Check {
        /// Limit the gate to one crate by name.
        #[arg(long)]
        scope: Option<String>,
        /// The ratchet baseline file, project-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_CONFORM_BASELINE)]
        baseline: String,
    },
    /// Rewrite the baseline to the current finding set.
    Freeze {
        /// The ratchet baseline file, project-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_CONFORM_BASELINE)]
        baseline: String,
    },
}

#[derive(Subcommand)]
enum TraceCmd {
    /// Render the traceability subgraph around one target (a spec:// URI
    /// or a code symbol).
    Explain {
        target: String,
        /// Emit the raw subgraph as JSON (for agents).
        #[arg(long)]
        json: bool,
        /// Deterministic prose render through the intent ledger.
        #[arg(long, conflicts_with = "json")]
        prose: bool,
    },
}

#[derive(Subcommand)]
enum CodemodCmd {
    /// Add a new cell to a crate: module + `#[cell]` manifest + REQ edge +
    /// smoke test + lib.rs registration, post-checked and rolled back on
    /// failure.
    AddCell {
        /// The crate directory, project-relative (e.g. `crates/app`).
        #[arg(long)]
        crate_dir: String,
        /// The new cell's snake_case module name.
        #[arg(long)]
        cell: String,
        /// The seam the cell implements.
        #[arg(long)]
        seam: String,
        /// The cell's variant name.
        #[arg(long)]
        variant: String,
        /// The spec:// unit the cell implements.
        #[arg(long)]
        spec_uri: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.path;
    match cli.command {
        Command::Init { namespace, force } => discipline_cli_rust::run_init(
            &root,
            &discipline_cli_rust::InitOptions { namespace, force },
        ),
        Command::Floor {
            keep_going,
            quiet,
            fast_loop,
        } => discipline_cli_rust::run_floor(
            &root,
            &discipline_cli_rust::FloorOptions {
                keep_going,
                quiet,
                fast_loop,
            },
        ),
        Command::Conform { cmd } => match cmd {
            ConformCmd::Check { scope, baseline } => {
                conform_cli_rust::run_check(&root, &baseline, scope.as_deref())
            }
            ConformCmd::Freeze { baseline } => conform_cli_rust::run_freeze(&root, &baseline),
        },
        Command::Specmap { check, gate } => {
            if gate {
                specmap_cli_rust::run_gate(&root)
            } else {
                specmap_cli_rust::run_specmap(&root, check)
            }
        }
        Command::Trace { cmd } => match cmd {
            TraceCmd::Explain {
                target,
                json,
                prose,
            } => discipline_cli_rust::run_trace_explain(&root, &target, json, prose),
        },
        Command::TestGate { baseline } => discipline_cli_rust::run_test_gate(&root, &baseline),
        Command::Tripwire { base, debt } => {
            discipline_cli_rust::run_tripwire(&root, base.as_deref(), &debt)
        }
        Command::Health { out } => discipline_cli_rust::run_health(&root, &out, &[]),
        Command::FastLoop {
            cell,
            budget,
            enforce_budget,
        } => discipline_cli_rust::run_fast_loop(&root, cell.as_deref(), budget, enforce_budget),
        Command::Codemod { cmd } => match cmd {
            CodemodCmd::AddCell {
                crate_dir,
                cell,
                seam,
                variant,
                spec_uri,
            } => discipline_cli_rust::run_codemod_add_cell(
                &root, &crate_dir, &cell, &seam, &variant, &spec_uri,
            ),
        },
        Command::Ledger { cmd } => match cmd {
            LedgerCmd::Render {
                check,
                debt,
                intent,
            } => discipline_cli_rust::run_ledger_render(&root, &debt, &intent, check),
        },
    }
}
