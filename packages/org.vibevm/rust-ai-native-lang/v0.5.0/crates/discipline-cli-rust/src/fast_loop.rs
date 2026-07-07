//! `cargo fast-loop` — the Class-E `cell-fast-loop-present`
//! checker (discipline card scaffold-e-fast-loop, Band 3): every cell
//! builds and tests in isolation inside the per-cell budget.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

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
pub fn run_fast_loop(
    root: &Path,
    cell: Option<&str>,
    budget_secs: u64,
    enforce_budget: bool,
) -> Result<()> {
    use specmap_core::testgate;

    let cells = match cell {
        Some(one) => vec![one.to_string()],
        None => workspace_members(root)?,
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
            .current_dir(root)
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
            .current_dir(root)
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
        "fast-loop: {}/{} cell(s) within the {budget_secs}s budget \
         ({:.0}%), {} red; report at {}.",
        within,
        runs.len(),
        100.0 * within as f64 / runs.len().max(1) as f64,
        red.len(),
        report_path
            .strip_prefix(root)
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
