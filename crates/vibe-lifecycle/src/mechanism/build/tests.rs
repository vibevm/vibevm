//! The executor's laws that need no toolchain: routing, refusal and
//! dependency order.
//!
//! The first two tests are the ones that make §3.1's routing real. A host
//! that routes `build:cargo` to a plugin, or a target that pins one, must
//! get the plugin's refusal — the out-of-process transport is a later atom
//! — and must NOT get a Cargo build instead. Each asserts the project tree
//! is untouched afterwards, which is the observable difference between
//! "the resolver chose" and "a builtin branch chose".

use std::path::{Path, PathBuf};

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactInput, ArtifactKind, ArtifactOutput, ExtensionHandler,
    ExtensionsControl, MechanismDecl, MechanismFreshness, MechanismRole, MechanismRoutes,
    ProviderPin,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_extension_registry::{MechanismRegistry, collect_mechanisms};

use super::super::MechanismError;
use super::super::cargo::plan_tests::{key, target};
use super::*;
use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider,
};

const PLUGIN_PIN: &str = "org.example/build-tools#cargo-v2";

fn host_source(mechanisms: Vec<MechanismDecl>) -> HostExtensionSource {
    HostExtensionSource {
        provider: HostProvider {
            identity: HostIdentity::ungrouped_project("demo"),
            root: PathBuf::from("."),
            version: "0.1.0".into(),
            kind: None,
            content_hash: None,
        },
        declarations: Vec::new(),
        controls: ExtensionsControl::default(),
        mechanisms,
    }
}

/// A world whose installed package declares a NATIVE `build:cargo`
/// provider — installed, collected, and inert until something selects it.
fn world_with_plugin() -> ExtensionWorld {
    let declaration = MechanismDecl {
        id: "cargo-v2".into(),
        role: MechanismRole::Build,
        name: "cargo".into(),
        handler: ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from("crates/cargo-provider")),
            prebuilt: None,
        },
        protocol: 1,
        config_schema: PathBuf::from("schemas/cargo-build-v1.jtd.json"),
        freshness: MechanismFreshness::Provider,
    };
    let (group, name, hash) = match (
        Group::parse("org.example"),
        PackageName::parse("build-tools"),
        ContentHash::parse("sha256:aa"),
    ) {
        (Ok(group), Ok(name), Ok(hash)) => (group, name, hash),
        _ => panic!("the fixture identity parses"),
    };
    ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(group, name),
                root: PathBuf::from("vibedeps/build-tools"),
                version: "1.0.0".into(),
                kind: PackageKind::Tool,
                content_hash: hash,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: vec![declaration],
        }],
        host: host_source(Vec::new()),
        effective_stack: None,
    }
}

fn registry(world: &ExtensionWorld) -> MechanismRegistry {
    match collect_mechanisms(world) {
        Ok(registry) => registry,
        Err(error) => panic!("the fixture world collects: {error}"),
    }
}

fn pin(spelling: &str) -> ProviderPin {
    match ProviderPin::parse(spelling) {
        Ok(parsed) => parsed,
        Err(error) => panic!("`{spelling}` is a provider pin: {error}"),
    }
}

fn execution<'a>(
    root: &'a Path,
    targets: &'a [ArtifactBuildTarget],
    registry: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
) -> BuildExecution<'a> {
    BuildExecution {
        project_root: root,
        targets,
        registry,
        routes,
        build_root: BuildExecution::default_build_root(),
        offline: true,
        created_at: "2026-08-30T00:00:00Z",
    }
}

/// Nothing was built and nothing was recorded.
fn nothing_happened(root: &Path) {
    assert!(!root.join("target").exists(), "no build output root");
    assert!(!root.join(".vibe").exists(), "no artifact record");
}

