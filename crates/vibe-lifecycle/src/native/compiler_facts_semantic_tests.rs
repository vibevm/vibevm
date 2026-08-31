use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::*;
use crate::native::compiler_facts::provider_digest_for_test;
use crate::native::provider::{ProviderFacts, ProviderHome};
use vibe_core::manifest::{
    ExtensionHandler, MechanismDecl, MechanismFreshness, MechanismRole, MechanismRoutes,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_extension_registry::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionRegistryRow,
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider,
};

#[test]
fn raw_witnesses_and_legacy_wire_match_pre_refactor_literals() {
    let provider = ProviderFacts {
        identity: "org.demo/fixed".to_owned(),
        root: PathBuf::from("root-is-not-read"),
        version: "1.0.0".to_owned(),
        content_hash: Some(format!("sha256:{}", "a".repeat(64))),
        home: ProviderHome::Dependency,
    };
    let source_raw =
        crate::native::witness::source_witness_digest(&provider).expect("fixed source witness");
    let source_wire = crate::native::witness::source_witness(&provider).expect("fixed source wire");
    assert_eq!(
        source_wire,
        "304513de2f3a706360786ce1cc24d818b607521f4ea526f8cbead3c24657a022"
    );
    assert_eq!(source_raw.hex(), source_wire);

    let rows: Vec<&ExtensionRegistryRow> = Vec::new();
    let config_raw = crate::native::witness::config_witness_digest(&rows);
    let config_wire = crate::native::witness::config_witness(&rows);
    assert_eq!(
        config_wire,
        "968a4b2c40942fcafc767bf9a0c93f96aef6551cae90ddca35cd95d2f4d5c35b"
    );
    assert_eq!(config_raw.hex(), config_wire);
}

fn mechanism(id: &str) -> MechanismDecl {
    MechanismDecl {
        id: id.to_owned(),
        role: MechanismRole::Build,
        name: "semantic".to_owned(),
        handler: ExtensionHandler::Script {
            base: PathBuf::from("tools/provider"),
        },
        protocol: 1,
        config_schema: PathBuf::from("schemas/provider-v1.jtd.json"),
        freshness: MechanismFreshness::Engine,
    }
}

fn host_provider(root: &Path) -> HostProvider {
    let Ok(content_hash) = ContentHash::parse("sha256:aa") else {
        panic!("FACTS fixture content hash");
    };
    HostProvider {
        identity: HostIdentity::ungrouped_project("facts-host"),
        root: root.to_path_buf(),
        version: "1.0.0".to_owned(),
        kind: Some(PackageKind::Tool),
        content_hash: Some(content_hash),
    }
}

fn host_digest(provider: HostProvider, declarations: Vec<MechanismDecl>, id: &str) -> [u8; 32] {
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider,
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: declarations,
        },
        effective_stack: None,
    };
    let Ok(registry) = collect_mechanisms(&world) else {
        panic!("FACTS fixture mechanism registry");
    };
    let Some(row) = registry
        .rows()
        .iter()
        .find(|row| row.declaration().id == id)
    else {
        panic!("FACTS fixture mechanism row");
    };
    let Ok(digest) = provider_digest_for_test(row) else {
        panic!("FACTS fixture provider digest");
    };
    digest
}

#[derive(Clone)]
struct DependencyShape {
    group: &'static str,
    name: &'static str,
    version: &'static str,
    kind: PackageKind,
    content_hash: &'static str,
}

fn dependency_digest(root: &Path, shape: &DependencyShape, declaration: MechanismDecl) -> [u8; 32] {
    let Ok(group) = Group::parse(shape.group) else {
        panic!("FACTS fixture group");
    };
    let Ok(name) = PackageName::parse(shape.name) else {
        panic!("FACTS fixture package name");
    };
    let Ok(content_hash) = ContentHash::parse(shape.content_hash) else {
        panic!("FACTS fixture content hash");
    };
    let provider = DependencyProvider {
        id: DependencyProviderId::new(group, name),
        root: root.join("ignored-slot-root"),
        version: shape.version.to_owned(),
        kind: shape.kind,
        content_hash,
    };
    let world = ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider,
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: vec![declaration],
        }],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::virtual_workspace(),
                root: root.to_path_buf(),
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
    let Ok(registry) = collect_mechanisms(&world) else {
        panic!("FACTS fixture dependency registry");
    };
    let Some(row) = registry
        .rows()
        .iter()
        .find(|row| row.declaration().id == "provider")
    else {
        panic!("FACTS fixture dependency row");
    };
    let Ok(digest) = provider_digest_for_test(row) else {
        panic!("FACTS fixture dependency digest");
    };
    digest
}

