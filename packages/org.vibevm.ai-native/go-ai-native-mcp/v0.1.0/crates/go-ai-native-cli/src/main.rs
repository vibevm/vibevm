//! `go-ai-native` — the umbrella discipline tool for Go trees: full
//! subcommand parity with `rust-ai-native` / `typescript-ai-native`.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "go-ai-native",
    about = "AI-Native discipline toolchain for Go: init, the seven-step floor, \
             conform, specmap, trace, test-gate, tripwire, health, fast-loop, \
             codemod"
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
    /// Bootstrap the six discipline artifacts in their Go shape.
    Init {
        /// The spec:// namespace (defaults to the directory name).
        #[arg(long)]
        namespace: Option<String>,
        /// Overwrite existing artifacts (topology re-derivation).
        #[arg(long)]
        force: bool,
    },
    /// The seven-step verification floor: gofmt → vet → tests →
    /// staticcheck+exhaustive → conform → specmap → test-gate.
    Floor {
        /// Run every step even after a failure.
        #[arg(long)]
        keep_going: bool,
        /// Suppress the per-step headers.
        #[arg(long)]
        quiet: bool,
    },
    /// The go-extract structural gate (go-ai-native-conform check/freeze).
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
    /// Run the tests under `go test -json` and diff xfail-strict
    /// against the tests baseline (BROWNFIELD §4).
    TestGate {
        /// Path to the baseline registry, root-relative.
        #[arg(long, default_value = go_ai_native_cli::DEFAULT_TESTS_BASELINE)]
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
        #[arg(long, default_value = go_ai_native_cli::DEFAULT_DEBT_REGISTRY)]
        debt: String,
    },
    /// The Discipline health collector over go-extract facts: the
    /// file-length danger band, the ban census, export Example
    /// coverage, and the orphan backlog. Never fails the build.
    Health {
        /// Where to write the JSON snapshot, root-relative.
        #[arg(long, default_value = go_ai_native_cli::DEFAULT_HEALTH_OUT)]
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
    /// Extract facts through the stdlib-only go-extract sidecar, run
    /// the Go rules, and gate against the ratchet baseline.
    Check {
        #[arg(long, default_value = go_ai_native_conform::DEFAULT_GO_BASELINE)]
        baseline: String,
        /// Report findings only under this root-relative path prefix.
        #[arg(long)]
        scope: Option<String>,
    },
    /// Rewrite the baseline to the current finding set.
    Freeze {
        #[arg(long, default_value = go_ai_native_conform::DEFAULT_GO_BASELINE)]
        baseline: String,
    },
}

#[derive(Subcommand, Debug)]
enum TraceCmd {
    /// Render the traceability subgraph around a declaration or a
    /// `spec://` unit URI.
    Explain {
        /// A `<file>::<decl>` symbol (exact or unique suffix), or a
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

#[derive(Subcommand, Debug)]
enum CodemodCmd {
    /// Add a new cell package: doc.go with its `//spec:scope` marker,
    /// the cell source with a New constructor, and a smoke test with
    /// an executed Example — post-checked by running the new cell's
    /// tests and rolled back on failure.
    AddCell {
        /// Cell package name, lowercase letters/digits.
        #[arg(long)]
        cell: String,
        /// The spec:// unit the cell implements — required; a cell
        /// without a REQ edge is an orphan the ratchet rejects.
        #[arg(long)]
        spec_uri: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = std::path::PathBuf::from(&cli.path);
    match cli.command {
        Cmd::Init { namespace, force } => go_ai_native_cli::run_init(
            &root,
            &go_ai_native_cli::InitOptions { namespace, force },
        ),
        Cmd::Floor { keep_going, quiet } => go_ai_native_cli::run_floor(
            &root,
            &go_ai_native_cli::FloorOptions { keep_going, quiet },
        ),
        Cmd::Conform {
            cmd: ConformCmd::Check { baseline, scope },
        } => go_ai_native_conform::run_check(&root, &baseline, scope.as_deref()),
        Cmd::Conform {
            cmd: ConformCmd::Freeze { baseline },
        } => go_ai_native_conform::run_freeze(&root, &baseline),
        Cmd::Specmap { check, gate } => {
            if gate {
                go_ai_native_specmap::run_gate(&root)
            } else {
                go_ai_native_specmap::run_specmap_go(&root, check)
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
        } => go_ai_native_cli::run_trace_explain(&root, &target, json, prose),
        Cmd::TestGate { baseline } => go_ai_native_cli::run_test_gate(&root, &baseline),
        Cmd::Tripwire { base, debt } => {
            go_ai_native_cli::run_tripwire(&root, base.as_deref(), &debt)
        }
        Cmd::Health { out } => go_ai_native_cli::run_health(&root, &out),
        Cmd::FastLoop {
            cell,
            budget,
            enforce_budget,
        } => go_ai_native_cli::run_fast_loop(&root, cell.as_deref(), budget, enforce_budget),
        Cmd::Codemod {
            cmd: CodemodCmd::AddCell { cell, spec_uri },
        } => go_ai_native_cli::run_codemod_add_cell(&root, &cell, &spec_uri),
    }
}
