//! REDs for portable compiler-pending evidence.

use std::collections::BTreeMap;
use tempfile::TempDir;
use vibe_core::manifest::{ExtensionKey, MechanismKey, SpecFormat};
use vibe_spec::CompilerPendingSet;

use super::test_support::{id, node};
use super::*;
use crate::Workspace;

fn native_plan(extra: bool) -> (TempDir, LoweredOwnerRuntimes) {
    let root = TempDir::new().expect("workspace");
    let extra = if extra {
        "\n[[extension]]\nid = \"extra\"\npoint = \"compile:lane\"\nhandler = { kind = \"native\", crate_dir = \"native/extra\" }\n"
    } else {
        ""
    };
    node(
        root.path(),
        &format!(
            "[project]\ngroup = \"org.demo\"\nname = \"demo\"\nversion = \"0.1.0\"\n\n[[extension]]\nid = \"one\"\npoint = \"compile:source\"\nhandler = {{ kind = \"native\", crate_dir = \"native/one\" }}\n\n[[extension]]\nid = \"two\"\npoint = \"compile:document\"\nhandler = {{ kind = \"native\", crate_dir = \"native/two\" }}\n\n[[extension]]\nid = \"three\"\npoint = \"compile:emitted\"\nhandler = {{ kind = \"native\", crate_dir = \"native/three\" }}\n{extra}"
        ),
    );
    let workspace = Workspace::load(root.path()).expect("workspace");
    let runtimes = lower_owner_runtimes(
        &workspace,
        &ExtensionWorldEpoch::empty(),
        OwnerRuntimeLowering::new(".", BTreeMap::new()),
    )
    .expect("runtimes");
    (root, runtimes)
}

fn pending(runtimes: &LoweredOwnerRuntimes, entries: &[(u32, &str)]) -> CompilerPendingSet {
    let plan = runtimes.node(".").expect("root runtime").transform_plan();
    CompilerPendingSet::from_plan_entries_for_test(
        plan,
        entries
            .iter()
            .map(|(order, key)| (*order, ExtensionKey::authored(*key)))
            .collect(),
    )
    .expect("pending set")
}

fn platform(value: &str) -> PendingPlatformKey {
    PendingPlatformKey::new(value).expect("platform")
}

fn fact(reference: &vibe_spec::CompilerPendingRef, seed: u8) -> PendingBuildFact {
    fact_values(
        reference,
        "linux-x86_64",
        seed,
        seed.wrapping_add(1),
        seed.wrapping_add(2),
    )
}

fn fact_values(
    reference: &vibe_spec::CompilerPendingRef,
    platform_key: &str,
    source: u8,
    config: u8,
    provider: u8,
) -> PendingBuildFact {
    PendingBuildFact::from_pending(
        reference,
        platform(platform_key),
        PendingSourceWitness::new([source; 32]),
        PendingHandlerConfigWitness::new([config; 32]),
        "build:cargo".parse::<MechanismKey>().expect("route"),
        PendingBuildProviderDigest::new([provider; 32]),
    )
    .expect("fact")
}

fn evidence(
    pending: &CompilerPendingSet,
    owner: OwnerRuntimeId,
    format: SpecFormat,
    seeds: &[u8],
) -> PendingArtifactEvidence {
    let facts = pending
        .iter()
        .zip(seeds)
        .map(|(reference, seed)| fact(reference, *seed))
        .collect();
    build_pending_artifact_evidence(
        pending,
        owner,
        PendingArtifactTarget::BootStatic,
        format,
        facts,
    )
    .expect("evidence")
    .expect("nonempty")
}

fn assert_fault(error: PendingEvidenceError, expected: &str) {
    let message = error.to_string();
    assert!(message.starts_with(expected), "unexpected fault: {message}");
    assert!(message.contains("(violates spec://"));
    assert!(message.contains("; fix: "));
}

