use std::path::PathBuf;

use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{
    ExtensionAppliesTo, ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionKey,
    ExtensionsControl,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};

use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider,
};

pub(super) fn provider_id(group: &str, name: &str) -> DependencyProviderId {
    DependencyProviderId::new(
        Group::parse(group).unwrap_or_else(|error| panic!("valid test group: {error}")),
        PackageName::parse(name).unwrap_or_else(|error| panic!("valid test package name: {error}")),
    )
}

pub(super) fn dependency(
    group: &str,
    name: &str,
    declarations: Vec<ExtensionDecl>,
) -> DependencyExtensionSource {
    dependency_with_kind(group, name, PackageKind::Tool, declarations)
}

pub(super) fn dependency_with_kind(
    group: &str,
    name: &str,
    kind: PackageKind,
    declarations: Vec<ExtensionDecl>,
) -> DependencyExtensionSource {
    dependency_with_controls(
        group,
        name,
        kind,
        declarations,
        ExtensionsControl::default(),
    )
}

pub(super) fn dependency_with_controls(
    group: &str,
    name: &str,
    kind: PackageKind,
    declarations: Vec<ExtensionDecl>,
    controls: ExtensionsControl,
) -> DependencyExtensionSource {
    DependencyExtensionSource {
        provider: DependencyProvider {
            id: provider_id(group, name),
            root: PathBuf::from(format!("vibedeps/{name}")),
            version: "1.2.3".into(),
            kind,
            content_hash: ContentHash::parse("sha256:aa")
                .unwrap_or_else(|error| panic!("valid test hash: {error}")),
        },
        declarations,
        controls,
    }
}

pub(super) fn host(
    declarations: Vec<ExtensionDecl>,
    controls: ExtensionsControl,
) -> HostExtensionSource {
    HostExtensionSource {
        provider: HostProvider {
            identity: HostIdentity::ungrouped_project("demo"),
            root: PathBuf::from("."),
            version: "0.1.0".into(),
            kind: None,
            content_hash: None,
        },
        declarations,
        controls,
    }
}

pub(super) fn world(
    installed: Vec<DependencyExtensionSource>,
    host: HostExtensionSource,
    effective_stack: Option<DependencyProviderId>,
) -> ExtensionWorld {
    ExtensionWorld {
        installed,
        host,
        effective_stack,
    }
}

pub(super) fn declaration(id: &str, point: &str) -> ExtensionDecl {
    ExtensionDecl {
        id: id.into(),
        point: point
            .parse::<ExtensionPoint>()
            .unwrap_or_else(|error| panic!("valid test extension point: {error}")),
        handler: ExtensionHandler::Builtin { name: id.into() },
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    }
}

pub(super) fn selected_declaration(
    id: &str,
    packages: Option<Vec<&str>>,
    paths: Option<Vec<&str>>,
) -> ExtensionDecl {
    let mut declaration = declaration(id, "compile:source");
    declaration.applies_to = Some(ExtensionAppliesTo {
        packages: packages.map(strings),
        paths: paths.map(strings),
    });
    declaration
}

pub(super) fn config(entries: &[(&str, &str)]) -> ExtensionConfig {
    let table = entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), toml::Value::String((*value).to_owned())))
        .collect();
    ExtensionConfig::from_table(table)
}

pub(super) fn package_key(group: &str, name: &str, id: &str) -> ExtensionKey {
    let provider = provider_id(group, name);
    ExtensionKey::for_package(provider.group(), provider.name(), id)
}

pub(super) fn strings(values: Vec<&str>) -> Vec<String> {
    values.into_iter().map(str::to_owned).collect()
}
