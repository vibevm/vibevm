use specmark::verifies;

use super::tests::MockSource;
use super::*;
use crate::DocTree;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, AbsorptionState, ArtifactId, ClosureContribution,
    ClosureDocument, ClosureIr, ClosureNodeId, ContributionAbsorption, ContributionMeta,
    DocumentAddress, LinkState, QualificationState,
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
    let closure = ClosureIr {
        artifact: ArtifactId::new("static-fragment").unwrap(),
        nodes: vec![ClosureDocument {
            address: DocumentAddress::Spec(address.clone()),
            origin: "org.demo/pkg".to_string(),
            tree: DocTree::parse("BODY"),
            aliases: Default::default(),
        }],
        edges: Vec::new(),
        contributions: vec![ClosureContribution::Normal {
            meta: meta.clone(),
            seed: ClosureNodeId(0),
            emission_order: vec![ClosureNodeId(0)],
        }],
        renames: Vec::new(),
        qualification: QualificationState::Applied(StaticCompileMode::Plain),
        absorption: AbsorptionState::Applied(AbsorptionPlan {
            mode: StaticCompileMode::Plain,
            contributions: vec![ContributionAbsorption::Normal {
                meta,
                seed: ClosureNodeId(0),
                seed_address: address.clone(),
                occurrences: vec![AbsorptionOccurrence {
                    node: ClosureNodeId(0),
                    address,
                    absorbed: false,
                }],
            }],
        }),
        link: LinkState::Unlinked,
        pending_sources: None,
        pending_embeds: None,
    };

    let panic = std::panic::catch_unwind(|| compile_static_continuation(closure));
    assert!(
        panic.is_err(),
        "legacy continuation must require named link"
    );
}
