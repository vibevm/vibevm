//! The package executor's laws: routing, dependency order and the
//! consumed-artifact resolution §6.0.2 makes law.
//!
//! The first two tests are what make §3.1's routing real for the package
//! role. A host that routes `package:static-skill` to a plugin, or a
//! target that pins one, must get the plugin's refusal — the
//! out-of-process transport is a later atom — and must NOT get a static
//! skill instead. Each asserts the project tree is untouched afterwards,
//! which is the observable difference between "the resolver chose" and "a
//! builtin branch chose".
//!
//! The input family is the other half. Every one of §6.0.2's four
//! refusals — unrecorded, unusable record, vanished file, stale digest —
//! is pinned here, because the whole point of reading the record is that
//! nothing else is read: a package that fell back to a guessed
//! `target/…` path would pass a test that only checked the happy case.

use std::path::PathBuf;

use specmark::verifies;
use vibe_core::manifest::{ArtifactInput, MechanismRoutes};
use vibe_wire::behaviour::artifact_record::validate;
use vibe_wire::generated::artifact_record::{
    ArtifactKind as RecordKind, ArtifactRecord, ArtifactShape, ContentDigest, DigestAlgorithm,
    FreshnessFingerprints, ProducerIdentity, ProviderIdentity, RelativeIdentity, RelativeRoot,
    VerificationState, VerificationStatus,
};

use super::support::*;
use super::*;
use crate::mechanism::MechanismError;

