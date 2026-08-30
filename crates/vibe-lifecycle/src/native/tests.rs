use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::tempdir;
use vibe_core::manifest::{
    ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionsControl, MechanismDecl,
    MechanismFreshness, MechanismRole, MechanismRoutes,
};

use crate::mechanism::cargo::message::{CargoMetadata, MetadataPackage};
use crate::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
    collect_mechanisms,
};

use super::cargo::{metadata_target_for_test, select_filename_for_test, select_metadata_for_test};
use super::path::contained_regular;
use super::provider::{ProviderFacts, ProviderHome};
use super::witness::source_witness;
use super::*;

fn native_decl(
    id: &str,
    crate_dir: Option<&str>,
    prebuilt: Option<BTreeMap<String, PathBuf>>,
    mode: Option<&str>,
) -> ExtensionDecl {
    ExtensionDecl {
        id: id.to_owned(),
        point: "phase:build".parse().unwrap(),
        handler: ExtensionHandler::Native {
            crate_dir: crate_dir.map(PathBuf::from),
            prebuilt,
        },
        config: mode.map(|mode| {
            ExtensionConfig::from_table(toml::from_str(&format!("mode = {mode:?}")).unwrap())
        }),
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    }
}

fn world(root: &Path, declarations: Vec<ExtensionDecl>) -> ExtensionWorld {
    ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("native-fixture"),
                root: root.to_path_buf(),
                version: "0.1.0".to_owned(),
                kind: None,
                content_hash: None,
            },
            declarations,
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    }
}

fn facts(root: &Path) -> ProviderFacts {
    ProviderFacts {
        identity: "__host__/native-fixture".to_owned(),
        root: root.to_path_buf(),
        version: "0.1.0".to_owned(),
        content_hash: None,
        home: ProviderHome::Host,
    }
}

fn current_platform() -> NativePlatform {
    NativePlatform::from_pair(std::env::consts::OS, std::env::consts::ARCH).unwrap()
}

fn write_cdylib_fixture(root: &Path) {
    let vibe_ext = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("vibe-ext")
        .display()
        .to_string()
        .replace('\\', "/");
    fs::create_dir_all(root.join("native/src")).unwrap();
    fs::write(
        root.join("native/Cargo.toml"),
        format!(
            "[package]\nname = \"native-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\nvibe-ext = {{ path = {vibe_ext:?} }}\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join("native/src/lib.rs"),
        r#"use vibe_ext::{Context, Manifest, ManifestExtension, Reply, ReplyStatus};

fn handle(_context: Context) -> Reply {
    Reply {
        artifacts: Vec::new(),
        envelope: 1,
        status: ReplyStatus::Ok,
        message: Some("fixture".to_owned()),
    }
}

vibe_ext::vibe_extension!(
    manifest = Manifest {
        extensions: vec![ManifestExtension {
            id: "first".to_owned(),
            point: "phase:build".to_owned(),
            ir_schema: None,
        }],
    },
    handler = handle,
);
"#,
    )
    .unwrap();
}

#[test]
fn platform_pairs_and_suffixes_are_closed_and_exact() {
    let cases = [
        ("windows", "x86_64", NativePlatform::WindowsX86_64, ".dll"),
        ("linux", "x86_64", NativePlatform::LinuxX86_64, ".so"),
        ("macos", "aarch64", NativePlatform::MacosAarch64, ".dylib"),
    ];
    for (os, arch, expected, suffix) in cases {
        let actual = NativePlatform::from_pair(os, arch).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.key(), format!("{os}-{arch}"));
        assert_eq!(actual.suffix(), suffix);
    }
    assert!(NativePlatform::from_pair("windows", "aarch64").is_err());
    assert!(NativePlatform::from_pair("darwin", "aarch64").is_err());
}

