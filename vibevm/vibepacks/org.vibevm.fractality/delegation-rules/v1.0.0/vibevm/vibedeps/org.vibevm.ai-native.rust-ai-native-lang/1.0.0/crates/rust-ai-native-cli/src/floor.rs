//! `rust-ai-native floor` — the portable verification floor (Sweep
//! Playbook Tier 0): format → tests → lints → the conform gate → the
//! specmap check → the xfail-strict test-gate (when a baseline registry
//! exists). One command, one exit code, per-step headers, and an
//! explicit line for every policy-disabled step so a reduced floor can
//! never masquerade as the full one — the Go/TypeScript `floor_disable`
//! twin (B-049). This is what a consumer runs instead of hand-assembling
//! a self-check script.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};
use conform_core::FloorDisable;

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

/// The floor's steps, in the order `run_floor` records them — the
/// dictionary a `[rust].floor_disable` entry is validated against (the
/// Go/TypeScript `STEPS` twin). A name not in this list is a hard
/// failure, never a silent ignore.
const STEPS: &[&str] = &[
    "fmt",
    "test",
    "clippy",
    "conform",
    "specmap",
    "test-gate",
    "fast-loop",
];

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

/// Validate a `[rust].floor_disable` list against the floor's known
/// steps — an unknown step name is a hard failure (never a silent
/// ignore), the Go/TypeScript twin posture. Returns `Ok(())` when every
/// named step exists; otherwise the error names the offender and lists
/// the valid steps. Pure (no I/O) so the twin's hard-fail posture is
/// unit-tested in isolation below.
fn validate_floor_disable(disabled: &[FloorDisable]) -> Result<()> {
    for d in disabled {
        if !STEPS.contains(&d.step.as_str()) {
            bail!(
                "floor: `[[rust.floor_disable]]` names unknown step `{}` (steps: {STEPS:?})",
                d.step
            );
        }
    }
    Ok(())
}

/// Whether `step` is policy-disabled — the predicate `run_floor` gates
/// every step on (the Go/TypeScript `is_disabled` twin). Pure (no I/O)
/// so the skip behaviour is unit-tested in isolation below.
fn is_step_disabled(step: &str, disabled: &[FloorDisable]) -> bool {
    disabled.iter().any(|d| d.step == step)
}

