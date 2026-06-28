//! Engine-level acceptance for ENGINE-CONFORM §3/§5 over a synthetic
//! mini-workspace: the store is incremental by content hash (a 1-file
//! diff re-extracts exactly 1 file, asserted via the producer log) and
//! the SARIF output is byte-identical across runs.

use std::path::Path;

use conform_core::{Config, ExtractionLog, Rule, Store, baseline, check, rules, sarif};
use conform_frontend_rust::RustFrontend;

fn seed(repo: &Path, rel: &str, text: &str) {
    let path = repo.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn mini_workspace(repo: &Path) {
    seed(
        repo,
        "crates/alpha/src/lib.rs",
        "pub mod cell_x;\npub mod cell_y;\n",
    );
    seed(
        repo,
        "crates/alpha/src/cell_x.rs",
        "use crate::cell_y::Y;\n\n#[cell(seam = \"S\", variant = \"x\")]\npub struct X;\n",
    );
    seed(
        repo,
        "crates/alpha/src/cell_y.rs",
        "#[cell(seam = \"S\", variant = \"y\")]\npub struct Y;\n",
    );
    seed(
        repo,
        "crates/beta/src/lib.rs",
        "pub fn danger() { unsafe { std::hint::black_box(()) } }\n",
    );
}

fn engine_rules() -> (rules::FlagSites, rules::CellIsolation, rules::UnsafeGate) {
    (
        rules::FlagSites {
            registry_file: "crates/cli/src/registry.rs".into(),
            gated_crate: "cli".into(),
        },
        rules::CellIsolation,
        rules::UnsafeGate {
            audit_crates: vec![],
        },
    )
}

#[test]
fn incremental_one_file_diff_reextracts_one_file() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    mini_workspace(repo);
    let store = Store::at_repo(repo, &Config::default());

    let mut cold = ExtractionLog::default();
    store
        .extract_workspace(repo, &RustFrontend, &mut cold)
        .unwrap();
    assert_eq!(cold.extracted.len(), 4, "cold run extracts everything");
    assert_eq!(cold.cached, 0);

    let mut warm = ExtractionLog::default();
    store
        .extract_workspace(repo, &RustFrontend, &mut warm)
        .unwrap();
    assert!(warm.extracted.is_empty(), "warm run is all cache hits");
    assert_eq!(warm.cached, 4);

    seed(
        repo,
        "crates/alpha/src/cell_y.rs",
        "#[cell(seam = \"S\", variant = \"y\")]\npub struct Y;\npub struct Extra;\n",
    );
    let mut touched = ExtractionLog::default();
    store
        .extract_workspace(repo, &RustFrontend, &mut touched)
        .unwrap();
    assert_eq!(
        touched.extracted,
        vec!["crates/alpha/src/cell_y.rs".to_string()],
        "a 1-file diff re-extracts exactly 1 file"
    );
    assert_eq!(touched.cached, 3);

    // Facts survive an epoch change (LEDGER §2): the store key is
    // (file content-hash, producer) — lockfile/toolchain context plays
    // no part, so touching Cargo.lock invalidates nothing here.
    seed(repo, "Cargo.lock", "dep graph changed\n");
    let mut after_epoch = ExtractionLog::default();
    store
        .extract_workspace(repo, &RustFrontend, &mut after_epoch)
        .unwrap();
    assert!(after_epoch.extracted.is_empty());
    assert_eq!(after_epoch.cached, 4);
}

#[test]
fn findings_and_sarif_are_deterministic_and_baseline_gates() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    mini_workspace(repo);
    let store = Store::at_repo(repo, &Config::default());

    let run = || {
        let mut log = ExtractionLog::default();
        let facts = store
            .extract_workspace(repo, &RustFrontend, &mut log)
            .unwrap();
        let (r1, r2, r3) = engine_rules();
        let rule_refs: Vec<&dyn Rule> = vec![&r1, &r2, &r3];
        let findings = check(&rule_refs, &facts, None);
        let report = sarif::render(&rule_refs, &findings);
        (findings, report)
    };

    let (findings_a, sarif_a) = run();
    let (findings_b, sarif_b) = run();
    assert_eq!(findings_a, findings_b);
    assert_eq!(sarif_a, sarif_b, "same inputs — byte-identical SARIF");

    // The synthetic tree carries exactly two violations: the sibling
    // cell import and the unsafe block.
    assert_eq!(findings_a.len(), 2, "{findings_a:?}");
    assert!(findings_a.iter().any(|f| f.rule == "R-002"));
    assert!(findings_a.iter().any(|f| f.rule == "unsafe-gate"));

    // Scope frontier: findings only inside scope, facts untouched.
    let (r1, r2, r3) = engine_rules();
    let rule_refs: Vec<&dyn Rule> = vec![&r1, &r2, &r3];
    let mut log = ExtractionLog::default();
    let facts = store
        .extract_workspace(repo, &RustFrontend, &mut log)
        .unwrap();
    let scoped = check(&rule_refs, &facts, Some("crates/beta/"));
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].rule, "unsafe-gate");

    // Baseline freeze: both findings frozen -> gate clean; removing a
    // frozen entry surfaces it as new again.
    let frozen = baseline::Baseline {
        schema: 1,
        findings: findings_a.iter().map(|f| f.fingerprint.clone()).collect(),
    };
    let (new, stale) = baseline::diff(&frozen, &findings_a);
    assert!(new.is_empty());
    assert!(stale.is_empty());
}
