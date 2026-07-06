//! `discipline-rust floor` — the portable verification floor (Sweep
//! Playbook Tier 0): format → tests → lints → the conform gate → the
//! specmap check → the xfail-strict test-gate (when a baseline registry
//! exists). One command, one exit code, per-step headers, and an explicit
//! per-policy origin line so a defaulted (nothing-gated) policy can never
//! masquerade as a configured green. This is what a consumer runs instead
//! of hand-assembling a self-check script.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};

/// Floor behaviour switches.
pub struct FloorOptions {
    /// Run every step even after a failure (report all, then exit non-zero).
    pub keep_going: bool,
    /// Suppress the per-step headers.
    pub quiet: bool,
    /// Also run the per-cell fast-loop (expensive: builds each cell in
    /// isolation).
    pub fast_loop: bool,
}

struct StepOutcome {
    label: &'static str,
    ok: bool,
}

fn header(opts: &FloorOptions, label: &str) {
    if !opts.quiet {
        eprintln!("\n=== {label} ===");
    }
}

fn run_cargo(root: &Path, args: &[&str]) -> Result<bool> {
    let status = Command::new("cargo").args(args).current_dir(root).status();
    match status {
        Ok(s) => Ok(s.success()),
        Err(e) => bail!("spawning cargo {}: {e}", args.join(" ")),
    }
}

/// Run the floor over the project at `root`. Green ⇒ `Ok(())`; any red step
/// ⇒ an error naming the failed steps.
pub fn run_floor(root: &Path, opts: &FloorOptions) -> Result<()> {
    let mut outcomes: Vec<StepOutcome> = Vec::new();
    let record = |outcomes: &mut Vec<StepOutcome>, label: &'static str, ok: bool| {
        if !ok {
            eprintln!("floor: `{label}` FAILED");
        }
        outcomes.push(StepOutcome { label, ok });
        ok
    };

    // 1. Formatting — the cheapest signal first.
    header(opts, "cargo fmt --all --check");
    let ok = run_cargo(root, &["fmt", "--all", "--check"])?;
    if !record(&mut outcomes, "fmt", ok) && !opts.keep_going {
        bail!("floor: `fmt` failed");
    }

    // 2. Tests.
    header(opts, "cargo test --workspace");
    let ok = run_cargo(root, &["test", "--workspace", "--quiet"])?;
    if !record(&mut outcomes, "test", ok) && !opts.keep_going {
        bail!("floor: `test` failed");
    }

    // 3. Lints as errors.
    header(
        opts,
        "cargo clippy --workspace --all-targets -- -D warnings",
    );
    let ok = run_cargo(
        root,
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--quiet",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    if !record(&mut outcomes, "clippy", ok) && !opts.keep_going {
        bail!("floor: `clippy` failed");
    }

    // 4. The conform gate (prints its own policy-origin line).
    header(opts, "conform check");
    let ok = conform_cli::run_check(root, crate::DEFAULT_CONFORM_BASELINE, None).is_ok();
    if !record(&mut outcomes, "conform", ok) && !opts.keep_going {
        bail!("floor: `conform` failed");
    }

    // 5. The specmap check (prints its own policy note when defaulted).
    header(opts, "specmap --check");
    let ok = specmap_cli::run_specmap(root, true).is_ok();
    if !record(&mut outcomes, "specmap", ok) && !opts.keep_going {
        bail!(
            "floor: `specmap` failed (fresh project? run `discipline-rust specmap` once to mint the index)"
        );
    }

    // 6. The xfail-strict test-gate — only when the registry exists (a
    // project that has not terraformed yet has no baseline to diff).
    let baseline = root.join(crate::DEFAULT_TESTS_BASELINE);
    if baseline.exists() {
        header(opts, "test-gate (xfail-strict)");
        let ok = crate::test_gate::run_test_gate(root, crate::DEFAULT_TESTS_BASELINE).is_ok();
        if !record(&mut outcomes, "test-gate", ok) && !opts.keep_going {
            bail!("floor: `test-gate` failed");
        }
    } else if !opts.quiet {
        eprintln!(
            "floor: no {} — test-gate skipped (run `discipline-rust init`, then fill the baseline)",
            crate::DEFAULT_TESTS_BASELINE
        );
    }

    // 7. Optional: per-cell fast loops.
    if opts.fast_loop {
        header(opts, "fast-loop (per-cell isolation)");
        let ok = crate::fast_loop::run_fast_loop(root, None, 60, false).is_ok();
        if !record(&mut outcomes, "fast-loop", ok) && !opts.keep_going {
            bail!("floor: `fast-loop` failed");
        }
    }

    let red: Vec<&str> = outcomes.iter().filter(|o| !o.ok).map(|o| o.label).collect();
    if red.is_empty() {
        if !opts.quiet {
            eprintln!("\nfloor: all green");
        }
        Ok(())
    } else {
        bail!("floor: {} step(s) failed: {}", red.len(), red.join(", "))
    }
}