#[test]
fn node_and_unit_goldens_use_dense_order_and_shared_comment_codec() {
    let (_root, runtimes) = native_plan(false);
    let awkward = "__host__/demo#dash--percent%é space";
    let set = pending(&runtimes, &[(0, awkward), (2, "org.demo/demo#three")]);
    let mut facts = set
        .iter()
        .enumerate()
        .map(|(index, reference)| fact(reference, (index + 1) as u8))
        .collect::<Vec<_>>();
    facts.reverse();
    let node = build_pending_artifact_evidence(
        &set,
        OwnerRuntimeId::Node {
            rel: "members/alpha".into(),
        },
        PendingArtifactTarget::BootStatic,
        SpecFormat::Xml,
        facts,
    )
    .expect("node evidence")
    .expect("nonempty");
    assert_eq!(
        node.fingerprint().sha256(),
        "sha256:ddcaf22a3812f67ccaa08f3ea0b75310f5acfaf7e61d7582268a688d05def585"
    );
    assert_eq!(
        node.header_payload(),
        format!(
            "vibe:transforms-pending {} 0={} 2={}",
            node.fingerprint().sha256(),
            vibe_specdoc::encode_generated_xml_comment(awkward),
            vibe_specdoc::encode_generated_xml_comment("org.demo/demo#three"),
        )
    );
    assert_eq!(
        node.header_payload(),
        vibe_spec::compiler_pending_header_payload(&set, node.fingerprint().as_bytes())
            .expect("the workspace and finalizer share one pending-header authority")
    );

    let unit = evidence(
        &set,
        OwnerRuntimeId::Unit {
            provider: id("org.pkg", "unit"),
        },
        SpecFormat::Markdown,
        &[1, 2],
    );
    assert_eq!(
        unit.fingerprint().sha256(),
        "sha256:759d76464846752277861fb5fa7e87c66793294992153ec605aa6dd4bdddabda"
    );
    assert_ne!(unit.fingerprint(), node.fingerprint());

    let (_other_checkout, other_runtimes) = native_plan(false);
    let other_set = pending(&other_runtimes, &[(0, awkward), (2, "org.demo/demo#three")]);
    assert_eq!(
        evidence(
            &other_set,
            OwnerRuntimeId::Unit {
                provider: id("org.pkg", "unit"),
            },
            SpecFormat::Markdown,
            &[1, 2],
        ),
        unit,
        "absolute checkout roots are outside the evidence type and digest"
    );
}

#[test]
fn every_representable_field_moves_the_fingerprint() {
    let (_root, runtimes) = native_plan(false);
    let base_set = pending(
        &runtimes,
        &[(0, "org.demo/demo#one"), (2, "org.demo/demo#three")],
    );
    let baseline = evidence(
        &base_set,
        OwnerRuntimeId::Node { rel: ".".into() },
        SpecFormat::Mixed,
        &[7, 8],
    );
    let changed_owner = evidence(
        &base_set,
        OwnerRuntimeId::Node {
            rel: "member".into(),
        },
        SpecFormat::Mixed,
        &[7, 8],
    );
    let changed_kind = evidence(
        &base_set,
        OwnerRuntimeId::Unit {
            provider: id("org.demo", "unit"),
        },
        SpecFormat::Mixed,
        &[7, 8],
    );
    let changed_group = evidence(
        &base_set,
        OwnerRuntimeId::Unit {
            provider: id("org.changed", "unit"),
        },
        SpecFormat::Mixed,
        &[7, 8],
    );
    let changed_name = evidence(
        &base_set,
        OwnerRuntimeId::Unit {
            provider: id("org.demo", "other"),
        },
        SpecFormat::Mixed,
        &[7, 8],
    );
    let changed_format = evidence(
        &base_set,
        OwnerRuntimeId::Node { rel: ".".into() },
        SpecFormat::Xml,
        &[7, 8],
    );
    let changed_witnesses = evidence(
        &base_set,
        OwnerRuntimeId::Node { rel: ".".into() },
        SpecFormat::Mixed,
        &[9, 8],
    );
    let refs = base_set.iter().collect::<Vec<_>>();
    let evidence_with_first = |first| {
        build_pending_artifact_evidence(
            &base_set,
            OwnerRuntimeId::Node { rel: ".".into() },
            PendingArtifactTarget::BootStatic,
            SpecFormat::Mixed,
            vec![first, fact(refs[1], 8)],
        )
        .expect("field evidence")
        .expect("nonempty")
    };
    let changed_source = evidence_with_first(fact_values(refs[0], "linux-x86_64", 9, 8, 9));
    let changed_config = evidence_with_first(fact_values(refs[0], "linux-x86_64", 7, 10, 9));
    let changed_provider = evidence_with_first(fact_values(refs[0], "linux-x86_64", 7, 8, 11));
    let changed_platform = evidence_with_first(fact_values(refs[0], "windows-x86_64", 7, 8, 9));
    let changed_key = pending(
        &runtimes,
        &[(0, "org.demo/demo#changed"), (2, "org.demo/demo#three")],
    );
    let changed_order = pending(
        &runtimes,
        &[(1, "org.demo/demo#one"), (2, "org.demo/demo#three")],
    );
    let changed_count = pending(&runtimes, &[(0, "org.demo/demo#one")]);
    let (_other_root, other_runtime) = native_plan(true);
    let changed_plan = pending(
        &other_runtime,
        &[(0, "org.demo/demo#one"), (2, "org.demo/demo#three")],
    );

    for changed in [
        changed_owner,
        changed_kind,
        changed_group,
        changed_name,
        changed_format,
        changed_witnesses,
        changed_source,
        changed_config,
        changed_provider,
        changed_platform,
        evidence(
            &changed_key,
            OwnerRuntimeId::Node { rel: ".".into() },
            SpecFormat::Mixed,
            &[7, 8],
        ),
        evidence(
            &changed_order,
            OwnerRuntimeId::Node { rel: ".".into() },
            SpecFormat::Mixed,
            &[7, 8],
        ),
        evidence(
            &changed_count,
            OwnerRuntimeId::Node { rel: ".".into() },
            SpecFormat::Mixed,
            &[7],
        ),
        evidence(
            &changed_plan,
            OwnerRuntimeId::Node { rel: ".".into() },
            SpecFormat::Mixed,
            &[7, 8],
        ),
    ] {
        assert_ne!(changed.fingerprint(), baseline.fingerprint());
    }
}