/// Nothing was packaged and nothing was recorded.
fn nothing_happened(root: &Path) {
    assert!(!root.join("target").exists(), "no package output root");
    assert!(!root.join(".vibe").exists(), "no artifact record");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_host_route_displaces_the_builtin_and_the_builtin_does_not_run() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    let world = world_with_plugin();
    let registry = registry(&world);
    let mut routes = MechanismRoutes::default();
    routes.insert(key("package:static-skill"), pin(PLUGIN_PIN));
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let refusal = execute_package_targets(&execution(root.path(), &targets, &registry, &routes))
        .expect_err("the routed provider needs a transport that has not landed");

    match &refusal {
        PackageError::TransportNotLanded { key, pin, kind } => {
            assert_eq!(key, "package:static-skill");
            assert_eq!(pin, PLUGIN_PIN);
            assert_eq!(kind, "native");
        }
        other => panic!("expected a transport refusal, got {other}"),
    }
    assert!(
        refusal
            .to_string()
            .contains("was NOT packaged by the builtin")
    );
    nothing_happened(root.path());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_exact_target_pin_routes_away_from_the_builtin_too() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    let world = world_with_plugin();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let mut declared = skill_target("demo", "skills/demo", &[]);
    declared.provider = Some(pin(PLUGIN_PIN));
    let targets = vec![declared];

    let refusal = execute_package_targets(&execution(root.path(), &targets, &registry, &routes))
        .expect_err("a pinned plugin needs the same transport");

    assert!(matches!(refusal, PackageError::TransportNotLanded { .. }));
    nothing_happened(root.path());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_installed_plugin_nobody_selects_leaves_the_builtin_selected() {
    // The same world, with no route and no pin: §3.1 step 3 answers, and
    // the executor reaches the builtin adapter — proven by the adapter's
    // own refusal over a project that holds no skill source, which only
    // the builtin can produce.
    let root = temp();
    let world = world_with_plugin();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();
    let targets = vec![skill_target("demo", "skills/demo", &[])];

    let refusal = execute_package_targets(&execution(root.path(), &targets, &registry, &routes))
        .expect_err("an empty directory holds no skill source");

    match refusal {
        PackageError::Provider(MechanismError::SourceMissing { path, .. }) => {
            assert_eq!(path, "skills/demo/SKILL.md");
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
    let mut declared = skill_target("demo", "skills/demo", &[]);
    // A logical key the engine ships no default for: `package:windows-zip`
    // used to be one and stopped being one when §7.0.8 landed its row, so
    // the fixture names a capability nothing implements instead.
    declared.mechanism = key("package:tarball");
    let targets = vec![declared];

    let refusal = execute_package_targets(&execution(root.path(), &targets, &registry, &routes))
        .expect_err("this world ships no `package:tarball` provider");

    assert!(matches!(refusal, PackageError::Resolution(_)));
    assert!(refusal.to_string().contains("no shipped builtin default"));
    nothing_happened(root.path());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_project_with_no_package_targets_packages_nothing() {
    let root = temp();
    let world = world_with_plugin();
    let registry = registry(&world);
    let routes = MechanismRoutes::default();

    let outcomes = match execute_package_targets(&execution(root.path(), &[], &registry, &routes)) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("an empty graph executes: {error}"),
    };

    assert!(outcomes.is_empty());
    nothing_happened(root.path());
}

/// A package target consuming another package target's output.
fn consumer(id: &str, consumed: &str) -> ArtifactPackageTarget {
    let mut target = skill_target(id, "skills/demo", &[]);
    target.inputs = Some(vec![ArtifactInput::Artifact {
        artifact: consumed.to_owned(),
    }]);
    target
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn dependency_order_runs_producers_first_whatever_the_declaration_order() {
    let targets = vec![
        consumer("late", "early.md"),
        skill_target("early", "s", &[]),
    ];

    let sequence = match order(&targets) {
        Ok(sequence) => sequence,
        Err(error) => panic!("the graph orders: {error}"),
    };

    assert_eq!(sequence, vec![1, 0]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_consumed_build_output_constrains_the_package_order_not_at_all() {
    // The phase-forward law: a build output has no producer in THIS set,
    // and that is not an error — the build phase already ran, and the
    // input resolver reads its record.
    let targets = vec![consumer("late", "vibe-helper.exe")];

    let sequence = match order(&targets) {
        Ok(sequence) => sequence,
        Err(error) => panic!("a build input is not a package edge: {error}"),
    };

    assert_eq!(sequence, vec![0]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_cycle_refuses_and_names_it() {
    let targets = vec![consumer("a", "b.md"), consumer("b", "a.md")];

    let refusal = order(&targets).expect_err("a cycle has no dependency order");

    match &refusal {
        PackageError::Cycle { cycle } => assert_eq!(cycle, "a -> b -> a"),
        other => panic!("expected a cycle refusal, got {other}"),
    }
}

/// One build-shaped artifact record, as the build executor would have
/// written it — the ONLY door a consumed artifact comes through.
fn record_for(root: &Path, id: &str, relative: &str, digest: &str) {
    let absolute = crate::mechanism::contain::forward_slashed(&root.join(relative));
    let record = ArtifactRecord {
        schema: 1,
        id: id.to_owned(),
        kind: RecordKind::Executable,
        shape: ArtifactShape::File,
        path_absolute: absolute,
        path_relative: RelativeIdentity {
            root: RelativeRoot::Project,
            path: relative.to_owned(),
        },
        digest: ContentDigest {
            algorithm: DigestAlgorithm::Sha256,
            value: digest.to_owned(),
        },
        producer: ProducerIdentity {
            target: "helper".to_owned(),
            mechanism: "build:cargo".to_owned(),
            provider: ProviderIdentity {
                key: "org.vibevm/vibe#cargo".to_owned(),
                version: None,
                content_hash: None,
            },
        },
        freshness: FreshnessFingerprints {
            inputs: None,
            config: None,
            toolchain: None,
        },
        created_at: match "2026-08-30T00:00:00Z".parse() {
            Ok(stamp) => stamp,
            Err(error) => panic!("the fixture clock parses: {error}"),
        },
        verification: VerificationState {
            status: VerificationStatus::Verified,
            evidence: None,
        },
        media_type: None,
        platform: None,
    };
    if let Err(error) = validate(&record) {
        panic!("the fixture record is valid: {error}");
    }
    if let Err(error) = crate::mechanism::record::write_record(root, &record) {
        panic!("the fixture record publishes: {error}");
    }
}

fn digest_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// A plugin target that consumes one artifact and places it.
fn placing(id: &str, artifact: &str, destination: &str) -> ArtifactPackageTarget {
    plugin_target(
        id,
        "plugin",
        vec![ArtifactInput::Artifact {
            artifact: artifact.to_owned(),
        }],
        &[(artifact, destination)],
    )
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_consumed_artifact_with_no_record_refuses_by_name() {
    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "build/helper.bin", "payload");
    let targets = vec![placing(
        "demo",
        "helper.bin",
        "com.example.tools/helper.bin",
    )];

    let refusal = run_default(root.path(), &targets).expect_err("nothing recorded `helper.bin`");

    match &refusal {
        PackageError::InputNotRecorded { target, input } => {
            assert_eq!(target, "demo");
            assert_eq!(input, "helper.bin");
        }
        other => panic!("expected an unrecorded-input refusal, got {other}"),
    }
    // The file really is there at a plausible path; only the RECORD is
    // missing. A resolver that guessed would have found it.
    assert!(root.path().join("build/helper.bin").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_recorded_artifact_that_vanished_refuses_by_name() {
    let root = temp();
    write_demo_plugin(root.path());
    record_for(
        root.path(),
        "helper.bin",
        "build/helper.bin",
        &digest_of(b"payload"),
    );
    let targets = vec![placing(
        "demo",
        "helper.bin",
        "com.example.tools/helper.bin",
    )];

    let refusal = run_default(root.path(), &targets).expect_err("the recorded file is gone");

    match &refusal {
        PackageError::InputArtifactMissing { input, path, .. } => {
            assert_eq!(input, "helper.bin");
            assert_eq!(path, "build/helper.bin");
        }
        other => panic!("expected a missing-artifact refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_recorded_artifact_whose_bytes_changed_refuses_as_stale() {
    let root = temp();
    write_demo_plugin(root.path());
    record_for(
        root.path(),
        "helper.bin",
        "build/helper.bin",
        &digest_of(b"payload"),
    );
    // The artifact changed behind its own record — the exact state a
    // guessed path would have packaged silently.
    write(root.path(), "build/helper.bin", "something else");
    let targets = vec![placing(
        "demo",
        "helper.bin",
        "com.example.tools/helper.bin",
    )];

    let refusal = run_default(root.path(), &targets).expect_err("the digest no longer matches");

    match &refusal {
        PackageError::InputStale {
            input,
            recorded,
            found,
            ..
        } => {
            assert_eq!(input, "helper.bin");
            assert_eq!(recorded, &digest_of(b"payload"));
            assert_eq!(found, &digest_of(b"something else"));
        }
        other => panic!("expected a stale-input refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_declared_source_path_that_escapes_the_project_refuses_before_any_read() {
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    let mut target = skill_target("demo", "skills/demo", &[]);
    target.inputs = Some(vec![ArtifactInput::Path {
        path: PathBuf::from("../outside.md"),
    }]);

    let refusal = run_default(root.path(), &[target]).expect_err("a traversal never resolves");

    match &refusal {
        PackageError::InputPathUnsafe { input, .. } => assert_eq!(input, "../outside.md"),
        other => panic!("expected a traversal refusal, got {other}"),
    }
}