#[test]
fn current_prebuilt_wins_and_corruption_never_falls_back() {
    let root = tempdir().unwrap();
    let platform = current_platform();
    fs::create_dir_all(root.path().join("prebuilt")).unwrap();
    let valid_relative = PathBuf::from(format!("prebuilt/native{}", platform.suffix()));
    fs::write(root.path().join(&valid_relative), b"native prebuilt").unwrap();
    let mut valid_entries = BTreeMap::new();
    valid_entries.insert(platform.key().to_owned(), valid_relative);
    let valid_world = world(
        root.path(),
        vec![native_decl(
            "native",
            Some("native"),
            Some(valid_entries),
            None,
        )],
    );
    let valid_extensions = collect_extensions(valid_world.clone()).unwrap();
    let valid_mechanisms = collect_mechanisms(&valid_world).unwrap();
    let valid_candidates = valid_extensions.rows().iter().collect::<Vec<_>>();
    let valid_execution = NativeBuildExecution {
        candidates: &valid_candidates,
        selected_project_root: root.path(),
        registry: &valid_mechanisms,
        routes: &MechanismRoutes::default(),
        platform,
        offline: true,
        created_at: "2026-08-31T00:00:00Z",
    };
    let resolved = resolve_native_artifact(&valid_execution, valid_candidates[0]).unwrap();
    assert_eq!(resolved.origin, NativeArtifactOrigin::Prebuilt);

    let mut entries = BTreeMap::new();
    entries.insert(
        platform.key().to_owned(),
        PathBuf::from(format!("prebuilt/missing{}", platform.suffix())),
    );
    let owned = world(
        root.path(),
        vec![native_decl("native", Some("native"), Some(entries), None)],
    );
    let extensions = collect_extensions(owned.clone()).unwrap();
    let mechanisms = collect_mechanisms(&owned).unwrap();
    let candidates = extensions.rows().iter().collect::<Vec<_>>();
    let execution = NativeBuildExecution {
        candidates: &candidates,
        selected_project_root: root.path(),
        registry: &mechanisms,
        routes: &MechanismRoutes::default(),
        platform,
        offline: true,
        created_at: "2026-08-31T00:00:00Z",
    };
    assert!(matches!(
        build_native_sources(&execution),
        Err(NativeArtifactError::PrebuiltUnavailable { .. })
    ));

    let mut foreign_only = BTreeMap::new();
    foreign_only.insert(
        "foreign-arch".to_owned(),
        PathBuf::from("unused/foreign.bin"),
    );
    let fallback_world = world(
        root.path(),
        vec![native_decl(
            "native",
            Some("native"),
            Some(foreign_only),
            None,
        )],
    );
    let fallback = collect_extensions(fallback_world).unwrap();
    let fallback_rows = fallback.rows().iter().collect::<Vec<_>>();
    assert_eq!(
        source_groups(&fallback_rows, platform, true).unwrap().len(),
        1
    );
}

#[test]
fn canonical_prebuilt_escape_is_refused() {
    let root = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let platform = current_platform();
    let name = format!("escape{}", platform.suffix());
    fs::write(outside.path().join(&name), b"outside").unwrap();
    let canonical_root = root.path().canonicalize().unwrap();
    let error = contained_regular(&outside.path().join(&name), &canonical_root).unwrap_err();
    assert_eq!(error, "canonical path escapes its owning root");
}

#[test]
fn metadata_requires_one_manifest_owner_and_one_cdylib() {
    let root = tempdir().unwrap();
    let manifest = root.path().join("Cargo.toml");
    fs::write(&manifest, "[package]\nname='fixture'\nversion='0.1.0'\n").unwrap();
    let package = |targets| MetadataPackage {
        id: "path+file:///fixture#0.1.0".to_owned(),
        name: "fixture".to_owned(),
        manifest_path: manifest.display().to_string(),
        targets,
    };
    let valid = CargoMetadata {
        packages: vec![package(vec![metadata_target_for_test(
            "fixture",
            &["cdylib"],
        )])],
    };
    assert!(select_metadata_for_test(&facts(root.path()), &manifest, &valid).is_ok());
    let ambiguous = CargoMetadata {
        packages: vec![package(vec![
            metadata_target_for_test("one", &["cdylib"]),
            metadata_target_for_test("two", &["cdylib"]),
        ])],
    };
    assert!(matches!(
        select_metadata_for_test(&facts(root.path()), &manifest, &ambiguous),
        Err(NativeArtifactError::CdylibTarget { found: 2, .. })
    ));
}

