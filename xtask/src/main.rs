//! `cargo xtask` — project-tooling entry point.
//!
//! Subcommands:
//!
//! - `codegen` — regenerate every Rust file under
//!   `crates/vibe-wire/src/generated/` from the JTD schemas under
//!   `schemas/`. Calls the locally-vendored `jtd-codegen` binary
//!   (see `tools/jtd-codegen/README.md`); errors actionably when
//!   the binary is missing.
//! - `check-codegen` — `codegen`, then run `git diff --exit-code` over
//!   the generated dir. Used by CI to assert no schema drift.
//! - `specmap` — regenerate the canonical `specmap.json` traceability
//!   index (PROP-014 §2.5); `--check` regenerates and byte-diffs, the
//!   `check-codegen` idiom.
//! - `test-gate` — run the workspace tests through nextest and diff the
//!   outcome against `terraform/registry/tests-baseline.json` with
//!   xfail-strict semantics (BROWNFIELD §4). Replaces bare `cargo test`
//!   in terraform acceptance lines.
//! - `tripwire` — list debt-registry entries whose `touch:` tripwires
//!   fire on the current change set. Warn-only.
//! - `fast-loop` — the Class-E `cell-fast-loop-present` checker
//!   (discipline card scaffold-e-fast-loop): every cell builds and
//!   tests in isolation inside the per-cell budget.
//!
//! Entry shape follows the standard `xtask` pattern. Keep this
//! crate dep-light: clap + anyhow + std; the heavy lifting lives in
//! `specmap-core`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

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
    /// Regenerate Rust types under `crates/vibe-wire/src/generated/`
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
        #[arg(long, default_value = "terraform/registry/tests-baseline.json")]
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
        #[arg(long, default_value = "terraform/registry/debt.json")]
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
        Cmd::TestGate { baseline } => run_test_gate(&baseline),
        Cmd::Tripwire { base, debt } => run_tripwire(base.as_deref(), &debt),
        Cmd::Conform {
            cmd: ConformCmd::Check { baseline, scope },
        } => run_conform_check(&baseline, scope.as_deref()),
        Cmd::Trace {
            cmd:
                TraceCmd::Explain {
                    target,
                    json,
                    prose,
                    ..
                },
        } => run_trace_explain(&target, json, prose),
        Cmd::FastLoop {
            cell,
            budget,
            enforce_budget,
        } => run_fast_loop(cell.as_deref(), budget, enforce_budget),
        Cmd::Codemod {
            cmd:
                CodemodCmd::AddCell {
                    crate_dir,
                    cell,
                    seam,
                    variant,
                    spec_uri,
                },
        } => run_codemod_add_cell(&crate_dir, &cell, &seam, &variant, &spec_uri),
    }
}

/// Pure content generation for `codemod add-cell` — split out so the
/// templates are unit-testable without touching a filesystem.
mod add_cell {
    /// `sat_solver` → `SatSolver`.
    pub fn pascal(cell: &str) -> String {
        cell.split('_')
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect()
    }

    pub fn module_source(cell: &str, seam: &str, variant: &str, spec_uri: &str) -> String {
        let ty = pascal(cell);
        format!(
            "//! `{ty}` — the `{variant}` {seam} cell.\n\
             //!\n\
             //! Scaffolded by `cargo xtask codemod add-cell`; the seam\n\
             //! implementation is the author's next edit. The `#[cell]`\n\
             //! manifest and the REQ edge are present from birth so the\n\
             //! selection registry and the specmap see the cell\n\
             //! immediately.\n\
             \n\
             use specmark::{{cell, spec}};\n\
             \n\
             #[cell(seam = \"{seam}\", variant = \"{variant}\")]\n\
             #[spec(implements = \"{spec_uri}\")]\n\
             pub struct {ty};\n"
        )
    }

    pub fn smoke_test_source(crate_ident: &str, cell: &str) -> String {
        let ty = pascal(cell);
        format!(
            "//! Smoke reference for the `{cell}` cell — the seed the\n\
             //! `cell-has-oracle` rule requires; replace with the real\n\
             //! differential/characterization oracle as the cell grows\n\
             //! behavior (card scaffold-d).\n\
             \n\
             use {crate_ident}::{cell}::{ty};\n\
             \n\
             #[test]\n\
             fn {cell}_cell_constructs() {{\n    let _cell = {ty};\n}}\n"
        )
    }

