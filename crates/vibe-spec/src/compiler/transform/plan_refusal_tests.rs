//! Refusal-law and bounded-diagnostic REDs of the typed transform-plan
//! family (PROP-054 `#TRANSFORM-PLAN-IDENTITY`): the frozen precedence, the
//! stage law, and the hostile-input bounds.

use specmark::verifies;
use vibe_core::manifest::ExtensionKey;
use vibe_core::{ContentHash, PackageKind};
use vibe_extension_registry::ExtensionProvider;

use super::plan::{
    TransformImplementation, TransformPlan, TransformProvider, TransformSeed, TransformStage,
};
use super::plan_test_support::{
    SelectorShape, build_or_panic, compiled_selector, default_dependency, dependency_provider,
    dependency_seed, ungrouped_host,
};
use super::plan_validate::{ScalarFault, TransformPlanError, bounded};

/// A present, behaviorally scoped selector refuses the lane and emitted
/// stages, naming the seed and stage.
#[test]
fn present_selectors_refuse_lane_and_emitted_stages() {
    let scoped = compiled_selector(SelectorShape::Dimensions {
        packages: Some(vec!["org.alpha/*"]),
        paths: None,
    });
    for stage in [TransformStage::Lane, TransformStage::Emitted] {
        let result = TransformPlan::build(vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&default_dependency()),
            stage.clone(),
            TransformImplementation::builtin_candidate("log", 1),
            None,
            Some(scoped.clone()),
        )]);
        assert!(
            matches!(
                result,
                Err(TransformPlanError::SelectorStage { seed: 0, .. })
            ),
            "stage {stage:?} must refuse a supplied selector"
        );
    }
}

/// The refusal law fires under its exact precedence: seed-major order, and
/// key scalar before duplicate key before provider scalar/hash before
/// implementation name/epoch before selector/stage.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn refusals_follow_the_frozen_precedence_exactly() {
    let expect_error = |seeds: Vec<TransformSeed>, expected: TransformPlanError, context: &str| {
        let error = TransformPlan::build(seeds).expect_err("unlawful plan refuses");
        assert_eq!(
            error, expected,
            "precedence picked wrong fault for {context}"
        );
    };

    // Within one seed: key scalar before provider scalar before the rest
    // (the version and builtin name below are also unlawful).
    expect_error(
        vec![TransformSeed::new(
            ExtensionKey::authored(""),
            TransformProvider::from(&dependency_provider(
                "org.demo",
                "tools",
                "\n",
                PackageKind::Tool,
                "sha256:aa",
            )),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("LOG", 1),
            None,
            None,
        )],
        TransformPlanError::Scalar {
            seed: 0,
            field: "key",
            fault: ScalarFault::Empty,
        },
        "key scalar first",
    );

    // The provider scalar law's other boundary, at build level: an EMPTY
    // exact version refuses as the provider.version field (the scalar
    // helper's empty case is not left covered only by unit test).
    expect_error(
        vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&dependency_provider(
                "org.demo",
                "tools",
                "",
                PackageKind::Tool,
                "sha256:aa",
            )),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            None,
        )],
        TransformPlanError::Scalar {
            seed: 0,
            field: "provider.version",
            fault: ScalarFault::Empty,
        },
        "empty provider version",
    );

    // Seed-major: seed 0's version fault beats seed 1's key fault.
    expect_error(
        vec![
            TransformSeed::new(
                ExtensionKey::authored("a"),
                TransformProvider::from(&dependency_provider(
                    "org.demo",
                    "tools",
                    "\t",
                    PackageKind::Tool,
                    "sha256:aa",
                )),
                TransformStage::Source,
                TransformImplementation::builtin_candidate("log", 1),
                None,
                None,
            ),
            dependency_seed("", TransformStage::Source),
        ],
        TransformPlanError::Scalar {
            seed: 0,
            field: "provider.version",
            fault: ScalarFault::ControlByte { position: 0 },
        },
        "seed-major iteration",
    );

    // Duplicate key with first/second indices.
    expect_error(
        vec![
            dependency_seed("dup", TransformStage::Source),
            dependency_seed("other", TransformStage::Source),
            dependency_seed("dup", TransformStage::Source),
        ],
        TransformPlanError::DuplicateKey {
            preview: bounded("dup"),
            first: 0,
            second: 2,
        },
        "duplicate indices",
    );

    // Provider hash recheck: a from_validated-constructed hash that never
    // parsed refuses at build — the exact back door the recheck closes.
    let mut provider = default_dependency();
    if let ExtensionProvider::Dependency(dependency) = &mut provider {
        dependency.content_hash = ContentHash::from_validated("sha256:nothex".to_owned());
    }
    expect_error(
        vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&provider),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            None,
        )],
        TransformPlanError::ContentHash {
            seed: 0,
            field: "provider.content_hash",
            preview: bounded("sha256:nothex"),
        },
        "hash recheck",
    );

    // Builtin name grammar: blank, uppercase and 65-byte names refuse, and
    // the name fault beats the selector/stage fault (both unlawful below).
    for name in ["", "LOG", &"l".repeat(65)] {
        let result = TransformPlan::build(vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&default_dependency()),
            TransformStage::Lane,
            TransformImplementation::builtin_candidate(name, 0),
            None,
            Some(compiled_selector(SelectorShape::Absent)),
        )]);
        assert!(
            matches!(
                result,
                Err(TransformPlanError::ImplementationName { seed: 0, .. })
            ),
            "name {name:?} must refuse as a grammar fault before stage/epoch"
        );
    }
    // A maximal lawful name (64 bytes) builds.
    let maximal = build_or_panic(vec![TransformSeed::new(
        ExtensionKey::authored("k"),
        TransformProvider::from(&default_dependency()),
        TransformStage::Source,
        TransformImplementation::builtin_candidate("l".repeat(64), 1),
        None,
        None,
    )]);
    assert_eq!(maximal.len(), 1);

    // Zero epoch refuses once the name is lawful.
    let result = TransformPlan::build(vec![TransformSeed::new(
        ExtensionKey::authored("k"),
        TransformProvider::from(&default_dependency()),
        TransformStage::Source,
        TransformImplementation::builtin_candidate("log", 0),
        None,
        None,
    )]);
    assert_eq!(
        result.expect_err("zero epoch refuses"),
        TransformPlanError::ImplementationEpoch { seed: 0 }
    );

    // Ungrouped host name scalar: empty and control-bearing names refuse.
    for name in ["", "bad\nname"] {
        let result = TransformPlan::build(vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&ungrouped_host(name)),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            None,
        )]);
        assert!(
            matches!(
                result,
                Err(TransformPlanError::Scalar {
                    seed: 0,
                    field: "host project name",
                    ..
                })
            ),
            "host name {name:?} must refuse"
        );
    }
}

