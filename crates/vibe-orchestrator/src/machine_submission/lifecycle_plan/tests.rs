//! Focused pure pre-effect plan tests.

use super::*;
use vibe_core::manifest::SpecFormat;
use vibe_lifecycle::process::capture_package_tree_witness;

fn local_package() -> (tempfile::TempDir, PackageTreeWitness) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("README.md"), b"source\n").unwrap();
    let pin = vibe_core::PackageRef::parse("flow:org.example/fixture@=1.1.0").unwrap();
    let source =
        capture_package_tree_witness(dir.path(), &pin, &["vibe.toml", "README.md"]).unwrap();
    (dir, source)
}

#[test]
fn source_bound_install_has_separate_v3_bytes_and_no_opaque_identity() {
    let (dir, source) = local_package();
    let selected = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let mut input = model(dir.path(), &selected, &policy, &install, &mode);
    input.requested = Phase::Install;
    let v2 = canonical_lifecycle_pre_effect_plan(&input).unwrap();
    input.package_source = Some(&source);
    let v3 = canonical_lifecycle_pre_effect_plan(&input).unwrap();
    assert_ne!(v2, v3);
    let value: Value = serde_json::from_slice(&v3).unwrap();
    assert_eq!(value["format"], FORMAT_SOURCE_V3);
    assert_eq!(
        value["source"]["package_pin"],
        source.package_pin.to_string()
    );
    assert_eq!(value["source"]["tree_sha256"], source.tree_sha256);
    assert_eq!(value["source"]["files"][0]["path"], "README.md");
    assert_eq!(value["source"]["files"][1]["path"], "vibe.toml");
    assert_eq!(
        value["source"]["tree_recipe"],
        "vibevm.lifecycle.package-tree/1"
    );
    assert!(value["source"].get("root_identity").is_none());
    assert!(value["source"]["root"].is_object());
    std::fs::write(dir.path().join("README.md"), b"Source\n").unwrap();
    let changed =
        capture_package_tree_witness(dir.path(), &source.package_pin, &["vibe.toml", "README.md"])
            .unwrap();
    input.package_source = Some(&changed);
    assert_ne!(canonical_lifecycle_pre_effect_plan(&input).unwrap(), v3);
}

#[test]
fn source_bound_plan_refuses_other_phase_and_fabricated_digest() {
    let (dir, source) = local_package();
    let mut fabricated = source.clone();
    fabricated.tree_sha256 = format!("sha256:{}", "0".repeat(64));
    let selected = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let mut input = model(dir.path(), &selected, &policy, &install, &mode);
    input.package_source = Some(&source);
    assert!(
        canonical_lifecycle_pre_effect_plan(&input)
            .unwrap_err()
            .to_string()
            .contains("only valid for Install")
    );
    input.requested = Phase::Install;
    input.package_source = Some(&fabricated);
    assert!(
        canonical_lifecycle_pre_effect_plan(&input)
            .unwrap_err()
            .to_string()
            .contains("tree digest")
    );
}

fn witness(byte: u8) -> SelectedFileWitness {
    SelectedFileWitness::from_bytes(vec![byte])
}

#[test]
fn v2_bytes_remain_exact_without_package_source() {
    let root = Path::new("/vibevm/fixture");
    let sources = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let bytes =
        canonical_lifecycle_pre_effect_plan(&model(root, &sources, &policy, &install, &mode))
            .unwrap();
    const EXPECTED: &str = r#"{"agent_mode":"cli","assume_yes":false,"chain":["validate","install","generate","build","test","create"],"effective_trace":false,"force":false,"format":"vibevm.lifecycle.pre-effect-plan/2","install":{"all_features":false,"exact":false,"features":[],"language":null,"no_default_features":false,"packages":[]},"lease_root":{"encoding":"unix-bytes-hex","value":"2f76696265766d2f66697874757265"},"lock":null,"manifest":{"sha256":"sha256:ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb","size":1},"policy":{"offline":false,"slot_integrity":"trust-presence","spec_format_default":null},"requested":"create","selected_node":".","selected_root":{"encoding":"unix-bytes-hex","value":"2f76696265766d2f66697874757265"},"trace_flag":false,"workspace_lock":null,"workspace_manifest":{"sha256":"sha256:ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb","size":1}}"#;
    #[cfg(unix)]
    assert_eq!(bytes, EXPECTED.as_bytes());
    #[cfg(not(unix))]
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap()["format"],
        FORMAT_V2
    );
}

fn model<'a>(
    root: &'a Path,
    sources: &'a SelectedManifestLockWitness,
    policy: &'a InstallPolicy,
    install: &'a InstallInputs,
    mode: &'a RunAgentMode,
) -> LifecyclePreEffectPlanInput<'a> {
    LifecyclePreEffectPlanInput {
        lease_root: root,
        selected_root: root,
        selected_node: ".",
        witness: sources,
        workspace_witness: sources,
        package_source: None,
        requested: Phase::Create,
        agent_mode: mode,
        trace_flag: false,
        effective_trace: false,
        force: false,
        assume_yes: false,
        policy,
        install_inputs: install,
    }
}

