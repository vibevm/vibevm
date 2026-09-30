//! Test-built typed values: corpus `valid` files exercise JTD shape only.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-EXACT-LOCK-IDENTITY");

use sha2::{Digest, Sha256};
use vibe_core::environment::{
    CapturedFile, DefinitionComposition, DefinitionId, DependencyRole, EnvironmentDefinition,
    EnvironmentLock, ExternalBackendClosure, ExternalCapture, Fact, LockClosure, LockId,
    LockTargetRole, PackageCapture, PlatformConstraints, PlatformFacts, Reference, Requirement,
    SelectedLockTarget, SelectedPackage, SelectedPackageFacts, ToolchainPlatforms,
};
use vibe_core::manifest::TargetOs;
use vibe_core::{ContentHash, PackageRef};
use vibe_wire::generated::environment::e1::exact_lock::ExactLock;

fn reference(value: &str) -> Reference {
    Reference::new(value).unwrap()
}
fn tree_hex(path: &str, bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(path.as_bytes());
    hash.update([0]);
    hash.update(bytes);
    hash.update([0]);
    format!("{:x}", hash.finalize())
}

fn with_model(
    authored_any: bool,
    run: impl FnOnce(
        &EnvironmentDefinition,
        &EnvironmentLock,
        &SelectedLockTarget<'_>,
        &[PackageCapture<'_>],
        &[ExternalCapture<'_>],
        &ExactLock,
    ),
) {
    let constraints = PlatformConstraints::new(
        if authored_any {
            Requirement::Any
        } else {
            Requirement::Exact(TargetOs::Linux)
        },
        if authored_any {
            Requirement::Any
        } else {
            Requirement::Exact(reference("x86_64"))
        },
        Requirement::Exact(reference("gnu")),
        Requirement::Exact(reference("glibc")),
    );
    let definition = EnvironmentDefinition::new(
        DefinitionId::new("definition-a", "author-a").unwrap(),
        DefinitionComposition {
            roots: vec![],
            features: vec![],
            choices: vec![],
            target_constraints: constraints.clone(),
            runtime_bindings: vec![],
            tool_exports: vec![],
            service_exports: vec![],
        },
    )
    .unwrap();
    let cap_a = reference("capture-a");
    let cap_b = reference("capture-b");
    let owner = reference("source-owner-a");
    let config_owner = reference("config-owner-a");
    let config_a = reference("config-a");
    let config_b = reference("config-b");
    let backend = reference("cargo");
    let closure = reference("closure-cargo");
    let external_config = reference("config-cargo");
    let external_owner = reference("backend-owner-a");
    let hex_a = tree_hex("a", b"A");
    let hex_b = tree_hex("b", b"B");
    let selected = [
        (&cap_a, &config_a, "app-a", &hex_a),
        (&cap_b, &config_b, "app-b", &hex_b),
    ];
    let packages = selected
        .iter()
        .map(|(capture, config, name, hex)| {
            SelectedPackage::new(SelectedPackageFacts {
                root: None,
                package: PackageRef::parse(&format!("flow:org.example/{name}@=1.1.0")).unwrap(),
                version: "1.1.0".parse().unwrap(),
                content: ContentHash::parse(&format!("sha256-tree/1:{hex}")).unwrap(),
                capture: (*capture).clone(),
                configuration: (*config).clone(),
                role: DependencyRole::BuildAndRuntime,
            })
            .unwrap()
        })
        .collect::<Vec<_>>();
    let lock = EnvironmentLock::new(
        LockId::new("lock-a", "resolver-a").unwrap(),
        &definition,
        LockClosure {
            definition: definition.id().clone(),
            packages,
            external_backends: vec![ExternalBackendClosure {
                backend: backend.clone(),
                closure: closure.clone(),
                configuration: external_config.clone(),
                role: DependencyRole::Build,
            }],
            target_constraints: constraints,
        },
    )
    .unwrap();
    let captures = [
        PackageCapture {
            capture: &cap_a,
            owner: &owner,
            files: vec![CapturedFile {
                relative_path: "a".into(),
                bytes: b"A",
            }],
            configuration_owner: &config_owner,
            configuration_bytes: Some(b"config"),
        },
        PackageCapture {
            capture: &cap_b,
            owner: &owner,
            files: vec![CapturedFile {
                relative_path: "b".into(),
                bytes: b"B",
            }],
            configuration_owner: &config_owner,
            configuration_bytes: Some(b"config"),
        },
    ];
    let external = [ExternalCapture {
        backend: &backend,
        closure: &closure,
        owner: &external_owner,
        bytes: b"backend-closure",
        configuration_owner: &config_owner,
        configuration_bytes: Some(b"external-config"),
    }];
    let platform = PlatformFacts::new(
        Fact::Known(TargetOs::Linux),
        Fact::Known(reference("x86_64")),
        Fact::Known(reference("gnu")),
        Fact::Known(reference("glibc")),
    );
    let toolchain = ToolchainPlatforms::new(platform.clone(), platform, None);
    let selected_target =
        SelectedLockTarget::from_toolchain(&toolchain, LockTargetRole::ProducedToolRuntime)
            .unwrap();
    let digest = vibe_core::environment::exact_lock_digest(
        &definition,
        &lock,
        &selected_target,
        &captures,
        &external,
    )
    .unwrap();
    let rows = selected
        .iter()
        .map(|(capture, config, name, hex)| {
            let config_digest = format!("{:x}", Sha256::digest(b"config"));
            serde_json::json!({
                "selected_root":null,"package_kind":"flow","package_group":"org.example",
                "package_name":name,"exact_version":"1.1.0","content_hash_recipe":"sha256-tree/1",
                "content_hash_value":hex,"role":"build_and_runtime",
                "source_capture_ref":capture.as_str(),"capture_owner":owner.as_str(),
                "capture_digest_sha256":hex,"configuration_ref":config.as_str(),
                "configuration_owner":config_owner.as_str(),"configuration_digest_sha256":config_digest
            })
        })
        .collect::<Vec<_>>();
    let external_digest = format!("{:x}", Sha256::digest(b"backend-closure"));
    let external_config_digest = format!("{:x}", Sha256::digest(b"external-config"));
    let wire: ExactLock = serde_json::from_value(serde_json::json!({
        "schema":1,"recipe_epoch":1,"lock_owner":"resolver-a","definition_ref":"definition-a",
        "target":{"role":"produced_tool_runtime","os":"linux","architecture":"x86_64","abi":"gnu","libc":"glibc"},
        "selected_captures":rows,
        "dependency_closure":[{"role":"build","name":"cargo","closure_ref":"closure-cargo",
            "configuration_ref":"config-cargo","configuration_owner":config_owner.as_str(),
            "configuration_digest_sha256":external_config_digest,
            "closure_owner":external_owner.as_str(),"closure_digest_sha256":external_digest}],
        "lock_digest_sha256":digest.as_str()
    })).unwrap();
    run(
        &definition,
        &lock,
        &selected_target,
        &captures,
        &external,
        &wire,
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-EXACT-LOCK-IDENTITY",
    r = 2
)]
fn exact_lock_accepts_complete_reordered_equivalent_projection() {
    with_model(
        false,
        |definition, lock, selected_target, captures, external, wire| {
            wire.validate_environment_exact_lock(
                definition,
                lock,
                selected_target,
                captures,
                external,
            )
            .unwrap();
            let mut reordered = wire.clone();
            reordered.selected_captures.reverse();
            assert_eq!(
                reordered
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .unwrap()
                    .as_str(),
                wire.lock_digest_sha256
            );
        },
    );
    with_model(
        true,
        |definition, lock, selected_target, captures, external, wire| {
            wire.validate_environment_exact_lock(
                definition,
                lock,
                selected_target,
                captures,
                external,
            )
            .unwrap();
        },
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-EXACT-LOCK-IDENTITY",
    r = 2
)]
fn exact_lock_refuses_stale_or_mismatched_claims() {
    with_model(
        false,
        |definition, lock, selected_target, captures, external, wire| {
            let mut changed = wire.clone();
            changed.target.abi = Some("musl".into());
            assert!(
                changed
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .is_err()
            );
            let mut changed = wire.clone();
            changed.target.role =
                vibe_wire::generated::environment::e1::exact_lock::TargetRole::Executor;
            assert!(
                changed
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .is_err()
            );
            let mut changed = wire.clone();
            changed.lock_digest_sha256 = "0".repeat(64);
            assert!(
                changed
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .is_err()
            );
            let mut changed = wire.clone();
            changed.selected_captures[0].package_name = "other".into();
            assert!(
                changed
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .is_err()
            );
            let mut changed = wire.clone();
            changed.selected_captures[0].role =
                vibe_wire::generated::environment::e1::exact_lock::DependencyRole::Runtime;
            assert!(
                changed
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .is_err()
            );
            let mut changed = wire.clone();
            changed.recipe_epoch = 2;
            assert!(
                changed
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .is_err()
            );
            let changed_bytes = [
                PackageCapture {
                    capture: captures[0].capture,
                    owner: captures[0].owner,
                    files: vec![CapturedFile {
                        relative_path: "a".into(),
                        bytes: b"changed",
                    }],
                    configuration_owner: captures[0].configuration_owner,
                    configuration_bytes: captures[0].configuration_bytes,
                },
                PackageCapture {
                    capture: captures[1].capture,
                    owner: captures[1].owner,
                    files: vec![CapturedFile {
                        relative_path: "b".into(),
                        bytes: b"B",
                    }],
                    configuration_owner: captures[1].configuration_owner,
                    configuration_bytes: captures[1].configuration_bytes,
                },
            ];
            assert!(
                wire.validate_environment_exact_lock(
                    definition,
                    lock,
                    selected_target,
                    &changed_bytes,
                    external
                )
                .is_err()
            );
            let mut changed_config = [
                PackageCapture {
                    capture: captures[0].capture,
                    owner: captures[0].owner,
                    files: vec![CapturedFile {
                        relative_path: "a".into(),
                        bytes: b"A",
                    }],
                    configuration_owner: captures[0].configuration_owner,
                    configuration_bytes: Some(b"changed-config"),
                },
                PackageCapture {
                    capture: captures[1].capture,
                    owner: captures[1].owner,
                    files: vec![CapturedFile {
                        relative_path: "b".into(),
                        bytes: b"B",
                    }],
                    configuration_owner: captures[1].configuration_owner,
                    configuration_bytes: captures[1].configuration_bytes,
                },
            ];
            assert!(
                wire.validate_environment_exact_lock(
                    definition,
                    lock,
                    selected_target,
                    &changed_config,
                    external
                )
                .is_err()
            );
            changed_config[0].configuration_bytes = None;
            assert!(
                wire.validate_environment_exact_lock(
                    definition,
                    lock,
                    selected_target,
                    &changed_config,
                    external
                )
                .is_err()
            );
            let mut changed = wire.clone();
            changed.selected_captures[0].configuration_digest_sha256 = "0".repeat(64);
            assert!(
                changed
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .is_err()
            );
            let mut changed = wire.clone();
            changed.dependency_closure[0].configuration_digest_sha256 = "0".repeat(64);
            assert!(
                changed
                    .validate_environment_exact_lock(
                        definition,
                        lock,
                        selected_target,
                        captures,
                        external
                    )
                    .is_err()
            );
        },
    );
}
