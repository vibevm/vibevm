#[test]
fn cargo_lock_reconciliation_emits_exact_graph_and_keeps_shared_transitives() {
    let before = br#"version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = [
 "core-ai-native-specmark",
 "product",
]

[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
dependencies = [
 "shared",
 "target-only",
]

[[package]]
name = "product"
version = "1.0.0"
dependencies = ["shared"]

[[package]]
name = "shared"
version = "1.0.0"

[[package]]
name = "target-only"
version = "1.0.0"
"#;
    let ((after, matches, _, spans), evidence) =
        prepare_cargo_lock(before, "core-ai-native-specmark").unwrap();
    let evidence = evidence.expect("a selected Cargo lock graph change has evidence");
    assert!(matches >= 3);
    assert!(!spans.is_empty());
    assert!(evidence.before_graph.len() > evidence.after_graph.len());
    assert_eq!(evidence.manager, "cargo");
    assert!(
        evidence
            .removed
            .iter()
            .any(|row| row.contains("core-ai-native-specmark"))
    );
    assert!(
        evidence
            .removed
            .iter()
            .any(|row| row.contains("target-only"))
    );
    assert!(!evidence.removed.iter().any(|row| row.contains("shared")));
    let after = std::str::from_utf8(&after).unwrap();
    assert!(!after.contains("core-ai-native-specmark"));
    assert!(!after.contains("target-only"));
    assert!(after.contains("product"));
    assert!(after.contains("shared"));

    let second = prepare_cargo_lock(before, "core-ai-native-specmark").unwrap();
    assert_eq!(second.0.0, after.as_bytes());
    assert_eq!(
        second.1.unwrap().before_graph,
        evidence.before_graph,
        "graph evidence is byte-order deterministic"
    );
}

#[test]
fn cargo_lock_reconciliation_refuses_ambiguous_or_non_root_authority() {
    let multiple_roots = br#"version = 4
[[package]]
name = "app-a"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "app-b"
version = "0.1.0"
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    assert!(matches!(
        prepare_cargo_lock(multiple_roots, "core-ai-native-specmark"),
        Err(ScrapeError::Blocked(message)) if message.contains("exactly one")
    ));

    let transitive_only = br#"version = 4
[[package]]
name = "app"
version = "0.1.0"
dependencies = ["product"]
[[package]]
name = "product"
version = "1.0.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    assert!(matches!(
        prepare_cargo_lock(transitive_only, "core-ai-native-specmark"),
        Err(ScrapeError::Blocked(message)) if message.contains("not a direct dependency")
    ));
}

#[test]
fn cargo_rewrite_carries_lock_graph_evidence_with_authorizing_id() {
    let root = tempfile::tempdir().unwrap();
    let manifest = br#"[package]
name = "app"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
"#;
    let lock = br#"version = 4
[[package]]
name = "app"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    std::fs::write(root.path().join("Cargo.toml"), manifest).unwrap();
    std::fs::write(root.path().join("Cargo.lock"), lock).unwrap();
    let project = Project::open(root.path()).unwrap();
    let entries = [
        ("Cargo.toml", manifest.as_slice()),
        ("Cargo.lock", lock.as_slice()),
    ]
    .into_iter()
    .map(|(path, bytes)| {
        let snapshot = project
            .read_file_snapshot_bounded(path, bytes.len())
            .unwrap()
            .unwrap();
        InventoryEntry {
            path: path.to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(bytes)),
            bytes: Some(bytes.len() as u64),
            unix_mode: snapshot.unix_mode,
            identity: Some(snapshot.identity),
        }
    })
    .collect::<Vec<_>>();
    let contract = contract_with(
        r#"
[[rewrite]]
id = "remove-specmark"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &entries).unwrap();
    assert!(prepared.blockers.is_empty());
    let lock = prepared
        .rewrites
        .iter()
        .find(|rewrite| rewrite.path == "Cargo.lock")
        .unwrap();
    let evidence = lock.native_lock_change.as_ref().unwrap();
    assert_eq!(evidence.manager, "cargo");
    assert_eq!(evidence.path, "Cargo.lock");
    assert_eq!(evidence.authorizing_rewrite_id, "remove-specmark");
    assert_eq!(evidence.before_sha256, lock.before_sha256);
    assert_eq!(evidence.after_sha256, lock.after_sha256);
    assert!(!evidence.before_graph.is_empty());
    assert!(!evidence.after_graph.is_empty());
    assert!(!evidence.removed.is_empty());
}