#[test]
fn same_inputs_have_same_bytes_and_no_identity_or_clock_field() {
    let dir = tempfile::tempdir().unwrap();
    let sources = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let input = model(dir.path(), &sources, &policy, &install, &mode);
    // Exact-field destructuring fails to compile if a future edit adds a
    // random run ID or start clock to this input type.
    let LifecyclePreEffectPlanInput {
        lease_root,
        selected_root,
        selected_node,
        witness,
        workspace_witness,
        package_source,
        requested,
        agent_mode,
        trace_flag,
        effective_trace,
        force,
        assume_yes,
        policy,
        install_inputs,
    } = &input;
    let _ = (
        lease_root,
        selected_root,
        selected_node,
        witness,
        workspace_witness,
        package_source,
        requested,
        agent_mode,
        trace_flag,
        effective_trace,
        force,
        assume_yes,
        policy,
        install_inputs,
    );
    let before = std::fs::read_dir(dir.path()).unwrap().count();
    let first = canonical_lifecycle_pre_effect_plan(&input).unwrap();
    let second = canonical_lifecycle_pre_effect_plan(&input).unwrap();
    assert_eq!(first, second);
    assert!(first.starts_with(b"{\"agent_mode\":\"cli\","));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    let parsed: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(parsed["format"], "vibevm.lifecycle.pre-effect-plan/2");
    assert_eq!(parsed["chain"][0], "validate");
    assert_eq!(parsed["requested"], "create");
    assert_eq!(parsed["manifest"], parsed["workspace_manifest"]);
    assert_eq!(parsed["lock"], parsed["workspace_lock"]);
    assert!(parsed.get("run_id").is_none() && parsed.get("started").is_none());
}

#[test]
fn workspace_manifest_and_lock_each_change_member_plan_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let member = dir.path().join("member");
    let selected = SelectedManifestLockWitness {
        manifest: witness(b's'),
        lock: None,
    };
    let workspace = SelectedManifestLockWitness {
        manifest: witness(b'w'),
        lock: None,
    };
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let mut input = model(dir.path(), &selected, &policy, &install, &mode);
    input.selected_root = &member;
    input.selected_node = "member";
    input.workspace_witness = &workspace;
    let baseline = canonical_lifecycle_pre_effect_plan(&input).unwrap();
    let changed_manifest = SelectedManifestLockWitness {
        manifest: witness(b'x'),
        lock: None,
    };
    input.workspace_witness = &changed_manifest;
    assert_changed(input, &baseline);

    let changed_lock = SelectedManifestLockWitness {
        manifest: witness(b'w'),
        lock: Some(witness(b'l')),
    };
    let mut input = model(dir.path(), &selected, &policy, &install, &mode);
    input.selected_root = &member;
    input.selected_node = "member";
    input.workspace_witness = &changed_lock;
    assert_changed(input, &baseline);
}

#[test]
fn fabricated_workspace_digest_refuses_before_plan_serialization() {
    let dir = tempfile::tempdir().unwrap();
    let selected = SelectedManifestLockWitness {
        manifest: witness(b's'),
        lock: None,
    };
    let mut workspace = SelectedManifestLockWitness {
        manifest: witness(b'w'),
        lock: None,
    };
    workspace.manifest.sha256 = format!("sha256:{}", "b".repeat(64));
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let member = dir.path().join("member");
    let mut input = model(dir.path(), &selected, &policy, &install, &mode);
    input.selected_root = &member;
    input.workspace_witness = &workspace;
    let error = canonical_lifecycle_pre_effect_plan(&input)
        .unwrap_err()
        .to_string();
    assert!(error.contains("digest does not match bytes"), "{error}");
}

fn assert_changed(input: LifecyclePreEffectPlanInput<'_>, baseline: &[u8]) {
    assert_ne!(
        canonical_lifecycle_pre_effect_plan(&input).unwrap(),
        baseline
    );
}