#[test]
fn exact_admission_refuses_empty_extra_missing_duplicate_conflict_and_identity_drift() {
    let (_root, runtimes) = native_plan(false);
    let empty = pending(&runtimes, &[]);
    assert!(
        build_pending_artifact_evidence(
            &empty,
            OwnerRuntimeId::Node { rel: ".".into() },
            PendingArtifactTarget::BootStatic,
            SpecFormat::Mixed,
            Vec::new(),
        )
        .expect("empty")
        .is_none()
    );
    assert_fault(
        build_pending_artifact_evidence(
            &empty,
            OwnerRuntimeId::Node {
                rel: "../outside".into(),
            },
            PendingArtifactTarget::BootStatic,
            SpecFormat::Mixed,
            Vec::new(),
        )
        .expect_err("invalid empty owner"),
        "pending node owner is not one bounded portable relative path",
    );
    let nonempty = pending(&runtimes, &[(0, "org.demo/demo#one")]);
    assert!(
        build_pending_artifact_evidence(
            &empty,
            OwnerRuntimeId::Node { rel: ".".into() },
            PendingArtifactTarget::BootStatic,
            SpecFormat::Mixed,
            vec![fact(nonempty.iter().next().expect("ref"), 1)],
        )
        .is_err()
    );

    let set = pending(
        &runtimes,
        &[(0, "org.demo/demo#one"), (2, "org.demo/demo#three")],
    );
    let refs = set.iter().collect::<Vec<_>>();
    let build = |facts| {
        build_pending_artifact_evidence(
            &set,
            OwnerRuntimeId::Node { rel: ".".into() },
            PendingArtifactTarget::BootStatic,
            SpecFormat::Mixed,
            facts,
        )
    };
    assert_fault(
        build(vec![fact(refs[0], 1)]).expect_err("missing"),
        "pending build fact is missing",
    );
    let extra = pending(&runtimes, &[(1, "org.demo/demo#two")]);
    assert_fault(
        build(vec![
            fact(refs[0], 1),
            fact(refs[1], 2),
            fact(extra.iter().next().expect("extra"), 3),
        ])
        .expect_err("extra"),
        "pending build facts contain an extra reference",
    );
    assert_fault(
        build(vec![fact(refs[0], 1), fact(refs[0], 1), fact(refs[1], 2)]).expect_err("duplicate"),
        "pending build facts contain one duplicate reference",
    );
    for conflict in [
        fact_values(refs[0], "windows-x86_64", 1, 2, 3),
        fact_values(refs[0], "linux-x86_64", 9, 2, 3),
        fact_values(refs[0], "linux-x86_64", 1, 9, 3),
        fact_values(refs[0], "linux-x86_64", 1, 2, 9),
    ] {
        assert_fault(
            build(vec![fact(refs[0], 1), conflict, fact(refs[1], 2)])
                .expect_err("same-ref semantic conflict"),
            "pending build facts conflict at one dense order",
        );
    }

    let key_conflict = pending(
        &runtimes,
        &[(0, "org.demo/demo#other"), (2, "org.demo/demo#three")],
    );
    assert_fault(
        build(vec![
            fact(refs[0], 1),
            fact(key_conflict.iter().next().expect("conflict"), 1),
            fact(refs[1], 2),
        ])
        .expect_err("conflicting facts"),
        "pending build facts conflict at one dense order",
    );
    assert_fault(
        build(vec![
            fact(key_conflict.iter().next().expect("conflict"), 1),
            fact(refs[1], 2),
        ])
        .expect_err("key"),
        "pending build fact carries a different qualified key",
    );
    let order_conflict = pending(&runtimes, &[(1, "org.demo/demo#one")]);
    assert_fault(
        build(vec![
            fact(order_conflict.iter().next().expect("order"), 1),
            fact(refs[1], 2),
        ])
        .expect_err("order"),
        "pending build fact carries a different dense order",
    );
    let (_other_root, other_runtime) = native_plan(true);
    let plan_conflict = pending(&other_runtime, &[(0, "org.demo/demo#one")]);
    assert_fault(
        build(vec![
            fact(plan_conflict.iter().next().expect("plan"), 1),
            fact(refs[1], 2),
        ])
        .expect_err("plan"),
        "pending build fact carries a different transform plan",
    );

    for valid in ["windows-x86_64", "linux-x86_64", "macos-aarch64"] {
        assert_eq!(platform(valid).as_str(), valid);
    }
    for invalid in ["linux-arm64", "linux-x86-64", "../linux", "windows-x86_65"] {
        assert_fault(
            PendingPlatformKey::new(invalid).expect_err("invalid platform"),
            "pending platform key is not one supported closed platform value",
        );
    }
    assert_fault(
        PendingBuildFact::from_pending(
            refs[0],
            platform("linux-x86_64"),
            PendingSourceWitness::new([1; 32]),
            PendingHandlerConfigWitness::new([2; 32]),
            "package:zip".parse().expect("other route"),
            PendingBuildProviderDigest::new([3; 32]),
        )
        .expect_err("invalid route"),
        "pending build route is not the exact `build:cargo` mechanism key",
    );
    for rel in [
        "/absolute",
        "C:/windows",
        "members\\alpha",
        "a/../b",
        "a//b",
    ] {
        assert!(
            build_pending_artifact_evidence(
                &set,
                OwnerRuntimeId::Node { rel: rel.into() },
                PendingArtifactTarget::BootStatic,
                SpecFormat::Mixed,
                vec![fact(refs[0], 1), fact(refs[1], 2)],
            )
            .is_err(),
            "invalid owner `{rel}` refuses"
        );
    }
}

#[test]
fn debug_and_source_surface_leak_no_semantic_or_environment_inputs() {
    let source = PendingSourceWitness::new([17; 32]);
    let debug = format!("{source:?}");
    assert!(debug.contains(&"11".repeat(32)));
    assert!(!debug.contains("[17"));

    let production = include_str!("pending.rs");
    for forbidden in [
        "std::fs",
        "std::path",
        "std::process",
        "vibe_lifecycle",
        "NativeArtifact",
        "journal",
        "publication",
        "replay",
        "parse_pending",
    ] {
        assert!(!production.contains(forbidden), "forbidden `{forbidden}`");
    }
    assert!(production.contains("PendingArtifactTarget::BootStatic => 0"));
    assert!(production.contains("compiler_pending_header_payload"));
    assert!(!production.contains("encode_generated_xml_comment"));
    assert!(!production.contains("vibe:transforms-pending"));

    let seam = include_str!("../../../vibe-spec/src/compiler/transform/native_policy.rs");
    assert!(seam.contains("#[cfg(any(test, feature = \"test-support\"))]"));
    assert!(seam.contains("from_plan_entries_for_test"));
}
