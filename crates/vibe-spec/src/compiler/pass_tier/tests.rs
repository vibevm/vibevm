use std::path::PathBuf;

use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{
    ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionIrLevel, ExtensionKey,
    ExtensionPass, ExtensionPassKind, ExtensionUse, ExtensionsControl,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_extension_registry::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionRegistry,
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
};

use super::fault::{CompilePlanLoweringError, PassTierFault};
use super::lowering::{CompilePlans, lower_effective_compile_rows};
use super::plan::{PassEntry, PassPlacement, PassPlan};
use crate::ArtifactPlan;
use crate::compiler::ir::ArtifactTarget;

fn host_key(id: &str) -> String {
    format!("__host__/demo#{id}")
}

fn dependency_key(id: &str) -> String {
    format!("org.demo/tools#{id}")
}

fn native() -> ExtensionHandler {
    ExtensionHandler::Native {
        crate_dir: Some(PathBuf::from("crates/plugin")),
        prebuilt: None,
    }
}

fn staged(id: &str, point: &str) -> ExtensionDecl {
    ExtensionDecl {
        id: id.to_owned(),
        point: point.parse::<ExtensionPoint>().unwrap(),
        handler: native(),
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    }
}

fn transform(level: ExtensionIrLevel, after: &str) -> ExtensionPass {
    ExtensionPass {
        kind: ExtensionPassKind::Transform,
        level: Some(level),
        from: None,
        to: None,
        after: Some(after.to_owned()),
        before: None,
        replace: None,
        formats: None,
        artifact: None,
    }
}

fn backend(artifact: &str) -> ExtensionPass {
    ExtensionPass {
        kind: ExtensionPassKind::Backend,
        level: None,
        from: None,
        to: None,
        after: None,
        before: None,
        replace: None,
        formats: None,
        artifact: Some(artifact.to_owned()),
    }
}

fn pass(id: &str, declaration: Option<ExtensionPass>) -> ExtensionDecl {
    ExtensionDecl {
        id: id.to_owned(),
        point: "compile:pass".parse().unwrap(),
        handler: ExtensionHandler::Builtin {
            name: format!("handler-{id}"),
        },
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: Some(true),
        pass: declaration,
        when: None,
    }
}

fn registry(
    host: Vec<ExtensionDecl>,
    dependency: Vec<ExtensionDecl>,
    uses: Vec<ExtensionUse>,
    disable: Vec<ExtensionKey>,
) -> ExtensionRegistry {
    let dependency_provider = DependencyProvider {
        id: DependencyProviderId::new(
            Group::parse("org.demo").unwrap(),
            PackageName::parse("tools").unwrap(),
        ),
        root: PathBuf::from("vibedeps/tools"),
        version: "1.0.0".into(),
        kind: PackageKind::Tool,
        content_hash: ContentHash::parse("sha256:aa").unwrap(),
    };
    collect_extensions(ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: dependency_provider,
            declarations: dependency,
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        }],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: PathBuf::from("."),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: host,
            controls: ExtensionsControl { uses, disable },
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .unwrap()
}

fn lower(registry: &ExtensionRegistry) -> Result<CompilePlans, CompilePlanLoweringError> {
    lower_effective_compile_rows(&registry.enabled_compile_rows())
}

