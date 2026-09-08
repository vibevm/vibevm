use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{
    ExtensionHandler, ExtensionsControl, Manifest, MechanismDecl, MechanismFreshness,
    MechanismRole, MechanismRoutes,
};
use vibe_extension_registry::collect_mechanisms;

use super::*;
use crate::{ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider};

fn declaration(
    role: MechanismRole,
    name: &str,
    crate_dir: Option<&str>,
    prebuilt: Option<BTreeMap<String, PathBuf>>,
) -> MechanismDecl {
    MechanismDecl {
        id: format!("{name}-provider"),
        role,
        name: name.to_owned(),
        handler: ExtensionHandler::Native {
            crate_dir: crate_dir.map(PathBuf::from),
            prebuilt,
        },
        protocol: 1,
        config_schema: PathBuf::from("schema.jtd.json"),
        freshness: MechanismFreshness::Provider,
    }
}

fn registry(root: &Path, mechanisms: Vec<MechanismDecl>) -> MechanismRegistry {
    collect_mechanisms(&ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: root.to_path_buf(),
                version: "1.0.0".to_owned(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms,
        },
        effective_stack: None,
    })
    .unwrap()
}

fn manifest() -> Manifest {
    Manifest::parse_str(
        "[project]\nname='demo'\nversion='1.0.0'\n\
         [[artifacts.build]]\nid='build'\nmechanism='build:cargo'\noutputs=[{id='app',kind='executable'}]\n\
         [[artifacts.package]]\nid='package'\nmechanism='package:pack'\noutputs=[{id='bundle',kind='file'}]\n\
         [[deploy.target]]\nid='one'\nartifact='app'\nmechanism='deploy:foreign'\n\
         [[deploy.target]]\nid='two'\nartifact='app'\nmechanism='deploy:foreign'\n",
    )
    .unwrap()
}

fn execution<'a>(
    root: &'a Path,
    registry: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
) -> NativeBuildExecution<'a> {
    NativeBuildExecution {
        candidates: &[],
        selected_project_root: root,
        registry,
        routes,
        platform: NativePlatform::current().unwrap(),
        offline: true,
        created_at: "2026-09-08T00:00:00Z",
    }
}

fn routes(package: bool) -> MechanismRoutes {
    let mut routes = MechanismRoutes::default();
    routes.insert(
        "deploy:foreign".parse().unwrap(),
        vibe_core::manifest::ProviderPin::parse("__host__/demo#foreign-provider").unwrap(),
    );
    if package {
        routes.insert(
            "package:pack".parse().unwrap(),
            vibe_core::manifest::ProviderPin::parse("__host__/demo#pack-provider").unwrap(),
        );
    }
    routes
}

