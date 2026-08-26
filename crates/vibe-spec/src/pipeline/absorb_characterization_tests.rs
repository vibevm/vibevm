//! Output-sensitive characterizations of the named absorb boundary.

use specmark::verifies;

use super::tests::MockSource;
use super::*;
use crate::compiler::absorb::{
    absorb_invocations, reset_absorb_invocations, validate_applied_absorption,
};
use crate::compiler::ir::{AbsorptionState, ClosureContribution, StaticCompileMode};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn both_public_modes_invoke_named_absorb_once_and_keep_exact_bytes() {
    let key = "spec://org.demo/pkg/boot/entry#root";
    let source = MockSource::new(&[(key, "# Entry {#root}\nBODY\n")]);
    let seed = SpecAddress::parse(key).unwrap();
    let expected_plain = concat!(
        "<!-- vibe:begin spec://org.demo/pkg/boot/entry#root -->\n",
        "# Entry {#root}\n",
        "BODY\n",
        "<!-- vibe:end spec://org.demo/pkg/boot/entry#root -->\n"
    );

    reset_absorb_invocations();
    let plain = compile_static(&seed, &source).unwrap();
    assert_eq!(absorb_invocations(), 1);
    assert_eq!(plain, expected_plain);

    reset_absorb_invocations();
    let (qualified, renames) = compile_static_qualified(&seed, &source).unwrap();
    assert_eq!(absorb_invocations(), 1);
    assert_eq!(
        qualified,
        concat!(
            "<!-- vibe:begin spec://org.demo/pkg/boot/entry#root -->\n",
            "# Entry {#org-demo--pkg--root}\n",
            "BODY\n",
            "<!-- vibe:end spec://org.demo/pkg/boot/entry#root -->\n"
        )
    );
    assert_eq!(renames.len(), 1);
    assert_eq!(renames[0].0, "org.demo/pkg");
    assert_eq!(renames[0].1.original, "root");
    assert_eq!(renames[0].1.qualified, "org-demo--pkg--root");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn named_absorb_is_the_only_owner_of_read_once_projection() {
    let entry = "spec://org.demo/pkg/common/doc#root";
    let sub = "spec://org.demo/pkg/common/doc#sub";
    let source = MockSource::new(&[
        (
            entry,
            &format!("# Root {{#root}}\n#use {sub}\n## Sub {{#sub}}\nSUB\n"),
        ),
        (sub, "## Sub {#sub}\nSUB\n"),
    ]);
    let seed = SpecAddress::parse(entry).unwrap();

    reset_absorb_invocations();
    let (output, renames) = compile_static_qualified(&seed, &source).unwrap();

    assert_eq!(absorb_invocations(), 1);
    assert!(output.contains("vibe:begin spec://org.demo/pkg/common/doc#root"));
    assert!(!output.contains("vibe:begin spec://org.demo/pkg/common/doc#sub"));
    assert_eq!(
        output.matches("{#org-demo--pkg--common-doc--sub}").count(),
        1,
        "{output}"
    );
    assert_eq!(
        renames
            .iter()
            .filter(|(_, rename)| rename.original == "sub")
            .count(),
        1
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn production_closure_carries_an_applied_auditable_plan() {
    let entry = "spec://org.demo/pkg/common/doc#root";
    let sub = "spec://org.demo/pkg/common/doc#sub";
    let source = MockSource::new(&[
        (
            entry,
            &format!("# Root {{#root}}\n#use {sub}\n## Sub {{#sub}}\nSUB\n"),
        ),
        (sub, "## Sub {#sub}\nSUB\n"),
    ]);
    let seed = SpecAddress::parse(entry).unwrap();

    let closure = crate::compiler::builtin::compile_absorbed_closure(
        &seed,
        &source,
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();

    assert!(matches!(closure.absorption, AbsorptionState::Applied(_)));
    validate_applied_absorption(&closure).unwrap();
    let [ClosureContribution::Normal { emission_order, .. }] = closure.contributions.as_slice()
    else {
        panic!("compatibility closure remains one normal contribution")
    };
    assert_eq!(emission_order.len(), 1);
}
