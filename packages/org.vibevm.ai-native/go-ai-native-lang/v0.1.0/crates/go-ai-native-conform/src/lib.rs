//! The `go-ai-native-conform` gate driver (the Go twin of
//! `rust-ai-native-conform` / `typescript-ai-native-conform`): load the
//! project's `conform.toml`, build the Go rule set from its `[go]`
//! table, extract through the `go-extract` frontend, and gate new
//! findings against the Go ratchet baseline.
//!
//! Same engine, same SARIF, same baseline mechanics as the sibling
//! gates — only the fact source and the rule subset differ (the
//! conform-frontend-go brief's "one rule engine, one finding grammar,
//! one ratchet baseline" promise). The baseline FILE is separate
//! (`go-ai-native-conform-baseline.json`) because `freeze` rewrites a
//! whole file and the gates must not clobber each other's frozen sets.

specmark::scope!("spec://go-ai-native-lang/go/tools/conform-frontend-go#division");

use std::path::Path;

use anyhow::{Context, Result, bail};
use conform_core::{Config, Rule, rules};
use go_ai_native_conform_frontend::GoExtractFrontend;

/// The default Go baseline path, root-relative.
pub const DEFAULT_GO_BASELINE: &str = "go-ai-native-conform-baseline.json";

fn load_config(root: &Path) -> Result<Config> {
    let (cfg, origin) = Config::load_or_default(root)?;
    match origin {
        conform_core::ConfigOrigin::Loaded => {
            eprintln!("go-ai-native-conform: policy conform.toml (loaded).");
        }
        conform_core::ConfigOrigin::Defaulted => eprintln!(
            "go-ai-native-conform: NO conform.toml — topology default in force \
             (roots = [\".\"], no cells gate); run `go-ai-native init` \
             to write a starting policy."
        ),
    }
    Ok(cfg)
}

/// The standing Go rule set, built from the policy in ONE place so
/// `run_check`, `run_freeze`, and the agentic oracle's enrichment
/// layer (`go-ai-native-tcg`, TCG-PROTOCOL-GO §3) cannot drift apart —
/// the gate and the oracle answer from the same rules.
///
/// ```
/// let (config, _) =
///     conform_core::Config::load_or_default(std::path::Path::new(".")).unwrap();
/// let rules = go_ai_native_conform::build_rules(&config);
/// assert!(!rules.is_empty());
/// ```
pub fn build_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    let mut out: Vec<Box<dyn Rule>> = Vec::new();
    out.push(Box::new(rules::GoUnsafeInDomain::new(
        config.go.cells_dir.as_deref(),
    )));
    // The dedicated seam-error rule (B-033) — the two halves that used to
    // ride the `go-unsafe-in-domain` umbrella now have their own id.
    // Always on: a seam error cites its REQ regardless of the cell layout.
    out.push(Box::new(rules::GoSeamErrorCitesReq));
    if let Some(cells_dir) = &config.go.cells_dir {
        out.push(Box::new(rules::GoCellIsolation::new(cells_dir)));
        // B-030: a gated cell carries the loud-conformance assertion
        // `var _ Seam = (*Impl)(nil)`; the extractor emits it (S2a) and
        // this fires for a gated cell that declares none. Conditional on
        // cells_dir, scoped to the gate list (exempt/ungated cells are out).
        out.push(Box::new(rules::GoConformanceAssertion::new(
            config.go.cells_dir.as_deref(),
            &config.go.gated,
        )));
    }
    out.push(Box::new(rules::FileLength {
        max_lines: config.max_file_lines,
    }));
    out.push(Box::new(rules::InvariantCommentPosition {
        markers: config.invariant_comment_markers.clone(),
        min_lines: config.invariant_comment_min_file_lines,
    }));
    out
}

fn extract(root: &Path, config: &Config) -> Result<Vec<conform_core::SourceFacts>> {
    use conform_core::{ExtractionLog, Store};
    let frontend = GoExtractFrontend::new(root)?;
    // Fail HARD on a broken toolchain before the gate can run on zero
    // facts — the bridge's taxonomy carries the fix surface.
    frontend
        .probe()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let store = Store::for_go(root, config);
    let mut log = ExtractionLog::default();
    let facts = store.extract_go(root, &frontend, &mut log)?;
    eprintln!(
        "go-ai-native-conform: extracted {} file(s), {} cached (producer go-extract-2).",
        log.extracted.len(),
        log.cached,
    );
    Ok(facts)
}

/// Announce the Go coverage posture after extraction — the sharper
/// empty-scope guard (a configured `[go]` scope that enumerated zero
/// packages warns loudly instead of passing silently) and the
/// vacuous-gate warning (a gated package the scan attributed no sources
/// to). Printed in both `run_check` and `run_freeze`, exactly where
/// Rust's `warn_vacuously_gated` sits; the count summary lives in
/// `run_check` alone (parity with the Rust driver).
fn announce_go_coverage(root: &Path, config: &Config) {
    let units = conform_core::go_units(root, &config.go);
    for w in conform_core::go_scope_warnings(&units, &config.go) {
        eprintln!("{w}");
    }
    for pkg in conform_core::go_vacuously_gated(&config.go.gated, &units) {
        eprintln!(
            "go-ai-native-conform: WARNING — gated package `{pkg}` matched no scanned sources; \
             its gates are green by vacuity. Point `roots` in conform.toml at the package dir \
             (a literal entry) or its parent (`<dir>/*`), or drop it from `[go] gated`."
        );
    }
}