#[test]
fn projected_package_is_deferred_and_deploy_bindings_dedup_exact_pin() {
    let root = TempDir::new().unwrap();
    let registry = registry(
        root.path(),
        vec![
            declaration(MechanismRole::Package, "pack", Some("native"), None),
            declaration(MechanismRole::Deploy, "foreign", Some("native"), None),
        ],
    );
    let manifest = manifest();
    let artifacts = manifest.artifacts.unwrap();
    let mut deploy = manifest.deploy.unwrap().targets;
    let selected_routes = routes(true);
    let package_only =
        project_native_mechanisms(&artifacts.package, &[], &registry, &selected_routes).unwrap();
    assert!(package_only.is_empty());
    let prepared = preflight_native_mechanisms(
        package_only,
        &execution(root.path(), &registry, &selected_routes),
    )
    .unwrap()
    .prepare(&execution(root.path(), &registry, &selected_routes))
    .unwrap();
    assert!(prepared.entries.is_empty());
    let plan = project_native_mechanisms(&artifacts.package, &deploy, &registry, &selected_routes)
        .unwrap();
    assert_eq!(plan.entry_count(), 1, "package-native is deferred");
    assert_eq!(plan.binding_count(), 2, "one image keeps both targets");
    let mut poisoned = selected_routes.clone();
    poisoned.insert(
        "deploy:foreign".parse().unwrap(),
        vibe_core::manifest::ProviderPin::parse("org.missing/provider#none").unwrap(),
    );
    assert!(
        project_native_mechanisms(&[], &[], &registry, &poisoned)
            .unwrap()
            .is_empty()
    );
    deploy[0].provider =
        Some(vibe_core::manifest::ProviderPin::parse("org.missing/provider#none").unwrap());
    assert!(project_native_mechanisms(&[], &deploy, &registry, &selected_routes).is_err());
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn one_logical_key_cannot_select_two_exact_pins() {
    let root = TempDir::new().unwrap();
    let registry = registry(
        root.path(),
        vec![declaration(
            MechanismRole::Deploy,
            "vibe-bin",
            Some("native"),
            None,
        )],
    );
    let mut targets = manifest().deploy.unwrap().targets;
    for target in &mut targets {
        target.mechanism = "deploy:vibe-bin".parse().unwrap();
    }
    targets[0].provider =
        Some(vibe_core::manifest::ProviderPin::parse("__host__/demo#vibe-bin-provider").unwrap());
    targets[1].provider =
        Some(vibe_core::manifest::ProviderPin::parse("org.vibevm/vibe#vibe-bin").unwrap());
    let error = project_native_mechanisms(&[], &targets, &registry, &MechanismRoutes::default())
        .unwrap_err();
    assert!(error.to_string().contains("different exact provider pins"));
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn current_prebuilt_prepares_immutable_image_and_corruption_never_falls_back() {
    let root = TempDir::new().unwrap();
    let platform = NativePlatform::current().unwrap();
    let relative = PathBuf::from(format!("prebuilt/provider{}", platform.suffix()));
    std::fs::create_dir_all(root.path().join("prebuilt")).unwrap();
    std::fs::write(root.path().join(&relative), b"mechanism prebuilt").unwrap();
    let prebuilt = BTreeMap::from([(platform.key().to_owned(), relative.clone())]);
    let registry = registry(
        root.path(),
        vec![declaration(
            MechanismRole::Deploy,
            "foreign",
            Some("native"),
            Some(prebuilt),
        )],
    );
    let targets = manifest().deploy.unwrap().targets;
    let routes = routes(false);
    let plan = project_native_mechanisms(&[], &targets, &registry, &routes).unwrap();
    let prepared = preflight_native_mechanisms(plan, &execution(root.path(), &registry, &routes))
        .unwrap()
        .prepare(&execution(root.path(), &registry, &routes))
        .unwrap();
    assert_eq!(prepared.entries.len(), 1);
    assert_eq!(prepared.entries[0].origin, NativeArtifactOrigin::Prebuilt);
    let binding = &prepared.entries[0].bindings[0];
    assert_eq!(binding.key.to_string(), "deploy:foreign");
    assert_eq!(binding.pin, "__host__/demo#foreign-provider");
    assert_eq!(binding.descriptor_id, "foreign-provider");
    assert_eq!(binding.protocol, 1);
    assert_eq!(prepared.entries[0].provider_version, "1.0.0");
    assert_eq!(prepared.entries[0].platform, platform);
    assert!(prepared.entries[0].record.is_none());
    assert!(prepared.entries[0].image.is_file());

    std::fs::remove_file(root.path().join(relative)).unwrap();
    let plan = project_native_mechanisms(&[], &targets, &registry, &routes).unwrap();
    assert!(
        preflight_native_mechanisms(plan, &execution(root.path(), &registry, &routes)).is_err()
    );
    assert!(!root.path().join("target").exists());
}

#[test]
fn source_build_records_revalidates_and_reuses_one_image() {
    let root = TempDir::new().unwrap();
    write_source(root.path());
    let registry = registry(
        root.path(),
        vec![declaration(
            MechanismRole::Deploy,
            "foreign",
            Some("native"),
            None,
        )],
    );
    let targets = manifest().deploy.unwrap().targets;
    let routes = routes(false);
    let prepare = || {
        let plan = project_native_mechanisms(&[], &targets, &registry, &routes).unwrap();
        preflight_native_mechanisms(plan, &execution(root.path(), &registry, &routes))
            .unwrap()
            .prepare(&execution(root.path(), &registry, &routes))
    };
    let mut images = Vec::new();
    for _ in 0..2 {
        let prepared = prepare().unwrap();
        assert_eq!(prepared.entries.len(), 1);
        assert_eq!(prepared.entries[0].bindings.len(), 2);
        assert_eq!(
            prepared.entries[0].origin,
            NativeArtifactOrigin::SourceRecord
        );
        assert!(prepared.entries[0].record.is_some());
        images.push(prepared.entries[0].image.clone());
    }
    assert_eq!(images[0], images[1]);
    let prepared = prepare().unwrap();
    let entry = &prepared.entries[0];
    let record = root.path().join(entry.record.as_ref().unwrap());
    let record_bytes = std::fs::read(&record).unwrap();
    let image_bytes = std::fs::read(&entry.image).unwrap();

    std::fs::write(&record, b"{}").unwrap();
    assert!(
        prepare().is_err(),
        "corrupt record cannot be overwritten as fresh"
    );
    std::fs::write(&record, &record_bytes).unwrap();

    std::fs::write(&entry.image, b"corrupt image").unwrap();
    assert!(
        prepare().is_err(),
        "digest-addressed image corruption refuses"
    );
    std::fs::write(&entry.image, &image_bytes).unwrap();

    std::fs::write(
        root.path().join("native/src/lib.rs"),
        format!(
            "{}\n// source changed after immutable publication\n",
            std::fs::read_to_string(root.path().join("native/src/lib.rs")).unwrap()
        ),
    )
    .unwrap();
    assert!(prepare().is_err(), "stale source witness refuses reuse");
    assert_eq!(std::fs::read(&entry.image).unwrap(), image_bytes);
    assert_eq!(entry.digest.len(), 64);
}

fn write_source(root: &Path) {
    let vibe_ext = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("vibe-ext")
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::create_dir_all(root.join("native/src")).unwrap();
    std::fs::write(
        root.join("native/Cargo.toml"),
        format!("[package]\nname='mechanism-source'\nversion='1.0.0'\nedition='2024'\n[lib]\ncrate-type=['cdylib']\n[dependencies]\nvibe-ext={{path={vibe_ext:?}}}\n"),
    )
    .unwrap();
    std::fs::write(
        root.join("native/src/lib.rs"),
        "use vibe_ext::{DeployReply,DeployRequest,MechanismManifest};fn manifest()->MechanismManifest{MechanismManifest{mechanisms:vec![]}}fn handle(_:DeployRequest)->DeployReply{panic!()}vibe_ext::vibe_mechanism_provider!(manifest=manifest(),handler=handle);\n",
    )
    .unwrap();
}