    /// Insert `pub mod <cell>;` into lib.rs at the alphabetical
    /// position inside the first contiguous `pub mod` block. Returns
    /// `None` when no such block exists (the codemod then refuses
    /// rather than guessing).
    pub fn insert_pub_mod(lib_source: &str, cell: &str) -> Option<String> {
        let lines: Vec<&str> = lib_source.lines().collect();
        let first = lines
            .iter()
            .position(|l| l.trim().starts_with("pub mod "))?;
        let mut end = first;
        while end < lines.len() && lines[end].trim().starts_with("pub mod ") {
            end += 1;
        }
        let decl = format!("pub mod {cell};");
        if lines[first..end].iter().any(|l| l.trim() == decl) {
            return None; // already registered — refuse, do not duplicate
        }
        let mut insert_at = end;
        for (i, line) in lines.iter().enumerate().take(end).skip(first) {
            if line.trim() > decl.as_str() {
                insert_at = i;
                break;
            }
        }
        let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        out.insert(insert_at, decl);
        let mut joined = out.join("\n");
        if lib_source.ends_with('\n') {
            joined.push('\n');
        }
        Some(joined)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn pascal_cases_snake_names() {
            assert_eq!(pascal("sat"), "Sat");
            assert_eq!(pascal("sat_solver"), "SatSolver");
        }

        #[test]
        fn module_source_carries_cell_and_spec() {
            let src = module_source("sat", "DepSolver", "sat", "spec://p/d#a");
            assert!(src.contains("#[cell(seam = \"DepSolver\", variant = \"sat\")]"));
            assert!(src.contains("#[spec(implements = \"spec://p/d#a\")]"));
            assert!(src.contains("pub struct Sat;"));
        }

        #[test]
        fn insert_is_alphabetical_and_idempotent_refusing() {
            let lib = "pub mod alpha;\npub mod gamma;\n\npub use alpha::A;\n";
            let out = insert_pub_mod(lib, "beta").unwrap();
            assert_eq!(
                out,
                "pub mod alpha;\npub mod beta;\npub mod gamma;\n\npub use alpha::A;\n"
            );
            assert!(
                insert_pub_mod(&out, "beta").is_none(),
                "duplicate must refuse"
            );
        }

        #[test]
        fn insert_appends_after_block_when_last() {
            let lib = "pub mod alpha;\npub mod beta;\nrest";
            let out = insert_pub_mod(lib, "zeta").unwrap();
            assert!(out.starts_with("pub mod alpha;\npub mod beta;\npub mod zeta;"));
        }
    }
}

