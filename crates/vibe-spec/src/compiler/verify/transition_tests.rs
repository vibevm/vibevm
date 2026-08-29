//! Transition-witness reds: the immutable pre-pass semantic witness that
//! authenticates absorption dispositions (alignment alone cannot), typestate
//! monotonicity, and the source/document identity law.

use specmark::verifies;

use super::super::absorb::AbsorbPass;
use super::super::ir::{
    AbsorptionState, ArtifactContext, ClosureContribution, ClosureIr, ClosureOccurrence,
    ContributionAbsorption, DocumentAddress, LaneFrame, LaneIr, LinkInputDigest, OriginRename,
    QualificationState, StaticCompileMode,
};
use super::super::pass::Pass;
use super::super::qualify::QualifyPass;
use super::super::verify::{LaneProvenanceField, TransitionError};
use super::closure_tests::{address, closure, node, occurrence, use_edge, verify};
use super::{IrVerifier, VerificationError};
use crate::DocTree;
use crate::compiler::pass::AnyIr;

// --- the transition witness ---------------------------------------------

fn overlap_fixture() -> ClosureIr {
    let nodes = vec![
        node(
            "spec://org.demo/pkg/common/contract/x#part",
            "# Part {#part}\ndetail\n",
        ),
        node(
            "spec://org.demo/pkg/common/contract/x#full",
            "# Full {#full}\n# Part {#part}\ndetail\n",
        ),
    ];
    let seed = 1;
    let emission = vec![
        occurrence("spec://org.demo/pkg/common/contract/x#part", 0),
        occurrence("spec://org.demo/pkg/common/contract/x#full", 1),
    ];
    closure(
        nodes,
        vec![use_edge(1, 0, "spec://org.demo/pkg/common/contract/x#part")],
        emission,
        seed,
        Vec::new(),
        QualificationState::Pending(StaticCompileMode::Plain),
        AbsorptionState::Unplanned,
    )
}

fn flip_plan(plan: &super::super::ir::AbsorptionPlan) -> super::super::ir::AbsorptionPlan {
    let mut flipped = plan.clone();
    for contribution in &mut flipped.contributions {
        if let ContributionAbsorption::Normal { occurrences, .. } = contribution {
            for occurrence in occurrences {
                occurrence.absorbed = !occurrence.absorbed;
            }
        }
    }
    flipped
}

