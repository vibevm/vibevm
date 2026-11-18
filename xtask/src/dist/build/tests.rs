use super::*;

#[test]
fn git_bash_selection_uses_the_observed_native_installation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Git installation with spaces");
    let bash = root.join("bin/bash.exe");
    fs::create_dir_all(bash.parent().unwrap()).unwrap();
    fs::write(&bash, b"fixture executable").unwrap();
    for architecture in ["mingw64", "mingw32"] {
        let exec_path = root.join(architecture).join("libexec/git-core");
        assert_eq!(bash::git_bash_from_exec_path(&exec_path).unwrap(), bash);
    }
    assert!(bash::git_bash_from_exec_path(&root.join("libexec/git-core")).is_err());
    fs::remove_file(&bash).unwrap();
    assert!(bash::git_bash_from_exec_path(&root.join("mingw64/libexec/git-core")).is_err());
    fs::create_dir(&bash).unwrap();
    assert!(bash::git_bash_from_exec_path(&root.join("mingw64/libexec/git-core")).is_err());
}

#[test]
fn artifact_names_are_version_and_target_exact() {
    let version = Version::parse("1.2.3").unwrap();
    assert_eq!(
        bundle_asset_name(&version, "aarch64-apple-darwin"),
        "vibevm-1.2.3-aarch64-apple-darwin.zip"
    );
    assert_eq!(
        fragment_asset_name(&version, "aarch64-apple-darwin"),
        "vibevm-1.2.3-aarch64-apple-darwin.fragment.json"
    );
    assert_eq!(aggregate_asset_name(), "DISTRIBUTIONS.json");
    assert_eq!(
        bootstrap_asset_name("x86_64-pc-windows-msvc"),
        "vibe-bootstrap-x86_64-pc-windows-msvc.exe"
    );
}

#[test]
fn cargo_messages_select_the_three_products_and_native_installer() {
    let temp = tempfile::tempdir().unwrap();
    let vibe = temp.path().join("vibe");
    let index = temp.path().join("vibe-index");
    let proxy = temp.path().join("vibe-app-proxy");
    let installer = temp.path().join("vibe-installer");
    std::fs::write(&vibe, b"vibe").unwrap();
    std::fs::write(&index, b"index").unwrap();
    std::fs::write(&proxy, b"proxy").unwrap();
    std::fs::write(&installer, b"installer").unwrap();
    let lines = [
        serde_json::json!({
            "reason": "compiler-artifact",
            "target": {"name": "vibe-installer", "kind": ["bin"]},
            "executable": installer
        }),
        serde_json::json!({
            "reason": "build-script-executed",
            "target": {"name": "ignored", "kind": ["bin"]},
            "executable": temp.path().join("ignored")
        }),
        serde_json::json!({
            "reason": "compiler-artifact",
            "target": {"name": "vibe", "kind": ["bin"]},
            "executable": vibe
        }),
        serde_json::json!({
            "reason": "compiler-artifact",
            "target": {"name": "vibe-index", "kind": ["bin"]},
            "executable": index
        }),
        serde_json::json!({
            "reason": "compiler-artifact",
            "target": {"name": "vibe-app-proxy", "kind": ["bin"]},
            "executable": proxy
        }),
    ];
    let input = lines
        .into_iter()
        .map(|line| serde_json::to_string(&line).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let selected = select_binaries(input.as_bytes()).unwrap();
    assert_eq!(selected.len(), 4);
    assert!(selected["vibe-installer"].is_file());
    assert!(selected[COMPONENT_VIBE].is_file());
    assert!(selected[COMPONENT_INDEX].is_file());
    assert!(selected[COMPONENT_PROXY].is_file());
}

#[test]
fn supported_target_inventory_is_closed() {
    assert_eq!(SUPPORTED_DISTRIBUTION_TARGETS.len(), 5);
    assert!(SUPPORTED_DISTRIBUTION_TARGETS.contains(&"x86_64-unknown-linux-musl"));
    assert!(SUPPORTED_DISTRIBUTION_TARGETS.contains(&"x86_64-unknown-linux-gnu"));
}

#[test]
fn cargo_build_respects_explicit_incremental_override() {
    let source = include_str!("../build.rs");
    assert!(source.contains("std::env::var_os(\"CARGO_INCREMENTAL\")"));
    assert!(source.contains("unwrap_or_else(|| \"0\".into())"));
}