#[test]
fn provider_digest_binds_complete_semantics_and_excludes_root_and_ordinals() {
    let root = tempdir().expect("FACTS fixture");
    let base_provider = host_provider(root.path());
    let base_decl = mechanism("provider");
    let baseline = host_digest(base_provider.clone(), vec![base_decl.clone()], "provider");

    let other_root = tempdir().expect("FACTS fixture");
    let mut root_only = base_provider.clone();
    root_only.root = other_root.path().to_path_buf();
    assert_eq!(
        host_digest(root_only, vec![base_decl.clone()], "provider"),
        baseline,
        "provider roots are excluded"
    );
    assert_eq!(
        host_digest(
            base_provider.clone(),
            vec![mechanism("earlier"), base_decl.clone()],
            "provider",
        ),
        baseline,
        "declaration ordinals are excluded"
    );

    let mut mutations = Vec::new();
    let mut provider = base_provider.clone();
    provider.identity = HostIdentity::ungrouped_project("changed-host");
    mutations.push(host_digest(provider, vec![base_decl.clone()], "provider"));
    let mut provider = base_provider.clone();
    provider.version = "2.0.0".to_owned();
    mutations.push(host_digest(provider, vec![base_decl.clone()], "provider"));
    let mut provider = base_provider.clone();
    provider.kind = None;
    mutations.push(host_digest(provider, vec![base_decl.clone()], "provider"));
    let mut provider = base_provider.clone();
    provider.content_hash = Some(ContentHash::parse("sha256:bb").expect("FACTS fixture"));
    mutations.push(host_digest(provider, vec![base_decl.clone()], "provider"));

    let mut declaration = base_decl.clone();
    declaration.id = "changed-pin".to_owned();
    mutations.push(host_digest(
        base_provider.clone(),
        vec![declaration],
        "changed-pin",
    ));
    let mut declaration = base_decl.clone();
    declaration.name = "changed-key".to_owned();
    mutations.push(host_digest(
        base_provider.clone(),
        vec![declaration],
        "provider",
    ));
    let mut declaration = base_decl.clone();
    declaration.handler = ExtensionHandler::Script {
        base: PathBuf::from("tools/changed"),
    };
    mutations.push(host_digest(
        base_provider.clone(),
        vec![declaration],
        "provider",
    ));
    let mut declaration = base_decl.clone();
    declaration.protocol = 2;
    mutations.push(host_digest(
        base_provider.clone(),
        vec![declaration],
        "provider",
    ));
    let mut declaration = base_decl.clone();
    declaration.config_schema = PathBuf::from("schemas/provider-v2.jtd.json");
    mutations.push(host_digest(
        base_provider.clone(),
        vec![declaration],
        "provider",
    ));
    let mut declaration = base_decl.clone();
    declaration.freshness = MechanismFreshness::Provider;
    mutations.push(host_digest(
        base_provider.clone(),
        vec![declaration],
        "provider",
    ));
    let dependency = DependencyShape {
        group: "org.demo",
        name: "facts-provider",
        version: "1.0.0",
        kind: PackageKind::Tool,
        content_hash: "sha256:aa",
    };
    mutations.push(dependency_digest(root.path(), &dependency, base_decl));

    assert!(mutations.into_iter().all(|digest| digest != baseline));
}