#[test]
fn cargo_rewrite_refuses_ambiguous_lock_without_fake_graph_evidence() {
    let root = tempfile::tempdir().unwrap();
    let manifest = br#"[package]
name = "app"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
"#;
    let lock = br#"version = 4
[[package]]
name = "app"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
[[package]]
name = "unrelated-root"
version = "1.0.0"
"#;
    std::fs::write(root.path().join("Cargo.toml"), manifest).unwrap();
    std::fs::write(root.path().join("Cargo.lock"), lock).unwrap();
    let project = Project::open(root.path()).unwrap();
    let entries = [
        ("Cargo.toml", manifest.as_slice()),
        ("Cargo.lock", lock.as_slice()),
    ]
    .into_iter()
    .map(|(path, bytes)| {
        let snapshot = project
            .read_file_snapshot_bounded(path, bytes.len())
            .unwrap()
            .unwrap();
        InventoryEntry {
            path: path.to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(bytes)),
            bytes: Some(bytes.len() as u64),
            unix_mode: snapshot.unix_mode,
            identity: Some(snapshot.identity),
        }
    })
    .collect::<Vec<_>>();
    let contract = contract_with(
        r#"
[[rewrite]]
id = "remove-specmark"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &entries).unwrap();
    assert!(prepared.rewrites.iter().any(|row| row.path == "Cargo.toml"));
    assert!(!prepared.rewrites.iter().any(|row| row.path == "Cargo.lock"));
    assert!(
        prepared
            .rewrites
            .iter()
            .all(|row| row.native_lock_change.is_none())
    );
    assert_eq!(prepared.blockers.len(), 1);
    assert_eq!(
        prepared.blockers[0].code,
        "native-lock-reconciliation-required"
    );
    assert_eq!(prepared.blockers[0].path.as_deref(), Some("Cargo.lock"));
    assert_eq!(
        prepared.blockers[0].rule_id.as_deref(),
        Some("remove-specmark")
    );
}

#[test]
fn cargo_lock_reconciliation_is_scoped_to_selected_project_manifests() {
    let root = tempfile::tempdir().unwrap();
    for dir in ["selected", "other"] {
        std::fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let selected_manifest = br#"[package]
name = "selected"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "../vibevm/specmark" }
"#;
    let other_manifest = br#"[package]
name = "other"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "../vibevm/specmark" }
"#;
    let selected_lock = br#"version = 4
[[package]]
name = "selected"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    let other_lock = br#"version = 4
[[package]]
name = "other"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    let files: [(&str, &[u8]); 4] = [
        ("selected/Cargo.toml", selected_manifest),
        ("selected/Cargo.lock", selected_lock),
        ("other/Cargo.toml", other_manifest),
        ("other/Cargo.lock", other_lock),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "selected-only"
kind = "cargo-package-remove-v1"
manifests = ["selected/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.blockers.is_empty());
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "selected/Cargo.toml")
    );
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "selected/Cargo.lock" && row.native_lock_change.is_some())
    );
    assert!(
        prepared
            .rewrites
            .iter()
            .all(|row| !row.path.starts_with("other/")),
        "an unselected independent Cargo project must remain byte-identical"
    );
}

