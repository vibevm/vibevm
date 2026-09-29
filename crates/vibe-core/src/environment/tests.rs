specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-PROOF-BOUNDARY");
use super::*;
use crate::{
    ContentHash, PackageRef,
    manifest::{TargetApplicability, TargetOs, TargetWhen},
};
use std::path::PathBuf;
#[path = "export_assertions.rs"]
mod export_assertions;
fn r(text: &str) -> Reference {
    Reference::new(text).unwrap()
}
fn linux() -> PlatformFacts {
    PlatformFacts::new(
        Fact::Known(TargetOs::Linux),
        Fact::Known(r("x86_64")),
        Fact::Known(r("gnu")),
        Fact::Known(r("glibc")),
    )
}
fn windows() -> PlatformFacts {
    PlatformFacts::new(
        Fact::Known(TargetOs::Windows),
        Fact::Known(r("x86_64")),
        Fact::Known(r("msvc")),
        Fact::NotApplicable,
    )
}
fn requirements() -> PlatformConstraints {
    PlatformConstraints::new(
        Requirement::Exact(TargetOs::Linux),
        Requirement::Exact(r("x86_64")),
        Requirement::Exact(r("gnu")),
        Requirement::Exact(r("glibc")),
    )
}
fn composition() -> DefinitionComposition {
    DefinitionComposition {
        roots: vec![DesiredRoot {
            name: r("tools"),
            package: PackageRef::parse("flow:org.example/tool@^1").unwrap(),
        }],
        features: vec![r("core")],
        choices: vec![ChoiceSelection {
            name: r("edition"),
            value: r("next"),
        }],
        target_constraints: requirements(),
        runtime_bindings: vec![RuntimeBinding {
            name: r("runtime"),
            platform: linux(),
        }],
        tool_exports: vec![NamedExport {
            name: r("tool"),
            package_root: r("tools"),
            runtime_binding: r("runtime"),
        }],
        service_exports: vec![],
    }
}
fn definition() -> EnvironmentDefinition {
    let id = DefinitionId::new("definition", "author").unwrap();
    EnvironmentDefinition::new(id, composition()).unwrap()
}

fn closure(d: &EnvironmentDefinition) -> LockClosure {
    LockClosure {
        definition: d.id().clone(),
        target_constraints: d.composition().target_constraints.clone(),
        packages: vec![
            SelectedPackage::new(SelectedPackageFacts {
                root: Some(r("tools")),
                package: PackageRef::parse("flow:org.example/tool@=1.1.0").unwrap(),
                version: semver::Version::new(1, 1, 0),
                content: ContentHash::parse("sha256:abc").unwrap(),
                capture: r("tools-capture"),
                configuration: r("configuration"),
                role: DependencyRole::BuildAndRuntime,
            })
            .unwrap(),
        ],
        external_backends: vec![ExternalBackendClosure {
            backend: r("cargo"),
            closure: r("cargo-closure"),
            configuration: r("cargo-config"),
            role: DependencyRole::Build,
        }],
    }
}
fn lock(d: &EnvironmentDefinition) -> EnvironmentLock {
    EnvironmentLock::new(LockId::new("lock", "resolver").unwrap(), d, closure(d)).unwrap()
}
fn inventory(l: &EnvironmentLock) -> GenerationInventory {
    GenerationInventory {
        lock: l.id().clone(),
        recipes: vec![r("compose-r1")],
        payloads: vec![PayloadMetadata {
            name: r("tool"),
            package_capture: r("tools-capture"),
            payload_reference: r("artifact-record"),
            metadata_reference: r("metadata"),
        }],
        dependency_closure: l.members().into_iter().collect(),
        boot_adapters: vec![r("boot-index")],
        service_catalog: vec![],
        activation_links: vec![r("tool-activation")],
    }
}
fn generation(l: &EnvironmentLock) -> EnvironmentGeneration {
    EnvironmentGeneration::new(
        GenerationId::new("generation", "producer").unwrap(),
        l,
        inventory(l),
    )
    .unwrap()
}
fn executor(platform: PlatformFacts) -> ExecutionContext {
    ExecutionContext::new(
        ExecutionId::new("execution", "admitting-surface").unwrap(),
        ExecutorFacts {
            platform,
            tool: r("cargo-tool"),
            provider: r("provider"),
            principal: r("executor-user"),
            network_capabilities: vec![],
            mounts: vec![MountCapability {
                name: r("workspace"),
                root_reference: r("admitted-workspace"),
                access: MountAccess::ReadWrite,
            }],
            process_capabilities: vec![r("spawn-selected-tool")],
        },
    )
    .unwrap()
}
fn local_root() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from("C:\\workspace")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/workspace")
    }
}
fn target_facts(
    d: &EnvironmentDefinition,
    l: &EnvironmentLock,
    mode: TargetMode,
    platform: PlatformFacts,
) -> TargetFacts {
    TargetFacts {
        definition: d.id().clone(),
        lock: l.id().clone(),
        generation: None,
        mode,
        platform,
        roots: TargetRoots::new(
            local_root().to_str().unwrap(),
            "/usr",
            None,
            r("storage-owner"),
            None,
        )
        .unwrap(),
        principal: r("target-user"),
        storage_backend: r("filesystem"),
    }
}
fn target(
    d: &EnvironmentDefinition,
    l: &EnvironmentLock,
    mode: TargetMode,
    platform: PlatformFacts,
) -> TargetBinding {
    TargetBinding::new(
        TargetInstanceId::new("instance", "storage-owner").unwrap(),
        d,
        l,
        None,
        target_facts(d, l, mode, platform),
    )
    .unwrap()
}
fn mapping(e: &ExecutionContext, t: &TargetBinding) -> LocalWorkspaceMapping {
    LocalWorkspaceMapping::new(e.id().clone(), t.instance().clone(), local_root()).unwrap()
}

