//! `cargo xtask conform …` — the Phase 4 conformance-engine gate
//! (ENGINE-CONFORM §5): fact extraction through the content-addressed
//! store, rules-as-queries, SARIF, and the ratchet baseline.
//!
//! The policy — which crates are gated, the file-length budget, the
//! sanctioned env-read roots, the exempt table — lives in the project's
//! `conform.toml`, not in this file. The checker is config-driven so it
//! runs on any project (PROP-024), not only on vibevm itself.

use std::path::Path;

use anyhow::{Context, Result, bail};
use conform_core::{Config, Rule, rules};

use crate::repo_root;

/// Load the project's conform policy (`conform.toml` at the repo root).
pub(crate) fn load_config(root: &Path) -> Result<Config> {
    Config::load(&root.join("conform.toml"))
}

/// Build the standing rule set from the policy, in one place so
/// `conform check` and `conform freeze` can never drift apart. The order
/// is the SARIF driver order the gate has always rendered.
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

pub(crate) fn run_conform_check(baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    use conform_core::{ExtractionLog, Frontend, Store, baseline, check, count_by_rule, sarif};
    use conform_frontend_rust::RustFrontend;

    let root = repo_root()?;
    let config = load_config(&root)?;
    let store = Store::at_repo(&root, &config);
    let mut log = ExtractionLog::default();
    let frontend = RustFrontend;
    let facts = store.extract_workspace(&root, &frontend, &mut log)?;
    eprintln!(
        "xtask conform: extracted {} file(s), {} cached (producer {}-{}).",
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
        "xtask conform check: {} finding(s) in scope {} ({:?}), {} frozen in baseline, {} new; SARIF at {}.",
        findings.len(),
        scope.unwrap_or("<workspace>"),
        counts,
        base.findings.len(),
        new.len(),
        sarif_path
            .strip_prefix(&root)
            .unwrap_or(&sarif_path)
            .display()
    );
    eprintln!(
        "xtask conform: {} crate(s) gated, {} exempt — see conform.toml for the why of each.",
        config.gated_crates.len(),
        config.exempt.len(),
    );
    if !new.is_empty() {
        bail!("conform: {} new finding(s) against the baseline", new.len());
    }
    Ok(())
}

/// `cargo xtask conform freeze` — rewrite the baseline to the current
/// finding set. The legal moments: a NEW rule landing (its pre-existing
/// findings freeze once), and a re-freeze after work that shrank the set.
/// The diff review is the guard: outside a new-rule landing the file may
/// only shrink.
pub(crate) fn run_conform_freeze(baseline_rel: &str) -> Result<()> {
    use conform_core::{ExtractionLog, Store, check, count_by_rule};
    use conform_frontend_rust::RustFrontend;

    let root = repo_root()?;
    let config = load_config(&root)?;
    let store = Store::at_repo(&root, &config);
    let mut log = ExtractionLog::default();
    let frontend = RustFrontend;
    let facts = store.extract_workspace(&root, &frontend, &mut log)?;
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
        "xtask conform freeze: {} fingerprint(s) frozen ({:?}) at {}.",
        fps.len(),
        counts,
        baseline_rel
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// Every workspace crate is classified exactly once — gated by the
    /// config's `gated_crates` or exempt-with-a-reason by `[[exempt]]`,
    /// never both and never neither. This turns the exemption *table*
    /// into an enforced *invariant*: add a crate and forget to place it,
    /// or delete one and leave a phantom entry, and this fails.
    #[test]
    fn every_crate_is_gated_or_exempt() {
        let root = crate::repo_root().expect("repo root");
        let config = super::load_config(&root).expect("load conform.toml");

        let gated: BTreeSet<&str> = config.gated_crates.iter().map(|s| s.as_str()).collect();
        let exempt: BTreeSet<&str> = config
            .exempt
            .iter()
            .map(|e| e.crate_name.as_str())
            .collect();

        assert_eq!(
            gated.len(),
            config.gated_crates.len(),
            "gated_crates carries a duplicate crate name"
        );
        assert_eq!(
            exempt.len(),
            config.exempt.len(),
            "exempt carries a duplicate crate name"
        );

        let both: Vec<&str> = gated.intersection(&exempt).copied().collect();
        assert!(both.is_empty(), "crates both gated and exempt: {both:?}");

        for e in &config.exempt {
            assert!(
                !e.reason.trim().is_empty(),
                "{} is exempt without a recorded reason — the one thing this \
                 table exists to forbid",
                e.crate_name
            );
        }

        // Coverage against the real workspace: every crate dir under
        // `crates/` is named in exactly one set, and every listed name
        // except the workspace-root `xtask` is a real crate (no typos).
        let crates_dir = root.join("crates");
        let mut on_disk: BTreeSet<String> = BTreeSet::new();
        for entry in std::fs::read_dir(&crates_dir).expect("read crates/") {
            let entry = entry.expect("dir entry");
            if entry.file_type().expect("file type").is_dir()
                && entry.path().join("Cargo.toml").exists()
            {
                on_disk.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
        for c in &on_disk {
            assert!(
                gated.contains(c.as_str()) || exempt.contains(c.as_str()),
                "crate `{c}` is neither gated nor exempt — classify it in conform.toml"
            );
        }
        for c in gated.union(&exempt) {
            if *c == "xtask" {
                continue; // the tooling crate lives at the workspace root, not under crates/
            }
            assert!(
                on_disk.contains(*c),
                "`{c}` is listed in conform.toml but has no crates/{c} directory — typo?"
            );
        }
    }
}
