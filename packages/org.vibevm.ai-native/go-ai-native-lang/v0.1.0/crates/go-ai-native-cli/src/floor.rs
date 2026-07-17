//! `go-ai-native floor` — the seven-step verification floor (D7, full
//! sibling parity): gofmt → vet → tests → staticcheck+exhaustive →
//! conform → specmap → test-gate. One command, one exit code, per-step
//! headers, and an explicit line for every policy-disabled step so a
//! reduced floor can never masquerade as the full one. Absent tooling
//! is a hard step failure carrying the install recipe — never a skip.

specmark::scope!("spec://go-ai-native-lang/go/GUIDE-AI-NATIVE-GO#baseline");

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
    "gofmt",
    "vet",
    "tests",
    "staticcheck",
    "conform",
    "specmap",
    "test-gate",
];

fn header(opts: &FloorOptions, label: &str) {
    if !opts.quiet {
        eprintln!("\n=== {label} ===");
    }
}

fn run_tool_step(mut cmd: Command, recipe: &str) -> bool {
    match cmd.status() {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("floor: the step's tool did not spawn ({e}) — {recipe}");
            false
        }
    }
}

/// The floor. Green ⇒ `Ok(())`; any red step ⇒ an error naming them.
pub fn run_floor(root: &Path, opts: &FloorOptions) -> Result<()> {
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let disabled = &config.go.floor_disable;
    for d in disabled {
        if !STEPS.contains(&d.step.as_str()) {
            bail!(
                "floor: `[[go.floor_disable]]` names unknown step `{}` (steps: {STEPS:?})",
                d.step
            );
        }
        eprintln!(
            "floor: step `{}` DISABLED by policy — {} (conform.toml [go])",
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

    // 1. Formatting — the cheapest signal first, and the one war the
    // language already won. `gofmt -l` lists unformatted files; any
    // output is a failure.
    if !is_disabled("gofmt") {
        header(opts, "gofmt -l .");
        let mut cmd = crate::tools::gofmt_command(root);
        cmd.args(["-l", "."]);
        let ok = match cmd.output() {
            Ok(out) if out.status.success() => {
                let listed = String::from_utf8_lossy(&out.stdout);
                let dirty: Vec<&str> = listed.lines().filter(|l| !l.trim().is_empty()).collect();
                for f in &dirty {
                    eprintln!("  gofmt: unformatted: {f}");
                }
                dirty.is_empty()
            }
            Ok(out) => {
                eprintln!("  gofmt exited {:?}", out.status.code());
                false
            }
            Err(e) => {
                eprintln!(
                    "floor: `gofmt` did not spawn ({e}) — install go >= 1.24 \
                     (gofmt ships with the toolchain)"
                );
                false
            }
        };
        if !record(&mut outcomes, "gofmt", ok) && !opts.keep_going {
            bail!("floor: `gofmt` failed");
        }
    }

    // 2. Vet — the toolchain's own correctness census.
    if !is_disabled("vet") {
        header(opts, "go vet ./...");
        let mut cmd = crate::tools::go_command(root);
        cmd.args(["vet", "./..."]);
        let ok = run_tool_step(cmd, "install go >= 1.24 and put it on PATH");
        if !record(&mut outcomes, "vet", ok) && !opts.keep_going {
            bail!("floor: `vet` failed");
        }
    }

    // 3. Tests — per-module `go test` (build + run in one verb; the
    // compile IS the first half of the signal).
    if !is_disabled("tests") {
        header(opts, "go test ./...");
        let mut cmd = crate::tools::go_command(root);
        cmd.args(["test", "./..."]);
        let ok = run_tool_step(cmd, "install go >= 1.24 and put it on PATH");
        if !record(&mut outcomes, "tests", ok) && !opts.keep_going {
            bail!("floor: `tests` failed");
        }
    }

    // 4. The evidence providers: staticcheck + the exhaustive linter
    // (the one Discipline rule a linter carries entirely — GUIDE §5).
    if !is_disabled("staticcheck") {
        header(opts, "staticcheck ./... && exhaustive ./...");
        let sc = run_tool_step(
            {
                let mut cmd = crate::tools::path_tool(root, "staticcheck");
                cmd.arg("./...");
                cmd
            },
            "go install honnef.co/go/tools/cmd/staticcheck@latest (or disable the \
             step with a reason in conform.toml [go].floor_disable)",
        );
        let ex = run_tool_step(
            {
                let mut cmd = crate::tools::path_tool(root, "exhaustive");
                cmd.arg("./...");
                cmd
            },
            "go install github.com/nishanths/exhaustive/cmd/exhaustive@latest (or \
             disable the step with a reason in conform.toml [go].floor_disable)",
        );
        if !record(&mut outcomes, "staticcheck", sc && ex) && !opts.keep_going {
            bail!("floor: `staticcheck` failed");
        }
    }

    // 5. The conform gate (the go-extract structural rules).
    if !is_disabled("conform") {
        header(opts, "go-ai-native-conform check");
        let ok = go_ai_native_conform::run_check(
            root,
            go_ai_native_conform::DEFAULT_GO_BASELINE,
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
        header(opts, "go-ai-native-specmap --check");
        let ok = go_ai_native_specmap::run_specmap_go(root, true)
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
    // (same condition as the sibling floors).
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
                 `go-ai-native init` writes it",
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