/// Run the floor over the project at `root`. Green ⇒ `Ok(())`; any red step
/// ⇒ an error naming the failed steps.
pub fn run_floor(root: &Path, opts: &FloorOptions) -> Result<()> {
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let disabled = &config.rust.floor_disable;
    validate_floor_disable(disabled)?;
    for d in disabled {
        eprintln!(
            "floor: step `{}` DISABLED by policy — {} (conform.toml [rust])",
            d.step, d.reason
        );
    }

    let mut outcomes: Vec<StepOutcome> = Vec::new();
    let record = |outcomes: &mut Vec<StepOutcome>, label: &'static str, ok: bool| {
        if !ok {
            eprintln!("floor: `{label}` FAILED");
        }
        outcomes.push(StepOutcome { label, ok });
        ok
    };

    // 1. Formatting — the cheapest signal first.
    if !is_step_disabled("fmt", disabled) {
        header(opts, "cargo fmt --all --check");
        let ok = run_cargo(root, &["fmt", "--all", "--check"])?;
        if !record(&mut outcomes, "fmt", ok) && !opts.keep_going {
            bail!("floor: `fmt` failed");
        }
    }

    // 2. Tests.
    if !is_step_disabled("test", disabled) {
        header(opts, "cargo test --workspace");
        let ok = run_cargo(root, &["test", "--workspace", "--quiet"])?;
        if !record(&mut outcomes, "test", ok) && !opts.keep_going {
            bail!("floor: `test` failed");
        }
    }

    // 3. Lints as errors.
    if !is_step_disabled("clippy", disabled) {
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
    }

    // 4. The conform gate (prints its own policy-origin line).
    if !is_step_disabled("conform", disabled) {
        header(opts, "conform check");
        let ok =
            rust_ai_native_conform::run_check(root, crate::DEFAULT_CONFORM_BASELINE, None).is_ok();
        if !record(&mut outcomes, "conform", ok) && !opts.keep_going {
            bail!("floor: `conform` failed");
        }
    }

    // 5. The specmap check (prints its own policy note when defaulted).
    if !is_step_disabled("specmap", disabled) {
        header(opts, "specmap --check");
        let ok = rust_ai_native_specmap::run_specmap(root, true).is_ok();
        if !record(&mut outcomes, "specmap", ok) && !opts.keep_going {
            bail!(
                "floor: `specmap` failed (fresh project? run `rust-ai-native specmap` once to mint the index)"
            );
        }
    }

    // 6. The xfail-strict test-gate — only when the registry exists (a
    // project that has not terraformed yet has no baseline to diff), and
    // not policy-disabled.
    if !is_step_disabled("test-gate", disabled) {
        let baseline = root.join(crate::DEFAULT_TESTS_BASELINE);
        if baseline.exists() {
            header(opts, "test-gate (xfail-strict)");
            let ok = crate::test_gate::run_test_gate(root, crate::DEFAULT_TESTS_BASELINE).is_ok();
            if !record(&mut outcomes, "test-gate", ok) && !opts.keep_going {
                bail!("floor: `test-gate` failed");
            }
        } else if !opts.quiet {
            eprintln!(
                "floor: no {} — test-gate skipped (run `rust-ai-native init`, then fill the baseline)",
                crate::DEFAULT_TESTS_BASELINE
            );
        }
    }

    // 7. Optional: per-cell fast loops.
    if opts.fast_loop && !is_step_disabled("fast-loop", disabled) {
        header(opts, "fast-loop (per-cell isolation)");
        let ok = crate::fast_loop::run_fast_loop(root, None, 60, false).is_ok();
        if !record(&mut outcomes, "fast-loop", ok) && !opts.keep_going {
            bail!("floor: `fast-loop` failed");
        }
    }

    let red: Vec<&str> = outcomes.iter().filter(|o| !o.ok).map(|o| o.label).collect();
    if red.is_empty() {
        eprintln!(
            "\nfloor: all green ({} step(s) run, {} disabled by policy).",
            outcomes.len(),
            disabled.len()
        );
        Ok(())
    } else {
        bail!("floor: {} step(s) failed: {}", red.len(), red.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disable(step: &str, reason: &str) -> FloorDisable {
        FloorDisable {
            step: step.to_string(),
            reason: reason.to_string(),
        }
    }

    /// (а) a step named in `[rust].floor_disable` is recognised as
    /// disabled, so `run_floor` skips it; a different step still runs.
    #[test]
    fn named_step_is_disabled_so_it_is_skipped() {
        let disabled = [disable("clippy", "pinned toolchain lints churn")];
        assert!(is_step_disabled("clippy", &disabled));
        assert!(!is_step_disabled("fmt", &disabled));
    }

    /// (б) an unknown step name is a hard failure — never a silent
    /// ignore (the twin posture); the error names the offender and lists
    /// every valid step so the operator can fix it.
    #[test]
    fn unknown_step_name_hard_fails() {
        let res = validate_floor_disable(&[disable("bogus", "no such step")]);
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("unknown step `bogus`"), "msg was: {msg}");
        for &step in STEPS {
            assert!(msg.contains(step), "msg missing valid step `{step}`: {msg}");
        }
    }

    /// (в) regression — an empty `floor_disable` validates (so the full
    /// floor still runs every step), and every real floor step is an
    /// accepted disable target.
    #[test]
    fn known_and_empty_disable_lists_validate() {
        assert!(validate_floor_disable(&[]).is_ok());
        for &step in STEPS {
            assert!(
                validate_floor_disable(&[disable(step, "ok")]).is_ok(),
                "step `{step}` should be a valid disable target"
            );
        }
    }
}
