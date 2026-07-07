//! `discipline-typescript floor` — the seven-step verification floor
//! (D7, full Rust-floor parity): format → typecheck → tests → lint →
//! conform → specmap → test-gate. One command, one exit code, per-step
//! headers, and an explicit line for every policy-disabled step so a
//! reduced floor can never masquerade as the full one. Absent tooling
//! is a hard step failure carrying the install recipe — never a skip.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};

pub struct FloorOptions {
    /// Run every step even after a failure (report all, then exit
    /// non-zero).
    pub keep_going: bool,
    /// Suppress the per-step headers.
    pub quiet: bool,
}

struct StepOutcome {
    label: &'static str,
    ok: bool,
}

const STEPS: &[&str] = &[
    "prettier",
    "tsc",
    "tests",
    "eslint",
    "conform",
    "specmap",
    "test-gate",
];

fn header(opts: &FloorOptions, label: &str) {
    if !opts.quiet {
        eprintln!("\n=== {label} ===");
    }
}

fn run_tool_step(mut cmd: Command) -> Result<bool> {
    match cmd.status() {
        Ok(status) => Ok(status.success()),
        Err(e) => bail!("spawning the step's tool: {e}"),
    }
}

/// The floor. Green ⇒ `Ok(())`; any red step ⇒ an error naming them.
pub fn run_floor(root: &Path, opts: &FloorOptions) -> Result<()> {
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let disabled = &config.typescript.floor_disable;
    for d in disabled {
        if !STEPS.contains(&d.step.as_str()) {
            bail!(
                "floor: `[[typescript.floor_disable]]` names unknown step `{}` \
                 (steps: {STEPS:?})",
                d.step
            );
        }
        eprintln!(
            "floor: step `{}` DISABLED by policy — {} (conform.toml [typescript])",
            d.step, d.reason
        );
    }
    let is_disabled = |step: &str| disabled.iter().any(|d| d.step == step);

    let mut outcomes: Vec<StepOutcome> = Vec::new();
    let record = |outcomes: &mut Vec<StepOutcome>, label: &'static str, ok: bool| {
        if !ok {
            eprintln!("floor: `{label}` FAILED");
        }
        outcomes.push(StepOutcome { label, ok });
        ok
    };

    // 1. Formatting — the cheapest signal first.
    if !is_disabled("prettier") {
        header(opts, "prettier --check .");
        let ok = match crate::tools::tool_command(root, "prettier") {
            Some(mut cmd) => {
                cmd.args(["--check", "."]);
                run_tool_step(cmd)?
            }
            None => {
                eprintln!(
                    "floor: `prettier` is not installed in this project — \
                     `npm install -D prettier` (or disable the step with a reason \
                     in conform.toml [typescript].floor_disable)"
                );
                false
            }
        };
        if !record(&mut outcomes, "prettier", ok) && !opts.keep_going {
            bail!("floor: `prettier` failed");
        }
    }

    // 2. Typecheck — the compiler is the TS half of `cargo test`'s
    // compile; the same install feeds the ts-tsc structural gate.
    if !is_disabled("tsc") {
        header(opts, "tsc --noEmit");
        let ok = match crate::tools::tool_command(root, "tsc") {
            Some(mut cmd) => {
                cmd.arg("--noEmit");
                run_tool_step(cmd)?
            }
            None => {
                eprintln!(
                    "floor: `tsc` is not installed in this project — \
                     `npm install -D typescript` (the structural gate needs it too)"
                );
                false
            }
        };
        if !record(&mut outcomes, "tsc", ok) && !opts.keep_going {
            bail!("floor: `tsc` failed");
        }
    }

    // 3. Tests — the project's script when it has one, `node --test`
    // otherwise (strip-types runs .ts directly on node >= 22.6).
    if !is_disabled("tests") {
        header(opts, "tests (node --test)");
        let ok = {
            let mut cmd = crate::tools::node_command(root);
            cmd.arg("--test");
            run_tool_step(cmd)?
        };
        if !record(&mut outcomes, "tests", ok) && !opts.keep_going {
            bail!("floor: `tests` failed");
        }
    }

    // 4. Lint.
    if !is_disabled("eslint") {
        header(opts, "eslint .");
        let ok = match crate::tools::tool_command(root, "eslint") {
            Some(mut cmd) => {
                cmd.arg(".");
                run_tool_step(cmd)?
            }
            None => {
                eprintln!(
                    "floor: `eslint` is not installed in this project — \
                     `npm install -D eslint typescript-eslint` (or disable the step \
                     with a reason in conform.toml [typescript].floor_disable)"
                );
                false
            }
        };
        if !record(&mut outcomes, "eslint", ok) && !opts.keep_going {
            bail!("floor: `eslint` failed");
        }
    }

    // 5. The conform gate (the ts-tsc structural rules).
    if !is_disabled("conform") {
        header(opts, "conform-typescript check");
        let ok = conform_cli_typescript::run_check(
            root,
            conform_cli_typescript::DEFAULT_TS_BASELINE,
            None,
        )
        .map(|()| true)
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            false
        });
        if !record(&mut outcomes, "conform", ok) && !opts.keep_going {
            bail!("floor: `conform` failed");
        }
    }

    // 6. The traceability check + orphan ratchet.
    if !is_disabled("specmap") {
        header(opts, "specmap-typescript --check");
        let ok = specmap_cli_typescript::run_specmap_typescript(root, true)
            .map(|()| true)
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                false
            });
        if !record(&mut outcomes, "specmap", ok) && !opts.keep_going {
            bail!("floor: `specmap` failed");
        }
    }

    // 7. The xfail-strict test-gate, when a baseline registry exists
    // (same condition as the Rust floor).
    if !is_disabled("test-gate") {
        let baseline = root.join(crate::DEFAULT_TESTS_BASELINE);
        if baseline.exists() {
            header(opts, "test-gate (xfail-strict)");
            let ok = crate::run_test_gate(root, crate::DEFAULT_TESTS_BASELINE)
                .map(|()| true)
                .unwrap_or_else(|e| {
                    eprintln!("{e}");
                    false
                });
            if !record(&mut outcomes, "test-gate", ok) && !opts.keep_going {
                bail!("floor: `test-gate` failed");
            }
        } else if !opts.quiet {
            eprintln!(
                "\nfloor: no tests baseline at {} — the test-gate step arms when \
                 `discipline-typescript init` writes it",
                crate::DEFAULT_TESTS_BASELINE
            );
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
        bail!("floor: {} step(s) failed: {}", red.len(), red.join(", "));
    }
}
