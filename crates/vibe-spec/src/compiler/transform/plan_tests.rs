//! Identity REDs of the typed transform-plan family (PROP-054
//! `#TRANSFORM-PLAN-IDENTITY`): empty-plan law, assigned order, provider
//! conversion and variants, exact spellings, config semantics and the
//! selector OR-set law. The refusal law and its bounds live in
//! `plan_refusal_tests`; the longhand digest goldens in `plan_digest_tests`.

use specmark::verifies;
use vibe_core::manifest::ExtensionKey;
use vibe_core::{Group, PackageKind, PackageName};
use vibe_extension_registry::{
    CompiledSelector, DependencyProvider, DependencyProviderId, ExtensionProvider, HostIdentity,
};

use super::config::{ConfigFloat, ConfigTable, ConfigValue};
use super::plan::{
    TransformConfig, TransformImplementation, TransformPlan, TransformProvider, TransformSeed,
    TransformStage,
};
use super::plan_test_support::{
    SelectorShape, build_or_panic, compiled_selectors, default_dependency, dependency_provider,
    dependency_seed, empty_config, host_with, ungrouped_host,
};
use super::plan_validate::TransformPlanError;

/// The empty plan owns no entries, no allocation and no digest, and an
/// empty seed vector canonicalizes to exactly it (ABI §7).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_empty_plan_has_no_digest_no_entries_and_no_allocation() {
    let empty = TransformPlan::empty();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.capacity(), 0, "empty() must not allocate");
    assert!(empty.digest().is_none());
    assert!(empty.entries().is_empty());
    let built = TransformPlan::build(Vec::new()).expect("empty input builds");
    assert_eq!(built, empty);
    assert_eq!(built.capacity(), 0);
    assert!(built.digest().is_none());
}

/// Dense order is assigned from the input sequence: swapping entries
/// reassigns 0/1 in the new input order and moves the digest; moving a
/// stage moves it too.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn entry_order_is_assigned_by_build_and_entry_swap_moves_the_digest() {
    let first = dependency_seed("org.demo/tools#a", TransformStage::Source);
    let second = dependency_seed("org.demo/tools#b", TransformStage::Document);
    let plan = build_or_panic(vec![first.clone(), second.clone()]);
    assert_eq!(plan.len(), 2);
    assert_eq!(plan.entries()[0].order(), 0);
    assert_eq!(plan.entries()[1].order(), 1);
    assert_eq!(plan.entries()[0].seed().key().as_str(), "org.demo/tools#a");

    let swapped = build_or_panic(vec![second, first]);
    assert_ne!(plan, swapped);
    assert_ne!(plan.digest(), swapped.digest());
    assert_eq!(swapped.entries()[0].order(), 0);
    assert_eq!(swapped.entries()[1].order(), 1);
    assert_eq!(
        swapped.entries()[0].seed().key().as_str(),
        "org.demo/tools#b"
    );

    // A stage move inside one entry moves identity and digest.
    let staged = build_or_panic(vec![
        dependency_seed("org.demo/tools#a", TransformStage::Lane),
        dependency_seed("org.demo/tools#b", TransformStage::Document),
    ]);
    assert_ne!(plan, staged);
    assert_ne!(plan.digest(), staged.digest());
}

/// Provider roots are filesystem state, not identity: converting two
/// registry providers that differ only in root yields one converted
/// provider and one plan identity.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn differing_provider_roots_cannot_reach_the_converted_plan_identity() {
    let left = default_dependency();
    let DependencyProvider {
        id,
        version,
        kind,
        content_hash,
        ..
    } = match &left {
        ExtensionProvider::Dependency(dependency) => dependency.clone(),
        _ => unreachable!("test provider is a dependency"),
    };
    let right = ExtensionProvider::Dependency(DependencyProvider {
        root: std::path::PathBuf::from("a/completely/other/slot"),
        id,
        version,
        kind,
        content_hash,
    });

    let provider_left = TransformProvider::from(&left);
    let provider_right = TransformProvider::from(&right);
    assert_eq!(provider_left, provider_right);

    let seed = |provider: TransformProvider| {
        TransformSeed::new(
            ExtensionKey::authored("org.demo/tools#a"),
            provider,
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            None,
        )
    };
    let plan_left = build_or_panic(vec![seed(provider_left)]);
    let plan_right = build_or_panic(vec![seed(provider_right)]);
    assert_eq!(plan_left, plan_right);
    assert_eq!(plan_left.digest(), plan_right.digest());
}

fn single_host_plan(provider: &ExtensionProvider) -> TransformPlan {
    build_or_panic(vec![TransformSeed::new(
        ExtensionKey::authored("__host__/demo#x"),
        TransformProvider::from(provider),
        TransformStage::Source,
        TransformImplementation::builtin_candidate("log", 1),
        None,
        None,
    )])
}

fn coordinate_demo() -> DependencyProviderId {
    DependencyProviderId::new(
        Group::parse("org.demo").unwrap(),
        PackageName::parse("tools").unwrap(),
    )
}

