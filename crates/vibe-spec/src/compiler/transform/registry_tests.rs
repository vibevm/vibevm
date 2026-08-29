//! The T5 behavior-registry RED matrix (R4-TRANSFORM-PLAN-ABI §6.1): the
//! empty production catalog, the exact sorted test golden, one frozen
//! input→identical-output vector per stage, and the bounded
//! registration/resolution refusals.
//!
//! The `test-identity-*` vehicles and the one test registry come from
//! `registry_test_support` — the single test authority T6 will consume too —
//! while the deliberately-invalid registration fixtures and the semantic
//! assertions stay local to this cell.

use std::sync::Arc;

use specmark::verifies;

use crate::DocTree;
use crate::compiler::ir::{
    ArtifactContext, DocumentAddress, DocumentIr, LaneFrame, LaneIr, LinkInputDigest,
    SourceFormatId, SourceIr, StaticCompileMode,
};

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::config::ConfigTable;
use super::plan::{TransformConfig, TransformImplementation, TransformPlan, TransformStage};
use super::plan_test_support::{build_or_panic, dependency_seed, empty_config};
use super::registry::TransformRegistry;
use super::registry::TransformRegistryError;
use super::registry_test_support::{identity_registry, identity_vehicles};

/// A behavior whose name is rejected by the backend-id grammar.
struct BadName;

impl TransformBehavior for BadName {
    fn name(&self) -> &str {
        "Not A Valid Id"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Source
    }
}

/// A behavior whose epoch is zero.
struct ZeroEpoch;

impl TransformBehavior for ZeroEpoch {
    fn name(&self) -> &str {
        "zero-epoch"
    }
    fn epoch(&self) -> u32 {
        0
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Source
    }
}

/// The frozen source vector: one simple static entry's canonical Markdown.
fn source_vector() -> SourceIr {
    SourceIr::new(
        DocumentAddress::StaticEntry {
            origin: "demo".to_owned(),
            path: "docs/alpha.md".to_owned(),
        },
        SourceFormatId::canonical_markdown(),
        "# Alpha {#root}\n\nBody text.\n",
    )
}

/// The frozen document vector: the source vector, parsed by the real tree.
fn document_vector() -> DocumentIr {
    let source = source_vector();
    let text = source.text().to_owned();
    DocumentIr::new(source, DocTree::parse(&text))
}

/// The frozen lane vector: one compatibility-frame lane with no
/// contributions, assembled through the real constructor.
fn lane_vector() -> LaneIr {
    LaneIr::assembled(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        0,
        LinkInputDigest([0; 32]),
        LaneFrame {
            generated_path: None,
            source_root: None,
            renames: Vec::new(),
        },
        Vec::new(),
    )
}