fn transition(input: &ClosureIr, output: &ClosureIr) -> Result<(), VerificationError> {
    let before = IrVerifier.witness(&AnyIr::Closure(input.clone())).unwrap();
    IrVerifier.verify_transition(&before, &AnyIr::Closure(output.clone()))
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn qualify_and_absorb_outputs_authenticate_against_their_own_witnesses() {
    let input = overlap_fixture();
    let qualified = QualifyPass::new().run(input.clone()).unwrap();
    transition(&input, &qualified).unwrap();
    verify(&qualified).unwrap();

    let absorbed = AbsorbPass::new().run(qualified.clone()).unwrap();
    transition(&qualified, &absorbed).unwrap();
    verify(&absorbed).unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn flipping_absorbed_bits_passes_alignment_but_fails_the_pre_pass_witness() {
    let input = overlap_fixture();
    let qualified = QualifyPass::new().run(input.clone()).unwrap();
    let AbsorptionState::Planned(plan) = &qualified.absorption else {
        unreachable!("qualify plans its input")
    };
    let flipped = flip_plan(plan);
    assert_ne!(&flipped, plan, "the fixture must flip a real bit");

    // Rebuild the qualify output with the flipped plan: occurrence alignment
    // still holds exactly, so both existing alignment helpers stay green —
    // only the pre-pass semantic witness can catch the forgery.
    let mut forged = qualified.clone();
    forged.absorption = AbsorptionState::Planned(flipped.clone());
    verify(&forged).expect("alignment alone is satisfied by the flipped plan");
    let error = transition(&input, &forged).unwrap_err();
    assert!(
        matches!(
            error,
            VerificationError::Transition(TransitionError::AbsorptionPlanUnauthenticated)
        ),
        "{error:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn flipping_bits_after_planning_with_a_matching_live_order_still_fails() {
    let input = overlap_fixture();
    let qualified = QualifyPass::new().run(input.clone()).unwrap();
    let AbsorptionState::Planned(plan) = &qualified.absorption else {
        unreachable!("qualify plans its input")
    };
    let flipped = flip_plan(plan);

    let mut forged = qualified.clone();
    forged.absorption = AbsorptionState::Applied(flipped.clone());
    let ClosureContribution::Normal { emission_order, .. } = &mut forged.contributions[0] else {
        unreachable!("the fixture holds one normal contribution")
    };
    let AbsorptionState::Applied(applied) = &forged.absorption else {
        unreachable!("just set it")
    };
    let ContributionAbsorption::Normal { occurrences, .. } = &applied.contributions[0] else {
        unreachable!("the fixture plans one normal contribution")
    };
    *emission_order = occurrences
        .iter()
        .filter(|occurrence| !occurrence.absorbed)
        .map(|occurrence| ClosureOccurrence {
            node: occurrence.node,
            requested_address: occurrence.requested_address.clone(),
        })
        .collect();
    verify(&forged).expect("the applied projection matches the flipped plan exactly");
    let error = transition(&qualified, &forged).unwrap_err();
    assert!(
        matches!(
            error,
            VerificationError::Transition(TransitionError::AbsorptionPlanMutated)
        ),
        "{error:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn absorption_typestate_never_regresses_or_skips_planning() {
    let input = overlap_fixture();
    let qualified = QualifyPass::new().run(input.clone()).unwrap();
    let absorbed = AbsorbPass::new().run(qualified.clone()).unwrap();

    let mut unplanned = qualified.clone();
    unplanned.absorption = AbsorptionState::Unplanned;
    assert!(matches!(
        transition(&qualified, &unplanned),
        Err(VerificationError::Transition(
            TransitionError::AbsorptionRegression { .. }
        ))
    ));

    let mut replanned = absorbed.clone();
    replanned.absorption = match &qualified.absorption {
        AbsorptionState::Planned(plan) => AbsorptionState::Planned(plan.clone()),
        _ => unreachable!("qualify planned the input"),
    };
    assert!(matches!(
        transition(&absorbed, &replanned),
        Err(VerificationError::Transition(
            TransitionError::AbsorptionRegression {
                from: "applied",
                to: "planned"
            }
        ))
    ));

    let skipped_plan = match &qualified.absorption {
        AbsorptionState::Planned(plan) => plan.clone(),
        _ => unreachable!("qualify planned the input"),
    };
    let mut skipped = input.clone();
    skipped.qualification = QualificationState::Applied(StaticCompileMode::Plain);
    skipped.absorption = AbsorptionState::Applied(skipped_plan);
    assert!(matches!(
        transition(&input, &skipped),
        Err(VerificationError::Transition(
            TransitionError::AbsorptionSkippedPlanning
        ))
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn qualification_never_regresses_and_identity_survives_only_text_transforms() {
    let input = overlap_fixture();
    let qualified = QualifyPass::new().run(input.clone()).unwrap();

    let mut regressed = qualified.clone();
    regressed.qualification = QualificationState::Pending(StaticCompileMode::Plain);
    regressed.absorption = AbsorptionState::Unplanned;
    assert!(matches!(
        transition(&qualified, &regressed),
        Err(VerificationError::Transition(
            TransitionError::QualificationRegression
        ))
    ));

    // A pre-qualify content transform is legal: it stays unplanned, and the
    // next planning pass derives its witness from the transformed view.
    let mut rewritten = input.clone();
    rewritten.nodes[1].tree = DocTree::parse("# Full {#full}\n# Part {#part}\ndetail\nmore\n");
    transition(&input, &rewritten).unwrap();
    let requirer = QualifyPass::new().run(rewritten.clone()).unwrap();
    transition(&rewritten, &requirer).unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_source_transform_may_change_text_but_never_identity() {
    let before = crate::compiler::ir::SourceIr::new(
        DocumentAddress::Spec(address("spec://org.demo/pkg/boot/entry#root")),
        crate::compiler::ir::SourceFormatId::new("markdown").unwrap(),
        "original\n",
    );
    let witness = IrVerifier.witness(&AnyIr::Source(before.clone())).unwrap();

    let honest = crate::compiler::ir::SourceIr::new(
        before.address().clone(),
        before.format().clone(),
        "rewritten text\n",
    );
    IrVerifier
        .verify_transition(&witness, &AnyIr::Source(honest))
        .unwrap();

    let retargeted = crate::compiler::ir::SourceIr::new(
        DocumentAddress::Spec(address("spec://org.evil/pkg/boot/entry#root")),
        before.format().clone(),
        "rewritten text\n",
    );
    assert!(matches!(
        IrVerifier.verify_transition(&witness, &AnyIr::Source(retargeted)),
        Err(VerificationError::Transition(
            TransitionError::Identity { .. }
        ))
    ));
}

// --- the lane witness ----------------------------------------------------

fn lane_transition(before: &LaneIr, after: &LaneIr) -> Result<(), VerificationError> {
    let witness = IrVerifier.witness(&AnyIr::Lane(before.clone())).unwrap();
    IrVerifier.verify_transition(&witness, &AnyIr::Lane(after.clone()))
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_lane_transform_owns_contributions_and_nothing_else() {
    // The witness variant used to be a UNIT carrying no evidence, so the
    // `(Lane, Lane)` transition fell through to the catch-all and a lane pass
    // was unchecked by construction. Every field named below is now real
    // evidence, and the working surface is deliberately not.
    let before = super::lane_tests::lane("# Title {#title}\nbody\n");
    lane_transition(&before, &before).expect("an unchanged lane transitions");

    let rewritten_body = super::lane_tests::lane("# Title {#title}\nrewritten\n");
    lane_transition(&before, &rewritten_body)
        .expect("contributions are the transform's working surface");

    let (context, count, digest, frame, contributions) = before.parts_for_test();
    let moved = |field: LaneProvenanceField, candidate: &LaneIr| {
        let error = lane_transition(&before, candidate).unwrap_err();
        let VerificationError::Transition(TransitionError::LaneProvenance {
            field: named,
            expected,
            actual,
        }) = error
        else {
            panic!("the lane arm owns its own variant, never `Identity`: {error:?}")
        };
        assert_eq!(named, field, "the moved field is named");
        assert_ne!(expected, actual);
    };

    moved(
        LaneProvenanceField::SourceNodeCount,
        &LaneIr::assembled(
            context.clone(),
            count + 7,
            digest.clone(),
            frame.clone(),
            contributions.to_vec(),
        ),
    );
    moved(
        LaneProvenanceField::SourceLinkDigest,
        &LaneIr::assembled(
            context.clone(),
            count,
            LinkInputDigest([7; 32]),
            frame.clone(),
            contributions.to_vec(),
        ),
    );
    for (field, path, root, renames) in [
        (
            LaneProvenanceField::FrameGeneratedPath,
            Some("boot/FORGED.md".to_string()),
            frame.source_root.clone(),
            frame.renames.clone(),
        ),
        (
            LaneProvenanceField::FrameSourceRoot,
            frame.generated_path.clone(),
            Some("forged/root".to_string()),
            frame.renames.clone(),
        ),
        (
            LaneProvenanceField::FrameRenames,
            frame.generated_path.clone(),
            frame.source_root.clone(),
            vec![OriginRename {
                origin: "org.demo/forged".to_string(),
                rename: crate::RenameEntry {
                    original: "root".to_string(),
                    qualified: "org-demo-forged--root".to_string(),
                },
            }],
        ),
    ] {
        moved(
            field,
            &LaneIr::assembled(
                context.clone(),
                count,
                digest.clone(),
                LaneFrame {
                    generated_path: path,
                    source_root: root,
                    renames,
                },
                contributions.to_vec(),
            ),
        );
    }
    moved(
        LaneProvenanceField::Context,
        &LaneIr::assembled(
            ArtifactContext::compatibility(StaticCompileMode::QualifyPerNode),
            count,
            digest.clone(),
            frame.clone(),
            contributions.to_vec(),
        ),
    );
}