/// Run the Go gate at `root` against `baseline_rel`; SARIF lands at
/// `target/conform/report-go.sarif`; any new finding fails.
pub fn run_check(root: &Path, baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    use conform_core::{baseline, check, count_by_rule, sarif};
    let config = load_config(root)?;
    config.validate_go_against_tree(root)?;
    let facts = extract(root, &config)?;
    announce_go_coverage(root, &config);
    let owned = build_rules(&config);
    let rule_refs: Vec<&dyn Rule> = owned.iter().map(|r| r.as_ref()).collect();

    let findings = check(&rule_refs, &facts, scope);
    let report = sarif::render(&rule_refs, &findings);
    let sarif_path = root.join("target").join("conform").join("report-go.sarif");
    if let Some(parent) = sarif_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&sarif_path, &report)?;

    let base = baseline::load(&root.join(baseline_rel))?;
    let (new, stale) = baseline::diff(&base, &findings);
    for f in &new {
        eprintln!(
            "  go-ai-native-conform: NEW {} {}:{} — {}",
            f.rule, f.file, f.line, f.message
        );
    }
    for fp in &stale {
        eprintln!("  go-ai-native-conform: baseline entry no longer fires — prune it: {fp}");
    }
    let counts = count_by_rule(&findings);
    eprintln!(
        "go-ai-native-conform check: {} finding(s) in scope {} ({:?}), {} frozen in baseline, {} new; SARIF at {}.",
        findings.len(),
        scope.unwrap_or("<workspace>"),
        counts,
        base.findings.len(),
        new.len(),
        sarif_path
            .strip_prefix(root)
            .unwrap_or(&sarif_path)
            .display()
    );
    eprintln!(
        "go-ai-native-conform: {} package(s) gated, {} exempt — see conform.toml for the why of each.",
        config.go.gated.len(),
        config.go.exempt.len(),
    );
    if !new.is_empty() {
        bail!(
            "go-ai-native-conform: {} new finding(s) against the baseline",
            new.len()
        );
    }
    Ok(())
}

/// Rewrite the Go baseline to the current finding set (the same two
/// legal moments as the sibling gates: a new rule landing, and a
/// re-freeze after the set shrank).
pub fn run_freeze(root: &Path, baseline_rel: &str) -> Result<()> {
    use conform_core::{check, count_by_rule};
    let config = load_config(root)?;
    config.validate_go_against_tree(root)?;
    let facts = extract(root, &config)?;
    announce_go_coverage(root, &config);
    let owned = build_rules(&config);
    let rule_refs: Vec<&dyn Rule> = owned.iter().map(|r| r.as_ref()).collect();
    let findings = check(&rule_refs, &facts, None);
    let counts = count_by_rule(&findings);
    let mut fps: Vec<&str> = findings.iter().map(|f| f.fingerprint.as_str()).collect();
    fps.sort_unstable();
    fps.dedup();
    let body = serde_json::json!({ "schema": 1, "findings": fps });
    let mut text = serde_json::to_string_pretty(&body).context("serialising baseline")?;
    text.push('\n');
    let path = root.join(baseline_rel);
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    eprintln!(
        "go-ai-native-conform freeze: {} fingerprint(s) frozen ({:?}) at {}.",
        fps.len(),
        counts,
        baseline_rel
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The coverage invariant (B-034) refuses an on-disk Go package that
    /// is neither gated nor exempt — the silent-green failure mode the
    /// gate now closes. Pure config + tree: no extraction, so no go
    /// toolchain floor (the `tests/gate.rs` pair carries the end-to-end
    /// half over the committed fixtures).
    #[test]
    fn validate_refuses_an_unclassified_go_package() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::create_dir_all(root.join("internal/cells/hello")).expect("mkdir");
        std::fs::write(
            root.join("internal/cells/hello/hello.go"),
            "// Package hello is the demo cell.\npackage hello\n",
        )
        .expect("go file");
        // roots = ["."], no gated/exempt → the hello package is on disk
        // but unclassified.
        std::fs::write(root.join("conform.toml"), "[go]\nroots = [\".\"]\n").expect("conform");
        let cfg = Config::load(&root.join("conform.toml")).expect("parses");
        let err = cfg.validate_go_against_tree(root).expect_err("must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("internal/cells/hello"),
            "must name the unclassified package: {msg}"
        );
        assert!(
            msg.contains("package") && msg.contains("neither gated nor exempt"),
            "must speak the Go noun and the refusal class: {msg}"
        );

        // Classifying the package (gated OR exempt) clears the refusal.
        std::fs::write(
            root.join("conform.toml"),
            "[go]\nroots = [\".\"]\ngated = [\"internal/cells/hello\"]\n",
        )
        .expect("conform");
        let cfg = Config::load(&root.join("conform.toml")).expect("parses");
        cfg.validate_go_against_tree(root)
            .expect("classified → green");
    }
}
