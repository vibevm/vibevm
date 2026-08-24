//! `go-ai-native fast-loop` — the Class-E `cell-fast-loop-present`
//! checker, Go shape (scaffold-e-fast-loop): every cell's tests run in
//! isolation inside the per-cell budget, measured wall-clock to the
//! verdict via `go test -json` (the same parser the test-gate uses, so
//! the two gates cannot disagree on what a test result is).
//!
//! Go hands this scaffold the strongest substrate of the three stacks:
//! per-package `go test ./<cell>/...` needs no project references and
//! no per-cell build isolation work — the toolchain was engineered for
//! it. A cell without tests fails the check — the card requires the
//! loop to EXIST.

specmark::scope!("spec://go-ai-native-lang/go/GUIDE-AI-NATIVE-GO#scaffolds");

use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};

/// Whether a cell directory carries any `*_test.go`.
fn has_tests(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if has_tests(&path) {
                return true;
            }
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with("_test.go"))
        {
            return true;
        }
    }
    false
}

pub fn run_fast_loop(
    root: &Path,
    cell: Option<&str>,
    budget_secs: u64,
    enforce_budget: bool,
) -> Result<()> {
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let Some(cells_dir) = config.go.cells_dir.clone() else {
        bail!(
            "fast-loop: no `cells_dir` in conform.toml [go] — the cell layout is \
             what the per-cell loop measures (set it, e.g. \
             cells_dir = \"internal/cells\")"
        );
    };
    let cells_root = root.join(&cells_dir);
    let mut cells: Vec<String> = match cell {
        Some(one) => vec![one.to_string()],
        None => {
            let Ok(entries) = std::fs::read_dir(&cells_root) else {
                bail!("fast-loop: `{cells_dir}` does not exist");
            };
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .collect()
        }
    };
    cells.sort();
    if cells.is_empty() {
        bail!("fast-loop: no cells under `{cells_dir}`");
    }

    let budget = budget_secs as f64;
    let mut over_budget: Vec<(String, f64)> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    for cell in &cells {
        let dir = cells_root.join(cell);
        if !has_tests(&dir) {
            eprintln!("  fast-loop: {cell}: NO TESTS — the cell has no fast loop");
            failed.push(cell.clone());
            continue;
        }
        let started = Instant::now();
        let mut cmd = crate::tools::go_command(root);
        cmd.args(["test", &format!("./{cells_dir}/{cell}/..."), "-json"]);
        let out = cmd.output()?;
        let seconds = started.elapsed().as_secs_f64();
        let results = crate::gotest::parse_gotest_json(&String::from_utf8_lossy(&out.stdout));
        let red = results
            .values()
            .filter(|s| **s == specmap_core::testgate::RunStatus::Fail)
            .count();
        let verdict_ok = out.status.success() && red == 0 && !results.is_empty();
        eprintln!(
            "  fast-loop: {cell}: {} test(s) in {seconds:.1}s{}{}",
            results.len(),
            if verdict_ok { "" } else { " — RED" },
            if seconds > budget {
                " — OVER BUDGET"
            } else {
                ""
            },
        );
        if !verdict_ok {
            failed.push(cell.clone());
        }
        if seconds > budget {
            over_budget.push((cell.clone(), seconds));
        }
    }

    if !failed.is_empty() {
        bail!(
            "fast-loop: {} cell(s) red: {}",
            failed.len(),
            failed.join(", ")
        );
    }
    if enforce_budget && !over_budget.is_empty() {
        bail!(
            "fast-loop: {} cell(s) over the {budget_secs}s budget",
            over_budget.len()
        );
    }
    eprintln!(
        "fast-loop: {} cell(s) green{}.",
        cells.len(),
        if over_budget.is_empty() {
            String::new()
        } else {
            format!(" ({} over budget — warned)", over_budget.len())
        }
    );
    Ok(())
}