#[test]
fn one_walk_partitions_without_reordering_effective_ordinals() {
    let registry = registry(
        vec![
            staged("source", "compile:source"),
            pass(
                "closure-pass",
                Some(transform(ExtensionIrLevel::Closure, "qualify")),
            ),
            staged("document", "compile:document"),
            pass("json", Some(backend("json"))),
        ],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let plans = lower(&registry).unwrap();
    assert_eq!(plans.transforms().len(), 2);
    assert_eq!(plans.passes().len(), 2);
    assert_eq!(
        plans
            .passes()
            .entries()
            .iter()
            .map(PassEntry::ordinal)
            .collect::<Vec<_>>(),
        [1, 3]
    );
    assert_eq!(
        plans.passes().entries()[0].key().as_str(),
        host_key("closure-pass")
    );
    assert!(plans.passes().digest_hex().is_some());
}

#[test]
fn dependency_pass_needs_activation_direct_host_is_owned_and_disable_wins() {
    let dependency = pass(
        "dependency-pass",
        Some(transform(ExtensionIrLevel::Closure, "qualify")),
    );
    let inactive = registry(Vec::new(), vec![dependency.clone()], Vec::new(), Vec::new());
    assert!(lower(&inactive).unwrap().passes().is_empty());

    let activation = ExtensionUse {
        reference: ExtensionKey::authored(dependency_key("dependency-pass")),
        config: Some(ExtensionConfig::from_table(
            "mode='activated'".parse().unwrap(),
        )),
    };
    let active = registry(Vec::new(), vec![dependency], vec![activation], Vec::new());
    let plan = lower(&active).unwrap().passes().clone();
    assert_eq!(plan.len(), 1);
    assert!(plan.entries()[0].config().is_some());

    let host = pass(
        "host-pass",
        Some(transform(ExtensionIrLevel::Closure, "qualify")),
    );
    assert_eq!(
        lower(&registry(
            vec![host.clone()],
            Vec::new(),
            Vec::new(),
            Vec::new()
        ))
        .unwrap()
        .passes()
        .len(),
        1
    );
    let disabled = registry(
        vec![host],
        Vec::new(),
        Vec::new(),
        vec![ExtensionKey::authored(host_key("host-pass"))],
    );
    assert!(lower(&disabled).unwrap().passes().is_empty());
}

#[test]
fn an_activated_pass_without_a_pass_table_refuses_by_key_and_row() {
    let registry = registry(
        vec![pass("missing", None)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let error = lower(&registry).unwrap_err();
    assert!(matches!(
        error.inner(),
        PassTierFault::MissingPass { row: 0, key } if key == &host_key("missing")
    ));
}

#[test]
fn entries_retain_handler_config_placement_provider_and_artifact() {
    let mut declaration = pass("xml", Some(backend("static-xml")));
    declaration.handler = native();
    declaration.config = Some(ExtensionConfig::from_table(
        "list=[3,1]\n[nested]\nvalue=nan".parse().unwrap(),
    ));
    let registry = registry(vec![declaration], Vec::new(), Vec::new(), Vec::new());
    let first = lower(&registry).unwrap();
    let second = lower(&registry).unwrap();
    assert_eq!(first, second);
    let entry = &first.passes().entries()[0];
    assert!(matches!(entry.handler(), ExtensionHandler::Native { .. }));
    assert!(entry.config().is_some());
    assert!(matches!(entry.placement(), PassPlacement::Intrinsic));
    assert_eq!(entry.declaration().artifact.as_deref(), Some("static-xml"));
    assert!(matches!(
        entry.provider().components(),
        crate::compiler::transform::plan::ProviderComponents::Host { .. }
    ));
}

#[test]
fn digest_is_canonical_and_moves_with_pass_semantics() {
    let configured = |after: &str, reverse: bool| {
        let mut table = toml::Table::new();
        if reverse {
            table.insert("z".into(), toml::Value::Integer(2));
            table.insert("a".into(), toml::Value::Integer(1));
        } else {
            table.insert("a".into(), toml::Value::Integer(1));
            table.insert("z".into(), toml::Value::Integer(2));
        }
        let mut declaration = pass("stable", Some(transform(ExtensionIrLevel::Closure, after)));
        declaration.config = Some(ExtensionConfig::from_table(table));
        declaration
    };
    let first = lower(&registry(
        vec![configured("qualify", false)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    let reordered_table = lower(&registry(
        vec![configured("qualify", true)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    assert_eq!(
        first.passes().digest_hex(),
        reordered_table.passes().digest_hex()
    );

    let moved = lower(&registry(
        vec![configured("link", false)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    assert_ne!(first.passes().digest_hex(), moved.passes().digest_hex());

    let mut other_handler = configured("qualify", false);
    other_handler.handler = ExtensionHandler::Builtin {
        name: "different-handler".into(),
    };
    let other_handler = lower(&registry(
        vec![other_handler],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    assert_ne!(
        first.passes().digest_hex(),
        other_handler.passes().digest_hex()
    );
}

#[test]
fn artifact_discriminator_filters_the_attached_plan_and_empty_is_identity() {
    let registry = registry(
        vec![
            pass("xml", Some(backend("static-xml"))),
            pass("json", Some(backend("json"))),
        ],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let passes = lower(&registry).unwrap().passes().clone();
    let base = ArtifactPlan::static_lane(
        ArtifactTarget::StaticXml,
        "vibevm/vibespecs/boot/STATIC.xml",
        "vibevm/vibespecs",
        Vec::new(),
    )
    .unwrap();
    assert_eq!(base.clone().with_passes(PassPlan::empty()), base);
    let selected = base.with_passes(passes);
    assert_eq!(selected.passes().len(), 1);
    assert_eq!(
        selected.passes().entries()[0].key().as_str(),
        host_key("xml")
    );
}

#[test]
fn pass_planning_registers_and_executes_nothing() {
    let source = concat!(include_str!("lowering.rs"), include_str!("plan.rs"));
    for forbidden in ["apply_builtin_edits", "PipelineEdit", ".run(", "invoke("] {
        assert!(
            !source.contains(forbidden),
            "unexpected execution token {forbidden}"
        );
    }
    let plan = PassPlan::empty();
    assert!(plan.is_empty());
    assert_eq!(plan.digest_hex(), None);
}
