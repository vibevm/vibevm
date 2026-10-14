use super::*;
use std::{fs, path::Path};
use vibe_safefs::Project;

fn fixture(family: &str, lane: &str) -> SourceInput {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/next/derivatives");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let row = manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == family)
        .unwrap();
    let owner = if lane == "base" { "upstream" } else { "local" };
    let coordinate =
        PackageRef::parse(&format!("flow:org.example.{owner}/{family}@=1.0.0")).unwrap();
    let source_root = root.join(family).join(lane);
    let project = Project::open(&source_root).unwrap();
    let files = row[lane]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| {
            let path = file["path"].as_str().unwrap();
            CapturedSourceFile::new(
                format!("payload/{path}"),
                project
                    .read_file_snapshot_bounded(path, 4096)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap()
        })
        .collect();
    let descriptor: SourceDescriptor = serde_json::from_value(if family == "commonmark" {
        serde_json::json!({"file":"payload/guide.md", "media":"text/markdown", "interpretation":"commonmark", "role":"documentation", "loading":"explicit"})
    } else {
        serde_json::json!({"directory":"payload", "media":"application/octet-stream", "interpretation":"opaque", "role":"resource", "loading":"explicit"})
    }).unwrap();
    SourceInput::new(coordinate, descriptor, files).unwrap()
}

fn lifecycle(family: &str) {
    let base = fixture(family, "base");
    let local = fixture(family, "local");
    let expected = fixture(family, "expected");
    let authored = AuthoredReplacement::new(&base, &local).unwrap();
    let derived = DerivativeService::derive(&base, &authored).unwrap();
    assert_eq!(derived.effects, Effects::NoExternalEffects);
    assert_eq!(
        DerivativeService::show(&derived.value).value.coordinate(),
        local.coordinate()
    );
    for (path, file) in derived.value.files() {
        assert_eq!(
            file.snapshot().bytes,
            expected.files()[path].snapshot().bytes
        );
    }
    assert_eq!(derived.value.files().len(), expected.files().len());
    let changes = DerivativeService::diff(&derived.value);
    assert_eq!(changes.effects, Effects::NoExternalEffects);
    if family == "nested-tree" {
        assert!(
            changes
                .value
                .contains(&Change::Removed("payload/docs/obsolete.txt".into()))
        );
        assert!(
            changes
                .value
                .contains(&Change::Added("payload/docs/local.txt".into()))
        );
        assert_eq!(changes.value.len(), 4);
    } else {
        assert_eq!(changes.value, [Change::Replaced("payload/guide.md".into())]);
    }
    let revised = AuthoredReplacement::new(&expected, &base).unwrap();
    let rebased = DerivativeService::rebase(&derived.value, &base, &expected, &revised).unwrap();
    assert_eq!(rebased.effects, Effects::NoExternalEffects);
    assert!(DerivativeService::apply(rebased.value, &base).is_err());
    let handoff = DerivativeService::apply(derived.value, &base).unwrap();
    assert_eq!(handoff.effects, Effects::NoExternalEffects);
    assert_eq!(
        handoff.value.output().authored_source().coordinate(),
        local.coordinate()
    );
}

#[test]
fn text_uses_common_lifecycle() {
    lifecycle("commonmark");
}
#[test]
fn opaque_tree_uses_common_lifecycle() {
    lifecycle("nested-tree");
}

#[test]
fn authored_provenance_survives_output_and_cache_deletion() {
    for family in ["commonmark", "nested-tree"] {
        let base = fixture(family, "base");
        let local = fixture(family, "local");
        let authored = AuthoredReplacement::new(&base, &local).unwrap();
        let cache = tempfile::tempdir().unwrap();
        let output = DerivativeService::derive(&base, &authored).unwrap();
        for (path, file) in output.value.files() {
            let target = cache.path().join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, &file.snapshot().bytes).unwrap();
        }
        drop(output);
        cache.close().unwrap();
        // Recapture durable local fixture files independently of deleted output.
        let retained = fixture(family, "local");
        assert!(retained.same_basis(&local));
        let rebuilt = DerivativeService::derive(&base, &authored).unwrap();
        assert!(rebuilt.value.authored_source().same_basis(&retained));
        assert!(rebuilt.value.base().same_basis(&fixture(family, "base")));
    }
}

#[test]
fn stale_basis_and_upstream_identity_are_refused() {
    let base = fixture("nested-tree", "base");
    let local = fixture("nested-tree", "local");
    let authored = AuthoredReplacement::new(&base, &local).unwrap();
    let stale = SourceInput::new(
        base.coordinate.clone(),
        base.descriptor.clone(),
        local.files.values().cloned().collect(),
    )
    .unwrap();
    assert!(matches!(
        DerivativeService::derive(&stale, &authored),
        Err(DerivativeError::Incompatible)
    ));
    assert!(matches!(
        AuthoredReplacement::new(&base, &stale),
        Err(DerivativeError::UpstreamIdentity)
    ));
    let output = DerivativeService::derive(&base, &authored).unwrap();
    assert!(DerivativeService::rebase(&output.value, &stale, &base, &authored).is_err());
    assert!(DerivativeService::apply(output.value, &stale).is_err());
}

#[test]
fn passive_inspection_has_no_provider_process_grant_or_slot_effect() {
    // Service has no I/O or callback ports. Sentinels check unchanged installed
    // bytes and absent effects alongside exact preservation of script bytes.
    let sandbox = tempfile::tempdir().unwrap();
    let slot = sandbox.path().join("installed");
    fs::write(&slot, b"immutable installed source").unwrap();
    let base = fixture("nested-tree", "base");
    let local = fixture("nested-tree", "local");
    let authored = AuthoredReplacement::new(&base, &local).unwrap();
    let output = DerivativeService::derive(&base, &authored).unwrap();
    let shown = DerivativeService::show(&output.value);
    assert_eq!(shown.effects, Effects::NoExternalEffects);
    assert_eq!(
        shown.value.files()["payload/scripts/helper.sh"]
            .snapshot()
            .bytes,
        base.files()["payload/scripts/helper.sh"].snapshot().bytes
    );
    assert!(!DerivativeService::diff(&output.value).value.is_empty());
    let handoff = DerivativeService::apply(output.value, &base).unwrap();
    assert_eq!(handoff.effects, Effects::NoExternalEffects);
    assert_eq!(fs::read(&slot).unwrap(), b"immutable installed source");
    assert_eq!(fs::read_dir(sandbox.path()).unwrap().count(), 1);
}

#[test]
fn invalid_inventory_cannot_become_output() {
    let base = fixture("nested-tree", "base");
    let file = base.files.values().next().unwrap().clone();
    assert!(
        SourceInput::new(
            base.coordinate.clone(),
            base.descriptor.clone(),
            vec![file.clone(), file.clone()]
        )
        .is_err()
    );
    assert!(CapturedSourceFile::new("../escape".into(), file.snapshot().clone()).is_err());
    let outside = CapturedSourceFile::new("outside.txt".into(), file.snapshot().clone()).unwrap();
    assert!(
        SourceInput::new(
            base.coordinate.clone(),
            base.descriptor.clone(),
            vec![outside]
        )
        .is_err()
    );
}
