//! `typescript-ai-native` — the umbrella discipline tool for
//! TypeScript trees: full subcommand parity with `discipline-rust`.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "typescript-ai-native",
    about = "AI-Native discipline toolchain for TypeScript: init, the seven-step \
             floor, conform, specmap, trace, test-gate, tripwire, health, \
             fast-loop, codemod"
)]
struct Cli {
    /// Project root.
    #[arg(long, global = true, default_value = ".")]
    path: String,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Bootstrap the six discipline artifacts in their TS shape.
    Init {
        /// The spec:// namespace (defaults to the directory name).
        #[arg(long)]
        namespace: Option<String>,
        /// Overwrite existing artifacts (topology re-derivation).
        #[arg(long)]
        force: bool,
    },
    /// The seven-step verification floor: prettier → tsc → tests →
    /// eslint → conform → specmap → test-gate.
    Floor {
        /// Run every step even after a failure.
        #[arg(long)]
        keep_going: bool,
        /// Suppress the per-step headers.
        #[arg(long)]
        quiet: bool,
    },
    /// The ts-tsc structural gate (typescript-ai-native-conform check/freeze).
    Conform {
        #[command(subcommand)]
        cmd: ConformCmd,
    },
    /// Build (or `--check`) the traceability index + orphan ratchet.
    Specmap {
        /// Byte-compare against the committed index instead of writing.
        #[arg(long)]
        check: bool,
        /// Orphan-coverage gate only; no index read or written.
        #[arg(long)]
        gate: bool,
    },
    /// Traceability queries over the specmap (PROP-014 §2.6).
    Trace {
        #[command(subcommand)]
        cmd: TraceCmd,
    },
    /// Run the tests under node's TAP reporter and diff xfail-strict
    /// against the tests baseline (BROWNFIELD §4).
    TestGate {
        /// Path to the baseline registry, root-relative.
        #[arg(long, default_value = typescript_ai_native_cli::DEFAULT_TESTS_BASELINE)]
        baseline: String,
    },
    /// List debt entries whose `touch:` tripwires fire on the current
    /// change set. Warn-only: always exits 0.
    Tripwire {
        /// Diff against this revision (`<base>...HEAD`) instead of the
        /// working-tree change set.
        #[arg(long)]
        base: Option<String>,
        /// Path to the debt registry, root-relative.
        #[arg(long, default_value = typescript_ai_native_cli::DEFAULT_DEBT_REGISTRY)]
        debt: String,
    },
    /// The Discipline health collector over ts-tsc facts: the
    /// file-length danger band, the unsafe census, export doc-example
    /// coverage, and the orphan backlog. Never fails the build.
    Health {
        /// Where to write the JSON snapshot, root-relative.
        #[arg(long, default_value = typescript_ai_native_cli::DEFAULT_HEALTH_OUT)]
        out: String,
    },
    /// The Class-E fast-loop checker: every cell's tests run in
    /// isolation inside the per-cell budget.
    FastLoop {
        /// Check a single cell (directory name) instead of all.
        #[arg(long)]
        cell: Option<String>,
        /// Per-cell first-signal budget, seconds (card default: 60).
        #[arg(long, default_value_t = 60)]
        budget: u64,
        /// Fail (non-zero) on budget overruns, not only on red tests.
        #[arg(long)]
        enforce_budget: bool,
    },
    /// Scaffolded edit operations (card scaffold-i-codemods).
    Codemod {
        #[command(subcommand)]
        cmd: CodemodCmd,
    },
}

#[derive(Subcommand, Debug)]
enum ConformCmd {
    /// Extract facts through the project's own typescript install, run
    /// the TS rules, and gate against the ratchet baseline.
    Check {
        #[arg(long, default_value = typescript_ai_native_conform::DEFAULT_TS_BASELINE)]
        baseline: String,
        /// Report findings only under this root-relative path prefix.
        #[arg(long)]
        scope: Option<String>,
    },
    /// Rewrite the baseline to the current finding set.
    Freeze {
        #[arg(long, default_value = typescript_ai_native_conform::DEFAULT_TS_BASELINE)]
        baseline: String,
    },
}

#[derive(Subcommand, Debug)]
enum TraceCmd {
    /// Render the traceability subgraph around an export or a
    /// `spec://` unit URI.
    Explain {
        /// A `<file>::<export>` symbol (exact or unique suffix), or a
        /// `spec://` unit URI.
        target: String,
        /// Deterministic structured text rendering (the default).
        #[arg(long)]
        text: bool,
        /// Raw subgraph as JSON (agent-friendly).
        #[arg(long, conflicts_with = "text")]
        json: bool,
        /// Prose render through the local ledger.
        #[arg(long, conflicts_with_all = ["text", "json"])]
        prose: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = std::path::PathBuf::from(&cli.path);
    match cli.command {
        Cmd::Init { namespace, force } => typescript_ai_native_cli::run_init(
            &root,
            &typescript_ai_native_cli::InitOptions { namespace, force },
        ),
        Cmd::Floor { keep_going, quiet } => typescript_ai_native_cli::run_floor(
            &root,
            &typescript_ai_native_cli::FloorOptions { keep_going, quiet },
        ),
        Cmd::Conform {
            cmd: ConformCmd::Check { baseline, scope },
        } => typescript_ai_native_conform::run_check(&root, &baseline, scope.as_deref()),
        Cmd::Conform {
            cmd: ConformCmd::Freeze { baseline },
        } => typescript_ai_native_conform::run_freeze(&root, &baseline),
        Cmd::Specmap { check, gate } => {
            if gate {
                typescript_ai_native_specmap::run_gate(&root)
            } else {
                typescript_ai_native_specmap::run_specmap_typescript(&root, check)
            }
        }
        Cmd::Trace {
            cmd:
                TraceCmd::Explain {
                    target,
                    json,
                    prose,
                    ..
                },
        } => typescript_ai_native_cli::run_trace_explain(&root, &target, json, prose),
        Cmd::TestGate { baseline } => typescript_ai_native_cli::run_test_gate(&root, &baseline),
        Cmd::Tripwire { base, debt } => {
            typescript_ai_native_cli::run_tripwire(&root, base.as_deref(), &debt)
        }
        Cmd::Health { out } => typescript_ai_native_cli::run_health(&root, &out),
        Cmd::FastLoop {
            cell,
            budget,
            enforce_budget,
        } => {
            typescript_ai_native_cli::run_fast_loop(&root, cell.as_deref(), budget, enforce_budget)
        }
        Cmd::Codemod {
            cmd: CodemodCmd::AddCell { cell, spec_uri },
        } => typescript_ai_native_cli::run_codemod_add_cell(&root, &cell, &spec_uri),
    }
}

#[derive(Subcommand, Debug)]
enum CodemodCmd {
    /// Add a new cell: the seam module (`index.ts` with a file-level
    /// `@scope` marker) + a node:test smoke test, post-checked by
    /// running the new cell's tests and rolled back on failure.
    AddCell {
        /// Cell directory name, lowercase kebab/snake.
        #[arg(long)]
        cell: String,
        /// The spec:// unit the cell implements — required; a cell
        /// without a REQ edge is an orphan the ratchet rejects.
        #[arg(long)]
        spec_uri: String,
    },
}
