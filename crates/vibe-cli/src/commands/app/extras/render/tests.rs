//! The human views of vibe app extras at a fixed width. Glyphs asserted here: ✓ ✗ ○.

use super::*;

fn listing() -> extras_wire::ApplicationExtras {
    serde_json::from_value(serde_json::json!({
        "protocol": "vibe-app-extras/1", "slot": "runtime",
        "key": "org.example/runtime@1.0.0", "version": "1.0.0",
        "ecosystems": [{
            "id": "npm", "name": "npm", "verbs": ["install", "list"],
            "recommendations": [
                {"id": "corepack", "name": "Corepack", "item": "corepack@latest",
                 "default": true, "enabled": true},
                {"id": "yarn", "name": "Yarn", "item": "yarn@1",
                 "default": false, "enabled": false},
                {"id": "pnpm", "name": "pnpm", "item": "pnpm@9",
                 "default": true, "enabled": true, "dropped": true}
            ],
            "add": ["lodash"], "drop": ["pnpm"], "items": ["corepack@latest", "lodash"],
            "applied": ["corepack@latest"], "applied_at": "2026-11-23T08:34:58Z",
            "observed": ["corepack@latest"]
        }],
        "notices": ["the drop `nope` names no recommendation of `npm`"]
    }))
    .unwrap()
}

fn run() -> wire::ApplicationEcosystem {
    serde_json::from_value(serde_json::json!({
        "protocol": "vibe-app-ecosystem/1", "ok": false, "slot": "runtime",
        "key": "org.example/runtime@1.0.0", "verb": "install", "work": "/h/w",
        "adapters": [
            {"id": "npm", "name": "npm", "outcome": "failed", "items": ["corepack@latest"],
             "log": "/h/w/npm.log", "reason": "the `install` verb exited with status 3",
             "retry": "vibe app extras apply runtime --ecosystem npm"},
            {"id": "pip", "name": "pip", "outcome": "done", "items": ["black"],
             "observed": ["black"], "log": "/h/w/pip.log"},
            {"id": "cargo", "name": "cargo", "outcome": "skipped", "items": [],
             "reason": "the item list is empty"}
        ]
    }))
    .unwrap()
}

#[test]
fn extras_listing_is_a_rule_over_measured_rows() {
    let mut document = listing();
    let npm = "\
── extras of runtime 1.0.0 · npm ───────────────────────────────────────────────

✓ corepack  corepack@latest  Corepack  [chosen]
○ yarn      yarn@1           Yarn      [not chosen]
✗ pnpm      pnpm@9           pnpm      [dropped]
  yours     lodash
  final list    corepack@latest lodash
  last applied  corepack@latest
  installed     corepack@latest";
    let notice = "  the drop nope names no recommendation of npm";
    assert_eq!(
        listing_text(&document, 80, 80),
        format!("{npm}\n\n{notice}")
    );
    assert_eq!(listing_quiet(&document), ["npm  corepack@latest lodash"]);

    document.ecosystems.push(
        serde_json::from_value(serde_json::json!({
            "id": "pip", "name": "Python packages", "verbs": ["install"],
            "recommendations": [], "items": ["black"], "add": ["black"]
        }))
        .unwrap(),
    );
    let shown = listing_text(&document, 80, 80);
    let sections = shown.split("\n\n").collect::<Vec<_>>();
    assert_eq!(sections.len(), 5);
    assert!(sections[0].starts_with("── extras of runtime 1.0.0 · npm ─"));
    assert!(sections[2].starts_with("── extras of runtime 1.0.0 · pip ─"));
    assert_eq!(sections[1], npm.split_once("\n\n").unwrap().1);
    assert_eq!(
        sections[3],
        "  yours  black\n  final list  black\n  installed   none"
    );
    assert_eq!(sections[4], notice);
    for heading in [sections[0], sections[2]] {
        assert!(!heading.ends_with("npm"));
        assert!(!heading.ends_with("pip"));
    }
    assert!(!shown.contains('✻'));
    assert_eq!(
        listing_quiet(&document),
        ["npm  corepack@latest lodash", "pip  black"]
    );
}

#[test]
fn extras_run_draws_one_block_per_adapter() {
    assert_eq!(
        run_text(&run(), 80),
        "\
✗ npm    install the extras · failed · log /h/w/npm.log
  why     the install verb exited with status 3
  retry   vibe app extras apply runtime --ecosystem npm
✓ pip    install the extras · installed black · log /h/w/pip.log
○ cargo  install the extras · skipped: the item list is empty

  work directory kept /h/w"
    );
}

#[test]
fn extras_failed_run_has_alert_trailer() {
    let mut document = run();
    assert_eq!(
        run_trailer(&document),
        (false, "extras of runtime partly applied · 1 failed".into())
    );
    document
        .adapters
        .retain(|row| row.outcome != wire::Outcome::Done);
    assert_eq!(
        run_trailer(&document),
        (false, "extras of runtime not applied · 1 failed".into())
    );
    document.ok = true;
    for row in &mut document.adapters {
        row.outcome = wire::Outcome::Done;
    }
    assert_eq!(
        run_trailer(&document),
        (true, "extras of runtime applied".into())
    );
}

#[test]
fn extras_empty_observation_and_skips_keep_their_meanings() {
    let mut document = listing();
    let row = &mut document.ecosystems[0];
    row.recommendations.clear();
    row.add.clear();
    row.items.clear();
    row.applied.clear();
    row.observed_failed = Some("the `list` verb failed".into());
    assert!(
        listing_text(&document, 80, 80)
            .contains("  final list    none\n  not observed  the list verb failed")
    );
    assert_eq!(listing_quiet(&document), ["npm  none"]);
    let mut document = run();
    document.adapters.clear();
    document.work.clear();
    assert_eq!(run_text(&document, 80), "");
    document.notices.push("the `list` was empty".into());
    assert_eq!(run_text(&document, 80), "  the list was empty");
}

#[test]
fn extras_empty_work_uses_the_invocation_trailer_route() {
    // This is the selector report_run matches to dispatch trailer_on rather
    // than trailer_verdict_at: None preserves the Context-owned path metadata.
    let mut document = run();
    assert_eq!(super::super::run_trailer_work(&document.work), Some("/h/w"));
    document.work.clear();
    assert_eq!(super::super::run_trailer_work(&document.work), None);
}
