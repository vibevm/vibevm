//! `go-ai-native-tcg bench` — the differential/latency harness
//! (ORACLE-GO §5, §8): every corpus case runs through the LIVE oracle
//! (warm ×3) and through `go build ./...` in ONE reused scratch
//! materialisation; agreement is EXISTENCE-grain (both red / both
//! green — gopls diagnostic codes are analysis names, go build errors
//! are prose, so a code-level mapping table waits for live evidence),
//! and known-gap cases assert the asymmetry as their expectation.
//! Targets are recorded, never CI-gated; the REPORT is the ratchet.

specmark::scope!("spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#fidelity");

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{Policy, enrich_validate, spawn_oracle};

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    file: String,
    /// Corpus-dir-relative content file; absent = the disk state.
    #[serde(default)]
    content_from: Option<String>,
    expect: Expect,
}

#[derive(Debug, Deserialize)]
struct Expect {
    /// `go build ./...` on the scratch copy must be red.
    #[serde(default)]
    build_red: bool,
    /// Substrings at least one oracle diagnostic message must carry.
    #[serde(default)]
    oracle_message_contains: Vec<String>,
    /// Non-baselined conform rules the enrichment must surface.
    #[serde(default)]
    conform_rules: Vec<String>,
    /// The documented-gap posture: the oracle stays SILENT (no
    /// error-grade diagnostic) while the build speaks — asserted as
    /// the pass condition (ORACLE-GO §5).
    #[serde(default)]
    known_gap: bool,
}

#[derive(Debug, serde::Serialize)]
struct CaseOutcome {
    name: String,
    pass: bool,
    known_gap: bool,
    oracle_error: bool,
    build_red: bool,
    conform_rules: Vec<String>,
    warm_ms: Vec<u128>,
    detail: String,
}

/// Copy the module into scratch (skipping build output, slots, VCS) so
/// every case's `go build` shares one module cache.
fn materialise(root: &Path, dst: &Path) -> Result<()> {
    const SKIP: &[&str] = &["target", "vibedeps", ".git", ".vibe", "node_modules"];
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if SKIP
                .iter()
                .any(|s| rel == *s || rel.starts_with(&format!("{s}/")))
            {
                continue;
            }
            let to = dst.join(&rel);
            if path.is_dir() {
                std::fs::create_dir_all(&to)?;
                stack.push(path);
            } else {
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&path, &to)?;
            }
        }
    }
    Ok(())
}

fn go_build_red(scratch: &Path) -> Result<bool> {
    let out = std::process::Command::new(go_ai_native_extract_bridge::go_binary())
        .args(["build", "./..."])
        .current_dir(scratch)
        .output()
        .context("spawning go build")?;
    Ok(!out.status.success())
}

pub fn run_bench(root: &Path, corpus_rel: &str, report_rel: &str) -> Result<()> {
    let corpus_path = root.join(corpus_rel);
    let corpus_dir = corpus_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.to_path_buf());
    let corpus_text = std::fs::read_to_string(&corpus_path)
        .with_context(|| format!("reading {}", corpus_path.display()))?;
    let cases: Vec<Case> = serde_json::from_str(&corpus_text)
        .with_context(|| format!("parsing {}", corpus_path.display()))?;
    if cases.is_empty() {
        bail!("bench: the corpus carries no cases");
    }

    let policy = Policy::load(root)?;
    let mut oracle = spawn_oracle(root)?;
    let scratch = tempfile::tempdir().context("scratch dir")?;

    let mut outcomes: Vec<CaseOutcome> = Vec::new();
    for case in &cases {
        let text = match &case.content_from {
            Some(rel) => std::fs::read_to_string(corpus_dir.join(rel))
                .with_context(|| format!("reading corpus content {rel}"))?,
            None => std::fs::read_to_string(root.join(&case.file))
                .with_context(|| format!("reading {}", case.file))?,
        };

        // Oracle: warm ×3, keep the last enriched answer.
        let mut warm_ms = Vec::new();
        let mut last = None;
        for _ in 0..3 {
            let started = Instant::now();
            let raw = oracle
                .validate(&case.file, Some(text.clone()))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            warm_ms.push(started.elapsed().as_millis());
            last = Some(raw);
        }
        let enriched = enrich_validate(
            &policy,
            &case.file,
            &text,
            last.expect("three validates ran"),
        );
        let oracle_error = enriched.diagnostics.iter().any(|d| d.category == "error");
        let rules: Vec<String> = enriched
            .conform_findings
            .iter()
            .filter(|f| !f.baselined)
            .map(|f| f.rule.clone())
            .collect();

        // The floor's verdict on the scratch copy carrying this case's
        // content.
        let dst = scratch.path().join(&case.name);
        materialise(root, &dst)?;
        let target = dst.join(&case.file);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, &text)?;
        let build_red = go_build_red(&dst)?;

        let mut detail = String::new();
        let pass = if case.expect.known_gap {
            // The documented gap: the build is red, the oracle silent.
            let ok = build_red && !oracle_error;
            if !ok {
                detail = format!(
                    "known-gap expected (build red + oracle silent); got build_red={build_red}, \
                     oracle_error={oracle_error} — the gap may have closed: promote the case"
                );
            }
            ok
        } else {
            let existence_ok = build_red == case.expect.build_red
                && oracle_error == case.expect.build_red;
            let messages_ok = case.expect.oracle_message_contains.iter().all(|needle| {
                enriched
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains(needle.as_str()))
            });
            let rules_ok = case
                .expect
                .conform_rules
                .iter()
                .all(|r| rules.iter().any(|have| have == r));
            if !existence_ok {
                detail = format!(
                    "existence mismatch: build_red={build_red}, oracle_error={oracle_error}, \
                     expected build_red={}",
                    case.expect.build_red
                );
            } else if !messages_ok {
                detail = "an expected oracle message substring is missing".to_string();
            } else if !rules_ok {
                detail = format!("expected conform rules {:?}, got {rules:?}", case.expect.conform_rules);
            }
            existence_ok && messages_ok && rules_ok
        };

        eprintln!(
            "bench: {} — {} (oracle_error={oracle_error}, build_red={build_red}, warm {:?} ms)",
            case.name,
            if pass { "PASS" } else { "FAIL" },
            warm_ms,
        );
        outcomes.push(CaseOutcome {
            name: case.name.clone(),
            pass,
            known_gap: case.expect.known_gap,
            oracle_error,
            build_red,
            conform_rules: rules,
            warm_ms,
            detail,
        });
    }
    let _ = oracle.shutdown();

    let failed = outcomes.iter().filter(|o| !o.pass).count();
    let report = serde_json::json!({
        "schema": 1,
        "harness": "go-ai-native-tcg bench",
        "cases": outcomes,
        "failed": failed,
    });
    let report_path = root.join(report_rel);
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    std::fs::write(&report_path, text)
        .with_context(|| format!("writing {}", report_path.display()))?;
    eprintln!(
        "bench: {} case(s), {failed} failed; report at {report_rel}.",
        outcomes.len()
    );
    if failed > 0 {
        bail!("bench: {failed} case(s) failed");
    }
    Ok(())
}
