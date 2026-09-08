//! Real installed-package native Build -> A2 record -> Package composition.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{
    ExtensionHandler, ExtensionsControl, Manifest, MechanismDecl, MechanismFreshness,
    MechanismRole, MechanismRoutes, ProviderPin, TargetOs,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_lifecycle::native::{NativePlatform, project_native_target_mechanisms};
use vibe_lifecycle::{
    BuildExecution, DependencyExtensionSource, DependencyProvider, DependencyProviderId,
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, PackageError,
    collect_mechanisms,
};

use super::{Fences, MechanismTargets};

const BUILD_PIN: &str = "org.example/native-pipeline#fixture-build";
const PACKAGE_PIN: &str = "org.example/native-pipeline#fixture-package";
const NOW: &str = "2026-09-08T12:00:00Z";

#[derive(Clone, Copy)]
enum PackageImage {
    Package,
    Build,
}

struct Fixture {
    _root: TempDir,
    project: PathBuf,
    manifest: Manifest,
    registry: vibe_lifecycle::MechanismRegistry,
    routes: MechanismRoutes,
    platform: NativePlatform,
    target_os: TargetOs,
    candidates: Vec<vibe_lifecycle::ExtensionRegistryRow>,
}

impl Fixture {
    fn new(package_image: PackageImage) -> Self {
        assert_eq!(
            vibe_native_loader_build_provider_fixture::fixture_marker(),
            "vibe-native-loader-build-provider-fixture"
        );
        assert_eq!(
            vibe_native_loader_package_provider_fixture::fixture_marker(),
            "vibe-native-loader-package-provider-fixture"
        );
        let root = must(TempDir::new(), "temp project");
        let project = root.path().join("project");
        let slot = project.join("vibevm/vibedeps/org.example.native-pipeline/1.0.0");
        must(
            std::fs::create_dir_all(slot.join("native")),
            "provider slot",
        );
        write(
            &project.join("Cargo.toml"),
            b"this is deliberately not Cargo TOML\n",
        );
        write(&slot.join("schema.jtd.json"), b"{}");

        let platform = must(NativePlatform::current(), "native platform");
        let target_os = must_some(TargetOs::current(), "target OS");
        let build_library = fixture_library("vibe_native_loader_build_provider_fixture");
        let package_library = fixture_library("vibe_native_loader_package_provider_fixture");
        let build_relative = copy_library(&slot, "build", &build_library);
        let selected_package_library = match package_image {
            PackageImage::Package => &package_library,
            PackageImage::Build => &build_library,
        };
        let package_relative = copy_library(&slot, "package", selected_package_library);

        let mechanisms = vec![
            declaration(
                "fixture-build",
                MechanismRole::Build,
                "cargo",
                platform,
                build_relative,
            ),
            declaration(
                "fixture-package",
                MechanismRole::Package,
                "static-file",
                platform,
                package_relative,
            ),
        ];
        let installed = DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(
                    must(Group::parse("org.example"), "provider group"),
                    must(PackageName::parse("native-pipeline"), "provider package"),
                ),
                root: slot,
                version: "1.0.0".to_owned(),
                kind: PackageKind::Tool,
                content_hash: must(
                    ContentHash::parse(&format!("sha256:{}", "a".repeat(64))),
                    "provider hash",
                ),
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms,
        };
        let world = ExtensionWorld {
            installed: vec![installed],
            host: HostExtensionSource {
                provider: HostProvider {
                    identity: HostIdentity::ungrouped_project("demo"),
                    root: project.clone(),
                    version: "1.0.0".to_owned(),
                    kind: None,
                    content_hash: None,
                },
                declarations: Vec::new(),
                controls: ExtensionsControl::default(),
                mechanisms: Vec::new(),
            },
            effective_stack: None,
        };
        let registry = must(collect_mechanisms(&world), "mechanism registry");
        let mut routes = MechanismRoutes::default();
        routes.insert(
            must("build:cargo".parse(), "build key"),
            must(ProviderPin::parse(BUILD_PIN), "build pin"),
        );
        routes.insert(
            must("package:static-file".parse(), "package key"),
            must(ProviderPin::parse(PACKAGE_PIN), "package pin"),
        );
        Self {
            _root: root,
            project,
            manifest: must(Manifest::parse_str(MANIFEST), "project manifest"),
            registry,
            routes,
            platform,
            target_os,
            candidates: Vec::new(),
        }
    }

    fn fences(&self) -> Fences<'_> {
        let artifacts = must_some(self.manifest.artifacts.as_ref(), "artifacts");
        let plan = must(
            project_native_target_mechanisms(
                &artifacts.build,
                &artifacts.package,
                &[],
                &self.registry,
                &self.routes,
            ),
            "native mechanism plan",
        );
        Fences {
            targets: MechanismTargets {
                project_root: &self.project,
                build: &artifacts.build,
                package: &artifacts.package,
                deploy_targets: &[],
                registry: &self.registry,
                routes: &self.routes,
                native_candidates: &self.candidates,
                native_platform: Some(self.platform),
                native: None,
                native_mechanisms: plan,
                target_os: self.target_os,
                offline: true,
                created_at: NOW,
                deploy: None,
            },
            prepared_native: Default::default(),
            build: Some(0),
            package: Some(0),
            deploy: None,
        }
    }
}

