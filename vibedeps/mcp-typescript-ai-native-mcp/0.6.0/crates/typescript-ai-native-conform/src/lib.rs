//! The `typescript-ai-native-conform` gate driver (the TS twin of `conform-cli`):
//! load the project's `conform.toml`, build the TypeScript rule set
//! from its `[typescript]` table, extract through the `ts-tsc`
//! frontend, and gate new findings against the TS ratchet baseline.
//!
//! Same engine, same SARIF, same baseline mechanics as the Rust gate —
//! only the fact source and the rule subset differ (the Ф6 brief's "one
//! rule engine, one finding grammar, one ratchet baseline" promise).
//! The baseline FILE is separate (`typescript-ai-native-conform-baseline.json`)
//! because `freeze` rewrites a whole file and the two gates must not
//! clobber each other's frozen sets.

use std::path::Path;

use anyhow::{Context, Result, bail};
use conform_core::{Config, Rule, rules};
use typescript_ai_native_conform_frontend::TsTscFrontend;

/// The default TS baseline path, root-relative.
pub const DEFAULT_TS_BASELINE: &str = "typescript-ai-native-conform-baseline.json";

fn load_config(root: &Path) -> Result<Config> {
    let (cfg, origin) = Config::load_or_default(root)?;
    match origin {
        conform_core::ConfigOrigin::Loaded => {
            eprintln!("typescript-ai-native-conform: policy conform.toml (loaded).");
        }
        conform_core::ConfigOrigin::Defaulted => eprintln!(
            "typescript-ai-native-conform: NO conform.toml — topology default in force \
             (roots = [\"src\"], no cells gate); run `typescript-ai-native init` \
             to write a starting policy."
        ),
    }
    Ok(cfg)
}

/// The standing TypeScript rule set, built from the policy in ONE place
/// so `run_check`, `run_freeze`, and the agentic oracle's enrichment
/// layer (`tcg-cli-typescript`, TCG-PROTOCOL-v0.1 §3) cannot drift
/// apart — the gate and the oracle answer from the same rules.
///
/// ```
/// let (config, _) =
///     conform_core::Config::load_or_default(std::path::Path::new(".")).unwrap();
/// let rules = typescript_ai_native_conform::build_rules(&config);
/// assert!(!rules.is_empty());
/// ```
pub fn build_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    let mut out: Vec<Box<dyn Rule>> = Vec::new();
    out.push(Box::new(rules::TsUnsafeInDomain));
    if let Some(cells_dir) = &config.typescript.cells_dir {
        out.push(Box::new(rules::TsCellIsolation::new(
            cells_dir,
            &config.typescript.seam,
        )));
    }
    out.push(Box::new(rules::FileLength {
        max_lines: config.max_file_lines,
    }));
    out
}

fn extract(root: &Path, config: &Config) -> Result<Vec<conform_core::SourceFacts>> {
    use conform_core::{ExtractionLog, Store};
    let frontend = TsTscFrontend::new(root)?;
    // Fail HARD on a broken toolchain before the gate can run on zero
    // facts — the bridge's taxonomy carries the fix surface.
    frontend
        .probe()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let store = Store::for_typescript(root, config);
    let mut log = ExtractionLog::default();
    let facts = store.extract_typescript(root, &frontend, &mut log)?;
    eprintln!(
        "typescript-ai-native-conform: extracted {} file(s), {} cached (producer ts-tsc-1).",
        log.extracted.len(),
        log.cached,
    );
    Ok(facts)
}

/// Run the TS gate at `root` against `baseline_rel`; SARIF lands at
/// `target/conform/report-typescript.sarif`; any new finding fails.
pub fn run_check(root: &Path, baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    use conform_core::{baseline, check, count_by_rule, sarif};
    let config = load_config(root)?;
    let facts = extract(root, &config)?;
    let owned = build_rules(&config);
    let rule_refs: Vec<&dyn Rule> = owned.iter().map(|r| r.as_ref()).collect();

    let findings = check(&rule_refs, &facts, scope);
    let report = sarif::render(&rule_refs, &findings);
    let sarif_path = root
        .join("target")
        .join("conform")
        .join("report-typescript.sarif");
    if let Some(parent) = sarif_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&sarif_path, &report)?;

    let base = baseline::load(&root.join(baseline_rel))?;
    let (new, stale) = baseline::diff(&base, &findings);
    for f in &new {
        eprintln!(
            "  typescript-ai-native-conform: NEW {} {}:{} — {}",
            f.rule, f.file, f.line, f.message
        );
    }
    for fp in &stale {
        eprintln!(
            "  typescript-ai-native-conform: baseline entry no longer fires — prune it: {fp}"
        );
    }
    let counts = count_by_rule(&findings);
    eprintln!(
        "typescript-ai-native-conform check: {} finding(s) in scope {} ({:?}), {} frozen in baseline, {} new; SARIF at {}.",
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
    if !new.is_empty() {
        bail!(
            "typescript-ai-native-conform: {} new finding(s) against the baseline",
            new.len()
        );
    }
    Ok(())
}

/// Rewrite the TS baseline to the current finding set (the same two
/// legal moments as the Rust gate: a new rule landing, and a re-freeze
/// after the set shrank).
pub fn run_freeze(root: &Path, baseline_rel: &str) -> Result<()> {
    use conform_core::{check, count_by_rule};
    let config = load_config(root)?;
    let facts = extract(root, &config)?;
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
        "typescript-ai-native-conform freeze: {} fingerprint(s) frozen ({:?}) at {}.",
        fps.len(),
        counts,
        baseline_rel
    );
    Ok(())
}
