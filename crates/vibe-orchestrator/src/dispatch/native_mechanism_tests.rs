use std::path::{Path, PathBuf};

use vibe_core::manifest::{
    ExtensionDecl, ExtensionHandler, ExtensionsControl, Manifest, MechanismDecl,
    MechanismFreshness, MechanismRole, MechanismRoutes,
};

use super::native_mechanism;
use vibe_lifecycle::native::{
    NativePlatform, enabled_native_candidates, project_native_mechanisms,
};
use vibe_lifecycle::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
    collect_mechanisms,
};

fn world(root: &Path) -> ExtensionWorld {
    let handler = ExtensionHandler::Native {
        crate_dir: Some(PathBuf::from("native")),
        prebuilt: None,
    };
    ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: root.to_path_buf(),
                version: "1.0.0".to_owned(),
                kind: None,
                content_hash: None,
            },
            declarations: vec![ExtensionDecl {
                id: "extension".to_owned(),
                point: "phase:build".parse().unwrap(),
                handler: handler.clone(),
                config: None,
                auto: None,
                inputs: None,
                applies_to: None,
                compiler_internals: None,
                pass: None,
                when: None,
            }],
            controls: ExtensionsControl::default(),
            mechanisms: vec![MechanismDecl {
                id: "provider".to_owned(),
                role: MechanismRole::Deploy,
                name: "foreign".to_owned(),
                handler,
                protocol: 1,
                config_schema: PathBuf::from("schema.jtd.json"),
                freshness: MechanismFreshness::Provider,
            }],
        },
        effective_stack: None,
    }
}

#[test]
fn same_provider_source_cannot_claim_extension_and_mechanism_abi() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("native/src")).unwrap();
    std::fs::write(
        root.path().join("native/Cargo.toml"),
        "[package]\nname='collision'\nversion='1.0.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("native/src/lib.rs"),
        "pub fn marker() {}\n",
    )
    .unwrap();
    let world = world(root.path());
    let extensions = collect_extensions(world.clone()).unwrap();
    let candidates = enabled_native_candidates(&extensions);
    let registry = collect_mechanisms(&world).unwrap();
    let mut routes = MechanismRoutes::default();
    routes.insert(
        "deploy:foreign".parse().unwrap(),
        vibe_core::manifest::ProviderPin::parse("__host__/demo#provider").unwrap(),
    );
    let manifest = Manifest::parse_str(
        "[project]\nname='demo'\nversion='1.0.0'\n\
         [[artifacts.build]]\nid='build'\nmechanism='build:cargo'\noutputs=[{id='app',kind='executable'}]\n\
         [[deploy.target]]\nid='deploy'\nartifact='app'\nmechanism='deploy:foreign'\n",
    )
    .unwrap();
    let plan =
        project_native_mechanisms(&[], &manifest.deploy.unwrap().targets, &registry, &routes)
            .unwrap();
    let error = native_mechanism::prepare(
        None,
        &candidates,
        plan,
        root.path(),
        &registry,
        &routes,
        NativePlatform::current().unwrap(),
        true,
        "2026-09-08T00:00:00Z",
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("both extension and mechanism ABI")
    );
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn same_provider_prebuilt_cannot_claim_both_abi_families() {
    let root = tempfile::tempdir().unwrap();
    let platform = NativePlatform::current().unwrap();
    let relative = PathBuf::from(format!("provider{}", platform.suffix()));
    std::fs::write(root.path().join(&relative), b"not loaded").unwrap();
    let mut world = world(root.path());
    let handler = ExtensionHandler::Native {
        crate_dir: None,
        prebuilt: Some(std::collections::BTreeMap::from([(
            platform.key().to_owned(),
            relative,
        )])),
    };
    world.host.declarations[0].handler = handler.clone();
    world.host.mechanisms[0].handler = handler;
    let extensions = collect_extensions(world.clone()).unwrap();
    let candidates = enabled_native_candidates(&extensions);
    let registry = collect_mechanisms(&world).unwrap();
    let mut routes = MechanismRoutes::default();
    routes.insert(
        "deploy:foreign".parse().unwrap(),
        vibe_core::manifest::ProviderPin::parse("__host__/demo#provider").unwrap(),
    );
    let target = Manifest::parse_str(
        "[project]\nname='demo'\nversion='1.0.0'\n\
         [[artifacts.build]]\nid='build'\nmechanism='build:cargo'\noutputs=[{id='app',kind='executable'}]\n\
         [[deploy.target]]\nid='deploy'\nartifact='app'\nmechanism='deploy:foreign'\n",
    )
    .unwrap()
    .deploy
    .unwrap()
    .targets;
    let plan = project_native_mechanisms(&[], &target, &registry, &routes).unwrap();
    let error = native_mechanism::prepare(
        None,
        &candidates,
        plan,
        root.path(),
        &registry,
        &routes,
        platform,
        true,
        "2026-09-08T00:00:00Z",
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("both extension and mechanism ABI")
    );
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn fence_prepares_mechanisms_before_replay_and_authored_targets() {
    let source = include_str!("mechanism.rs");
    let prepare = source.find("native_mechanism::prepare(").unwrap();
    let replay = source.find(".replay(&mut factory)").unwrap();
    let authored = source
        .find(".execute_build_targets(&BuildExecution")
        .unwrap();
    assert!(prepare < replay && replay < authored);
    assert!(source.contains(".execute_package_targets(&PackageExecution"));

    let phase = include_str!("../phase.rs");
    let projection = phase
        .find("project_native_target_mechanisms(")
        .expect("phase uses the canonical active-target projection");
    let build = phase[projection..].find("&build_targets").unwrap();
    let package = phase[projection..].find("&package_targets").unwrap();
    let deploy = phase[projection..].find("&deploy_targets").unwrap();
    assert!(build < package && package < deploy);
}