#[test]
fn installed_native_build_record_feeds_native_package_and_repeats_exactly() {
    let fixture = Fixture::new(PackageImage::Package);
    let mut first = fixture.fences();
    must(first.fire_build(0), "first build fence");
    let build_path = fixture
        .project
        .join("target/vibe-native/foreign-build/foreign.bin");
    let build_record_path = fixture
        .project
        .join(".vibe/state/artifacts/foreign-bin.json");
    let prior_bytes = read(&build_path);
    let prior_record = read(&build_record_path);
    let artifacts = must_some(fixture.manifest.artifacts.as_ref(), "artifacts");
    let reused = must(
        first
            .prepared_native
            .execute_build_targets(&BuildExecution {
                project_root: &fixture.project,
                targets: &artifacts.build,
                registry: &fixture.registry,
                routes: &fixture.routes,
                build_root: BuildExecution::default_build_root(),
                offline: true,
                created_at: NOW,
            }),
        "real provider-fresh build reuse",
    );
    let reused_output = &reused[0].produced[0];
    assert!(reused_output.fresh);
    assert_eq!(reused[0].provider, BUILD_PIN);
    assert_eq!(
        reused_output.path_relative,
        "target/vibe-native/foreign-build/foreign.bin"
    );
    assert_eq!(reused_output.bytes, prior_bytes.len() as u64);
    assert_eq!(read(&build_path), prior_bytes);
    assert_eq!(read(&build_record_path), prior_record);
    must(first.fire_package(0), "first package fence");
    let first_snapshot = snapshot(&fixture.project);
    let first_images = image_identities(&first);
    assert_eq!(first_images.len(), 2);

    let build = record(&fixture.project, "foreign-bin");
    let package = record(&fixture.project, "foreign-package");
    assert_record_provider(&build, BUILD_PIN);
    assert_record_provider(&package, PACKAGE_PIN);
    assert_eq!(package["platform"], serde_json::Value::Null);
    let inputs = text(&package["freshness"]["inputs"]);
    let config = text(&package["freshness"]["config"]);
    let provider = text(&package["freshness"]["toolchain"]);
    assert!(inputs != config && inputs != provider && config != provider);
    let evidence = text(&package["verification"]["evidence"]);
    assert!(evidence.contains(first.prepared_native.entries[1].platform.key()));
    assert!(evidence.contains(&first.prepared_native.entries[1].digest));
    let built = read(
        &fixture
            .project
            .join("target/vibe-native/foreign-build/foreign.bin"),
    );
    let packaged = read(
        &fixture
            .project
            .join("target/vibe-package/foreign-package-target/foreign-package.bin"),
    );
    assert_eq!(packaged, built);
    assert_eq!(package["digest"]["value"], build["digest"]["value"]);
    assert_eq!(read(&fixture.project.join("Cargo.toml")), INVALID_CARGO);
    assert!(!fixture.project.join("Cargo.lock").exists());
    assert!(!fixture.project.join("target/debug").exists());
    assert!(!fixture.project.join("target/release").exists());

    let mut second = fixture.fences();
    must(second.fire_build(0), "second build fence");
    must(second.fire_package(0), "second package fence");
    assert_eq!(snapshot(&fixture.project), first_snapshot);
    assert_eq!(image_identities(&second), first_images);
}

#[test]
fn stale_build_output_stops_package_before_native_reset_or_record() {
    let fixture = Fixture::new(PackageImage::Package);
    let mut fences = fixture.fences();
    must(fences.fire_build(0), "build fence");
    write(
        &fixture
            .project
            .join("target/vibe-native/foreign-build/foreign.bin"),
        b"tampered",
    );
    let error = must_err(fences.fire_package(0), "stale package input");
    assert!(error.downcast_ref::<PackageError>().is_some_and(|error| {
        matches!(error, PackageError::InputStale { input, .. } if input == "foreign-bin")
    }));
    assert_no_package_effect(&fixture.project);
}

#[test]
fn build_provider_image_cannot_impersonate_package_provider() {
    let fixture = Fixture::new(PackageImage::Build);
    let mut fences = fixture.fences();
    must(fences.fire_build(0), "build fence");
    let error = must_err(fences.fire_package(0), "wrong package image");
    assert!(error.downcast_ref::<PackageError>().is_some_and(|error| {
        matches!(
            error,
            PackageError::NativeTransport {
                operation: "admit",
                ..
            }
        )
    }));
    assert!(
        fixture
            .project
            .join(".vibe/state/artifacts/foreign-bin.json")
            .is_file()
    );
    assert_no_package_effect(&fixture.project);
}