fn source_projection(
    d: &EnvironmentDefinition,
    e: &ExecutionContext,
    t: &TargetBinding,
) -> OperationCompatibility {
    workspace_source_projection_compatibility(d, e, t, &mapping(e, t))
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-ROLE-SEPARATION",
    r = 1
)]
fn role_ids_preserve_producers_and_do_not_conflate_equal_reference_text() {
    let d = DefinitionId::new("same", "owner").unwrap();
    let l = LockId::new("same", "owner").unwrap();
    let e = ExecutionId::new("same", "owner").unwrap();
    let t = TargetInstanceId::new("same", "owner").unwrap();
    let g = GenerationId::new("same", "owner").unwrap();
    assert_eq!(d.reference(), l.reference());
    assert_eq!(e.reference(), t.reference());
    assert_eq!(t.reference(), g.reference());
    assert_ne!(format!("{d:?}"), format!("{l:?}"));
    assert_ne!(d, DefinitionId::new("same", "another-producer").unwrap());
    let opaque = Reference::new(" capture:\nopaque ").unwrap();
    assert_eq!(opaque.as_str(), " capture:\nopaque ");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ABI-ROLE-SEPARATION",
    r = 1
)]
fn windows_executor_linux_payload_and_compiler_output_keep_three_roles() {
    let d = definition();
    let l = lock(&d);
    let e = executor(windows());
    let t = target(&d, &l, TargetMode::OfflineLinuxRootfs, linux());
    let emitted = PlatformFacts::new(
        Fact::NotApplicable,
        Fact::Known(r("wasm32")),
        Fact::Known(r("wasm")),
        Fact::NotApplicable,
    );
    let roles = ToolchainPlatforms::new(
        e.facts().platform.clone(),
        t.facts().platform.clone(),
        Some(emitted.clone()),
    );
    assert_eq!(roles.build_executor().os(), &Fact::Known(TargetOs::Windows));
    assert_eq!(
        roles.produced_tool_runtime().os(),
        &Fact::Known(TargetOs::Linux)
    );
    assert_eq!(roles.compiler_output(), Some(&emitted));
    assert_eq!(native_extension_platform(&e).abi(), &Fact::Known(r("msvc")));
    assert_eq!(
        payload_launcher_family(&t),
        LauncherDescription::Family(LauncherFamily::PosixShell)
    );
    let absent = ToolchainPlatforms::new(windows(), linux(), None);
    assert!(absent.compiler_output().is_none());
    let unresolved = PlatformFacts::new(Fact::Unknown, Fact::Unknown, Fact::Unknown, Fact::Unknown);
    assert!(
        ToolchainPlatforms::new(windows(), linux(), Some(unresolved))
            .compiler_output()
            .is_some()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#LOCAL-WORKSPACE-COMPATIBILITY",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#LEGACY-HOST-INTERPRETATION",
    r = 1
)]
fn explicit_local_workspace_mapping_preserves_legacy_guard_behavior() {
    let mut bare = composition();
    bare.target_constraints = PlatformConstraints::new(
        Requirement::Exact(TargetOs::Linux),
        Requirement::Exact(r("x86_64")),
        Requirement::Any,
        Requirement::Any,
    );
    let bare =
        EnvironmentDefinition::new(DefinitionId::new("bare", "author").unwrap(), bare).unwrap();
    let bare_lock = lock(&bare);
    let unknown = PlatformFacts::new(
        Fact::Known(TargetOs::Linux),
        Fact::Known(r("x86_64")),
        Fact::Unknown,
        Fact::Unknown,
    );
    let bare_executor = executor(unknown.clone());
    let bare_target = target(&bare, &bare_lock, TargetMode::WorkspaceProjection, unknown);
    assert_eq!(
        source_projection(&bare, &bare_executor, &bare_target),
        OperationCompatibility::SupportedLocalWorkspaceSourceProjection
    );
    let strict = definition();
    let strict_lock = lock(&strict);
    let strict_target = target(
        &strict,
        &strict_lock,
        TargetMode::WorkspaceProjection,
        bare_target.facts().platform.clone(),
    );
    let OperationCompatibility::PlatformMismatch(missing) =
        source_projection(&strict, &bare_executor, &strict_target)
    else {
        panic!()
    };
    assert!(missing.missing.contains(&PlatformField::Abi));
    assert!(missing.missing.contains(&PlatformField::Libc));
    let mismatch = target(
        &strict,
        &strict_lock,
        TargetMode::WorkspaceProjection,
        PlatformFacts::new(
            Fact::Known(TargetOs::Linux),
            Fact::Known(r("x86_64")),
            Fact::Known(r("musl")),
            Fact::Known(r("musl")),
        ),
    );
    let OperationCompatibility::PlatformMismatch(result) =
        source_projection(&strict, &executor(linux()), &mismatch)
    else {
        panic!()
    };
    assert!(result.unsatisfied.contains(&PlatformField::Abi));
    assert!(result.unsatisfied.contains(&PlatformField::Libc));

    let d = definition();
    let l = lock(&d);
    let e = executor(linux());
    let t = target(&d, &l, TargetMode::WorkspaceProjection, linux());
    let m = mapping(&e, &t);
    assert_eq!(
        workspace_source_projection_compatibility(&d, &e, &t, &m),
        OperationCompatibility::SupportedLocalWorkspaceSourceProjection
    );
    for guard in [
        None,
        Some(TargetWhen::new(vec![TargetOs::Windows]).unwrap()),
        Some(TargetWhen::new(vec![TargetOs::Linux, TargetOs::Macos]).unwrap()),
    ] {
        assert_eq!(
            legacy_host_applicability("tool", guard.as_ref(), &e).unwrap(),
            TargetApplicability::decide("tool", guard.as_ref(), TargetOs::Linux)
        );
    }
    let win = executor(windows());
    let guard = TargetWhen::new(vec![TargetOs::Windows]).unwrap();
    assert!(
        legacy_host_applicability("linux-payload", Some(&guard), &win)
            .unwrap()
            .is_active()
    );
    assert_eq!(
        payload_launcher_family(&t),
        LauncherDescription::Family(LauncherFamily::PosixShell)
    );
    for bad in [
        LocalWorkspaceMapping::new(
            ExecutionId::new("other", "surface").unwrap(),
            t.instance().clone(),
            local_root(),
        )
        .unwrap(),
        LocalWorkspaceMapping::new(
            e.id().clone(),
            TargetInstanceId::new("other", "storage").unwrap(),
            local_root(),
        )
        .unwrap(),
        LocalWorkspaceMapping::new(
            e.id().clone(),
            t.instance().clone(),
            local_root().join("other"),
        )
        .unwrap(),
    ] {
        assert!(matches!(
            workspace_source_projection_compatibility(&d, &e, &t, &bad),
            OperationCompatibility::BindingMismatch(_)
        ));
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#COMPATIBILITY-FACTS",
    r = 1
)]
fn os_match_does_not_hide_abi_libc_mismatch_or_missing_facts() {
    let bad = PlatformFacts::new(
        Fact::Known(TargetOs::Linux),
        Fact::Known(r("x86_64")),
        Fact::Known(r("musl")),
        Fact::Unknown,
    );
    let check = requirements().compare(&bad);
    assert_eq!(check.unsatisfied, [PlatformField::Abi]);
    assert_eq!(check.missing, [PlatformField::Libc]);
    let na = PlatformFacts::new(
        Fact::Known(TargetOs::Linux),
        Fact::Known(r("x86_64")),
        Fact::Known(r("gnu")),
        Fact::NotApplicable,
    );
    assert_eq!(
        requirements().compare(&na).unsatisfied,
        [PlatformField::Libc]
    );
    let d = definition();
    let l = lock(&d);
    let t = target(&d, &l, TargetMode::WorkspaceProjection, linux());
    let e = executor(PlatformFacts::new(
        Fact::Known(TargetOs::Linux),
        Fact::NotApplicable,
        Fact::Known(r("gnu")),
        Fact::Known(r("glibc")),
    ));
    let OperationCompatibility::PlatformMismatch(result) =
        workspace_source_projection_compatibility(&d, &e, &t, &mapping(&e, &t))
    else {
        panic!()
    };
    assert_eq!(result.unsatisfied, [PlatformField::Architecture]);
    assert!(!result.unsatisfied.contains(&PlatformField::Os));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#UNSUPPORTED-TARGET-OPERATIONS",
    r = 1
)]
fn future_target_modes_are_describable_but_operations_unsupported() {
    let d = definition();
    let l = lock(&d);
    let e = executor(linux());
    for mode in [
        TargetMode::UserProfile,
        TargetMode::OfflineLinuxRootfs,
        TargetMode::LiveLinuxSystem,
    ] {
        let t = target(&d, &l, mode, linux());
        assert_eq!(
            workspace_source_projection_compatibility(&d, &e, &t, &mapping(&e, &t)),
            OperationCompatibility::UnsupportedTargetMode(mode)
        );
        assert!(t.facts().roots.physical_evidence().is_none());
    }
    assert!(
        TargetBinding::new(
            TargetInstanceId::new("instance", "owner").unwrap(),
            &d,
            &l,
            None,
            target_facts(&d, &l, TargetMode::LiveLinuxSystem, windows())
        )
        .is_err()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-DEFINITION",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LOCK",
    r = 1
)]
fn definition_and_lock_reject_duplicate_or_foreign_bindings() {
    export_assertions::check_export_namespaces();
    let d = definition();
    let mut c = composition();
    c.roots.push(c.roots[0].clone());
    assert!(EnvironmentDefinition::new(d.id().clone(), c).is_err());
    let mut c = composition();
    c.tool_exports[0].runtime_binding = r("foreign");
    assert!(EnvironmentDefinition::new(d.id().clone(), c).is_err());
    let mut c = closure(&d);
    c.definition = DefinitionId::new("foreign", "author").unwrap();
    assert!(EnvironmentLock::new(LockId::new("lock", "resolver").unwrap(), &d, c).is_err());
    let mut c = closure(&d);
    c.packages.push(c.packages[0].clone());
    assert!(EnvironmentLock::new(LockId::new("lock", "resolver").unwrap(), &d, c).is_err());
    let mut c = closure(&d);
    let mut facts = c.packages[0].facts().clone();
    facts.root = Some(r("unknown"));
    c.packages[0] = SelectedPackage::new(facts).unwrap();
    assert!(EnvironmentLock::new(LockId::new("lock", "resolver").unwrap(), &d, c).is_err());
    let mut c = composition();
    c.roots[0].package = PackageRef::parse("tool@^1").unwrap();
    let partial = EnvironmentDefinition::new(d.id().clone(), c).unwrap();
    assert!(
        EnvironmentLock::new(
            LockId::new("lock", "resolver").unwrap(),
            &partial,
            closure(&partial)
        )
        .is_ok()
    );
    let mut facts = closure(&d).packages[0].facts().clone();
    facts.package = PackageRef::parse("flow:org.example/tool@^1").unwrap();
    assert!(SelectedPackage::new(facts).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-GENERATION",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-INSTANCE-CONTINUITY",
    r = 1
)]
fn generation_and_target_reject_cross_reference_tampering() {
    let d = definition();
    let l = lock(&d);
    let g = generation(&l);
    let mut i = inventory(&l);
    i.lock = LockId::new("foreign", "resolver").unwrap();
    assert!(EnvironmentGeneration::new(g.id().clone(), &l, i).is_err());
    let mut i = inventory(&l);
    i.payloads[0].package_capture = r("foreign");
    assert!(EnvironmentGeneration::new(g.id().clone(), &l, i).is_err());
    let mut i = inventory(&l);
    i.dependency_closure.pop();
    assert!(EnvironmentGeneration::new(g.id().clone(), &l, i).is_err());
    let mut facts = target_facts(&d, &l, TargetMode::WorkspaceProjection, linux());
    facts.generation = Some(g.id().clone());
    assert!(
        TargetBinding::new(
            TargetInstanceId::new("a", "storage").unwrap(),
            &d,
            &l,
            None,
            facts.clone()
        )
        .is_err()
    );
    let a = TargetBinding::new(
        TargetInstanceId::new("a", "storage").unwrap(),
        &d,
        &l,
        Some(&g),
        facts.clone(),
    )
    .unwrap();
    let b = TargetBinding::new(
        TargetInstanceId::new("b", "storage").unwrap(),
        &d,
        &l,
        Some(&g),
        facts.clone(),
    )
    .unwrap();
    assert_ne!(a.instance(), b.instance());
    assert_eq!(a.facts().generation, b.facts().generation);
    facts.lock = LockId::new("foreign", "resolver").unwrap();
    assert!(TargetBinding::new(a.instance().clone(), &d, &l, Some(&g), facts).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#INJECTED-CONSTRUCTION-AUTHORITY",
    r = 1
)]
fn constructors_reject_blank_inputs_and_need_no_ambient_state() {
    assert!(Reference::new(" \n").is_err());
    assert!(DefinitionId::new("id", "").is_err());
    assert!(ExecutionId::new("", "caller").is_err());
    assert!(TargetRoots::new("", "/usr", None, r("owner"), None).is_err());
    assert!(TargetRoots::new("/root", "/usr", Some(" ".into()), r("owner"), None).is_err());
    assert!(
        LocalWorkspaceMapping::new(
            ExecutionId::new("e", "caller").unwrap(),
            TargetInstanceId::new("t", "caller").unwrap(),
            PathBuf::from("relative")
        )
        .is_err()
    );
    let d = definition();
    let l = lock(&d);
    let g = generation(&l);
    assert_eq!(
        l.closure().packages[0].facts().content.as_str(),
        "sha256:abc"
    );
    assert_eq!(
        g.inventory().payloads[0].payload_reference.as_str(),
        "artifact-record"
    );
    let t = target(&d, &l, TargetMode::OfflineLinuxRootfs, linux());
    assert_eq!(t.facts().roots.prefix(), "/usr");
    assert!(t.facts().roots.home().is_none());
    let e = executor(PlatformFacts::new(
        Fact::Unknown,
        Fact::Unknown,
        Fact::Unknown,
        Fact::Unknown,
    ));
    assert!(legacy_host_applicability("target", None, &e).is_err());
}