/// The frozen emitted vector: owned bytes of a tiny pretty-printed stream.
fn emitted_vector() -> Vec<u8> {
    b"<?xml version=\"1.0\"?>\n<spec>\n  <a/>\n</spec>\n".to_vec()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_production_catalog_is_empty_and_resolves_nothing() {
    let production = TransformRegistry::builtins();
    assert!(
        production.catalog().is_empty(),
        "no shipping no-op builtin name is reserved"
    );
    for (name, stage) in [
        ("test-identity-source", TransformStage::Source),
        ("xml-minify", TransformStage::Emitted),
        ("log", TransformStage::Source),
    ] {
        let error = production
            .resolve(&TransformImplementation::builtin_candidate(name, 1), &stage)
            .err()
            .expect("the empty production registry resolves nothing");
        assert!(
            matches!(error, TransformRegistryError::UnknownBuiltin { .. }),
            "{name}: {error:?}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_test_catalog_golden_is_exact_and_sorted() {
    let catalog: Vec<(String, u32, TransformStage)> = identity_registry()
        .catalog()
        .into_iter()
        .map(|(name, epoch, stage)| (name.to_owned(), epoch, stage.clone()))
        .collect();
    assert_eq!(
        catalog,
        vec![
            (
                "test-identity-document".to_owned(),
                1,
                TransformStage::Document
            ),
            (
                "test-identity-emitted".to_owned(),
                1,
                TransformStage::Emitted
            ),
            ("test-identity-lane".to_owned(), 1, TransformStage::Lane),
            ("test-identity-source".to_owned(), 1, TransformStage::Source),
        ],
        "changing any name, epoch or stage breaks the golden"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn every_identity_returns_its_frozen_input_unchanged_at_none_and_authored_empty_config() {
    let empty = empty_config();
    for (behavior, _, stage) in identity_vehicles() {
        match stage {
            TransformStage::Source => {
                let input = source_vector();
                assert_eq!(behavior.run_source(None, input.clone()).unwrap(), input);
                assert_eq!(
                    behavior.run_source(Some(&empty), input.clone()).unwrap(),
                    input
                );
            }
            TransformStage::Document => {
                let input = document_vector();
                assert_eq!(behavior.run_document(None, input.clone()).unwrap(), input);
                assert_eq!(
                    behavior.run_document(Some(&empty), input.clone()).unwrap(),
                    input
                );
            }
            TransformStage::Lane => {
                let input = lane_vector();
                assert_eq!(behavior.run_lane(None, input.clone()).unwrap(), input);
                assert_eq!(
                    behavior.run_lane(Some(&empty), input.clone()).unwrap(),
                    input
                );
            }
            TransformStage::Emitted => {
                let input = emitted_vector();
                assert_eq!(behavior.run_emitted(None, input.clone()).unwrap(), input);
                assert_eq!(
                    behavior.run_emitted(Some(&empty), input.clone()).unwrap(),
                    input
                );
            }
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn calling_a_nondeclared_stage_method_refuses_with_the_typed_wrong_stage() {
    for (behavior, _, declared_stage) in identity_vehicles() {
        for called_stage in [
            TransformStage::Source,
            TransformStage::Document,
            TransformStage::Lane,
            TransformStage::Emitted,
        ] {
            let result = match called_stage {
                TransformStage::Source => behavior.run_source(None, source_vector()).map(|_| ()),
                TransformStage::Document => {
                    behavior.run_document(None, document_vector()).map(|_| ())
                }
                TransformStage::Lane => behavior.run_lane(None, lane_vector()).map(|_| ()),
                TransformStage::Emitted => behavior.run_emitted(None, emitted_vector()).map(|_| ()),
            };
            if called_stage == declared_stage {
                assert!(
                    result.is_ok(),
                    "{declared_stage:?} identity refused its own stage"
                );
            } else {
                let error = result.expect_err("a nondeclared stage must refuse");
                assert!(
                    matches!(
                        error,
                        TransformBehaviorError::WrongStage { declared, called, .. }
                            if declared == declared_stage && called == called_stage
                    ),
                    "{declared_stage:?} vehicle at {called_stage:?}"
                );
            }
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn resolution_requires_the_matching_name_epoch_and_stage() {
    let registry = identity_registry();
    for (_, name, stage) in identity_vehicles() {
        // Matching triple resolves; the Arc IS the registered implementation.
        let original = registry
            .resolve(&TransformImplementation::builtin_candidate(name, 1), &stage)
            .expect("the matching triple resolves");
        assert_eq!(original.name(), name);
        assert_eq!(original.epoch(), 1);
        assert_eq!(original.stage(), stage);

        // Wrong epoch names the requested and catalog epochs.
        let error = registry
            .resolve(&TransformImplementation::builtin_candidate(name, 2), &stage)
            .err()
            .expect("a stale epoch must refuse");
        assert!(matches!(
            error,
            TransformRegistryError::EpochMismatch {
                requested: 2,
                catalog: 1,
                ..
            }
        ));

        // Wrong stage names both stages.
        let wrong = match stage {
            TransformStage::Source | TransformStage::Document | TransformStage::Lane => {
                TransformStage::Emitted
            }
            TransformStage::Emitted => TransformStage::Source,
        };
        let error = registry
            .resolve(&TransformImplementation::builtin_candidate(name, 1), &wrong)
            .err()
            .expect("the wrong stage must refuse");
        assert!(matches!(
            error,
            TransformRegistryError::StageMismatch { ref requested, ref catalog, .. }
                if *requested == wrong && *catalog == stage
        ));
    }

    // Lifecycle and not-yet-shipping names are unknown here.
    for unknown in ["log", "xml-minify", "minify"] {
        let error = registry
            .resolve(
                &TransformImplementation::builtin_candidate(unknown, 1),
                &TransformStage::Emitted,
            )
            .err()
            .expect("an off-catalog name must refuse");
        assert!(
            matches!(error, TransformRegistryError::UnknownBuiltin { .. }),
            "{unknown}: {error:?}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn an_empty_registry_makes_a_former_test_identity_unknown() {
    let registry = identity_registry();
    drop(registry);
    let error = TransformRegistry::builtins()
        .resolve(
            &TransformImplementation::builtin_candidate("test-identity-source", 1),
            &TransformStage::Source,
        )
        .err()
        .expect("dropping the catalog turns its rows unknown");
    assert!(matches!(
        error,
        TransformRegistryError::UnknownBuiltin { .. }
    ));
}

#[test]
fn registration_refuses_invalid_name_zero_epoch_and_collision() {
    let mut registry = TransformRegistry::builtins();
    assert!(matches!(
        registry.register(Arc::new(BadName)),
        Err(TransformRegistryError::InvalidName { .. })
    ));
    assert!(matches!(
        registry.register(Arc::new(ZeroEpoch)),
        Err(TransformRegistryError::EpochZero { .. })
    ));
    let (vehicle, _, _) = identity_vehicles()
        .into_iter()
        .next()
        .expect("four identity vehicles");
    registry
        .register(vehicle.clone())
        .expect("the first registration succeeds");
    assert!(matches!(
        registry.register(vehicle),
        Err(TransformRegistryError::Collision { .. })
    ));
    assert_eq!(
        registry.catalog().len(),
        1,
        "refused rows never enter the catalog"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_hostile_multimegabyte_name_refuses_bounded() {
    let hostile = format!("{}{}", "a".repeat(3 * 1024 * 1024), "!");
    let error = identity_registry()
        .resolve(
            &TransformImplementation::builtin_candidate(hostile, 1),
            &TransformStage::Source,
        )
        .err()
        .expect("a hostile name is unknown and must refuse bounded");
    let TransformRegistryError::UnknownBuiltin { preview } = error else {
        panic!("expected the bounded unknown refusal");
    };
    let rendered = preview.to_string();
    assert!(rendered.len() <= 64, "rendered {rendered}");
    assert!(!rendered.contains(&"a".repeat(16)), "an echo is impossible");
    // The preview names the true byte count (3 MiB + 1) — a number, never
    // the payload itself.
    assert!(
        rendered.contains("total length 3145729 bytes"),
        "the true length is named: {rendered}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn resolve_returns_the_registered_implementation_not_a_substitute() {
    let (registered, _, _) = identity_vehicles()
        .into_iter()
        .next()
        .expect("four identity vehicles");
    let mut registry = TransformRegistry::builtins();
    registry
        .register(registered.clone())
        .expect("the vehicle registers");
    let resolved = registry
        .resolve(
            &TransformImplementation::builtin_candidate("test-identity-source", 1),
            &TransformStage::Source,
        )
        .expect("the identity resolves");
    assert!(
        Arc::ptr_eq(&registered, &resolved),
        "resolution lends the registered object, not a fresh substitute"
    );
    // Three strong references: the registry's row, the test's own handle and
    // the resolved clone — the pointer identity above is the real proof.
    assert_eq!(Arc::strong_count(&registered), 3);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_plan_stays_grammar_only_and_off_catalog_candidates_remain_legal() {
    // An off-catalog T2 candidate still builds a legal plan value: the
    // registry consult happens at resolution, never inside `build`.
    let plan = build_or_panic(vec![dependency_seed(
        "org.demo/tools#log",
        TransformStage::Source,
    )]);
    assert_eq!(plan.entries().len(), 1);
    assert_eq!(
        plan.entries()[0].seed().implementation().builtin_name(),
        "log"
    );
    assert!(plan.digest().is_some());
    // The empty plan law is untouched by the registry's existence.
    assert_eq!(TransformPlan::empty().digest(), None);
    // An authored-empty config wrapper still wraps the neutral empty table.
    assert_eq!(empty_config(), TransformConfig::new(ConfigTable::new()));
}