fn declaration(
    id: &str,
    role: MechanismRole,
    name: &str,
    platform: NativePlatform,
    relative: PathBuf,
) -> MechanismDecl {
    MechanismDecl {
        id: id.to_owned(),
        role,
        name: name.to_owned(),
        handler: ExtensionHandler::Native {
            crate_dir: None,
            prebuilt: Some(BTreeMap::from([(platform.key().to_owned(), relative)])),
        },
        protocol: 1,
        config_schema: PathBuf::from("schema.jtd.json"),
        freshness: MechanismFreshness::Provider,
    }
}

fn copy_library(slot: &Path, name: &str, source: &Path) -> PathBuf {
    let file = must_some(source.file_name(), "library filename");
    let relative = PathBuf::from(format!("native/{name}-{}", file.to_string_lossy()));
    must(
        std::fs::copy(source, slot.join(&relative)),
        "copy fixture library",
    );
    relative
}

fn fixture_library(stem: &str) -> PathBuf {
    let executable = must(std::env::current_exe(), "test executable");
    let directory = must_some(executable.parent(), "test executable directory");
    let profile = if directory.file_name().is_some_and(|name| name == "deps") {
        must_some(directory.parent(), "test profile directory")
    } else {
        directory
    };
    let exact = format!(
        "{}{stem}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let matches = [profile.to_path_buf(), profile.join("deps")]
        .into_iter()
        .flat_map(|directory| {
            must(std::fs::read_dir(directory), "fixture artifact directory").filter_map(Result::ok)
        })
        .map(|entry| entry.path())
        .filter(|path| path.file_name().is_some_and(|name| name == exact.as_str()))
        .collect::<Vec<_>>();
    let [one] = matches.as_slice() else {
        panic!("expected one `{exact}` fixture artifact, found {matches:?}")
    };
    one.clone()
}

fn image_identities(fences: &Fences<'_>) -> BTreeSet<(PathBuf, String)> {
    fences
        .prepared_native
        .entries
        .iter()
        .map(|entry| (entry.image.clone(), entry.digest.clone()))
        .collect()
}

fn assert_record_provider(record: &serde_json::Value, pin: &str) {
    assert_eq!(record["producer"]["provider"]["key"], pin);
    assert_eq!(record["producer"]["provider"]["version"], "1.0.0");
    assert_eq!(
        record["producer"]["provider"]["content_hash"],
        format!("sha256:{}", "a".repeat(64))
    );
    assert_ne!(
        record["producer"]["provider"]["key"],
        "org.vibevm/vibe#cargo"
    );
    assert_ne!(
        record["producer"]["provider"]["key"],
        "org.vibevm/vibe#static-file"
    );
}

fn assert_no_package_effect(root: &Path) {
    assert!(
        !root
            .join("target/vibe-package/foreign-package-target")
            .exists()
    );
    assert!(
        !root
            .join(".vibe/state/artifacts/foreign-package.json")
            .exists()
    );
}

fn record(root: &Path, id: &str) -> serde_json::Value {
    let bytes = read(&root.join(format!(".vibe/state/artifacts/{id}.json")));
    must(serde_json::from_slice(&bytes), "artifact record")
}

fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut rows = Vec::new();
    collect(root, root, &mut rows);
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    rows
}

fn collect(root: &Path, directory: &Path, rows: &mut Vec<(String, Vec<u8>)>) {
    for entry in must(std::fs::read_dir(directory), "snapshot directory").filter_map(Result::ok) {
        let path = entry.path();
        if must(entry.file_type(), "snapshot file type").is_dir() {
            collect(root, &path, rows);
        } else {
            let relative = must(path.strip_prefix(root), "snapshot relative path")
                .to_string_lossy()
                .replace('\\', "/");
            rows.push((relative, read(&path)));
        }
    }
}

fn text(value: &serde_json::Value) -> &str {
    must_some(value.as_str(), "record text")
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        must(std::fs::create_dir_all(parent), "fixture directory");
    }
    must(std::fs::write(path, bytes), "fixture write");
}

fn read(path: &Path) -> Vec<u8> {
    must(std::fs::read(path), "fixture read")
}

fn must<T, E: std::fmt::Debug>(value: Result<T, E>, context: &str) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error:?}"),
    }
}

fn must_err<T, E: std::fmt::Debug>(value: Result<T, E>, context: &str) -> E {
    match value {
        Err(error) => error,
        Ok(_) => panic!("{context} unexpectedly succeeded"),
    }
}

fn must_some<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("{context}"),
    }
}

const INVALID_CARGO: &[u8] = b"this is deliberately not Cargo TOML\n";
const MANIFEST: &str = r#"
[project]
name = "demo"
version = "1.0.0"

[[artifacts.build]]
id = "foreign-build"
mechanism = "build:cargo"
workdir = "."
config = { foreign = true }
outputs = [{ id = "foreign-bin", kind = "executable" }]

[[artifacts.package]]
id = "foreign-package-target"
mechanism = "package:static-file"
provider = "org.example/native-pipeline#fixture-package"
inputs = [{ artifact = "foreign-bin" }]
config = { foreign = true }
outputs = [{ id = "foreign-package", kind = "file" }]
"#;