/// `cargo xtask codemod add-cell` — the card-I prototype: one checked,
/// atomic, multi-file operation. Writes the module, registers it in
/// lib.rs, seeds the smoke test, then runs `cargo check -p <crate>`;
/// any failure rolls every write back.
fn run_codemod_add_cell(
    crate_dir_rel: &str,
    cell: &str,
    seam: &str,
    variant: &str,
    spec_uri: &str,
) -> Result<()> {
    let root = repo_root()?;
    let crate_dir = root.join(crate_dir_rel);
    let crate_name = crate_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .context("crate dir has no terminal component")?;
    let crate_ident = crate_name.replace('-', "_");

    if !cell
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        || cell.is_empty()
    {
        bail!("--cell must be snake_case ASCII, got `{cell}`");
    }
    if !spec_uri.starts_with("spec://") {
        bail!("--spec-uri must be a spec:// unit URI, got `{spec_uri}`");
    }

    let lib_path = crate_dir.join("src").join("lib.rs");
    let module_path = crate_dir.join("src").join(format!("{cell}.rs"));
    let test_path = crate_dir.join("tests").join(format!("{cell}_smoke.rs"));

    if module_path.exists() {
        bail!("`{}` already exists — refusing", module_path.display());
    }
    if test_path.exists() {
        bail!("`{}` already exists — refusing", test_path.display());
    }
    let lib_before = std::fs::read_to_string(&lib_path)
        .with_context(|| format!("reading {}", lib_path.display()))?;
    let lib_after = add_cell::insert_pub_mod(&lib_before, cell).with_context(|| {
        format!(
            "no `pub mod` block found in {} (or `{cell}` already registered)",
            lib_path.display()
        )
    })?;

    // All content is computed; now write atomically-by-rollback.
    std::fs::write(
        &module_path,
        add_cell::module_source(cell, seam, variant, spec_uri),
    )?;
    if let Some(parent) = test_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&test_path, add_cell::smoke_test_source(&crate_ident, cell))?;
    std::fs::write(&lib_path, &lib_after)?;

    eprintln!(
        "xtask codemod add-cell: wrote {}, {}, registered in lib.rs — post-check…",
        module_path
            .strip_prefix(&root)
            .unwrap_or(&module_path)
            .display(),
        test_path
            .strip_prefix(&root)
            .unwrap_or(&test_path)
            .display(),
    );
    let check = Command::new("cargo")
        .args(["check", "-p", &crate_name, "--all-targets"])
        .current_dir(&root)
        .status()
        .context("spawning cargo check")?;
    if !check.success() {
        // Roll back: the operation is all-or-nothing.
        let _ = std::fs::remove_file(&module_path);
        let _ = std::fs::remove_file(&test_path);
        std::fs::write(&lib_path, &lib_before)?;
        bail!("post-check failed — all three writes rolled back; the tree is as before");
    }
    eprintln!(
        "xtask codemod add-cell: ok. Next edits: implement the seam on `{}`, replace the \
         smoke test with the real oracle (card scaffold-d), and run \
         `cargo xtask fast-loop --cell {crate_name}`.",
        add_cell::pascal(cell)
    );
    Ok(())
}

/// One cell's fast-loop measurement.
struct CellRun {
    cell: String,
    seconds: f64,
    tests: usize,
    passed: bool,
}

/// Workspace member names via `cargo metadata --no-deps`, sorted.
fn workspace_members(root: &Path) -> Result<Vec<String>> {
    let out = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .context("spawning cargo metadata")?;
    if !out.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let meta: serde_json::Value =
        serde_json::from_slice(&out.stdout).context("parsing cargo metadata JSON")?;
    let mut names: Vec<String> = meta["packages"]
        .as_array()
        .context("cargo metadata: no packages array")?
        .iter()
        .filter_map(|p| p["name"].as_str().map(str::to_string))
        .collect();
    names.sort();
    Ok(names)
}