#[test]
fn critical_posture_witness_and_path_changes_change_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let sources = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    let mut policy = InstallPolicy::default();
    let mut install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let baseline =
        canonical_lifecycle_pre_effect_plan(&model(dir.path(), &sources, &policy, &install, &mode))
            .unwrap();

    let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
    changed.effective_trace = true;
    assert_changed(changed, &baseline);
    let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
    changed.trace_flag = true;
    assert_changed(changed, &baseline);
    let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
    changed.force = true;
    assert_changed(changed, &baseline);
    let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
    changed.assume_yes = true;
    assert_changed(changed, &baseline);
    let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
    changed.requested = Phase::Build;
    assert_changed(changed, &baseline);
    let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
    changed.selected_node = "member";
    assert_changed(changed, &baseline);
    let other_root = dir.path().join("other");
    let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
    changed.selected_root = &other_root;
    assert_changed(changed, &baseline);
    let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
    changed.lease_root = &other_root;
    assert_changed(changed, &baseline);
    let agent_mode = RunAgentMode::Agent;
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &agent_mode),
        &baseline,
    );

    let with_lock = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: Some(witness(b'b')),
    };
    assert_changed(
        model(dir.path(), &with_lock, &policy, &install, &mode),
        &baseline,
    );
    let changed_sources = SelectedManifestLockWitness {
        manifest: witness(b'b'),
        lock: None,
    };
    assert_changed(
        model(dir.path(), &changed_sources, &policy, &install, &mode),
        &baseline,
    );

    policy.offline = true;
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
    policy.offline = false;
    policy.spec_format_default = Some(SpecFormat::Xml);
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
    policy.spec_format_default = None;
    policy.slot_integrity = SlotIntegrity::Verify;
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
    policy.slot_integrity = SlotIntegrity::TrustPresence;

    install.packages.push("org.example/demo".into());
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
    install.packages.clear();
    install.features.push("extra".into());
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
    install.features.clear();
    install.language = Some("rust".into());
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
    install.language = None;
    install.no_default_features = true;
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
    install.no_default_features = false;
    install.all_features = true;
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
    install.all_features = false;
    install.exact = true;
    assert_changed(
        model(dir.path(), &sources, &policy, &install, &mode),
        &baseline,
    );
}

#[test]
fn oversized_fields_refuse_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let sources = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    let policy = InstallPolicy::default();
    let mut install = InstallInputs::default();
    install.packages.push("x".repeat(INPUT_STRING_CAP + 1));
    let mode = RunAgentMode::Cli;
    let before = std::fs::read_dir(dir.path()).unwrap().count();
    let error =
        canonical_lifecycle_pre_effect_plan(&model(dir.path(), &sources, &policy, &install, &mode))
            .unwrap_err()
            .to_string();
    assert!(error.contains("packages exceeds"), "{error}");
    install.packages = vec!["x".repeat(3000); INPUT_COUNT_CAP];
    let error =
        canonical_lifecycle_pre_effect_plan(&model(dir.path(), &sources, &policy, &install, &mode))
            .unwrap_err()
            .to_string();
    assert!(error.contains("canonical_plan exceeds"), "{error}");
    install.packages.clear();
    let oversized_root = dir.path().join("x".repeat(PATH_CAP + 1));
    let mut input = model(dir.path(), &sources, &policy, &install, &mode);
    input.selected_root = &oversized_root;
    let error = canonical_lifecycle_pre_effect_plan(&input)
        .unwrap_err()
        .to_string();
    assert!(error.contains("selected_root exceeds"), "{error}");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
}

#[test]
fn fabricated_same_format_digest_refuses_before_serialization() {
    let dir = tempfile::tempdir().unwrap();
    let mut sources = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    sources.manifest.sha256 = format!("sha256:{}", "b".repeat(64));
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let before = std::fs::read_dir(dir.path()).unwrap().count();
    let error =
        canonical_lifecycle_pre_effect_plan(&model(dir.path(), &sources, &policy, &install, &mode))
            .unwrap_err()
            .to_string();
    assert!(error.contains("digest does not match bytes"), "{error}");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
}

#[cfg(unix)]
#[test]
fn unix_path_encoding_preserves_non_utf8_bytes_without_writing() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let selected = dir.path().join(OsStr::from_bytes(b"member-\xff"));
    let sources = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let mut input = model(dir.path(), &sources, &policy, &install, &mode);
    input.selected_root = &selected;
    let before = std::fs::read_dir(dir.path()).unwrap().count();
    let bytes = canonical_lifecycle_pre_effect_plan(&input).unwrap();
    let parsed: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(parsed["selected_root"]["encoding"], "unix-bytes-hex");
    assert!(
        parsed["selected_root"]["value"]
            .as_str()
            .unwrap()
            .ends_with("ff")
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
}

#[cfg(windows)]
#[test]
fn windows_path_encoding_preserves_wide_units_without_writing() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let selected = dir.path().join(OsString::from_wide(&[0xd800]));
    let sources = SelectedManifestLockWitness {
        manifest: witness(b'a'),
        lock: None,
    };
    let policy = InstallPolicy::default();
    let install = InstallInputs::default();
    let mode = RunAgentMode::Cli;
    let mut input = model(dir.path(), &sources, &policy, &install, &mode);
    input.selected_root = &selected;
    let before = std::fs::read_dir(dir.path()).unwrap().count();
    let bytes = canonical_lifecycle_pre_effect_plan(&input).unwrap();
    let parsed: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        parsed["selected_root"]["encoding"],
        "windows-utf16-units-hex"
    );
    assert!(
        parsed["selected_root"]["value"]
            .as_str()
            .unwrap()
            .ends_with("d800")
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
}
