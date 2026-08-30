//! Shared fixtures for the package-phase suites.
//!
//! One home, for the reason the production cells have one: the routing
//! reds, the two provider-law suites, the two end-to-end tests and the
//! chained test all need the same world, the same registry and the same
//! target shapes, and a second copy of any of them would be a second thing
//! to drift from the manifest grammar.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{
    ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget, ExtensionConfig,
    ExtensionHandler, ExtensionsControl, MechanismDecl, MechanismFreshness, MechanismKey,
    MechanismRole, MechanismRoutes, ProviderPin,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_extension_registry::{MechanismRegistry, collect_mechanisms};

use crate::mechanism::package::{PackageExecution, PackageOutcome, execute_package_targets};
use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider,
};

/// The pin of the installed plugin provider the routing reds select.
pub(crate) const PLUGIN_PIN: &str = "org.example/packagers#skill-v2";

pub(crate) fn temp() -> TempDir {
    match TempDir::new() {
        Ok(root) => root,
        Err(error) => panic!("a temp project opens: {error}"),
    }
}

pub(crate) fn key(spelling: &str) -> MechanismKey {
    match spelling.parse() {
        Ok(parsed) => parsed,
        Err(error) => panic!("`{spelling}` is a mechanism key: {error}"),
    }
}

pub(crate) fn pin(spelling: &str) -> ProviderPin {
    match ProviderPin::parse(spelling) {
        Ok(parsed) => parsed,
        Err(error) => panic!("`{spelling}` is a provider pin: {error}"),
    }
}

pub(crate) fn config(toml_text: &str) -> ExtensionConfig {
    match toml_text.parse::<toml::Table>() {
        Ok(parsed) => ExtensionConfig::from_table(parsed),
        Err(error) => panic!("the fixture table parses: {error}"),
    }
}

/// Write one fixture file, creating its parents.
pub(crate) fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        panic!("the fixture directory creates: {error}");
    }
    if let Err(error) = std::fs::write(&path, contents) {
        panic!("the fixture file writes: {error}");
    }
}

/// Write one fixture file and mark it executable.
///
/// `#[cfg(unix)]` because the execute bit is: on Windows the equivalent
/// law is the program-extension list, which its own test exercises.
#[cfg(unix)]
pub(crate) fn write_executable(root: &Path, relative: &str, contents: &str) {
    use std::os::unix::fs::PermissionsExt;
    write(root, relative, contents);
    let path = root.join(relative);
    if let Err(error) = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)) {
        panic!("the fixture permission sets: {error}");
    }
}

fn host_source() -> HostExtensionSource {
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
        mechanisms: Vec::new(),
    }
}

/// A world with no installed package at all.
pub(crate) fn empty_world() -> ExtensionWorld {
    ExtensionWorld {
        installed: Vec::new(),
        host: host_source(),
        effective_stack: None,
    }
}

/// A world whose installed package declares a NATIVE
/// `package:static-skill` provider — installed, collected, and inert until
/// something selects it.
pub(crate) fn world_with_plugin() -> ExtensionWorld {
    let declaration = MechanismDecl {
        id: "skill-v2".into(),
        role: MechanismRole::Package,
        name: "static-skill".into(),
        handler: ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from("crates/skill-provider")),
            prebuilt: None,
        },
        protocol: 1,
        config_schema: PathBuf::from("schemas/package-static-skill-v1.jtd.json"),
        freshness: MechanismFreshness::Engine,
    };
    let (group, name, hash) = match (
        Group::parse("org.example"),
        PackageName::parse("packagers"),
        ContentHash::parse("sha256:aa"),
    ) {
        (Ok(group), Ok(name), Ok(hash)) => (group, name, hash),
        _ => panic!("the fixture identity parses"),
    };
    ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(group, name),
                root: PathBuf::from("vibedeps/packagers"),
                version: "1.0.0".into(),
                kind: PackageKind::Tool,
                content_hash: hash,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: vec![declaration],
        }],
        host: host_source(),
        effective_stack: None,
    }
}

pub(crate) fn registry(world: &ExtensionWorld) -> MechanismRegistry {
    match collect_mechanisms(world) {
        Ok(registry) => registry,
        Err(error) => panic!("the fixture world collects: {error}"),
    }
}

/// One `package:static-skill` target over a source directory and its
/// declared textual resources.
pub(crate) fn skill_target(id: &str, source: &str, resources: &[&str]) -> ArtifactPackageTarget {
    ArtifactPackageTarget {
        id: id.to_owned(),
        mechanism: key("package:static-skill"),
        provider: None,
        inputs: Some(
            resources
                .iter()
                .map(|path| ArtifactInput::Path {
                    path: PathBuf::from(path),
                })
                .collect(),
        ),
        outputs: vec![ArtifactOutput {
            id: format!("{id}.md"),
            kind: ArtifactKind::File,
            select: None,
        }],
        config: Some(config(&format!("source = \"{source}\""))),
    }
}

/// One `package:agent-plugin` target over a source tree, with declared
/// inputs and their placements.
pub(crate) fn plugin_target(
    id: &str,
    source: &str,
    inputs: Vec<ArtifactInput>,
    place: &[(&str, &str)],
) -> ArtifactPackageTarget {
    let mut table = format!("source = \"{source}\"\n");
    if !place.is_empty() {
        table.push_str("[place]\n");
        for (name, destination) in place {
            table.push_str(&format!("\"{name}\" = \"{destination}\"\n"));
        }
    }
    ArtifactPackageTarget {
        id: id.to_owned(),
        mechanism: key("package:agent-plugin"),
        provider: None,
        inputs: Some(inputs),
        outputs: vec![ArtifactOutput {
            id: format!("{id}.dir"),
            // §6.2's package unit is a directory and its recorded KIND is
            // `agent-plugin` — the two are different questions, and only
            // the second tells a canonical plugin from a projection of one.
            kind: ArtifactKind::AgentPlugin,
            select: None,
        }],
        config: Some(config(&table)),
    }
}

pub(crate) fn execution<'a>(
    root: &'a Path,
    targets: &'a [ArtifactPackageTarget],
    registry: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
) -> PackageExecution<'a> {
    PackageExecution {
        project_root: root,
        targets,
        registry,
        routes,
        package_root: PackageExecution::default_package_root(),
        created_at: "2026-08-30T00:00:00Z",
    }
}

/// Run one package execution over a default-routed empty world.
pub(crate) fn run_default(
    root: &Path,
    targets: &[ArtifactPackageTarget],
) -> Result<Vec<PackageOutcome>, crate::PackageError> {
    let world = empty_world();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    execute_package_targets(&execution(root, targets, &registry, &routes))
}

/// The canonical minimal skill source, written under `skills/demo`.
pub(crate) fn write_demo_skill(root: &Path, body: &str) {
    write(
        root,
        "skills/demo/SKILL.md",
        &format!(
            "---\nname: demo\ndescription: A demonstration skill for the packaging tests.\n\
             ---\n{body}"
        ),
    );
}

/// The canonical minimal plugin source, written under `plugin`.
pub(crate) fn write_demo_plugin(root: &Path) {
    write(
        root,
        "plugin/plugin.json",
        "{\n  \"name\": \"demo-plugin\",\n  \"version\": \"1.0.0\",\n  \
         \"description\": \"A demonstration plugin.\"\n}\n",
    );
    write(
        root,
        "plugin/skills/demo/SKILL.md",
        "---\nname: demo\ndescription: A packaged skill.\n---\n\nBody.\n",
    );
}