#[test]
fn dependency_provider_digest_binds_identity_version_kind_and_content_hash() {
    let root = tempdir().expect("FACTS fixture");
    let base = DependencyShape {
        group: "org.demo",
        name: "facts-provider",
        version: "1.0.0",
        kind: PackageKind::Tool,
        content_hash: "sha256:aa",
    };
    let baseline = dependency_digest(root.path(), &base, mechanism("provider"));
    let other_root = tempdir().expect("FACTS fixture");
    assert_eq!(
        dependency_digest(other_root.path(), &base, mechanism("provider")),
        baseline,
        "dependency roots are excluded"
    );
    for changed in [
        DependencyShape {
            group: "org.changed",
            ..base.clone()
        },
        DependencyShape {
            name: "other-provider",
            ..base.clone()
        },
        DependencyShape {
            version: "2.0.0",
            ..base.clone()
        },
        DependencyShape {
            kind: PackageKind::Stack,
            ..base.clone()
        },
        DependencyShape {
            content_hash: "sha256:bb",
            ..base.clone()
        },
    ] {
        assert_ne!(
            dependency_digest(root.path(), &changed, mechanism("provider")),
            baseline
        );
    }
}

fn digest_for_handler(root: &Path, handler: ExtensionHandler) -> [u8; 32] {
    let provider = host_provider(root);
    let mut declaration = mechanism("provider");
    declaration.handler = handler;
    host_digest(provider, vec![declaration], "provider")
}

#[test]
fn every_reachable_handler_member_moves_provider_identity_and_agent_is_unreachable() {
    let root = tempdir().expect("FACTS fixture");
    let binary = digest_for_handler(
        root.path(),
        ExtensionHandler::Binary {
            name: "provider-one".to_owned(),
        },
    );
    assert_ne!(
        digest_for_handler(
            root.path(),
            ExtensionHandler::Binary {
                name: "provider-two".to_owned(),
            },
        ),
        binary
    );

    let native = digest_for_handler(
        root.path(),
        ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from("crates/provider")),
            prebuilt: Some(BTreeMap::from([(
                "linux-x86_64".to_owned(),
                PathBuf::from("bin/provider.so"),
            )])),
        },
    );
    assert_ne!(
        digest_for_handler(
            root.path(),
            ExtensionHandler::Native {
                crate_dir: Some(PathBuf::from("crates/provider-v2")),
                prebuilt: Some(BTreeMap::from([(
                    "linux-x86_64".to_owned(),
                    PathBuf::from("bin/provider.so"),
                )])),
            },
        ),
        native,
        "crate_dir is semantic"
    );
    assert_ne!(
        digest_for_handler(
            root.path(),
            ExtensionHandler::Native {
                crate_dir: Some(PathBuf::from("crates/provider")),
                prebuilt: Some(BTreeMap::from([(
                    "linux-x86_64".to_owned(),
                    PathBuf::from("bin/provider-v2.so"),
                )])),
            },
        ),
        native,
        "prebuilt path is semantic"
    );
    assert_ne!(
        digest_for_handler(
            root.path(),
            ExtensionHandler::Native {
                crate_dir: Some(PathBuf::from("crates/provider")),
                prebuilt: None,
            },
        ),
        native,
        "prebuilt presence is semantic"
    );

    let mut agent = mechanism("agent");
    agent.handler = ExtensionHandler::Agent {
        prompt: "spec://org.demo/prompt/entry".to_owned(),
    };
    let error = agent.validate().unwrap_err();
    assert!(error.contains("not authorable"));
}

#[test]
fn builtin_cargo_admission_keeps_the_existing_record_pin() {
    let root = tempdir().expect("FACTS fixture");
    let (_, mechanisms) = registries(root.path(), Vec::new());
    let rows: Vec<&ExtensionRegistryRow> = Vec::new();
    let routes = MechanismRoutes::default();
    let execution = execution(&rows, root.path(), &mechanisms, &routes);
    let selected = crate::native::select_build_provider(&execution).expect("FACTS fixture");
    assert_eq!(selected.key.to_string(), "build:cargo");
    assert_eq!(selected.pin(), "org.vibevm/vibe#cargo");
    let cargo_digest = provider_digest_for_test(selected.row).expect("FACTS fixture");
    for row in mechanisms
        .rows()
        .iter()
        .filter(|row| row.is_builtin() && row.key().to_string() != "build:cargo")
    {
        assert_ne!(
            provider_digest_for_test(row).expect("FACTS fixture"),
            cargo_digest
        );
    }
}