#[test]
fn cargo_filenames_are_selected_by_unique_suffix_not_order() {
    let root = tempdir().unwrap();
    let target = root.path().join("target");
    fs::create_dir_all(&target).unwrap();
    let platform = current_platform();
    let first = target.join(format!("first{}", platform.suffix()));
    let second = target.join(format!("second{}", platform.suffix()));
    fs::write(&first, b"first").unwrap();
    fs::write(&second, b"second").unwrap();
    let filenames = vec![
        first.display().to_string(),
        target.join("foreign.rlib").display().to_string(),
        second.display().to_string(),
    ];
    assert!(matches!(
        select_filename_for_test(&facts(root.path()), platform, &target, &filenames),
        Err(NativeArtifactError::CdylibFilename { found: 2, .. })
    ));
}

#[test]
fn dependency_ignore_is_scoped_and_host_is_inert() {
    let installed = tempdir().unwrap();
    let dep_root = installed
        .path()
        .join(vibe_workspace::vibedeps::VIBEDEPS_DIR)
        .join("org.example+native")
        .join("1.0.0");
    fs::create_dir_all(&dep_root).unwrap();
    let dependency = ProviderFacts {
        identity: "org.example/native".to_owned(),
        root: dep_root,
        version: "1.0.0".to_owned(),
        content_hash: Some(format!("sha256:{}", "0".repeat(64))),
        home: ProviderHome::Dependency,
    };
    prepare_dependency_ignore(&dependency).unwrap();
    assert!(
        installed
            .path()
            .join(vibe_workspace::vibedeps::VIBEDEPS_DIR)
            .join(".gitignore")
            .is_file()
    );

    let unrelated = tempdir().unwrap();
    prepare_dependency_ignore(&facts(unrelated.path())).unwrap();
    assert!(!unrelated.path().join(".gitignore").exists());
}

#[test]
fn host_route_displacing_build_cargo_refuses_unlanded_transport() {
    let root = tempdir().unwrap();
    let mut owned = world(root.path(), Vec::new());
    owned.host.mechanisms.push(MechanismDecl {
        id: "cargo-alt".to_owned(),
        role: MechanismRole::Build,
        name: "cargo".to_owned(),
        handler: ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from("native")),
            prebuilt: None,
        },
        protocol: 1,
        config_schema: PathBuf::from("schemas/build-cargo.jtd.json"),
        freshness: MechanismFreshness::Provider,
    });
    let registry = collect_mechanisms(&owned).unwrap();
    let displaced = registry
        .rows()
        .iter()
        .find(|row| !row.is_builtin() && row.key().to_string() == "build:cargo")
        .unwrap();
    let mut routes = MechanismRoutes::default();
    routes.insert("build:cargo".parse().unwrap(), displaced.pin().clone());
    let candidates = Vec::new();
    let execution = NativeBuildExecution {
        candidates: &candidates,
        selected_project_root: root.path(),
        registry: &registry,
        routes: &routes,
        platform: current_platform(),
        offline: true,
        created_at: "2026-08-31T00:00:00Z",
    };

    assert!(matches!(
        select_build_provider(&execution),
        Err(NativeArtifactError::TransportNotLanded { provider, kind })
            if provider == displaced.pin().to_string() && kind == "native"
    ));
}

#[test]
fn source_witness_binds_the_complete_labelled_content_hash() {
    let root = tempdir().unwrap();
    let provider = |content_hash: String| ProviderFacts {
        identity: "org.example/native".to_owned(),
        root: root.path().to_path_buf(),
        version: "1.0.0".to_owned(),
        content_hash: Some(content_hash),
        home: ProviderHome::Dependency,
    };
    let payload = "a".repeat(64);
    let legacy = source_witness(&provider(format!("sha256:{payload}"))).unwrap();
    let tree = source_witness(&provider(format!("sha256-tree/1:{payload}"))).unwrap();
    assert_ne!(legacy, tree, "recipe labels are part of the witness domain");
    assert!(matches!(
        source_witness(&provider(format!("unknown:{payload}"))),
        Err(NativeArtifactError::SourceWitness { .. })
    ));
}

