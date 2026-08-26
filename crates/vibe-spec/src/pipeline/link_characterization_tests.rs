use specmark::verifies;

use super::tests::MockSource;
use super::*;
use crate::DocTree;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, AbsorptionState, ArtifactContext, ClosureContribution,
    ClosureDocument, ClosureIr, ClosureNodeId, ClosureOccurrence, ContributionAbsorption,
    ContributionMeta, DocumentAddress, LinkState, QualificationState, StaticCompileMode,
};
use crate::compiler::link::{link_invocations, reset_link_invocations};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn public_modes_invoke_link_once_and_keep_exact_bytes() {
    let key = "spec://org.demo/pkg/boot/entry#root";
    let source = MockSource::new(&[(key, "# Entry {#root}\nBODY\n")]);
    let seed = SpecAddress::parse(key).unwrap();

    reset_link_invocations();
    let plain = compile_static(&seed, &source).unwrap();
    assert_eq!(link_invocations(), 1);
    assert_eq!(
        plain,
        concat!(
            "<!-- vibe:begin spec://org.demo/pkg/boot/entry#root -->\n",
            "# Entry {#root}\nBODY\n",
            "<!-- vibe:end spec://org.demo/pkg/boot/entry#root -->\n",
        )
    );

    reset_link_invocations();
    let (qualified, renames) = compile_static_qualified(&seed, &source).unwrap();
    assert_eq!(link_invocations(), 1);
    assert_eq!(
        qualified,
        concat!(
            "<!-- vibe:begin spec://org.demo/pkg/boot/entry#root -->\n",
            "# Entry {#org-demo--pkg--root}\nBODY\n",
            "<!-- vibe:end spec://org.demo/pkg/boot/entry#root -->\n",
        )
    );
    assert_eq!(renames.len(), 1);
}

#[test]
fn legacy_continuation_rejects_an_unlinked_applied_closure() {
    let address = SpecAddress::parse("spec://org.demo/pkg/boot/entry#root").unwrap();
    let meta = ContributionMeta {
        origin: "org.demo/pkg".to_string(),
        path: "boot/entry".to_string(),
    };
    let closure = ClosureIr::testing(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        vec![ClosureDocument {
            address: DocumentAddress::Spec(address.clone()),
            origin: "org.demo/pkg".to_string(),
            tree: DocTree::parse("BODY"),
            aliases: Default::default(),
        }],
        Vec::new(),
        vec![ClosureContribution::Normal {
            meta: meta.clone(),
            seed: ClosureNodeId(0),
            seed_address: address.clone(),
            emission_order: vec![ClosureOccurrence {
                node: ClosureNodeId(0),
                requested_address: address.clone(),
            }],
        }],
        Vec::new(),
        QualificationState::Applied(StaticCompileMode::Plain),
        AbsorptionState::Applied(AbsorptionPlan {
            mode: StaticCompileMode::Plain,
            contributions: vec![ContributionAbsorption::Normal {
                meta,
                seed: ClosureNodeId(0),
                seed_address: address.clone(),
                occurrences: vec![AbsorptionOccurrence {
                    node: ClosureNodeId(0),
                    requested_address: address,
                    absorbed: false,
                }],
            }],
        }),
        LinkState::Unlinked,
        None,
        None,
    );

    let panic = std::panic::catch_unwind(|| compile_static_continuation(closure));
    assert!(
        panic.is_err(),
        "legacy continuation must require named link"
    );
}