/// A coordinate host and a dependency with the same coordinate are two
/// identities; an ungrouped host's raw authored name is what frames (never
/// a percent-coded rendering); a virtual-workspace host is its own identity.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn host_variants_never_counterfeit_dependency_or_each_other() {
    let coordinate_host = host_with(HostIdentity::coordinate(coordinate_demo()), None, None);
    let host_plan = single_host_plan(&coordinate_host);
    let dependency_plan = single_host_plan(&default_dependency());
    // Same coordinate, different discriminant: provider and plan differ.
    assert_ne!(
        TransformProvider::from(&coordinate_host),
        TransformProvider::from(&default_dependency())
    );
    assert_ne!(host_plan, dependency_plan);
    assert_ne!(host_plan.digest(), dependency_plan.digest());

    // The ungrouped raw name frames verbatim: a space-containing name and
    // its percent-coded rendering are two different identities, so a
    // rendered owner coordinate can never stand in for the authored bytes.
    let raw_space = single_host_plan(&ungrouped_host("my app"));
    let rendered = single_host_plan(&ungrouped_host("my%20app"));
    assert_ne!(
        HostIdentity::ungrouped_project("my app").to_string(),
        HostIdentity::ungrouped_project("my%20app").to_string()
    );
    assert_ne!(raw_space, rendered);
    assert_ne!(raw_space.digest(), rendered.digest());

    // A virtual-workspace host is distinct from any ungrouped spelling.
    let virtual_host = single_host_plan(&host_with(HostIdentity::virtual_workspace(), None, None));
    assert_ne!(virtual_host, raw_space);
    assert_ne!(virtual_host.digest(), raw_space.digest());
}

/// Exact version, kind and content hash each bind plan identity, and the
/// two accepted hash recipe spellings stay distinct identities.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn exact_version_kind_and_hash_spellings_each_bind_identity() {
    let digest_of = |provider: &ExtensionProvider| single_host_plan(provider).digest();
    let bare = host_with(HostIdentity::coordinate(coordinate_demo()), None, None);
    let with_kind = host_with(
        HostIdentity::coordinate(coordinate_demo()),
        Some(PackageKind::Feat),
        None,
    );
    let other_kind = host_with(
        HostIdentity::coordinate(coordinate_demo()),
        Some(PackageKind::Lang),
        None,
    );
    let with_hash = host_with(
        HostIdentity::coordinate(coordinate_demo()),
        None,
        Some("sha256:aa"),
    );
    let tree_hash = host_with(
        HostIdentity::coordinate(coordinate_demo()),
        None,
        Some("sha256-tree/1:aa"),
    );
    let other_hex = host_with(
        HostIdentity::coordinate(coordinate_demo()),
        None,
        Some("sha256:ab"),
    );
    let digests = [
        digest_of(&bare),
        digest_of(&with_kind),
        digest_of(&other_kind),
        digest_of(&with_hash),
        digest_of(&tree_hash),
        digest_of(&other_hex),
    ];
    for left in 0..digests.len() {
        for right in (left + 1)..digests.len() {
            assert_ne!(
                digests[left], digests[right],
                "host provider variants {left}/{right} collided"
            );
        }
    }

    // Exact version spelling on a dependency; no SemVer normalization.
    let versioned = |version: &str| {
        single_entry_digest(&dependency_provider(
            "org.demo",
            "tools",
            version,
            PackageKind::Tool,
            "sha256:aa",
        ))
    };
    assert_ne!(versioned("1.2.3"), versioned("1.2.4"));
    assert_ne!(versioned("1.2.3"), versioned("v1.2.3"));
    assert_ne!(versioned("1.2.3"), versioned(" 1.2.3"));
}

fn single_entry_digest(provider: &ExtensionProvider) -> super::plan_digest::PlanDigest {
    build_or_panic(vec![TransformSeed::new(
        ExtensionKey::authored("k"),
        TransformProvider::from(provider),
        TransformStage::Source,
        TransformImplementation::builtin_candidate("log", 1),
        None,
        None,
    )])
    .digest()
    .expect("nonempty plan digests")
}

