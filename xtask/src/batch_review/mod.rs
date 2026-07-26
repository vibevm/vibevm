//! `cargo xtask batch-review` — the mechanical half of a Phase-B markup review.
//!
//! A markup batch (PROP-043 wave-2, Phase B) arrives as a diff of N markdown
//! files. Reviewing it has two halves: a mechanical one (did the words survive,
//! did the gate move by the predicted amount, is anything outside scope) and a
//! judgement one (is the split sense-preserving, is the anchor name good, is an
//! `@unknown` honest). **This is the first half only**, and its output ends
//! with the list of what it did not check — that list is the actual review.
//!
//! # Why this does not call `progress-core`
//!
//! It would be one import away. It is deliberately not taken: the value of this
//! tool is that it is a **second opinion**, and a cross-check that shares the
//! instrument's bugs is not a cross-check. This campaign has found the real
//! parser wrong three times (F-083 a unit its own grammar allows, F-084 a
//! marker swallowed beside a quoted fence, F-085 the URI grammar). A checker
//! built on it would have agreed with it every time.
//!
//! The cost is that the scanning here is hand-rolled and approximate, and
//! every approximation is named at its function. The rule is: an approximation
//! may only ever ADMIT a candidate for checking, never silently suppress one.
//!
//! # Why there is no `regex` dependency
//!
//! Every bug the first (Python) implementation shipped lived in a regex that
//! approximated a spec rule instead of reading it — a bullet stripper that ate
//! a wrapped `+` from prose, a shorthand pattern that matched `@ts-ignore`, a
//! heading test that also matched `##ANCHOR`. Hand-rolled scanning is longer
//! and it is inspectable line by line, which is what this tool trades in.
//!
//! # Calibration
//!
//! The negative controls are `#[test]`s, so the floor runs them on every
//! commit rather than when someone remembers a flag. `--selftest` additionally
//! replays landed batches out of git history, which the hermetic tests cannot.

mod checks;
mod index;
mod report;
mod text;

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use checks::*;
use index::c11_task_index;
use report::Report;
use text::word_stream;

// ---------------------------------------------------------------- plumbing
fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn window(v: &[String], at: usize) -> String {
    let lo = at.saturating_sub(6);
    let hi = (at + 6).min(v.len());
    v[lo..hi].join(" ")
}

fn git_show(root: &Path, rev: &str, path: &str) -> Result<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(["show", &format!("{rev}:{path}")])
        .output()
        .context("git show")?;
    if !out.status.success() {
        anyhow::bail!("git show {rev}:{path} failed");
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn git_lines(root: &Path, args: &[&str]) -> Result<Vec<String>> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .context("git")?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

fn read_list(p: &Path) -> Result<Vec<String>> {
    Ok(std::fs::read_to_string(p)
        .with_context(|| format!("reading {}", p.display()))?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

/// Arguments for one review run.
pub struct BatchReviewArgs {
    pub base: String,
    pub commit: Option<String>,
    pub gate_log: Option<std::path::PathBuf>,
    pub scope: Option<std::path::PathBuf>,
    pub expect_unmarked: Option<usize>,
    pub expect_residual: Option<std::path::PathBuf>,
    pub expect_total: Option<usize>,
    pub campaign: Option<std::path::PathBuf>,
    pub selftest: bool,
}

pub fn run_batch_review(root: &Path, a: BatchReviewArgs) -> Result<()> {
    if a.selftest {
        return selftest(root);
    }

    let (base, files) = match &a.commit {
        Some(c) => {
            let base = format!("{c}~1");
            let files = git_lines(root, &["diff", "--name-only", &base, c])?;
            (base, files)
        }
        None => {
            let files = git_lines(root, &["diff", "--name-only", &a.base])?;
            (a.base.clone(), files)
        }
    };
    let files: Vec<String> = files.into_iter().filter(|f| f.ends_with(".md")).collect();
    if files.is_empty() {
        println!("no markdown files changed -- nothing to review");
        return Ok(());
    }
    if a.commit.is_some() {
        println!("NOTE: --commit reviews a landed batch; C3 reads the worktree, so this is");
        println!("      only meaningful when the worktree still matches that commit.");
    }

    let scope = a.scope.as_deref().map(read_list).transpose()?;
    let residual = a.expect_residual.as_deref().map(read_list).transpose()?;
    let gate = a
        .gate_log
        .as_deref()
        .map(|p| std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display())))
        .transpose()?;
    if gate.is_none() {
        println!("NOTE: no --gate-log; C4 and C5 skipped. Run the gate and pass its output.");
    }

    let mut r = Report::default();
    c1_scope(&files, scope.as_ref(), &mut r);
    c2_lazy_continuation(&files, Some(&base), root, &mut r);
    c3_words(&files, &base, root, &mut r);
    if let Some(g) = &gate {
        c4_gate(
            g,
            &files,
            a.expect_unmarked,
            residual.as_ref(),
            a.expect_total,
            &mut r,
        );
        c5_error_classes(g, &files, &mut r);
    }
    c6_vocabulary(&files, root, &mut r);
    c7_anchors(&files, root, &mut r);
    c8_encoding(&files, root, &mut r);
    c9_markers_in_fences(&files, root, &mut r);
    c10_unknowns(&files, root, &mut r);
    if let Some(zone) = &a.campaign {
        c11_task_index(&root.join(zone), &mut r);
    }

    r.emit();
    if r.failed() {
        anyhow::bail!("mechanical checks failed");
    }
    Ok(())
}

/// Replay landed batches out of git history.
///
/// The negative controls live in `#[test]`s so the floor runs them; this half
/// needs real history and so cannot.
fn selftest(root: &Path) -> Result<()> {
    let mut failures = 0usize;
    println!("=== calibration: landed batches must come back clean ===");
    for (name, commit) in [("B5 go", "d3242f99"), ("B6 typescript", "12e12d4c")] {
        let base = format!("{commit}~1");
        let files: Vec<String> = git_lines(root, &["diff", "--name-only", &base, commit])?
            .into_iter()
            .filter(|f| f.ends_with(".md"))
            .collect();
        if files.is_empty() {
            println!("  SKIP {name}: commit {commit} not in this history");
            continue;
        }
        let diverged: Vec<&String> = files
            .iter()
            .filter(|f| {
                let (a, b) = (
                    git_show(root, &base, f).map(|t| word_stream(&t)),
                    git_show(root, commit, f).map(|t| word_stream(&t)),
                );
                matches!((a, b), (Ok(x), Ok(y)) if x != y)
            })
            .collect();
        if diverged.is_empty() {
            println!(
                "  ok   {name}: {} files word-identical across the batch",
                files.len()
            );
        } else {
            println!("  FAIL {name}: word stream diverges in {diverged:?}");
            failures += 1;
        }
    }
    println!(
        "\n{}",
        if failures == 0 {
            "calibration clean -- the tool may be trusted (negative controls run in `cargo test`)"
        } else {
            "calibration FAILED"
        }
    );
    if failures > 0 {
        anyhow::bail!("calibration failed");
    }
    Ok(())
}
