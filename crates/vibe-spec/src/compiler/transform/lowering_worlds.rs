//! Real collected registries for the T10B lowering tests.
//!
//! Nothing here fabricates a row: every registry is produced by the kernel's
//! own `collect_extensions` over an `ExtensionWorld`, so keys, tiers,
//! effective order, compiled selectors and effective configs are exactly what
//! a workspace-collected owner view carries. The lowering is then exercised
//! on `enabled_compile_rows()` — the same input contract the workspace
//! supplies — rather than on a hand-built slice a caller could shape to fit.

use std::path::PathBuf;

use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{
    ExtensionAppliesTo, ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionKey,
    ExtensionUse, ExtensionsControl,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_extension_registry::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionRegistry,
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
};

/// One authored declaration, in the shape a manifest carries it.
pub(super) struct Declared {
    pub(super) id: &'static str,
    pub(super) point: &'static str,
    pub(super) handler: ExtensionHandler,
    pub(super) config: Option<ExtensionConfig>,
    pub(super) applies_to: Option<ExtensionAppliesTo>,
    /// `compile:pass` is the one point the manifest grammar requires this
    /// flag at (PROP-054 `##COMPILER-INTERNALS-FLAG`), so a pass fixture is
    /// only collectible with it set.
    pub(super) compiler_internals: Option<bool>,
}

impl Declared {
    /// A builtin declaration at one point, with no config and no selector.
    pub(super) fn builtin(id: &'static str, point: &'static str, name: &str) -> Self {
        Self {
            id,
            point,
            handler: ExtensionHandler::Builtin {
                name: name.to_owned(),
            },
            config: None,
            applies_to: None,
            compiler_internals: None,
        }
    }

    /// A builtin `compile:pass` declaration, with the flag its point
    /// requires.
    pub(super) fn pass(id: &'static str, name: &str) -> Self {
        Self {
            compiler_internals: Some(true),
            ..Self::builtin(id, "compile:pass", name)
        }
    }

    /// A `native` handler at one compile point — the one non-builtin kind
    /// the manifest grammar admits there, and the arm R5 fills in.
    pub(super) fn native(id: &'static str, point: &'static str) -> Self {
        Self {
            handler: ExtensionHandler::Native {
                // The grammar requires one of the two source fields.
                crate_dir: Some(PathBuf::from("crates/demo-transform")),
                prebuilt: None,
            },
            ..Self::builtin(id, point, "unused")
        }
    }

    /// The same, carrying an authored `applies_to.paths` dimension.
    pub(super) fn scoped(mut self, paths: &[&str]) -> Self {
        self.applies_to = Some(ExtensionAppliesTo {
            packages: None,
            paths: Some(paths.iter().map(|path| (*path).to_owned()).collect()),
        });
        self
    }

    /// The same, carrying an authored (possibly cleared) configuration.
    pub(super) fn configured(mut self, config: ExtensionConfig) -> Self {
        self.config = Some(config);
        self
    }

    fn decl(self) -> ExtensionDecl {
        ExtensionDecl {
            id: self.id.to_owned(),
            point: self
                .point
                .parse::<ExtensionPoint>()
                .expect("a valid test extension point"),
            handler: self.handler,
            config: self.config,
            auto: None,
            inputs: None,
            applies_to: self.applies_to,
            compiler_internals: self.compiler_internals,
            pass: None,
            when: None,
        }
    }
}

/// The world's one host: an ungrouped project named `demo`.
fn host_provider() -> HostProvider {
    HostProvider {
        identity: HostIdentity::ungrouped_project("demo"),
        root: PathBuf::from("."),
        version: "0.1.0".to_owned(),
        kind: None,
        content_hash: None,
    }
}

/// The world's one installed dependency: `org.demo/tools`.
fn dependency_provider() -> DependencyProvider {
    DependencyProvider {
        id: DependencyProviderId::new(
            Group::parse("org.demo").expect("a valid test group"),
            PackageName::parse("tools").expect("a valid test package name"),
        ),
        root: PathBuf::from("vibedeps/tools"),
        version: "1.2.3".to_owned(),
        kind: PackageKind::Tool,
        content_hash: ContentHash::parse("sha256:aa").expect("a valid test hash"),
    }
}

/// The printable key one host declaration takes.
pub(super) fn host_key(id: &str) -> String {
    format!("__host__/demo#{id}")
}

/// The printable key one dependency declaration takes.
pub(super) fn dependency_key(id: &str) -> String {
    format!("org.demo/tools#{id}")
}

/// Collect one owner view: host declarations, dependency declarations, and
/// the host activations that make dependency compile rows enabled.
///
/// A dependency's compile contribution is inert until the host activates it
/// (`active_by_default: !is_compile`), so `activate` is what puts a
/// dependency row inside `enabled_compile_rows()` at all — and it moves the
/// row into the activation tier, which is how a test can observe that the
/// lowering preserves the kernel's ONE effective order rather than any
/// per-point grouping.
pub(super) fn collected(
    host: Vec<Declared>,
    installed: Vec<Declared>,
    activate: &[&str],
) -> ExtensionRegistry {
    let world = ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: dependency_provider(),
            declarations: installed.into_iter().map(Declared::decl).collect(),
            controls: ExtensionsControl::default(),
        }],
        host: HostExtensionSource {
            provider: host_provider(),
            declarations: host.into_iter().map(Declared::decl).collect(),
            controls: ExtensionsControl {
                uses: activate
                    .iter()
                    .map(|id| ExtensionUse {
                        reference: ExtensionKey::authored(dependency_key(id)),
                        config: None,
                    })
                    .collect(),
                disable: Vec::new(),
            },
        },
        effective_stack: None,
    };
    collect_extensions(world).expect("the test world collects")
}

/// The one-host-declaration shorthand.
pub(super) fn collected_host(host: Vec<Declared>) -> ExtensionRegistry {
    collected(host, Vec::new(), &[])
}
