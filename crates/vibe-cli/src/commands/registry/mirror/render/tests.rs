//! Synthetic formatter fixtures: no registry operations or ambient configuration.

use super::*;
use crate::output::language::{Meta, rule_at, trailer};
use serde_json::json;
use std::time::Duration;
use vibe_wire::generated::registry_mirror_report::TargetKind;

fn report(action: &str) -> RegistryMirrorReport {
    serde_json::from_value(json!({
        "ok": true,
        "command": "registry:mirror",
        "source": "corp",
        "source_url": "file:///synthetic/source",
        "listing": "index",
        "target": "opaque wire target; never a human name",
        "target_kind": "localdir",
        "versions": [{
            "action": action, "frozen": true, "package": "org.x/a",
            "version": "1.0.0", "yanked": false
        }],
        "facts_recorded": 0, "index_refreshed": false,
        "retried": false, "dry_run": false, "bundles": 0
    }))
    .unwrap()
}

fn target() -> Target {
    Target {
        layout: LayoutTarget::Localdir {
            root: std::env::temp_dir().join("synthetic-mirror-record"),
        },
        kind: TargetKind::Localdir,
        name: "mirror".into(),
        url: "file:///synthetic/target".into(),
        shown: "opaque target description".into(),
    }
}

#[test]
fn mirror_rerun_says_current() {
    let mut report = report("unchanged");
    report.versions = vec![report.versions[0].clone(); 3];
    let before = serde_json::to_value(&report).unwrap();
    let target = target();
    let record = target_record(&target);
    assert_eq!(
        record,
        std::env::temp_dir()
            .join("synthetic-mirror-record")
            .display()
            .to_string()
    );
    assert_eq!(
        summary(&report, &target.name, 0),
        "mirror is current · 3 unchanged"
    );
    assert!(version_rows(&report).rows.is_empty());
    let line = trailer(
        true,
        &summary(&report, &target.name, 0),
        vec![
            Meta::note("5 steps"),
            Meta::Time(Duration::from_secs(4)),
            Meta::note(&record),
        ],
        None,
    );
    assert_eq!(
        line,
        format!("✻ mirror is current · 3 unchanged · 5 steps · 4s · {record}")
    );
    assert_eq!(serde_json::to_value(&report).unwrap(), before);
}

#[test]
fn mirror_rerun_says_current_only_without_effects_or_incomplete_work() {
    let original = report("unchanged");
    let mut cases = Vec::new();
    for action in [
        "copied",
        "updated",
        "alarm",
        "unavailable",
        "snapshot-differs",
    ] {
        cases.push(report(action));
    }
    let mut changed = original.clone();
    changed.ok = false;
    cases.push(changed);
    let mut changed = original.clone();
    changed.facts_recorded = 1;
    cases.push(changed);
    let mut changed = original.clone();
    changed.index_refreshed = true;
    cases.push(changed);
    let mut changed = original.clone();
    changed.commit = Some("abc123".into());
    cases.push(changed);
    let mut changed = original.clone();
    changed.bundles = 1;
    cases.push(changed);
    let mut changed = original.clone();
    changed.removed = Some(1);
    cases.push(changed);
    let mut changed = original.clone();
    changed.pruned = serde_json::from_value(json!([{
        "kind": "version", "package": "org.x/a", "path": "org.x/a/v1.0.0",
        "reason": "no longer selected"
    }]))
    .unwrap();
    cases.push(changed);
    let mut changed = original.clone();
    changed.dry_run = true;
    cases.push(changed);
    for changed in cases {
        assert!(
            !summary(&changed, "mirror", 0).contains("is current"),
            "{changed:?}"
        );
    }
    assert!(!summary(&original, "mirror", 1).contains("is current"));
}

#[test]
fn mirror_rerun_says_current_copy_alarm_and_dry_run_are_distinct() {
    let mut copied = report("copied");
    copied.versions = vec![copied.versions[0].clone(); 3];
    copied.facts_recorded = 3;
    assert_eq!(
        summary(&copied, "mirror", 0),
        "mirrored corp into mirror · 3 copied · 3 facts recorded"
    );
    assert_eq!(
        version_rows(&report("copied")).render_at(200),
        "✓ org.x/a@1.0.0  copied"
    );
    copied.dry_run = true;
    assert_eq!(
        summary(&copied, "mirror", 0),
        "would mirror corp into mirror · 3 would be copied · 3 facts would be recorded"
    );
    assert_eq!(
        version_rows(&copied).render_at(200).lines().next(),
        Some("✓ org.x/a@1.0.0  would be copied")
    );
    assert_eq!(
        rule_at(&heading_label("corp", "mirror", true), None, 48),
        format!("── mirror corp into mirror · dry run {}", "─".repeat(11))
    );
    for (action, verb) in [
        ("alarm", "ALARM"),
        ("unavailable", "unavailable at the source"),
    ] {
        let mut failed = report(action);
        failed.ok = false;
        assert_eq!(
            version_rows(&failed).render_at(200),
            format!("✗ org.x/a@1.0.0  {verb}")
        );
        assert!(
            summary(&failed, "mirror", 0).starts_with("mirror corp into mirror incomplete · 1 ")
        );
    }
    let mut committed = report("unchanged");
    committed.commit = Some("abc123".into());
    committed.removed = Some(2);
    assert_eq!(
        summary(&committed, "mirror", 0),
        "mirrored corp into mirror · 1 unchanged · 2 removed · commit abc123"
    );
    let mut remote = target();
    remote.layout = LayoutTarget::Monorepo {
        url: "https://fixture-user:fixture-password@example.invalid/registry.git".into(),
        branch: "main".into(),
    };
    let record = target_record(&remote);
    assert!(record.contains("example.invalid/registry.git"));
    assert!(!record.contains("fixture-password"));
}
