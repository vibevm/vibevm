//! The conform engine driver (ENGINE-CONFORM §5): load the project's
//! `conform.toml`, build the standing rule set from it, and run the gate
//! (`run_check`) or rewrite the ratchet baseline (`run_freeze`) over a
//! project tree.
//!
//! This was extracted out of vibevm's `xtask` so the rust-ai-native package
//! ships a *runnable* engine, not a description of one (PROP-024 code-bearing
//! packages). vibevm's `cargo xtask conform` is now a thin shim over this
//! library, and the `conform` binary (`src/main.rs`) is what an installed
//! consumer runs in its own project. The policy is data (`conform.toml`),
//! never hardcoded here — the same engine runs on any layout (PROP-024 §2.2).

use std::path::Path;

use anyhow::{Context, Result, bail};
use conform_core::{Config, Rule, rules};

/// Load the project's conform policy (`conform.toml` at the tree root).
pub fn load_config(root: &Path) -> Result<Config> {
    Config::load(&root.join("conform.toml"))
}

/// Build the standing rule set from the policy, in one place so `run_check`
/// and `run_freeze` can never drift apart. The order is the SARIF driver
/// order the gate has always rendered.
fn build_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    let mut out: Vec<Box<dyn Rule>> = Vec::new();
    if let (Some(reg_file), Some(reg_crate)) = (
        config.registry_file.as_ref(),
        config.registry_gated_crate.as_ref(),
    ) {
        out.push(Box::new(rules::FlagSites {
            registry_file: reg_file.clone(),
            gated_crate: reg_crate.clone(),
        }));
    }
    out.push(Box::new(rules::CellIsolation));
    out.push(Box::new(rules::UnsafeGate {
        audit_crates: config.audit_crates.clone(),
    }));
    out.push(Box::new(rules::SeamHasDoctest {
        gated_crates: config.gated_crates.clone(),
    }));
    out.push(Box::new(rules::PubDoctest {
        gated_crates: config.gated_pub_doctest.clone(),
    }));
    out.push(Box::new(rules::ErrorEnumCitesReq {
        gated_crates: config.gated_crates.clone(),
    }));
    out.push(Box::new(rules::CellHasOracle));
    out.push(Box::new(rules::ErrorMessageCitesReq {
        gated_crates: config.gated_crates.clone(),
    }));
    out.push(Box::new(rules::FileLength {
        max_lines: config.max_file_lines,
    }));
    out.push(Box::new(rules::NoUnwrapInDomain {
        gated_crates: config.gated_crates.clone(),
    }));
    out.push(Box::new(rules::AmbientEnv {
        gated_crates: config.gated_crates.clone(),
        audit_crates: config.audit_crates.clone(),
        roots: config.env_roots.clone(),
    }));
    out
}

/// Run the conform gate over the tree at `root`, against the ratchet
/// baseline at `baseline_rel` (a `root`-relative path), optionally scoped
/// to one crate by name. Writes a SARIF report under `root/target/conform`
/// and errors on any new finding past the baseline.
pub fn run_check(root: &Path, baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    use conform_core::{ExtractionLog, Frontend, Store, baseline, check, count_by_rule, sarif};
    use conform_frontend_rust::RustFrontend;

    let config = load_config(root)?;
    let store = Store::at_repo(root, &config);
    let mut log = ExtractionLog::default();
    let frontend = RustFrontend;
    let facts = store.extract_workspace(root, &frontend, &mut log)?;
    eprintln!(
        "conform: extracted {} file(s), {} cached (producer {}-{}).",
        log.extracted.len(),
        log.cached,
        Frontend::id(&frontend),
        Frontend::version(&frontend),
    );

    let owned = build_rules(&config);
    let rule_refs: Vec<&dyn Rule> = owned.iter().map(|r| r.as_ref()).collect();

    let findings = check(&rule_refs, &facts, scope);
    let report = sarif::render(&rule_refs, &findings);
    let sarif_path = root.join("target").join("conform").join("report.sarif");
    if let Some(parent) = sarif_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&sarif_path, &report)?;

    let base = baseline::load(&root.join(baseline_rel))?;
    let (new, stale) = baseline::diff(&base, &findings);
    for f in &new {
        eprintln!(
            "  conform: NEW {} {}:{} — {}",
            f.rule, f.file, f.line, f.message
        );
    }
    for fp in &stale {
        eprintln!("  conform: baseline entry no longer fires — prune it: {fp}");
    }
    let counts = count_by_rule(&findings);
    eprintln!(
        "conform check: {} finding(s) in scope {} ({:?}), {} frozen in baseline, {} new; SARIF at {}.",
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
        "conform: {} crate(s) gated, {} exempt — see conform.toml for the why of each.",
        config.gated_crates.len(),
        config.exempt.len(),
    );
    if !new.is_empty() {
        bail!("conform: {} new finding(s) against the baseline", new.len());
    }
    Ok(())
}

/// `conform freeze` — rewrite the baseline to the current finding set. The
/// legal moments: a NEW rule landing (its pre-existing findings freeze once),
/// and a re-freeze after work that shrank the set. The diff review is the
/// guard: outside a new-rule landing the file may only shrink.
pub fn run_freeze(root: &Path, baseline_rel: &str) -> Result<()> {
    use conform_core::{ExtractionLog, Store, check, count_by_rule};
    use conform_frontend_rust::RustFrontend;

    let config = load_config(root)?;
    let store = Store::at_repo(root, &config);
    let mut log = ExtractionLog::default();
    let frontend = RustFrontend;
    let facts = store.extract_workspace(root, &frontend, &mut log)?;
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
        "conform freeze: {} fingerprint(s) frozen ({:?}) at {}.",
        fps.len(),
        counts,
        baseline_rel
    );
    Ok(())
}
