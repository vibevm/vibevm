//! `cargo xtask` — project-tooling entry point.
//!
//! Subcommands:
//!
//! - `codegen` — regenerate the Rust types under each owning crate's
//!   `src/generated/` from the JTD schemas under `schemas/` (most in
//!   `vibe-wire`; `specmap` in `specmap-core`). Calls the locally-vendored
//!   `jtd-codegen` binary (see `tools/jtd-codegen/README.md`); errors
//!   actionably when the binary is missing.
//! - `check-codegen` — `codegen`, then run `git diff --exit-code` over
//!   the generated dirs. Used by CI to assert no schema drift.
//! - `specmap` — regenerate the canonical `specmap.json` traceability
//!   index (PROP-014 §2.5); `--check` regenerates and byte-diffs, the
//!   `check-codegen` idiom.
//! - `test-gate` / `tripwire` / `trace` / `health` / `fast-loop` /
//!   `codemod` — thin shims over the packaged `discipline-cli-rust` library
//!   (stack:org.vibevm/rust-ai-native, PROP-024): the drivers ship with
//!   the discipline; xtask keeps vibevm's flag-compatible surface and
//!   composes the vibevm-only extras (the health `--mirrors` probe).
//!
//! Entry shape follows the standard `xtask` pattern: this file holds
//! the clap surface and the dispatch; the heavy lifting lives in the
//! packaged engine crates.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod codegen;
mod conform;
mod mirror;
mod specmap;
mod sync_engines;

use codegen::{run_check_codegen, run_codegen};
use conform::{run_conform_check, run_conform_freeze};
use mirror::run_mirror;
use specmap::run_specmap;
use sync_engines::run_sync_engines;

