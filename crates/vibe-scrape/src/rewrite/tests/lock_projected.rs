#[test]
fn workspace_member_binds_root_lock_and_root_dependency_identity() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("crates/member/src")).unwrap();
    let workspace = br#"[workspace]
members = ["crates/*"]
resolver = "3"
[workspace.dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
"#;
    let member = br#"[package]
name = "member"
version = "0.1.0"
edition = "2024"
[dependencies]
specmark = { workspace = true }
"#;
    let source = b"specmark::scope!(\"spec://member\");\npub fn member() {}\n";
    let lock = br#"version = 4
[[package]]
name = "member"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    let files: [(&str, &[u8]); 4] = [
        ("Cargo.toml", workspace),
        ("Cargo.lock", lock),
        ("crates/member/Cargo.toml", member),
        ("crates/member/src/lib.rs", source),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "strip-member"
kind = "rust-specmark-strip-v1"
patterns = ["crates/member/src/**/*.rs"]
forms = ["scope"]
matches = "one-or-more"
[[rewrite]]
id = "remove-member-specmark"
kind = "cargo-package-remove-v1"
manifests = ["crates/member/Cargo.toml"]
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
            .any(|row| row.path == "crates/member/src/lib.rs")
    );
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "crates/member/Cargo.toml")
    );
    assert!(prepared.rewrites.iter().any(|row| {
        row.path == "Cargo.lock"
            && row
                .native_lock_change
                .as_ref()
                .is_some_and(|change| change.authorizing_rewrite_id == "remove-member-specmark")
    }));
}

#[test]
fn workspace_membership_ambiguity_blocks_all_manifest_and_lock_rewrites() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("nested/member")).unwrap();
    let outer = br#"[workspace]
members = ["nested/member"]
[workspace.dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
"#;
    let inner = br#"[workspace]
members = ["member"]
[workspace.dependencies]
specmark = { package = "core-ai-native-specmark", path = "../vibevm/specmark" }
"#;
    let member = br#"[package]
name = "member"
version = "0.1.0"
[dependencies]
specmark = { workspace = true }
"#;
    let outer_lock = br#"version = 4
[[package]]
name = "member"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    let files: [(&str, &[u8]); 4] = [
        ("Cargo.toml", outer),
        ("Cargo.lock", outer_lock),
        ("nested/Cargo.toml", inner),
        ("nested/member/Cargo.toml", member),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "ambiguous-member"
kind = "cargo-package-remove-v1"
manifests = ["nested/member/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.rewrites.is_empty());
    assert!(prepared.blockers.iter().any(|blocker| {
        blocker.code == "cargo-ownership-ambiguous"
            && blocker.path.as_deref() == Some("nested/member/Cargo.toml")
    }));
}

#[test]
fn preparation_reports_native_lock_blockers_without_losing_the_plan_census() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("web")).unwrap();
    let package_json = b"{\"dependencies\":{}}\n";
    let package_lock =
        b"{\"lockfileVersion\":3,\"packages\":{\"node_modules/vibe-tool\":{\"version\":\"1\"}}}\n";
    std::fs::write(root.path().join("web/package.json"), package_json).unwrap();
    std::fs::write(root.path().join("web/package-lock.json"), package_lock).unwrap();
    let project = Project::open(root.path()).unwrap();
    let entries = [
        ("web/package.json", package_json.as_slice()),
        ("web/package-lock.json", package_lock.as_slice()),
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
            bytes: Some(u64::try_from(bytes.len()).unwrap()),
            unix_mode: snapshot.unix_mode,
            identity: Some(snapshot.identity),
        }
    })
    .collect::<Vec<_>>();
    let contract = contract_with(
        r#"
[[rewrite]]
id = "node"
kind = "node-package-remove-v1"
package_json = "web/package.json"
lockfile = "web/package-lock.json"
manager = "npm"
packages = ["vibe-tool"]
script_paths = []
config_paths = []
matches = "zero-or-more"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, entries.as_slice()).unwrap();
    assert!(prepared.rewrites.is_empty());
    assert_eq!(prepared.blockers.len(), 1);
    assert_eq!(
        prepared.blockers[0].code,
        "native-lock-reconciliation-required"
    );
    assert_eq!(
        prepared.blockers[0].path.as_deref(),
        Some("web/package-lock.json")
    );
}

#[test]
fn relocation_rejects_file_ancestor_of_mapped_destination() {
    let mut contract = contract_with("");
    contract.relocate.push(crate::contract::Relocation {
        id: "move".to_owned(),
        from: "src".to_owned(),
        to: "release/src".to_owned(),
        conflict: crate::contract::ConflictPolicy::Refuse,
        required: true,
    });
    let inventory = vec![
        InventoryEntry {
            path: "src".to_owned(),
            kind: EntryKind::Directory,
            sha256: None,
            bytes: None,
            unix_mode: None,
            identity: None,
        },
        InventoryEntry {
            path: "src/lib.rs".to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(b"x")),
            bytes: Some(1),
            unix_mode: None,
            identity: None,
        },
        InventoryEntry {
            path: "release".to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(b"red")),
            bytes: Some(3),
            unix_mode: None,
            identity: None,
        },
    ];
    assert!(validate_relocations(&contract, &inventory).is_err());
}

