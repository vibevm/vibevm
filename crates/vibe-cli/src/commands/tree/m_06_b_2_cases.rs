use vibe_workspace::boot_debugger::measurement::{Count, capture_workspace};

#[test]
fn tree_and_closure_transport_share_exact_static_dynamic_capture() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[project]\nname='fixture'\nversion='1.0.0'\n",
    )
    .unwrap();
    let dir = tmp.path().join(vibe_core::layout::current_boot_dir());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(tmp.path().join("a.md"), "雪").unwrap();
    std::fs::write(
        dir.join("STATIC.md"),
        "<!-- vibe:static org.demo/a — a.md -->\n雪\n<!-- vibe:static org.demo/a — a.md -->\n雪",
    )
    .unwrap();
    let mut lock = vibe_core::manifest::Lockfile::empty("fixture", "2026-10-15T00:00:00Z");
    let mut package = super::tests::locked("org.demo", "a", "1.0.0", None);
    package.content_hash =
        vibe_core::ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("boot.md"), "雪").unwrap();
    std::fs::write(
        source.path().join("vibe.toml"),
        "[package]\nname='a'\ngroup='org.demo'\nkind='flow'\nversion='1.0.0'\nformat='simple'\n",
    )
    .unwrap();
    // Fixture preparation uses the existing materializer to retain the slot
    // witness required by tree's pre-existing source classification. The
    // subsequent measurement operation itself only reads.
    vibe_workspace::vibedeps::materialise_with_spec_format(
        tmp.path(),
        &package.group,
        "a",
        &package.version,
        source.path(),
        vibe_workspace::vibedeps::CopyMode::Copy,
        vibe_core::manifest::SpecFormat::Mixed,
        &package.content_hash,
    )
    .unwrap();
    lock.packages.push(package);
    lock.write(tmp.path().join("vibe.lock")).unwrap();
    let dynamic_path = vibe_core::layout::current_vibedeps_root().join("org.demo.a/1.0.0/boot.md");
    let dynamic_path = vibe_core::machine_json_path(&dynamic_path);
    std::fs::write(tmp.path().join(vibe_core::layout::current_boot_index()), format!("[[entry]]\npath='{dynamic_path}'\nkind='dynamic'\n[[entry]]\npath='{dynamic_path}'\nkind='dynamic'\nwhen='os:unknown'\n")).unwrap();
    let before = capture_workspace(tmp.path());
    let tree = crate::commands::tree::build::build_tree(tmp.path()).unwrap();
    let encoded = serde_json::to_value(&tree).unwrap();
    let pair = super::measurement_pair(Some(&before), tree.boot.measurements.as_ref());
    assert_eq!(pair["before"], encoded["boot"]["measurements"]);
    assert_eq!(pair["after"], encoded["boot"]["measurements"]);
    assert_eq!(before.lanes[1].totals.repeated_raw_bytes, Count::exact(6));
    assert!(matches!(
        before.packages["org.demo/a"].inclusive_bytes,
        Count::Unavailable { .. }
    ));
    assert_eq!(
        before.packages["org.demo/a"].repeated_raw_bytes,
        Count::exact(12)
    );
    assert_eq!(before.lanes[0].totals.inclusive_bytes, Count::exact(7));
    assert_eq!(
        before.lanes[1].snippets[0].package.as_deref(),
        Some("org.demo/a")
    );
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../resources/package-tree.schema.v1.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(
        validator.is_valid(&encoded),
        "{:?}",
        validator.iter_errors(&encoded).collect::<Vec<_>>()
    );
    std::fs::write(tmp.path().join("a.md"), "changed").unwrap();
    let after = capture_workspace(tmp.path());
    let pair = super::measurement_pair(Some(&before), Some(&after));
    assert_ne!(pair["before"]["capture_id"], pair["after"]["capture_id"]);
    assert_eq!(pair["before"]["lanes"][1]["snippets"][0]["raw"]["bytes"], 3);
}

#[test]
fn default_transport_none_is_unavailable_and_never_zero() {
    let pair = super::measurement_pair(None, None);
    assert!(pair["before"].is_null());
    assert!(pair["after"].is_null());
    assert_eq!(pair["none_means"], "declared-unavailable");
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[project]\nname='empty'\nversion='1.0.0'\n",
    )
    .unwrap();
    let tree = crate::commands::tree::build::build_tree(tmp.path()).unwrap();
    assert_eq!(
        tree.boot.measurements.as_ref().unwrap(),
        &capture_workspace(tmp.path())
    );
}
