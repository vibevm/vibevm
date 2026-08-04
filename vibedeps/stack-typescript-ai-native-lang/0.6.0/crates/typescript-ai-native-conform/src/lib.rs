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
/// layer (`typescript-ai-native-tcg`, TCG-PROTOCOL-v0.1 §3) cannot drift
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
    // ts-seam-error-cites-req (B-033) — the TS twin of the Go/Rust
    // seam-error rules; always on (it fires only where the extractor
    // finds a discriminated-union error alias, so a project without the
    // idiom stays quiet).
    out.push(Box::new(rules::TsSeamErrorCitesReq));
    if let Some(cells_dir) = &config.typescript.cells_dir {
        out.push(Box::new(rules::TsCellIsolation::new(
            cells_dir,
            &config.typescript.seam,
        )));
    }
    // ts-flag-sites (B-039) mounts ONLY when the policy names a
    // composition root — the TS twin of Rust mounting R-001 only with
    // `registry_file`. Absent the field, the rule is off (a project
    // without the flag idiom), so the dirty fixture's gate count and the
    // `None` default are unchanged.
    if let Some(root) = &config.typescript.composition_root {
        out.push(Box::new(rules::TsFlagSites::new(root)));
    }
    out.push(Box::new(rules::FileLength {
        max_lines: config.max_file_lines,
    }));
    out.push(Box::new(rules::InvariantCommentPosition {
        markers: config.invariant_comment_markers.clone(),
        min_lines: config.invariant_comment_min_file_lines,
    }));
    out.push(Box::new(rules::DeclaredTestMatrices));
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
        "typescript-ai-native-conform: extracted {} file(s), {} cached (producer ts-tsc-2).",
        log.extracted.len(),
        log.cached,
    );
    Ok(facts)
}

/// Announce the TypeScript coverage posture after extraction — the
/// sharper empty-scope guard (a configured `cells_dir` that enumerated
/// zero cells warns loudly instead of passing silently) and the
/// vacuous-gate warning (a gated cell the scan attributed no sources
/// to). Printed in both `run_check` and `run_freeze`, exactly where
/// Rust's `warn_vacuously_gated` sits; the count summary lives in
/// `run_check` alone (parity with the Rust driver).
fn announce_ts_coverage(root: &Path, config: &Config) {
    let units = conform_core::ts_units(root, &config.typescript);
    for w in conform_core::ts_scope_warnings(&units, &config.typescript) {
        eprintln!("{w}");
    }
    for cell in conform_core::ts_vacuously_gated(&config.typescript.gated, &units) {
        eprintln!(
            "typescript-ai-native-conform: WARNING — gated cell `{cell}` matched no scanned \
             sources; its gates are green by vacuity. Point `cells_dir` in conform.toml at the \
             cells tree, or drop it from `[typescript] gated`."
        );
    }
}

/// Run the TS gate at `root` against `baseline_rel`; SARIF lands at
/// `target/conform/report-typescript.sarif`; any new finding fails.
pub fn run_check(root: &Path, baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    use conform_core::{baseline, check, count_by_rule, sarif};
    let config = load_config(root)?;
    config.validate_typescript_against_tree(root)?;
    let facts = extract(root, &config)?;
    announce_ts_coverage(root, &config);
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
    eprintln!(
        "typescript-ai-native-conform: {} cell(s) gated, {} exempt — see conform.toml for the why of each.",
        config.typescript.gated.len(),
        config.typescript.exempt.len(),
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
    config.validate_typescript_against_tree(root)?;
    let facts = extract(root, &config)?;
    announce_ts_coverage(root, &config);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The coverage invariant (B-034) refuses an on-disk TS cell that is
    /// neither gated nor exempt — the silent-green failure mode the gate
    /// now closes. Pure config + tree: no extraction, so no node
    /// toolchain floor (the `tests/gate.rs` pair carries the end-to-end
    /// half over the committed fixtures).
    #[test]
    fn validate_refuses_an_unclassified_ts_cell() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::create_dir_all(root.join("src/cells/greeting")).expect("mkdir");
        std::fs::write(
            root.join("src/cells/greeting/index.ts"),
            "export const x = 1;\n",
        )
        .expect("ts file");
        // roots = ["src"], cells_dir set, no gated/exempt → the greeting
        // cell is on disk but unclassified.
        std::fs::write(
            root.join("conform.toml"),
            "[typescript]\nroots = [\"src\"]\ncells_dir = \"src/cells\"\n",
        )
        .expect("conform");
        let cfg = Config::load(&root.join("conform.toml")).expect("parses");
        let err = cfg
            .validate_typescript_against_tree(root)
            .expect_err("must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("greeting"),
            "must name the unclassified cell: {msg}"
        );
        assert!(
            msg.contains("cell") && msg.contains("neither gated nor exempt"),
            "must speak the TS noun and the refusal class: {msg}"
        );

        // Classifying the cell (gated OR exempt) clears the refusal.
        std::fs::write(
            root.join("conform.toml"),
            "[typescript]\nroots = [\"src\"]\ncells_dir = \"src/cells\"\ngated = [\"greeting\"]\n",
        )
        .expect("conform");
        let cfg = Config::load(&root.join("conform.toml")).expect("parses");
        cfg.validate_typescript_against_tree(root)
            .expect("classified → green");
    }

    /// The B-039 demo (`research/ts-demo`) instantiates the runtime-flag
    /// tier the guide used to only describe: `src/main.ts` is the
    /// composition root, so `[typescript] composition_root` is set, the
    /// `ts-flag-sites` rule mounts, and the root's own env read is the one
    /// legal site. Pure config + tree + rule construction — no node
    /// toolchain floor (the live demo run is the boss's acceptance).
    #[test]
    fn demo_config_mounts_flag_sites_and_validates_green() {
        let demo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../../../research/ts-demo");
        assert!(demo.is_dir(), "demo tree must exist at {}", demo.display());
        assert!(
            demo.join("src/main.ts").is_file(),
            "the composition root src/main.ts must exist"
        );

        let cfg = Config::load(&demo.join("conform.toml")).expect("demo conform.toml");
        assert_eq!(
            cfg.typescript.composition_root.as_deref(),
            Some("src/main.ts"),
            "demo must name its composition root",
        );

        // main.ts lives OUTSIDE cells_dir, so the on-disk cell set
        // (greeting, farewell) is unchanged and the coverage invariant
        // still holds — adding a root file never adds a cell.
        cfg.validate_typescript_against_tree(&demo)
            .expect("demo tree validates green");

        let rules = build_rules(&cfg);
        let ids: Vec<&str> = rules.iter().map(|r| r.id()).collect();
        assert!(
            ids.contains(&"ts-flag-sites"),
            "the rule must mount when composition_root is set: {ids:?}"
        );
    }

    /// `ts-flag-sites` is OFF when `[typescript] composition_root` is
    /// absent — the default and the dirty fixture's posture. The dirty
    /// fixture's gate therefore stays at its 5 findings; a project without
    /// the flag idiom is never surprised by the rule.
    #[test]
    fn flag_sites_is_unmounted_without_a_composition_root() {
        let (cfg, _) = Config::load_or_default(std::path::Path::new(".")).unwrap();
        assert!(cfg.typescript.composition_root.is_none());
        let ids: Vec<&str> = build_rules(&cfg).iter().map(|r| r.id()).collect();
        assert!(
            !ids.contains(&"ts-flag-sites"),
            "the rule must NOT mount without composition_root: {ids:?}"
        );
    }
}
