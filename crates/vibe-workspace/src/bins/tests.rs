use super::*;
use vibe_core::Group;

const LOCK: &str = r#"
[meta]
generated_by = "vibe-test"
generated_at = "2026-07-07T00:00:00Z"
schema_version = 6

[[package]]
kind = "stack"
group = "org.vibevm"
name = "typescript-ai-native-lang"
version = "0.4.0"
registry = "vibespecs"
source_url = "file://packages"
source_ref = "v0.4.0"
content_hash = "sha256:deadbeef"
files_written = []
"#;

fn fixture_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .expect("vibe.toml");
    std::fs::write(dir.path().join("vibe.lock"), LOCK).expect("vibe.lock");
    let slot = dir
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.vibevm.typescript-ai-native-lang")
        .join("0.4.0");
    write_provider_manifest(
        &slot,
        "org.vibevm",
        "typescript-ai-native-lang",
        "0.4.0",
        "typescript-ai-native-tcg",
    );
    dir
}

fn write_provider_manifest(
    slot: &std::path::Path,
    group: &str,
    package: &str,
    version: &str,
    binary: &str,
) {
    std::fs::create_dir_all(slot).expect("slot");
    std::fs::write(
        slot.join("vibe.toml"),
        format!(
            r#"[package]
name = "{package}"
group = "{group}"
kind = "stack"
version = "{version}"
authors = ["x"]
license = "EULA"
description = "fixture"
keywords = []

[[binary]]
name = "{binary}"
crate = "crates/{binary}"
"#
        ),
    )
    .expect("slot manifest");
}

#[test]
fn collect_walks_lockfile_slots_and_sorts() {
    let dir = fixture_project();
    let bins = collect_binaries(dir.path()).expect("collect");
    assert_eq!(bins.len(), 1);
    assert_eq!(bins[0].decl.name, "typescript-ai-native-tcg");
    assert_eq!(bins[0].group, "org.vibevm");
    assert_eq!(
        bins[0].vibedeps_root,
        dir.path().join(vibe_core::layout::current_vibedeps_root())
    );
    assert!(bins[0].artifact().to_string_lossy().contains("release"));
}

#[test]
fn artifact_prefers_debug_over_release() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = DeclaredBinary {
        decl: vibe_core::manifest::BinaryDecl {
            name: "typescript-ai-native-tcg".into(),
            crate_dir: "crates/typescript-ai-native-tcg".into(),
            description: None,
        },
        package: "org.vibevm/typescript-ai-native-lang".into(),
        group: "org.vibevm".into(),
        vibedeps_root: dir.path().to_path_buf(),
        slot: dir.path().to_path_buf(),
    };
    assert_eq!(bin.artifact(), bin.release_artifact());
    let debug = bin.debug_artifact();
    std::fs::create_dir_all(debug.parent().expect("debug parent")).expect("debug dir");
    std::fs::write(&debug, b"stub").expect("debug artifact");
    assert_eq!(bin.artifact(), debug);
    let release = bin.release_artifact();
    std::fs::create_dir_all(release.parent().expect("release parent")).expect("release dir");
    std::fs::write(&release, b"stub").expect("release artifact");
    assert_eq!(bin.artifact(), bin.debug_artifact());
}

#[test]
fn missing_lockfile_is_an_empty_set() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .expect("vibe.toml");
    assert!(collect_binaries(dir.path()).expect("collect").is_empty());
}

#[test]
fn unknown_binary_names_the_known_set() {
    let dir = fixture_project();
    let bins = collect_binaries(dir.path()).expect("collect");
    let err = find_binary(&bins, "nope").expect_err("unknown");
    let msg = err.to_string();
    assert!(
        msg.contains("nope") && msg.contains("typescript-ai-native-tcg"),
        "{msg}"
    );
}

#[test]
fn explicit_build_authorization_preserves_direct_consent_semantics() {
    let dir = fixture_project();
    let mut foreign = collect_binaries(dir.path()).unwrap().remove(0);
    assert!(
        build::authorize_build(
            &foreign,
            BuildAuthorization::ExplicitOperator { assume_yes: false }
        )
        .is_ok(),
        "the historical org.vibevm direct-build allow-list remains"
    );
    foreign.group = "com.example".to_string();
    foreign.package = "com.example/thing".to_string();

    let direct = BuildAuthorization::ExplicitOperator { assume_yes: false };
    let error = build::authorize_build(&foreign, direct).expect_err("direct gate");
    assert!(error.to_string().contains("--assume-yes"), "{error}");
    assert!(
        build::authorize_build(
            &foreign,
            BuildAuthorization::ExplicitOperator { assume_yes: true }
        )
        .is_ok()
    );
    assert!(
        build::authorize_build(
            &foreign,
            BuildAuthorization::InstalledExtension {
                home: BinaryProviderHome::InstalledSlot,
            },
        )
        .is_ok(),
        "installing the extension is the authorization"
    );
}

#[test]
fn provider_scoped_lookup_disambiguates_colliding_binary_names() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(vibe_core::layout::current_vibedeps_root());
    let alpha = root.join("org.alpha.tools").join("1.0.0");
    let beta = root.join("org.beta.tools").join("2.0.0");
    write_provider_manifest(&alpha, "org.alpha", "tools", "1.0.0", "runner");
    write_provider_manifest(&beta, "org.beta", "tools", "2.0.0", "runner");

    let alpha_bin = find_binary_in_provider_slot(
        &alpha,
        &Group::parse("org.alpha").unwrap(),
        "tools",
        "1.0.0",
        "runner",
    )
    .unwrap();
    let beta_bin = find_binary_in_provider_slot(
        &beta,
        &Group::parse("org.beta").unwrap(),
        "tools",
        "2.0.0",
        "runner",
    )
    .unwrap();

    assert_eq!(alpha_bin.package, "org.alpha/tools");
    assert_eq!(alpha_bin.slot, alpha);
    assert_eq!(beta_bin.package, "org.beta/tools");
    assert_eq!(beta_bin.slot, beta);
}

#[test]
fn provider_scoped_lookup_rejects_a_coordinate_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let slot = dir.path().join("org.alpha.tools/1.0.0");
    write_provider_manifest(&slot, "org.alpha", "tools", "1.0.0", "runner");
    let error = find_binary_in_provider_slot(
        &slot,
        &Group::parse("org.beta").unwrap(),
        "tools",
        "1.0.0",
        "runner",
    )
    .expect_err("coordinate mismatch");
    assert!(matches!(error, BinsError::ProviderMismatch { .. }));
}