#[test]
fn rust_refuses_invalid_registered_scope_grammar_and_shadowing() {
    let aliases = BTreeSet::from(["specmark".to_owned()]);
    let forms = BTreeSet::from(["scope".to_owned()]);
    assert!(prepare_rust(b"specmark::scope!(value);\n", &aliases, &forms).is_err());
    assert!(prepare_rust(b"specmark::scope!(\"ordinary\");\n", &aliases, &forms).is_err());
    assert!(prepare_rust(b"fn f() { let specmark = 1; }\n", &aliases, &forms).is_err());
}

#[test]
fn cargo_lock_reconciliation_removes_identity_and_exact_edges() {
    let before = br#"version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = ["core-ai-native-specmark", "keep"]

[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"

[[package]]
name = "core-ai-native-specmark-extra"
version = "1.0.0"
"#;
    assert!(matches!(
        prepare_cargo_lock(before, "core-ai-native-specmark"),
        Err(ScrapeError::Blocked(_))
    ));
}

#[test]
fn node_manifest_and_safe_npm_lock_are_structural() {
    let package = br#"{
  "dependencies": { "vibe-tool": "1", "keep": "1" },
  "scripts": { "vibe": "vibe run", "build": "tsc" },
  "nested": { "vibe-tool": "shadow" }
}
"#;
    let packages = vec!["vibe-tool".to_owned()];
    let scripts = vec![vec!["scripts".to_owned(), "vibe".to_owned()]];
    let (after, count, _, _) = prepare_node_manifest(package, &packages, &scripts, &[]).unwrap();
    assert_eq!(count, 2);
    let value: serde_json::Value = serde_json::from_slice(&after).unwrap();
    assert!(value["dependencies"].get("vibe-tool").is_none());
    assert_eq!(value["nested"]["vibe-tool"], "shadow");

    let lock = br#"{
  "name": "app",
  "lockfileVersion": 3,
  "packages": {
    "": { "dependencies": { "vibe-tool": "1", "keep": "1" } },
    "node_modules/vibe-tool": { "version": "1.0.0" }
  }
}
"#;
    assert!(matches!(
        prepare_node_lock(lock, NodeManager::Npm, &packages),
        Err(ScrapeError::Blocked(_))
    ));
    assert!(prepare_node_lock(lock, NodeManager::Pnpm, &packages).is_err());
}

#[test]
fn public_preparation_is_read_only_and_records_exact_preimages() {
    let root = tempfile::tempdir().unwrap();
    let before = b"alpha old omega\r\n";
    std::fs::write(root.path().join("notes.txt"), before).unwrap();
    let contract = Contract::parse(
        format!(
            r#"schema = 1
id = "scrape-test"
[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"
[scope]
closed_roots = ["vibevm"]
outside = "implicit-keep"
[commit]
contract = "preserve"

[[classify]]
id = "keep-notes"
kind = "keep"
patterns = ["notes.txt"]
owner = "project"
require_match = true

[[rewrite]]
id = "replace-old"
kind = "text-exact-replace-v1"
path = "notes.txt"
sha256 = "{}"
before = "old"
after = "new"
occurrences = 1

[[assert]]
id = "old-absent"
kind = "text-literal-absent-v1"
patterns = ["notes.txt"]
needles = ["old"]

[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "deny"
max_stdout_bytes = 1024
max_stderr_bytes = 1024
max_result_bytes = 1048576
termination_grace_seconds = 1

[[healthcheck]]
id = "health"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "python"
argv = []
protocol = "exit-code"
reads = []
writes = []
spawn = false
timeout_seconds = 1
network = "deny"
"#,
            digest(before)
        )
        .as_bytes(),
    )
    .unwrap();
    let project = Project::open(root.path()).unwrap();
    let snapshot = project
        .read_file_snapshot_bounded("notes.txt", before.len())
        .unwrap()
        .unwrap();
    let inventory = vec![InventoryEntry {
        path: "notes.txt".to_owned(),
        kind: EntryKind::File,
        sha256: Some(digest(before)),
        bytes: Some(before.len() as u64),
        unix_mode: None,
        identity: Some(snapshot.identity),
    }];
    let prepared = prepare_rewrites(&project, &contract, inventory.as_slice()).unwrap();
    assert!(prepared.blockers.is_empty());
    assert_eq!(prepared.rewrites.len(), 1);
    assert_eq!(prepared.rewrites[0].before_sha256, digest(before));
    assert_eq!(prepared.rewrites[0].after_bytes, b"alpha new omega\r\n");
    assert_eq!(
        std::fs::read(root.path().join("notes.txt")).unwrap(),
        before
    );

    let mut drifted = inventory.clone();
    drifted[0].sha256 = Some(digest(b"different"));
    assert!(prepare_rewrites(&project, &contract, drifted.as_slice()).is_err());

    let mut relocation_contract = contract.clone();
    relocation_contract
        .relocate
        .push(crate::contract::Relocation {
            id: "move-notes".to_owned(),
            from: "notes.txt".to_owned(),
            to: "release/notes.txt".to_owned(),
            conflict: crate::contract::ConflictPolicy::Refuse,
            required: true,
        });
    assert!(validate_relocations(&relocation_contract, &inventory).is_ok());
    let mut collided = inventory.clone();
    collided.push(InventoryEntry {
        path: "release/notes.txt".to_owned(),
        kind: EntryKind::File,
        sha256: Some(digest(b"collision")),
        bytes: Some(9),
        unix_mode: None,
        identity: Some(snapshot.identity),
    });
    assert!(validate_relocations(&relocation_contract, &collided).is_err());
}