#[test]
fn rust_alias_authority_never_crosses_into_an_unselected_crate() {
    let root = tempfile::tempdir().unwrap();
    for dir in ["crates/owned/src", "crates/unrelated/src"] {
        std::fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let owned_manifest = br#"[package]
name = "owned"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "../../vibevm/specmark" }
"#;
    let unrelated_manifest = br#"[package]
name = "unrelated"
version = "0.1.0"
[dependencies]
specmark = { package = "ordinary-product-macros", version = "1" }
"#;
    let owned_source = b"specmark::scope!(\"spec://owned\");\npub fn owned() {}\n";
    let unrelated_source = b"specmark::scope!(\"ordinary product syntax\");\npub fn product() {}\n";
    let files: [(&str, &[u8]); 4] = [
        ("crates/owned/Cargo.toml", owned_manifest),
        ("crates/owned/src/lib.rs", owned_source),
        ("crates/unrelated/Cargo.toml", unrelated_manifest),
        ("crates/unrelated/src/lib.rs", unrelated_source),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "strip-scopes"
kind = "rust-specmark-strip-v1"
patterns = ["crates/**/src/**/*.rs"]
forms = ["scope"]
matches = "one-or-more"
[[rewrite]]
id = "remove-owned-specmark"
kind = "cargo-package-remove-v1"
manifests = ["crates/owned/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "crates/owned/src/lib.rs")
    );
    assert!(
        prepared
            .rewrites
            .iter()
            .all(|row| row.path != "crates/unrelated/src/lib.rs")
    );
    assert!(prepared.blockers.is_empty());
}

#[test]
fn rust_aliases_are_resolved_independently_for_each_owning_crate() {
    let root = tempfile::tempdir().unwrap();
    for dir in ["crates/a/src", "crates/b/src"] {
        std::fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let manifest_a = br#"[package]
name = "a"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "../../vibevm/specmark" }
"#;
    let manifest_b = br#"[package]
name = "b"
version = "0.1.0"
[dependencies]
marks = { package = "core-ai-native-specmark", path = "../../vibevm/specmark" }
"#;
    let source_a = b"specmark::scope!(\"spec://a\");\npub fn a() {}\n";
    let source_b = b"marks::scope!(\"spec://b\");\npub fn b() {}\n";
    let files: [(&str, &[u8]); 4] = [
        ("crates/a/Cargo.toml", manifest_a),
        ("crates/a/src/lib.rs", source_a),
        ("crates/b/Cargo.toml", manifest_b),
        ("crates/b/src/lib.rs", source_b),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "strip-scopes"
kind = "rust-specmark-strip-v1"
patterns = ["crates/**/src/**/*.rs"]
forms = ["scope"]
matches = "one-or-more"
[[rewrite]]
id = "remove-a"
kind = "cargo-package-remove-v1"
manifests = ["crates/a/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
[[rewrite]]
id = "remove-b"
kind = "cargo-package-remove-v1"
manifests = ["crates/b/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["marks"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.blockers.is_empty());
    for path in ["crates/a/src/lib.rs", "crates/b/src/lib.rs"] {
        assert!(prepared.rewrites.iter().any(|row| row.path == path));
    }
}

#[test]
fn rust_source_without_an_owning_manifest_is_a_plan_blocker() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("src")).unwrap();
    let source = b"specmark::scope!(\"spec://orphan\");\npub fn orphan() {}\n";
    std::fs::write(root.path().join("src/lib.rs"), source).unwrap();
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &[("src/lib.rs", source.as_slice())]);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "strip-orphan"
kind = "rust-specmark-strip-v1"
patterns = ["src/**/*.rs"]
forms = ["scope"]
matches = "one-or-more"
[[rewrite]]
id = "unmatched-package-rule"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "zero-or-more"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.rewrites.is_empty());
    assert!(prepared.blockers.iter().any(|blocker| {
        blocker.code == "rust-cargo-ownership-unresolved"
            && blocker.path.as_deref() == Some("src/lib.rs")
    }));
}