fn temp() -> TempDir {
    match TempDir::new() {
        Ok(root) => root,
        Err(error) => panic!("a temp project opens: {error}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_host_route_displaces_the_builtin_and_the_builtin_does_not_run() {
    let root = temp();
    let world = world_with_plugin();
    let registry = registry(&world);
    let mut routes = MechanismRoutes::default();
    routes.insert(key("build:cargo"), pin(PLUGIN_PIN));
    let targets = vec![target("vibe-helper")];

    let refusal = execute_build_targets(&execution(root.path(), &targets, &registry, &routes))
        .expect_err("the routed provider needs a transport that has not landed");

    match &refusal {
        BuildError::TransportNotLanded { key, pin, kind } => {
            assert_eq!(key, "build:cargo");
            assert_eq!(pin, PLUGIN_PIN);
            assert_eq!(kind, "native");
        }
        other => panic!("expected a transport refusal, got {other}"),
    }
    assert!(refusal.to_string().contains("was NOT built by the builtin"));
    nothing_happened(root.path());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_exact_target_pin_routes_away_from_the_builtin_too() {
    let root = temp();
    let world = world_with_plugin();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let mut declared = target("vibe-helper");
    declared.provider = Some(pin(PLUGIN_PIN));
    let targets = vec![declared];

    let refusal = execute_build_targets(&execution(root.path(), &targets, &registry, &routes))
        .expect_err("a pinned plugin needs the same transport");

    assert!(matches!(refusal, BuildError::TransportNotLanded { .. }));
    nothing_happened(root.path());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_installed_plugin_nobody_selects_leaves_the_builtin_selected() {
    // The same world, with no route and no pin: §3.1 step 3 answers, and
    // the executor reaches the builtin adapter — proven here by the
    // adapter's own refusal over a project that holds no Cargo manifest,
    // which only the builtin can produce.
    let root = temp();
    let world = world_with_plugin();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = vec![target("vibe-helper")];

    let refusal = execute_build_targets(&execution(root.path(), &targets, &registry, &routes))
        .expect_err("an empty directory is not a Cargo workspace");

    match refusal {
        BuildError::Provider(MechanismError::NonZero { program, .. }) => {
            assert!(program.starts_with("cargo metadata"), "{program}");
        }
        other => panic!("expected the builtin adapter's own refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_unroutable_key_refuses_through_the_one_resolver() {
    let root = temp();
    let world = world_with_plugin();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let mut declared = target("vibe-helper");
    declared.mechanism = key("build:zig");
    let targets = vec![declared];

    let refusal = execute_build_targets(&execution(root.path(), &targets, &registry, &routes))
        .expect_err("this world ships no `build:zig` provider");

    assert!(matches!(refusal, BuildError::Resolution(_)));
    assert!(refusal.to_string().contains("no shipped builtin default"));
    nothing_happened(root.path());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_project_with_no_build_targets_builds_nothing() {
    let root = temp();
    let world = world_with_plugin();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();

    let outcomes = match execute_build_targets(&execution(root.path(), &[], &registry, &routes)) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("an empty graph executes: {error}"),
    };

    assert!(outcomes.is_empty());
    nothing_happened(root.path());
}

/// A target consuming another target's output.
fn consumer(id: &str, consumed: &str) -> ArtifactBuildTarget {
    ArtifactBuildTarget {
        id: id.to_owned(),
        mechanism: key("build:cargo"),
        provider: None,
        workdir: ".".to_owned(),
        inputs: Some(vec![ArtifactInput::Artifact {
            artifact: consumed.to_owned(),
        }]),
        outputs: vec![ArtifactOutput {
            id: format!("{id}.exe"),
            kind: ArtifactKind::Executable,
            select: None,
        }],
        config: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn dependency_order_runs_producers_first_whatever_the_declaration_order() {
    // Declared consumer-first; executed producer-first.
    let targets = vec![consumer("late", "early.exe"), target("early")];

    let sequence = match order(&targets) {
        Ok(sequence) => sequence,
        Err(error) => panic!("the graph orders: {error}"),
    };

    assert_eq!(sequence, vec![1, 0]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn independent_targets_keep_declaration_order() {
    let targets = vec![target("a"), target("b"), target("c")];

    let sequence = match order(&targets) {
        Ok(sequence) => sequence,
        Err(error) => panic!("the graph orders: {error}"),
    };

    assert_eq!(sequence, vec![0, 1, 2]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_cycle_refuses_and_names_it() {
    let targets = vec![consumer("a", "b.exe"), consumer("b", "a.exe")];

    let refusal = order(&targets).expect_err("a cycle has no dependency order");

    match &refusal {
        BuildError::Cycle { cycle } => assert_eq!(cycle, "a -> b -> a"),
        other => panic!("expected a cycle refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn an_input_no_build_target_produces_refuses() {
    let targets = vec![consumer("late", "absent.exe")];

    let refusal = order(&targets).expect_err("the consumed artifact has no producer here");

    match &refusal {
        BuildError::UnknownInput { target, input } => {
            assert_eq!(target, "late");
            assert_eq!(input, "absent.exe");
        }
        other => panic!("expected an unknown-input refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_target_that_consumes_its_own_output_is_not_a_cycle() {
    let mut declared = target("self");
    declared.inputs = Some(vec![ArtifactInput::Artifact {
        artifact: "self.exe".to_owned(),
    }]);

    let sequence = match order(std::slice::from_ref(&declared)) {
        Ok(sequence) => sequence,
        Err(error) => panic!("a self-edge is not a dependency: {error}"),
    };

    assert_eq!(sequence, vec![0]);
}
