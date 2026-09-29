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

mod perimeter;

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

/// The files `gofmt -l .` reported as unformatted, after dropping the
/// `[go].exclude_substrings` entries — the conform engine's own skip
/// (`store.rs::go_sources`): normalise `\` → `/`, then `String::contains`
/// against each exclude. gofmt on Windows prints back-slashed paths, so
/// the separator is normalised before the match while the original line
/// is kept verbatim for the `unformatted:` print. Pure (no I/O), so the
/// floor's exclusion is unit-tested in isolation below.
fn filter_gofmt_listed(raw: &str, excludes: &[String]) -> Vec<String> {
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| {
            let norm = l.replace('\\', "/");
            !excludes.iter().any(|s| norm.contains(s.as_str()))
        })
        .map(str::to_owned)
        .collect()
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

    // The floor's shared package perimeter: `go list ./...` once, then the
    // same `[go].exclude_substrings` filter gofmt applies to files — so
    // vet/test/staticcheck/exhaustive judge the identical boundary. Lazy:
    // nothing spawns `go list` when every consuming step is disabled.
    let needs_perimeter =
        !is_disabled("vet") || !is_disabled("tests") || !is_disabled("staticcheck");
    let perimeter =
        perimeter::resolve_perimeter(root, &config.go.exclude_substrings, needs_perimeter);

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
                let dirty = filter_gofmt_listed(&listed, &config.go.exclude_substrings);
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

    // 2. Vet — the toolchain's own correctness census, over the SAME
    // perimeter gofmt judged: the fixtures kept deliberately wrong stay out
    // of `go vet` exactly as they stay out of `gofmt -l`.
    if !is_disabled("vet") {
        header(
            opts,
            &format!(
                "go vet (floor perimeter: {})",
                perimeter::perimeter_scope(perimeter.as_ref())
            ),
        );
        if let Some(ok) = perimeter::run_perimeter_step(perimeter.as_ref(), "vet", "vet", |pkgs| {
            let mut cmd = crate::tools::go_command(root);
            cmd.arg("vet");
            perimeter::add_packages_or_wildcard(&mut cmd, pkgs);
            run_tool_step(cmd, "install go >= 1.24 and put it on PATH")
        }) && !record(&mut outcomes, "vet", ok)
            && !opts.keep_going
        {
            bail!("floor: `vet` failed");
        }
    }

    // 3. Tests — per-module `go test` (build + run in one verb; the
    // compile IS the first half of the signal), over the same perimeter.
    if !is_disabled("tests") {
        header(
            opts,
            &format!(
                "go test (floor perimeter: {})",
                perimeter::perimeter_scope(perimeter.as_ref())
            ),
        );
        if let Some(ok) =
            perimeter::run_perimeter_step(perimeter.as_ref(), "tests", "test", |pkgs| {
                let mut cmd = crate::tools::go_command(root);
                cmd.arg("test");
                perimeter::add_packages_or_wildcard(&mut cmd, pkgs);
                run_tool_step(cmd, "install go >= 1.24 and put it on PATH")
            })
            && !record(&mut outcomes, "tests", ok)
            && !opts.keep_going
        {
            bail!("floor: `tests` failed");
        }
    }

    // 4. The evidence providers: staticcheck + the exhaustive linter (the
    // one Discipline rule a linter carries entirely — GUIDE §5), both over
    // the same perimeter.
    if !is_disabled("staticcheck") {
        header(
            opts,
            &format!(
                "staticcheck + exhaustive (floor perimeter: {})",
                perimeter::perimeter_scope(perimeter.as_ref())
            ),
        );
        if let Some(ok) =
            perimeter::run_perimeter_step(perimeter.as_ref(), "staticcheck", "lint", |pkgs| {
                let sc = run_tool_step(
                    {
                        let mut cmd = crate::tools::path_tool(root, "staticcheck");
                        perimeter::add_packages_or_wildcard(&mut cmd, pkgs);
                        cmd
                    },
                    "go install honnef.co/go/tools/cmd/staticcheck@latest (or disable the \
                 step with a reason in conform.toml [go].floor_disable)",
                );
                let ex = run_tool_step(
                    {
                        let mut cmd = crate::tools::path_tool(root, "exhaustive");
                        perimeter::add_packages_or_wildcard(&mut cmd, pkgs);
                        cmd
                    },
                    "go install github.com/nishanths/exhaustive/cmd/exhaustive@latest (or \
                 disable the step with a reason in conform.toml [go].floor_disable)",
                );
                sc && ex
            })
            && !record(&mut outcomes, "staticcheck", ok)
            && !opts.keep_going
        {
            bail!("floor: `staticcheck` failed");
        }
    }

    // 5. The conform gate (the go-extract structural rules).
    if !is_disabled("conform") {
        header(opts, "go-ai-native-conform check");
        let ok =
            go_ai_native_conform::run_check(root, go_ai_native_conform::DEFAULT_GO_BASELINE, None)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The Go default `[go].exclude_substrings`, as `run_floor` sees it —
    /// kept here as a fixture so the filter tests track the real policy.
    fn go_excludes() -> Vec<String> {
        vec![
            "/testdata/".to_string(),
            "/vendor/".to_string(),
            "/fixtures/".to_string(),
        ]
    }

    /// (а) a file under `/fixtures/` is dropped — on both POSIX and the
    /// back-slashed Windows form gofmt prints (B-003's exact symptom).
    #[test]
    fn fixture_files_are_dropped_posix_and_windows() {
        let posix = "tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go\n";
        assert!(filter_gofmt_listed(posix, &go_excludes()).is_empty());

        // gofmt on Windows prints back-slashed paths; the match must
        // normalise `\` → `/` before applying the exclude (store.rs:441).
        let windows = "tools\\go-extract\\test\\fixtures\\dirty\\plan.go\n";
        assert!(filter_gofmt_listed(windows, &go_excludes()).is_empty());
    }

    /// (б) an ordinary source file passes through untouched.
    #[test]
    fn ordinary_files_pass_through() {
        let raw = "internal/cells/plan/plan.go\ninternal/registry/registry.go\n";
        assert_eq!(
            filter_gofmt_listed(raw, &go_excludes()),
            vec![
                "internal/cells/plan/plan.go".to_string(),
                "internal/registry/registry.go".to_string(),
            ]
        );
    }

    /// (в) non-empty raw output that is entirely excluded yields an empty
    /// list — the gofmt step goes green, the fixtures never print as
    /// unformatted.
    #[test]
    fn all_excluded_yields_empty_so_the_step_is_green() {
        let raw = "tools/go-extract/test/fixtures/dirty/a.go\n\
                   tools/go-extract/test/fixtures/clean/b.go\n";
        let got = filter_gofmt_listed(raw, &go_excludes());
        assert!(got.is_empty(), "expected no unformatted files, got {got:?}");
    }

    /// Blank/whitespace-only lines in gofmt's output are dropped (the
    /// floor never printed them before, and must not start now).
    #[test]
    fn blank_lines_are_dropped() {
        let raw = "\ninternal/cells/plan/plan.go\n\n   \n";
        assert_eq!(
            filter_gofmt_listed(raw, &go_excludes()),
            vec!["internal/cells/plan/plan.go".to_string()]
        );
    }
}