/// Hostile multi-megabyte inputs render bounded diagnostics: no refusal
/// echoes its payload, through Display or Debug.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn hostile_multimegabyte_inputs_render_bounded_diagnostics() {
    let megabytes = 3 * 1024 * 1024;
    let hostile_key = format!("{}{}", "a".repeat(megabytes), "\0");
    let hostile_legible = "a".repeat(megabytes);
    let hostile_hash = ContentHash::from_validated(format!("sha256:{}g", "a".repeat(megabytes)));

    let mut dependency = match default_dependency() {
        ExtensionProvider::Dependency(dependency) => dependency,
        _ => unreachable!("test provider is a dependency"),
    };
    dependency.content_hash = hostile_hash;
    let hostile_hash_provider = ExtensionProvider::Dependency(dependency);

    let errors = [
        TransformPlan::build(vec![TransformSeed::new(
            ExtensionKey::authored(hostile_key),
            TransformProvider::from(&default_dependency()),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            None,
        )])
        .expect_err("control-bearing key refuses"),
        TransformPlan::build(vec![
            dependency_seed(&hostile_legible, TransformStage::Source),
            dependency_seed(&hostile_legible, TransformStage::Source),
        ])
        .expect_err("duplicate hostile key refuses"),
        TransformPlan::build(vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&ungrouped_host(&format!("{}{}", hostile_legible, "\t"))),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            None,
        )])
        .expect_err("hostile host name refuses"),
        TransformPlan::build(vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&default_dependency()),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("A".repeat(megabytes), 1),
            None,
            None,
        )])
        .expect_err("hostile builtin name refuses"),
        TransformPlan::build(vec![TransformSeed::new(
            ExtensionKey::authored("k"),
            TransformProvider::from(&hostile_hash_provider),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            None,
        )])
        .expect_err("hostile invalid hash refuses"),
    ];

    for error in &errors {
        let rendered = error.to_string();
        assert!(
            rendered.len() <= 256,
            "diagnostic rendered {} bytes: {rendered}",
            rendered.len()
        );
        // An echo would be megabytes; the head preview contributes at most
        // eight characters, so a 16-character run is impossible without one.
        assert!(
            !rendered.contains(&"a".repeat(16)),
            "diagnostic echoed payload: {rendered}"
        );
        let debugged = format!("{error:?}");
        assert!(
            debugged.len() <= 512,
            "debug render carried the payload ({} bytes)",
            debugged.len()
        );
        assert!(!debugged.contains(&"a".repeat(1024)));
    }
}
