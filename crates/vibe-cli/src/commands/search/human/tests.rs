//! The human answer of vibe search at a fixed width. Glyphs asserted here: └ ✗ ○.

use super::super::report::{HitRow, UnreachableRegistry};
use super::*;

fn report() -> SearchReport {
    SearchReport {
        ok: true,
        command: "search",
        project: "/p".into(),
        query: "node".into(),
        registries_searched: vec!["corp".into(), "mirror".into()],
        registries_unconfigured: vec!["quiet".into()],
        registries_unreachable: vec![UnreachableRegistry {
            name: "broken".into(),
            reason: "probe of http://x/repomd.json failed".into(),
        }],
        registries_policy_skipped: vec![],
        registries_full_scanned: vec![],
        registries_full_scan_unsupported: vec![],
        hit_count: 2,
        hits: vec![
            HitRow {
                kind: "app".into(),
                group: Some("org.vibevm.node".into()),
                name: "node-runtime".into(),
                latest_stable: Some("1.0.0".into()),
                score: 1,
                matched_tokens: vec!["node".into()],
                description: Some("A test application package for node-runtime".into()),
                short_names: vec!["node".into()],
                registry: "corp".into(),
                source: "index",
            },
            HitRow {
                kind: "flow".into(),
                group: None,
                name: "wal".into(),
                latest_stable: None,
                score: 1,
                matched_tokens: vec![],
                description: None,
                short_names: vec![],
                registry: "mirror".into(),
                source: "full-scan",
            },
        ],
    }
}

#[test]
fn search_view_rows_fit_viewport() {
    let expected = format!(
        "{}\n  {:<4}  {:<28}  {:<5}  corp · score 1\n        └ A test application package for node-runtime\n  {:<4}  {:<28}  {:<5}  mirror · score 1  [full-scan]\n  searched corp · mirror\n\n{}\n  node  org.vibevm.node/node-runtime · corp\n  type these bare; every other hit needs its group/name\n\n{}\n✗ broken  unreachable\n  └ probe of http://x/repomd.json failed\n○ quiet   no index URL\n  └ set VIBEVM_INDEX_URL_<R> to search it",
        rule_at("search node", Some("2 hits · 2 registries searched"), 80),
        "app",
        "org.vibevm.node/node-runtime",
        "1.0.0",
        "flow",
        "wal",
        "-",
        rule_at("short names", None, 80),
        rule_at("registries", None, 80),
    );
    assert_eq!(view(&report(), 80, 80), expected);
    let mut long = report();
    long.query = "界".repeat(80);
    for width in [0, 1, 7, 8, 9, 20, 40] {
        for line in view(&long, width, width).lines() {
            assert!(
                console::measure_text_width(line) <= width,
                "width {width}: {line:?}"
            );
        }
    }
}

#[test]
fn search_view_no_match_is_one_line() {
    let mut report = report();
    report.hits.clear();
    report.hit_count = 0;
    report.registries_unconfigured.clear();
    report.registries_unreachable.clear();
    report.query = "zzzz".into();
    assert_eq!(
        view(&report, 80, 80),
        "○ no package matches zzzz · searched corp · mirror"
    );
}

#[test]
fn search_view_short_names_sit_under_a_rule() {
    let view = view(&report(), 80, 80);
    let mut lines = view.lines();
    let rule = rule_at("short names", None, 80);
    assert!(lines.any(|line| line == rule));
    assert_eq!(
        lines.next(),
        Some("  node  org.vibevm.node/node-runtime · corp")
    );
}

#[test]
fn search_view_qualified_names_and_registry_order() {
    let mut report = report();
    assert_eq!(qualified(&report.hits[0]), "org.vibevm.node/node-runtime");
    assert_eq!(qualified(&report.hits[1]), "wal");
    report.hits[1].group = Some(String::new());
    assert_eq!(qualified(&report.hits[1]), "/wal");
    report.registries_full_scanned.push("mirror".into());
    report
        .registries_policy_skipped
        .push(super::super::report::PolicySkippedRegistry {
            name: "policy".into(),
            observation: "unknown",
            reason: "contact refused".into(),
        });
    report
        .registries_full_scan_unsupported
        .push(UnreachableRegistry {
            name: "unsupported".into(),
            reason: "unsupported host".into(),
        });
    let text = view(&report, 80, 80);
    assert!(text.contains("  full-scanned mirror"));
    assert!(text.find("✗ broken").unwrap() < text.find("○ policy").unwrap());
    assert!(text.find("○ policy").unwrap() < text.find("○ unsupported").unwrap());
    assert!(text.find("○ unsupported").unwrap() < text.find("○ quiet").unwrap());
}