#[derive(Parser, Debug)]
#[command(
    name = "xtask",
    about = "vibevm project tooling — codegen, drift checks, build helpers"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Regenerate Rust types under each owning crate's `src/generated/`
    /// from JTD schemas under `schemas/`.
    Codegen,

    /// Run `codegen`, then assert via `git diff --exit-code` that the
    /// generated tree matches what's checked in. CI runs this to catch
    /// schema drift.
    CheckCodegen,

    /// Regenerate the canonical `specmap.json` traceability index
    /// (PROP-014 §2.5).
    Specmap {
        /// Regenerate and byte-diff against the committed index instead
        /// of writing; non-zero exit on drift.
        #[arg(long)]
        check: bool,
    },

    /// Run workspace tests via nextest and diff against the xfail-strict
    /// baseline (BROWNFIELD §4). Fails on newly-failing and on
    /// unexpectedly-passing-unpromoted.
    TestGate {
        /// Path to the baseline registry, repo-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_TESTS_BASELINE)]
        baseline: String,
    },

    /// List debt entries whose `touch:` tripwires fire on the current
    /// change set (worktree + staged + untracked; or `--base <rev>`).
    /// Warn-only: always exits 0.
    Tripwire {
        /// Diff against this revision (`<base>...HEAD`) instead of the
        /// working-tree change set.
        #[arg(long)]
        base: Option<String>,

        /// Path to the debt registry, repo-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_DEBT_REGISTRY)]
        debt: String,
    },

    /// Traceability queries over the specmap (PROP-014 §2.6). Pilot
    /// home: promotion to `vibe trace` is a Phase 4 decision.
    Trace {
        #[command(subcommand)]
        cmd: TraceCmd,
    },

    /// The conformance engine gate (ENGINE-CONFORM §5; PLAYBOOK
    /// Phase 4). Replaces the Phase 3 `conform-lite` interim lints.
    Conform {
        #[command(subcommand)]
        cmd: ConformCmd,
    },

    /// Scaffolded edit operations (discipline card scaffold-i-codemods,
    /// [E-hyp] pilot prototype): a recurring multi-file change offered
    /// as ONE parameterized, checked, atomic operation. The weakest
    /// agent tier should call these with the documented fixed
    /// parameter shapes only (free parameterization is the open R4
    /// question the pilot measures).
    Codemod {
        #[command(subcommand)]
        cmd: CodemodCmd,
    },

    /// The Class-E fast-loop checker, `cell-fast-loop-present`
    /// (discipline card scaffold-e-fast-loop, Band 3): every cell —
    /// a workspace crate today — builds and tests in isolation
    /// inside the per-cell budget. Test failures always fail the
    /// command; budget overruns warn unless `--enforce-budget`.
    FastLoop {
        /// Check a single cell (workspace member name) instead of all.
        #[arg(long)]
        cell: Option<String>,

        /// Per-cell first-signal budget, seconds (card default: 60).
        #[arg(long, default_value_t = 60)]
        budget: u64,

        /// Fail (non-zero exit) on budget overruns, not only on
        /// red tests. Off during Phase-1 remediation; the gate mode.
        #[arg(long)]
        enforce_budget: bool,
    },

    /// The Discipline health collector (the Sweep Playbook's
    /// fact-gatherer): advisory early-warning + coverage facts that sit
    /// above the binary conform/specmap gates — per-crate public-type
    /// doctest coverage, the file-length danger band, the pub-doctest
    /// drain/promotion backlog, and the deviation-debt census. Reuses the
    /// conform fact frontend so the numbers never drift from the gates.
    /// Deterministic given the tree; never fails the build (the gates do).
    Health {
        /// Where to write the JSON snapshot, repo-relative.
        #[arg(long, default_value = discipline_cli_rust::DEFAULT_HEALTH_OUT)]
        out: String,

        /// Also probe whether every `mirrors.toml` target is in sync with
        /// local mainline. A network call — OFF by default, so the default
        /// run stays deterministic and offline.
        #[arg(long)]
        mirrors: bool,
    },

    /// Mirror the authored neutral engine crates
    /// (flow:org.vibevm/discipline-core) into every stack's
    /// `crates/vendor/` per `sync-engines.toml` (DEFERRALS-CLOSEOUT D1).
    /// `--check` byte-compares instead of writing and exits non-zero on
    /// drift; self-check runs it, so a vendored copy can never diverge
    /// from the authored engine silently.
    SyncEngines {
        /// Compare instead of mirroring; non-zero exit on any drift.
        #[arg(long)]
        check: bool,
    },

    /// Fan the local mainline out to every target in `mirrors.toml`
    /// (the benevolent-dictator / hub-and-spoke mirror model, no primary):
    /// push `main` + tags to every `push` target, fast-forward-only and
    /// never `--force`. `--check` verifies sync without pushing; `--from
    /// <name>` fast-forwards local mainline to a host's accepted-PR merge
    /// before fanning out. Auth is the maintainer's per-host SSH keys.
    Mirror {
        /// Verify every target is in sync with local mainline; push nothing.
        #[arg(long)]
        check: bool,

        /// Before fanning out, fast-forward local `main` to this target's
        /// `main` (a PR accepted/merged via that host's web UI).
        #[arg(long)]
        from: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum CodemodCmd {
    /// Add a new cell to a crate: the module file with its `#[cell]`
    /// manifest + REQ edge, the `pub mod` registration in lib.rs
    /// (alphabetical), and a smoke test referencing the cell (so
    /// `cell-has-oracle` is satisfied from birth). All-or-nothing:
    /// files are written together and rolled back if the post-check
    /// (`cargo check -p <crate>`) fails.
    ///
    /// Fixed-parameter example (the weakest-tier invocation shape):
    ///   cargo xtask codemod add-cell --crate-dir crates/vibe-resolver \
    ///     --cell sat --seam DepSolver --variant sat \
    ///     --spec-uri "spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade"
    AddCell {
        /// Crate directory, repo-relative (e.g. crates/vibe-resolver).
        #[arg(long)]
        crate_dir: String,

        /// Cell module name, snake_case (e.g. `sat`).
        #[arg(long)]
        cell: String,

        /// The seam trait the cell will implement (manifest metadata).
        #[arg(long)]
        seam: String,

        /// The cell's variant label in the `#[cell]` manifest.
        #[arg(long)]
        variant: String,

        /// The spec:// unit the cell implements — required, because a
        /// cell without a REQ edge is an orphan the ratchet rejects
        /// (A1 by construction, not by follow-up).
        #[arg(long)]
        spec_uri: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConformCmd {
    /// Extract facts (incremental, content-addressed), run the rules,
    /// emit SARIF, and gate new findings against the ratchet baseline.
    Check {
        /// Path to the frozen-findings baseline, repo-relative.
        #[arg(long, default_value = "conform-baseline.json")]
        baseline: String,

        /// Report findings only under this repo-relative path prefix
        /// (facts are still extracted workspace-wide — B5).
        #[arg(long)]
        scope: Option<String>,
    },
    /// Rewrite the baseline to the current finding set (sorted
    /// fingerprints). Legal uses: freezing a NEW rule's pre-existing
    /// findings when it first lands, and re-freezing after work that
    /// shrank the set — verify with `git diff` that the file only
    /// shrinks outside a new-rule landing.
    Freeze {
        /// Path to the baseline to rewrite, repo-relative.
        #[arg(long, default_value = "conform-baseline.json")]
        baseline: String,
    },
}

#[derive(Subcommand, Debug)]
enum TraceCmd {
    /// Render the traceability subgraph around a code symbol or a
    /// `spec://` URI.
    Explain {
        /// A module-qualified symbol (exact or unique suffix), or a
        /// `spec://` unit URI.
        target: String,

        /// Deterministic structured text rendering (the default).
        #[arg(long)]
        text: bool,

        /// Raw subgraph as JSON (agent-friendly).
        #[arg(long, conflicts_with = "text")]
        json: bool,

        /// Prose render through the local ledger (LEDGER §6 query
        /// kind 2): template producer, epoch-keyed cache under
        /// `.ledger/`, provenance line on every render.
        #[arg(long, conflicts_with_all = ["text", "json"])]
        prose: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Codegen => run_codegen(),
        Cmd::CheckCodegen => run_check_codegen(),
        Cmd::Specmap { check } => run_specmap(check),
        Cmd::SyncEngines { check } => run_sync_engines(check),
        Cmd::TestGate { baseline } => discipline_cli_rust::run_test_gate(&repo_root()?, &baseline),
        Cmd::Tripwire { base, debt } => {
            discipline_cli_rust::run_tripwire(&repo_root()?, base.as_deref(), &debt)
        }
        Cmd::Conform {
            cmd: ConformCmd::Check { baseline, scope },
        } => run_conform_check(&baseline, scope.as_deref()),
        Cmd::Conform {
            cmd: ConformCmd::Freeze { baseline },
        } => run_conform_freeze(&baseline),
        Cmd::Trace {
            cmd:
                TraceCmd::Explain {
                    target,
                    json,
                    prose,
                    ..
                },
        } => discipline_cli_rust::run_trace_explain(&repo_root()?, &target, json, prose),
        Cmd::FastLoop {
            cell,
            budget,
            enforce_budget,
        } => discipline_cli_rust::run_fast_loop(
            &repo_root()?,
            cell.as_deref(),
            budget,
            enforce_budget,
        ),
        Cmd::Codemod {
            cmd:
                CodemodCmd::AddCell {
                    crate_dir,
                    cell,
                    seam,
                    variant,
                    spec_uri,
                },
        } => discipline_cli_rust::run_codemod_add_cell(
            &repo_root()?,
            &crate_dir,
            &cell,
            &seam,
            &variant,
            &spec_uri,
        ),
        Cmd::Health { out, mirrors } => {
            let root = repo_root()?;
            // The packaged collector is offline by contract; the vibevm-only
            // mirror-sync probe rides in as an extra section (PROP-016).
            let extra = if mirrors {
                vec![("mirrors".to_string(), mirror_health_section(&root)?)]
            } else {
                Vec::new()
            };
            discipline_cli_rust::run_health(&root, &out, &extra)
        }
        Cmd::Mirror { check, from } => run_mirror(check, from.as_deref()),
    }
}

/// The `--mirrors` extra section for `xtask health`: probe every
/// `mirrors.toml` target against local mainline and render both the human
/// lines and the JSON block the packaged collector appends verbatim.
fn mirror_health_section(root: &std::path::Path) -> Result<serde_json::Value> {
    use mirror::SyncState;
    let (head, statuses) = mirror::sync_report(root)?;
    println!(
        "mirror sync vs local main @ {}:",
        &head[..7.min(head.len())]
    );
    let mut target_rows = Vec::new();
    for s in &statuses {
        let (label, state, remote) = match &s.state {
            SyncState::InSync => ("sync   ", "in_sync", serde_json::Value::Null),
            SyncState::Drift(sha) => ("DRIFT  ", "drift", serde_json::json!(sha)),
            SyncState::Missing => ("MISSING", "missing", serde_json::Value::Null),
        };
        println!("  {label}{}", s.name);
        target_rows
            .push(serde_json::json!({ "name": s.name, "state": state, "remote_main": remote }));
    }
    Ok(serde_json::json!({ "local_main": head, "targets": target_rows }))
}

fn repo_root() -> Result<PathBuf> {
    // `cargo xtask` runs the binary from the workspace root by
    // default, but be defensive: walk up from CARGO_MANIFEST_DIR
    // (which is `<root>/xtask`) to find the workspace root.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .context("CARGO_MANIFEST_DIR not set; is xtask running under cargo?")?;
    let manifest_dir = PathBuf::from(manifest_dir);
    let parent = manifest_dir
        .parent()
        .context("xtask manifest dir has no parent")?;
    Ok(parent.to_path_buf())
}
