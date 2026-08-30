//! Shared builders for the transform-plan tests: providers lowered the way
//! T10 will lower them, and selectors obtained the only way anything
//! outside the kernel can obtain one — collected by the real registry from
//! a declaration's `applies_to`, then cloned read-only. No second selector
//! compiler exists to drift against.

use std::path::PathBuf;

use vibe_core::manifest::{
    ExtensionAppliesTo, ExtensionDecl, ExtensionHandler, ExtensionKey, ExtensionsControl,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_extension_registry::{
    CompiledSelector, DependencyProvider, DependencyProviderId, ExtensionProvider, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
};

use super::plan::TransformPlan;
use super::plan::{
    TransformConfig, TransformImplementation, TransformProvider, TransformSeed, TransformStage,
};

/// A dependency provider with the given coordinate, version, kind and hash.
pub(super) fn dependency_provider(
    group: &str,
    name: &str,
    version: &str,
    kind: PackageKind,
    hash: &str,
) -> ExtensionProvider {
    ExtensionProvider::Dependency(DependencyProvider {
        id: DependencyProviderId::new(
            Group::parse(group).expect("valid test group"),
            PackageName::parse(name).expect("valid test package name"),
        ),
        root: PathBuf::from(format!("vibedeps/{name}")),
        version: version.to_owned(),
        kind,
        content_hash: ContentHash::parse(hash).expect("valid test hash"),
    })
}

/// A dependency provider with the default test metadata.
pub(super) fn default_dependency() -> ExtensionProvider {
    dependency_provider("org.demo", "tools", "1.2.3", PackageKind::Tool, "sha256:aa")
}

/// An ungrouped-host provider with the default test metadata.
pub(super) fn ungrouped_host(name: &str) -> ExtensionProvider {
    ExtensionProvider::Host(HostProvider {
        identity: HostIdentity::ungrouped_project(name),
        root: PathBuf::from("."),
        version: "0.1.0".to_owned(),
        kind: None,
        content_hash: None,
    })
}

/// A host provider with an explicit kind and hash.
pub(super) fn host_with(
    identity: HostIdentity,
    kind: Option<PackageKind>,
    hash: Option<&str>,
) -> ExtensionProvider {
    ExtensionProvider::Host(HostProvider {
        identity,
        root: PathBuf::from("."),
        version: "0.1.0".to_owned(),
        kind,
        content_hash: hash.map(|hash| ContentHash::parse(hash).expect("valid test hash")),
    })
}

/// One `compile:source` builtin declaration carrying a selector shape.
fn selector_declaration(id: &str, shape: SelectorShape) -> ExtensionDecl {
    let mut declaration = ExtensionDecl {
        id: id.to_owned(),
        point: "compile:source"
            .parse()
            .expect("compile:source is a valid test extension point"),
        handler: ExtensionHandler::Builtin {
            name: "log".to_owned(),
        },
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    };
    if let SelectorShape::Dimensions { packages, paths } = shape {
        declaration.applies_to = Some(ExtensionAppliesTo {
            packages: packages
                .map(|members| members.iter().map(|member| member.to_string()).collect()),
            paths: paths.map(|members| members.iter().map(|member| member.to_string()).collect()),
        });
    }
    declaration
}

/// The authored shape of one test declaration's `applies_to` table.
pub(super) enum SelectorShape {
    /// No `applies_to` at all.
    Absent,
    /// An `applies_to` table with optional dimensions in authored order.
    Dimensions {
        packages: Option<Vec<&'static str>>,
        paths: Option<Vec<&'static str>>,
    },
}

/// Collect real compiled selectors for the given authored shapes.
///
/// The registry is the single glob compiler; these selectors are exactly
/// what a workspace adapter would clone off a collected row.
pub(super) fn compiled_selectors(shapes: &[SelectorShape]) -> Vec<CompiledSelector> {
    let declarations = shapes
        .iter()
        .enumerate()
        .map(|(index, shape)| selector_declaration(&format!("sel{index}"), clone_shape(shape)))
        .collect();
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: PathBuf::from("."),
                version: "0.1.0".to_owned(),
                kind: None,
                content_hash: None,
            },
            declarations,
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    let registry = collect_extensions(world).expect("test world collects");
    (0..shapes.len())
        .map(|index| {
            let suffix = format!("#sel{index}");
            registry
                .rows()
                .iter()
                .find(|row| row.key().as_str().ends_with(&suffix))
                .unwrap_or_else(|| panic!("selector test row with suffix `{suffix}` exists"))
                .compiled_selector()
                .clone()
        })
        .collect()
}

/// One compiled selector for a single authored shape.
pub(super) fn compiled_selector(shape: SelectorShape) -> CompiledSelector {
    compiled_selectors(&[shape]).pop().expect("one selector")
}

fn clone_shape(shape: &SelectorShape) -> SelectorShape {
    match shape {
        SelectorShape::Absent => SelectorShape::Absent,
        SelectorShape::Dimensions { packages, paths } => SelectorShape::Dimensions {
            packages: packages.clone(),
            paths: paths.clone(),
        },
    }
}

/// A dependency seed with the default metadata and no config/selector.
pub(super) fn dependency_seed(key: &str, stage: TransformStage) -> TransformSeed {
    TransformSeed::new(
        ExtensionKey::authored(key),
        TransformProvider::from(&default_dependency()),
        stage,
        TransformImplementation::builtin_candidate("log", 1),
        None,
        None,
    )
}

/// Build a plan from seeds, panicking on refusal (for laws already proven
/// by the refusal tests).
pub(super) fn build_or_panic(seeds: Vec<TransformSeed>) -> TransformPlan {
    TransformPlan::build(seeds).expect("lawful test plan builds")
}

/// An authored-empty effective config.
pub(super) fn empty_config() -> TransformConfig {
    TransformConfig::new(super::config::ConfigTable::new())
}