#[test]
fn real_provider_cdylib_build_is_grouped_fresh_and_revalidated() {
    let root = tempdir().unwrap();
    write_cdylib_fixture(root.path());
    let declarations = vec![
        native_decl("first", Some("native"), None, Some("same")),
        native_decl("second", Some("native"), None, Some("same")),
    ];
    let owned = world(root.path(), declarations.clone());
    let extensions = collect_extensions(owned.clone()).unwrap();
    let mechanisms = collect_mechanisms(&owned).unwrap();
    let candidates = extensions.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let execution = NativeBuildExecution {
        candidates: &candidates,
        selected_project_root: root.path(),
        registry: &mechanisms,
        routes: &routes,
        platform: current_platform(),
        offline: true,
        created_at: "2026-08-31T00:00:00Z",
    };
    let first = build_native_sources(&execution).unwrap();
    assert_eq!(first.len(), 1, "one provider/crate group builds once");
    let artifact = PathBuf::from(&first[0].path_absolute);
    assert!(first[0].path_relative.starts_with("target/"));
    assert!(
        artifact
            .canonicalize()
            .unwrap()
            .starts_with(root.path().join("target").canonicalize().unwrap())
    );
    assert!(!root.path().join("native/target").exists());
    assert_eq!(first[0].record_id.len(), 64);
    assert!(
        first[0]
            .record_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
    let modified = fs::metadata(&artifact).unwrap().modified().unwrap();
    let first_resolved = resolve_native_artifact(&execution, candidates[0]).unwrap();
    assert_eq!(first_resolved.digest, first[0].digest);

    std::thread::sleep(Duration::from_millis(1_100));
    let second = build_native_sources(&execution).unwrap();
    assert_eq!(second.len(), 1);
    assert!(
        second[0].fresh,
        "the second scheduled build still calls Cargo"
    );
    assert_eq!(
        fs::metadata(&artifact).unwrap().modified().unwrap(),
        modified
    );
    let resolved = resolve_native_artifact(&execution, candidates[0]).unwrap();
    assert_eq!(resolved.origin, NativeArtifactOrigin::SourceRecord);
    assert_eq!(resolved.digest, second[0].digest);

    let original_artifact = fs::read(&artifact).unwrap();
    fs::write(&artifact, b"corrupt artifact").unwrap();
    assert!(matches!(
        resolve_native_artifact(&execution, candidates[0]),
        Err(NativeArtifactError::SourceState { .. })
    ));
    fs::write(&artifact, &original_artifact).unwrap();
    resolve_native_artifact(&execution, candidates[0]).unwrap();

    let record_path = root.path().join(&second[0].record);
    let original_record = fs::read(&record_path).unwrap();
    let mut record_json: serde_json::Value = serde_json::from_slice(&original_record).unwrap();
    *record_json.pointer_mut("/platform").unwrap() = serde_json::json!("foreign-platform");
    fs::write(
        &record_path,
        serde_json::to_vec_pretty(&record_json).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        resolve_native_artifact(&execution, candidates[0]),
        Err(NativeArtifactError::SourceState { .. })
    ));
    fs::write(&record_path, &original_record).unwrap();

    let mut record_json: serde_json::Value = serde_json::from_slice(&original_record).unwrap();
    *record_json.pointer_mut("/path_relative/path").unwrap() =
        serde_json::json!("native/Cargo.toml");
    fs::write(
        &record_path,
        serde_json::to_vec_pretty(&record_json).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        resolve_native_artifact(&execution, candidates[0]),
        Err(NativeArtifactError::SourceState { .. })
    ));
    fs::write(&record_path, &original_record).unwrap();

    let changed_world = world(
        root.path(),
        vec![
            native_decl("first", Some("native"), None, Some("changed")),
            declarations[1].clone(),
        ],
    );
    let changed = collect_extensions(changed_world).unwrap();
    let changed_candidates = changed.rows().iter().collect::<Vec<_>>();
    let changed_execution = NativeBuildExecution {
        candidates: &changed_candidates,
        ..execution
    };
    assert!(matches!(
        resolve_native_artifact(&changed_execution, changed_candidates[0]),
        Err(NativeArtifactError::SourceState { .. })
    ));

    fs::write(
        root.path().join("native/src/lib.rs"),
        "// source witness mutation after the SDK cdylib build\n",
    )
    .unwrap();
    assert!(matches!(
        resolve_native_artifact(&execution, candidates[0]),
        Err(NativeArtifactError::SourceState { .. })
    ));
}