/// The fast-loop checker: per cell, run `cargo nextest run -p <cell>`
/// in isolation and measure wall-clock to the verdict. The verdict
/// (pass/fail + test count) comes from the same nextest output the
/// test-gate parses, so the two gates cannot disagree on what a test
/// result is.
fn run_fast_loop(cell: Option<&str>, budget_secs: u64, enforce_budget: bool) -> Result<()> {
    use specmap_core::testgate;

    let root = repo_root()?;
    let cells = match cell {
        Some(one) => vec![one.to_string()],
        None => workspace_members(&root)?,
    };
    let budget = budget_secs as f64;

    let mut runs: Vec<CellRun> = Vec::new();
    for name in &cells {
        let started = std::time::Instant::now();
        let out = Command::new("cargo")
            .args([
                "nextest",
                "run",
                "-p",
                name,
                "--no-fail-fast",
                // A cell with zero tests still fast-loops: the build IS
                // the first signal there (stub and generated crates).
                "--no-tests=pass",
                "--status-level",
                "all",
                "--color",
                "never",
            ])
            .current_dir(&root)
            .output()
            .context("spawning cargo nextest (install: `cargo install cargo-nextest --locked`)")?;
        let seconds = started.elapsed().as_secs_f64();

        let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
        combined.push('\n');
        combined.push_str(&String::from_utf8_lossy(&out.stderr));
        let results = testgate::parse_nextest_output(&combined);
        let failed = results
            .values()
            .filter(|s| **s == testgate::RunStatus::Fail)
            .count();
        // Doctests ride the same loop (card scaffold-g-doctests):
        // nextest does not run them, so the loop would otherwise leave
        // Class-G checks outside the cell's first signal.
        let doc = Command::new("cargo")
            .args(["test", "--doc", "-p", name, "--quiet"])
            .current_dir(&root)
            .output()
            .context("spawning cargo test --doc")?;
        // `cargo test --doc` fails on crates with no lib target; that
        // is the no-tests case again, not a red cell.
        let doc_failed =
            !doc.status.success() && String::from_utf8_lossy(&doc.stderr).contains("test failed");
        // "No tests" is a legal cell state (stub crates); nextest exits
        // zero there. A non-zero exit with zero parsed results is a
        // build failure — isolation is broken, report it as red.
        let passed = out.status.success() && failed == 0 && !doc_failed;

        let over = if seconds > budget { " OVER BUDGET" } else { "" };
        eprintln!(
            "  fast-loop: {name} — {} in {seconds:.1}s ({} test result(s)){over}",
            if passed { "ok" } else { "RED" },
            results.len(),
        );
        runs.push(CellRun {
            cell: name.clone(),
            seconds,
            tests: results.len(),
            passed,
        });
    }

    // Machine-readable report for the adoption LOG (derived data,
    // never committed — same contract as target/conform/).
    let report_dir = root.join("target").join("fast-loop");
    std::fs::create_dir_all(&report_dir)?;
    let json: Vec<serde_json::Value> = runs
        .iter()
        .map(|r| {
            serde_json::json!({
                "cell": r.cell,
                "seconds": (r.seconds * 10.0).round() / 10.0,
                "tests": r.tests,
                "passed": r.passed,
                "within_budget": r.seconds <= budget,
            })
        })
        .collect();
    let report_path = report_dir.join("report.json");
    std::fs::write(&report_path, serde_json::to_string_pretty(&json)?)?;

    let red: Vec<&CellRun> = runs.iter().filter(|r| !r.passed).collect();
    let over: Vec<&CellRun> = runs.iter().filter(|r| r.seconds > budget).collect();
    let within = runs.len() - over.len();
    eprintln!(
        "xtask fast-loop: {}/{} cell(s) within the {budget_secs}s budget \
         ({:.0}%), {} red; report at {}.",
        within,
        runs.len(),
        100.0 * within as f64 / runs.len().max(1) as f64,
        red.len(),
        report_path
            .strip_prefix(&root)
            .unwrap_or(&report_path)
            .display()
    );
    if !red.is_empty() {
        bail!(
            "fast-loop: {} cell(s) RED in isolation: {}",
            red.len(),
            red.iter()
                .map(|r| r.cell.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if enforce_budget && !over.is_empty() {
        bail!(
            "fast-loop: {} cell(s) over the {budget_secs}s budget: {}",
            over.len(),
            over.iter()
                .map(|r| format!("{} ({:.1}s)", r.cell, r.seconds))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

fn run_trace_explain(target: &str, json: bool, prose: bool) -> Result<()> {
    let root = repo_root()?;
    // Build fresh in-memory: explain answers for the tree as it is,
    // never for a stale committed artefact.
    let map = specmap_core::index::build(&root);
    if prose {
        let render = specmap_core::ledger::prose_explain(&root, &map, target)?;
        print!("{}", render.text);
        let t = specmap_core::ledger::load_telemetry(&root);
        eprintln!(
            "xtask trace explain --prose: {} (epoch {}; ledger telemetry: {} hit(s), {} miss(es)).",
            if render.cached {
                "cache hit"
            } else {
                "computed fresh"
            },
            render.epoch.short(),
            t.hits,
            t.misses
        );
    } else if json {
        let v = specmap_core::explain::explain_json(&map, target)?;
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        print!("{}", specmap_core::explain::explain_text(&map, target)?);
    }
    Ok(())
}

fn run_specmap(check: bool) -> Result<()> {
    let root = repo_root()?;
    if check {
        match specmap_core::index::check(&root)? {
            Ok(summary) => {
                eprintln!("xtask specmap --check: clean ({summary}).");
            }
            Err(msg) => bail!("{msg}"),
        }
    } else {
        let (path, summary) = specmap_core::index::write(&root)?;
        eprintln!("xtask specmap: wrote {} ({summary}).", path.display());
    }
    run_ratchet_gate(&root, check)
}

/// The Phase 2 ratchet: the orphan gate over non-exempt crates
/// (PLAYBOOK #phase2 "flip the ratchet"). Reported in both modes;
/// blocking only under `--check`. Absent ratchet file = gate off.
fn run_ratchet_gate(root: &std::path::Path, blocking: bool) -> Result<()> {
    let Some(ratchet) = specmap_core::ratchet::load(root)? else {
        return Ok(());
    };
    let map = specmap_core::index::build(root);
    let orphans = specmap_core::ratchet::orphans(root, &map, &ratchet);
    let mut blockers = 0usize;
    for o in &orphans {
        match &o.disposition {
            Some(debt) => eprintln!(
                "  ratchet: orphan dispositioned ({debt}): `{}` ({}) at {}:{}",
                o.symbol, o.item_kind, o.file, o.line
            ),
            None => {
                blockers += 1;
                eprintln!(
                    "  ratchet: ORPHAN `{}` ({}) at {}:{} — tag it, scope! its module, \
                     or disposition it in specmap-ratchet.json with a debt id",
                    o.symbol, o.item_kind, o.file, o.line
                );
            }
        }
    }
    eprintln!(
        "xtask specmap: ratchet gate — {} gated orphan(s), {} dispositioned ({} crate(s) exempt).",
        blockers,
        orphans.len() - blockers,
        ratchet.exempt.len()
    );
    if blocking && blockers > 0 {
        bail!(
            "specmap ratchet: {blockers} orphan(s) in gated crates — \
             see the list above (PLAYBOOK #phase2 acceptance: empty or dispositioned)"
        );
    }
    Ok(())
}

/// The Phase 4 conform gate (ENGINE-CONFORM §5):
/// `cargo xtask conform check --baseline conform-baseline.json
/// [--scope crates/vibe-resolver]`. Facts are extracted workspace-wide
/// through the content-addressed store (incremental by file hash);
/// findings are reported only inside `--scope`; new findings against
/// the baseline fail the gate; SARIF lands under `target/conform/`.
fn run_conform_check(baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    use conform_core::{
        ExtractionLog, Frontend, Rule, Store, baseline, check, count_by_rule, rules, sarif,
    };
    use conform_frontend_rust::RustFrontend;

    let root = repo_root()?;
    let store = Store::at_repo(&root);
    let mut log = ExtractionLog::default();
    let frontend = RustFrontend;
    let facts = store.extract_workspace(&root, &frontend, &mut log)?;
    eprintln!(
        "xtask conform: extracted {} file(s), {} cached (producer {}-{}).",
        log.extracted.len(),
        log.cached,
        Frontend::id(&frontend),
        Frontend::version(&frontend),
    );

    let flag_sites = rules::FlagSites {
        registry_file: "crates/vibe-cli/src/registry.rs",
        gated_crate: "vibe-cli",
    };
    let isolation = rules::CellIsolation;
    // No crate is a designated unsafe-audit crate today; the list grows
    // by owner decision, not by accretion.
    let unsafe_gate = rules::UnsafeGate { audit_crates: &[] };
    // Classes G and F (adopt-v0.3 Phase 2). Gated crates grow with the
    // cell sweep — a crate enters when its seams are doctested and its
    // error layer carries REQ edges, the same expand-as-you-conform
    // rhythm the orphan ratchet used.
    let seam_doctests = rules::SeamHasDoctest {
        gated_crates: &["vibe-resolver", "conform-core", "specmap-core"],
    };
    let err_req = rules::ErrorEnumCitesReq {
        gated_crates: &["vibe-resolver", "conform-core", "specmap-core"],
    };
    // Class D (adopt-v0.3 Phase 4): self-scoping — gates exactly the
    // crates that declare #[cell] manifests.
    let cell_oracle = rules::CellHasOracle;
    let rule_refs: Vec<&dyn Rule> = vec![
        &flag_sites,
        &isolation,
        &unsafe_gate,
        &seam_doctests,
        &err_req,
        &cell_oracle,
    ];

    let findings = check(&rule_refs, &facts, scope);
    let report = sarif::render(&rule_refs, &findings);
    let sarif_path = root.join("target").join("conform").join("report.sarif");
    if let Some(parent) = sarif_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&sarif_path, &report)?;

    let base = baseline::load(&root.join(baseline_rel))?;
    let (new, stale) = baseline::diff(&base, &findings);
    for f in &new {
        eprintln!(
            "  conform: NEW {} {}:{} — {}",
            f.rule, f.file, f.line, f.message
        );
    }
    for fp in &stale {
        eprintln!("  conform: baseline entry no longer fires — prune it: {fp}");
    }
    let counts = count_by_rule(&findings);
    eprintln!(
        "xtask conform check: {} finding(s) in scope {} ({:?}), {} frozen in baseline, {} new; SARIF at {}.",
        findings.len(),
        scope.unwrap_or("<workspace>"),
        counts,
        base.findings.len(),
        new.len(),
        sarif_path
            .strip_prefix(&root)
            .unwrap_or(&sarif_path)
            .display()
    );
    if !new.is_empty() {
        bail!("conform: {} new finding(s) against the baseline", new.len());
    }
    Ok(())
}

fn run_test_gate(baseline_rel: &str) -> Result<()> {
    use specmap_core::testgate;

    let root = repo_root()?;
    let baseline_path = root.join(baseline_rel);
    let baseline_json = std::fs::read_to_string(&baseline_path)
        .with_context(|| format!("reading {}", baseline_path.display()))?;
    let baseline = testgate::parse_baseline(&baseline_json)?;

    eprintln!("xtask test-gate: running `cargo nextest run --workspace --no-fail-fast` …");
    let out = Command::new("cargo")
        .args([
            "nextest",
            "run",
            "--workspace",
            "--no-fail-fast",
            "--status-level",
            "all",
            "--color",
            "never",
        ])
        .current_dir(&root)
        .output()
        .context("spawning cargo nextest (install: `cargo install cargo-nextest --locked`)")?;

    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push('\n');
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    let results = testgate::parse_nextest_output(&combined);

    // A gate that parsed nothing must never report green: that is how
    // gates get gamed by accident (PLAYBOOK §8).
    if results.is_empty() {
        bail!(
            "test-gate parsed zero test results out of the nextest run \
             (nextest exit: {:?}); refusing to conclude anything",
            out.status.code()
        );
    }

    let report = testgate::evaluate(&baseline, &results);
    let total = results.len();
    let skipped = results
        .values()
        .filter(|s| **s == testgate::RunStatus::Skip)
        .count();
    let failed = results
        .values()
        .filter(|s| **s == testgate::RunStatus::Fail)
        .count();
    eprintln!(
        "xtask test-gate: {total} results parsed ({failed} failed, {skipped} skipped), \
         baseline entries: {}",
        baseline.len()
    );
    for (test, status) in &report.flaky_observed {
        eprintln!("  flaky (never gating): {test} — {status}");
    }
    for test in &report.missing_from_run {
        eprintln!(
            "  warning: baseline entry never appeared in the run \
             (renamed or deleted? shrink the baseline via the promotion \
             protocol): {test}"
        );
    }
    if report.is_green() {
        eprintln!("xtask test-gate: green (xfail-strict).");
        return Ok(());
    }
    for test in &report.newly_failing {
        eprintln!("  NEWLY FAILING: {test}");
    }
    for test in &report.unexpectedly_passing {
        eprintln!("  UNEXPECTEDLY PASSING (unpromoted — see PLAYBOOK §7.2): {test}");
    }
    bail!(
        "test-gate failed: {} newly failing, {} unexpectedly passing",
        report.newly_failing.len(),
        report.unexpectedly_passing.len()
    );
}

/// Collect the change set as repo-relative forward-slash paths.
fn changed_paths(root: &Path, base: Option<&str>) -> Result<Vec<String>> {
    let mut args_sets: Vec<Vec<String>> = Vec::new();
    match base {
        Some(rev) => {
            args_sets.push(vec![
                "diff".into(),
                "--name-only".into(),
                format!("{rev}...HEAD"),
            ]);
            // Plus whatever is uncommitted right now.
            args_sets.push(vec!["diff".into(), "--name-only".into(), "HEAD".into()]);
        }
        None => {
            args_sets.push(vec!["diff".into(), "--name-only".into(), "HEAD".into()]);
            args_sets.push(vec!["diff".into(), "--name-only".into(), "--cached".into()]);
        }
    }
    args_sets.push(vec![
        "ls-files".into(),
        "--others".into(),
        "--exclude-standard".into(),
    ]);

    let mut paths: Vec<String> = Vec::new();
    for args in args_sets {
        let out = Command::new("git")
            .args(&args)
            .current_dir(root)
            .output()
            .context("spawning git")?;
        if !out.status.success() {
            bail!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let p = line.trim().replace('\\', "/");
            if !p.is_empty() && !paths.contains(&p) {
                paths.push(p);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

fn run_tripwire(base: Option<&str>, debt_rel: &str) -> Result<()> {
    let root = repo_root()?;
    let debt_path = root.join(debt_rel);
    let debt_json = std::fs::read_to_string(&debt_path)
        .with_context(|| format!("reading {}", debt_path.display()))?;
    let changed = changed_paths(&root, base)?;
    if changed.is_empty() {
        eprintln!("xtask tripwire: change set is empty — nothing to match.");
        return Ok(());
    }
    let fired = specmap_core::tripwire::evaluate(&debt_json, &changed)?;
    if fired.is_empty() {
        eprintln!(
            "xtask tripwire: no debt tripwires fire on {} changed path(s).",
            changed.len()
        );
        return Ok(());
    }
    eprintln!(
        "xtask tripwire: {} debt entr{} fire on this change set — address \
         each in the PR description: pulled-in, re-dispositioned, or \
         consciously deferred (PLAYBOOK §7.5):",
        fired.len(),
        if fired.len() == 1 { "y" } else { "ies" }
    );
    for f in fired {
        eprintln!(
            "  [{}] {} — {} ({})",
            f.severity, f.id, f.title, f.disposition
        );
        for (pattern, paths) in &f.hits {
            for p in paths {
                eprintln!("      {pattern}  ←  {p}");
            }
        }
        for wire in &f.unevaluated {
            eprintln!("      {wire}  (not yet evaluable — needs specmap revisions, Phase 1)");
        }
    }
    // Warn-only by contract.
    Ok(())
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

/// Locate the jtd-codegen binary. Prefer the project-local copy under
/// `tools/jtd-codegen/`; fall back to PATH if the local copy is
/// absent so contributors who chose a system-wide install still work.
fn find_jtd_codegen(root: &Path) -> Result<PathBuf> {
    let exe = if cfg!(windows) {
        "jtd-codegen.exe"
    } else {
        "jtd-codegen"
    };
    let local = root.join("tools").join("jtd-codegen").join(exe);
    if local.exists() {
        return Ok(local);
    }
    // Fall back to PATH lookup.
    let probe = Command::new(exe).arg("--version").output();
    match probe {
        Ok(out) if out.status.success() => Ok(PathBuf::from(exe)),
        _ => bail!(
            "jtd-codegen not found. Looked at:\n  \
             1. {} (project-local, preferred)\n  \
             2. `{exe}` on PATH (fallback)\n\n\
             Install per `tools/jtd-codegen/README.md`. PROP-000 §16 \
             pins the JTD + codegen toolchain as project-local; the PATH \
             fallback is a courtesy for contributors who already have \
             it installed system-wide.",
            local.display()
        ),
    }
}

fn run_codegen() -> Result<()> {
    let root = repo_root()?;
    let schemas_dir = root.join("schemas");
    let out_dir = root.join("crates/vibe-wire/src/generated");

    if !schemas_dir.exists() {
        bail!(
            "`schemas/` directory not found at {}",
            schemas_dir.display()
        );
    }
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating output dir {}", out_dir.display()))?;

    let binary = find_jtd_codegen(&root)?;

    // Find every `*.jtd.json` under `schemas/`.
    let schemas: Vec<PathBuf> = std::fs::read_dir(&schemas_dir)
        .with_context(|| format!("reading {}", schemas_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".jtd.json"))
                    .unwrap_or(false)
        })
        .collect();

    if schemas.is_empty() {
        eprintln!(
            "no `*.jtd.json` schemas under `{}` — nothing to do.",
            schemas_dir.display()
        );
        return Ok(());
    }

    eprintln!(
        "xtask codegen: {} schema{} → {}",
        schemas.len(),
        if schemas.len() == 1 { "" } else { "s" },
        out_dir.display()
    );

    // Wipe everything under `out_dir` first so a removed schema doesn't
    // leave a stale submodule that the synthesised top-level `mod.rs`
    // would no longer reference. We rebuild from scratch each run; the
    // codegen output is fast enough that this is fine, and it makes the
    // `check-codegen` invariant exact: what's on disk matches what the
    // generator would produce *only* from the current `schemas/`.
    if out_dir.exists() {
        for entry in std::fs::read_dir(&out_dir)
            .with_context(|| format!("scanning {}", out_dir.display()))?
        {
            let entry = entry.context("reading entry under out_dir")?;
            let path = entry.path();
            // Preserve a `.gitkeep` if one is present so an empty
            // (no-schema) state still leaves a tracked path; otherwise
            // remove. Subdirs and `mod.rs` are codegen output.
            if path.file_name().and_then(|n| n.to_str()) == Some(".gitkeep") {
                continue;
            }
            if path.is_dir() {
                std::fs::remove_dir_all(&path)
                    .with_context(|| format!("removing stale {}", path.display()))?;
            } else {
                std::fs::remove_file(&path)
                    .with_context(|| format!("removing stale {}", path.display()))?;
            }
        }
    }

    // jtd-codegen 0.4.1 writes a single `mod.rs` per `--rust-out` and
    // overwrites whatever is there. To keep all schemas in one tree
    // without each one stomping the others, give each schema its own
    // subdirectory and synthesise a top-level `mod.rs` that re-exports
    // every per-schema submodule.
    let mut module_names: Vec<String> = Vec::new();
    for schema in &schemas {
        let stem = schema
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".jtd.json"))
            .with_context(|| format!("schema name not `*.jtd.json`: {}", schema.display()))?
            .to_string();
        let sub_out = out_dir.join(&stem);
        std::fs::create_dir_all(&sub_out)
            .with_context(|| format!("creating per-schema dir {}", sub_out.display()))?;
        eprintln!("  - {} → {}/", schema.display(), stem);
        let status = Command::new(&binary)
            .arg("--rust-out")
            .arg(&sub_out)
            .arg(schema)
            .status()
            .with_context(|| format!("spawning {}", binary.display()))?;
        if !status.success() {
            bail!(
                "jtd-codegen failed for `{}` (exit code {:?})",
                schema.display(),
                status.code()
            );
        }
        module_names.push(stem);
    }

    // Synthesise the top-level `mod.rs` that fans out to each per-schema
    // submodule. Module names are sorted for determinism so `check-codegen`
    // stays stable across platforms (filesystem read order is not
    // guaranteed).
    module_names.sort();
    let mut top = String::new();
    top.push_str("// Generated by `cargo xtask codegen`. DO NOT EDIT.\n");
    top.push_str("//\n");
    top.push_str("// Each submodule is generated by `jtd-codegen` from the matching\n");
    top.push_str("// `*.jtd.json` schema under `schemas/` at the repo root. Editing\n");
    top.push_str("// this file by hand will be overwritten on the next codegen run.\n\n");
    for name in &module_names {
        top.push_str(&format!("pub mod {name};\n"));
    }
    let top_path = out_dir.join("mod.rs");
    std::fs::write(&top_path, top).with_context(|| format!("writing {}", top_path.display()))?;

    eprintln!("xtask codegen: ok ({} submodules).", module_names.len());
    Ok(())
}

fn run_check_codegen() -> Result<()> {
    run_codegen()?;
    let root = repo_root()?;
    let out_dir = root.join("crates/vibe-wire/src/generated");
    let status = Command::new("git")
        .arg("diff")
        .arg("--exit-code")
        .arg("--")
        .arg(&out_dir)
        .current_dir(&root)
        .status()
        .context("spawning git diff")?;
    if !status.success() {
        bail!(
            "generated code under `{}` is out of date relative to `schemas/`. \
             Run `cargo xtask codegen` and commit the result.",
            out_dir.display()
        );
    }
    eprintln!("xtask check-codegen: clean.");
    Ok(())
}