/// Absent config and authored-empty config are two identities, and every
/// semantic the T1 config digest carries flows through into the plan
/// digest.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn config_absence_differs_from_authored_empty_and_every_config_semantic_binds() {
    let seeded = |config: Option<TransformConfig>| {
        build_or_panic(vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&default_dependency()),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            config,
            None,
        )])
    };
    let absent = seeded(None);
    let authored_empty = seeded(Some(empty_config()));
    assert_ne!(absent, authored_empty);
    assert_ne!(absent.digest(), authored_empty.digest());
    // The Option distinction is structural, not only digested: the
    // authored-empty entry carries a config digest, the absent one none.
    assert!(absent.entries()[0].config_digest().is_none());
    assert!(authored_empty.entries()[0].config_digest().is_some());

    let mut one = ConfigTable::new();
    one.insert("v".to_owned(), ConfigValue::Integer(1));
    let integer_one = seeded(Some(TransformConfig::new(one)));
    let mut two = ConfigTable::new();
    two.insert("v".to_owned(), ConfigValue::Integer(2));
    let integer_two = seeded(Some(TransformConfig::new(two)));
    let mut float_one = ConfigTable::new();
    float_one.insert("v".to_owned(), ConfigValue::Float(ConfigFloat::new(1.0)));
    let float_one = seeded(Some(TransformConfig::new(float_one)));
    for other in [authored_empty.clone(), integer_two.clone(), float_one] {
        assert_ne!(integer_one, other);
        assert_ne!(integer_one.digest(), other.digest());
    }

    // T1 table-order insensitivity flows through: two spellings of one
    // table are one plan.
    let mut nested_a = ConfigTable::new();
    nested_a.insert("x".to_owned(), ConfigValue::Boolean(true));
    let mut nested_b = ConfigTable::new();
    nested_b.insert("x".to_owned(), ConfigValue::Boolean(true));
    let mut left = ConfigTable::new();
    left.insert("alpha".to_owned(), ConfigValue::Table(nested_a));
    left.insert("beta".to_owned(), ConfigValue::Integer(9));
    let mut right = ConfigTable::new();
    right.insert("beta".to_owned(), ConfigValue::Integer(9));
    right.insert("alpha".to_owned(), ConfigValue::Table(nested_b));
    assert_eq!(
        seeded(Some(TransformConfig::new(left))),
        seeded(Some(TransformConfig::new(right)))
    );
}

/// Selector OR-set law at the plan layer: reorder/duplicate members keep
/// one identity, a changed member moves seed, plan and digest, absent and
/// present-empty dimensions differ, and a behaviorally unscoped selector
/// canonicalizes to outer absence at source/document.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn selector_or_set_law_holds_at_the_plan_layer() {
    let selectors: [_; 5] = compiled_selectors(&[
        SelectorShape::Dimensions {
            packages: Some(vec!["org.beta/tool", "org.alpha/*", "org.alpha/*"]),
            paths: Some(vec!["docs/**", "src/*.md"]),
        },
        SelectorShape::Dimensions {
            packages: Some(vec!["org.alpha/*", "org.beta/tool"]),
            paths: Some(vec!["src/*.md", "docs/**"]),
        },
        SelectorShape::Dimensions {
            packages: Some(vec!["org.alpha/*", "org.gamma/kit"]),
            paths: Some(vec!["docs/**", "src/*.md"]),
        },
        SelectorShape::Dimensions {
            packages: Some(Vec::new()),
            paths: None,
        },
        SelectorShape::Absent,
    ])
    .try_into()
    .expect("five selectors collected");
    let [raw, reordered, changed, empty_dimension, unscoped] = selectors;

    let seeded = |selector: Option<CompiledSelector>| {
        TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&default_dependency()),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            selector,
        )
    };

    // Reordered + duplicated members are one seed/plan/digest.
    let seed_raw = seeded(Some(raw.clone()));
    let seed_reordered = seeded(Some(reordered.clone()));
    assert_eq!(seed_raw, seed_reordered);
    let plan_raw = build_or_panic(vec![seed_raw]);
    let plan_reordered = build_or_panic(vec![seed_reordered]);
    assert_eq!(plan_raw, plan_reordered);
    assert_eq!(plan_raw.digest(), plan_reordered.digest());

    // A changed member moves seed, plan and digest.
    let plan_changed = build_or_panic(vec![seeded(Some(changed))]);
    assert_ne!(plan_raw, plan_changed);
    assert_ne!(plan_raw.digest(), plan_changed.digest());

    // Dimension absence differs from present-empty, and the present-empty
    // selector survives canonicalization as outer presence.
    let plan_empty_dimension = build_or_panic(vec![seeded(Some(empty_dimension))]);
    let plan_absent = build_or_panic(vec![seeded(None)]);
    assert_ne!(plan_empty_dimension, plan_absent);
    assert_ne!(plan_empty_dimension.digest(), plan_absent.digest());
    assert!(
        plan_empty_dimension.entries()[0]
            .seed()
            .selector()
            .is_some()
    );

    // A behaviorally unscoped selector canonicalizes to outer absence at
    // source/document — one identity with authored absence.
    let plan_unscoped = build_or_panic(vec![seeded(Some(unscoped.clone()))]);
    assert_eq!(plan_unscoped, plan_absent);
    assert_eq!(plan_unscoped.digest(), plan_absent.digest());
    assert!(plan_unscoped.entries()[0].seed().selector().is_none());

    // The refusal law sees the raw seed, not the canonicalized plan: an
    // unscoped selector still refuses the lane stage.
    assert!(matches!(
        TransformPlan::build(vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&default_dependency()),
            TransformStage::Lane,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            Some(unscoped),
        )]),
        Err(TransformPlanError::SelectorStage { .. })
    ));

    // Absence is legal at every stage, including lane and emitted.
    for stage in [
        TransformStage::Source,
        TransformStage::Document,
        TransformStage::Lane,
        TransformStage::Emitted,
    ] {
        let plan = build_or_panic(vec![dependency_seed("k", stage)]);
        assert!(plan.entries()[0].seed().selector().is_none());
    }
}
