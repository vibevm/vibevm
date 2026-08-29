//! The T8 verdict table, asserted directly on the admission gate.
//!
//! These are the rows a live compile cannot all reach today: every producer
//! currently mints `Undetermined` for a declared document and `Unclaimed` for
//! a reached one, so no coordinate arm exists in production until the
//! owner-view adapter lands. The table is a property of the gate, not of
//! today's producers, so it is asserted where every row is spellable — and
//! the rows that ARE live are asserted again end to end in
//! `schedule_selector_tests`.

use specmark::verifies;
use vibe_core::{Group, PackageName};

use crate::compiler::ir::{DocumentProvider, DocumentSubject};

use super::plan_test_support::{SelectorShape, compiled_selector};
use super::selector_admission::{SelectorAdmissionError, SelectorGate, SelectorVerdict};

/// One gate over the authored dimensions, compiled by the kernel exactly as
/// a collected registry row would be.
fn gate(packages: Option<Vec<&'static str>>, paths: Option<Vec<&'static str>>) -> SelectorGate {
    SelectorGate::new(&compiled_selector(SelectorShape::Dimensions {
        packages,
        paths,
    }))
}

/// The one declared path every subject below carries, so the `packages` rows
/// vary in exactly one member.
const DECLARED_PATH: &str = "boot/alpha.md";

fn subject(provider: DocumentProvider) -> DocumentSubject {
    DocumentSubject::declared(provider, DECLARED_PATH)
}

fn coordinate(group: &str, name: &str) -> (Group, PackageName) {
    (
        Group::parse(group).expect("valid test group"),
        PackageName::parse(name).expect("valid test package name"),
    )
}

fn dependency(group: &str, name: &str) -> DocumentProvider {
    let (group, name) = coordinate(group, name);
    DocumentProvider::Dependency { group, name }
}

fn host_coordinate(group: &str, name: &str) -> DocumentProvider {
    let (group, name) = coordinate(group, name);
    DocumentProvider::HostCoordinate { group, name }
}

/// The six provider arms, each beside the spelling its typed kernel identity
/// renders through the kernel's own codec. `None` is "no coordinate exists".
fn provider_arms() -> Vec<(DocumentProvider, Option<&'static str>)> {
    vec![
        (dependency("org.demo", "tools"), Some("org.demo/tools")),
        (
            DocumentProvider::HostUngrouped {
                name: "demo".to_string(),
            },
            Some("__host__/demo"),
        ),
        (
            host_coordinate("org.demo", "hosted"),
            Some("org.demo/hosted"),
        ),
        (
            DocumentProvider::HostVirtualWorkspace,
            Some("<virtual-workspace>"),
        ),
        (DocumentProvider::Unclaimed, None),
        (DocumentProvider::Undetermined, None),
    ]
}

/// Row 1 of the table: a coordinate arm is rebuilt as the typed kernel
/// identity and judged by the kernel.
///
/// The rendered spelling appears here only as the EXPECTED pattern a test
/// authors — the gate builds `DependencyProviderId`/`HostIdentity` from the
/// subject's validated components and lets the kernel render them at match
/// time, so nothing on the production path parses a spelling back into
/// identity. All four coordinate arms are covered, including the two that
/// only a host can carry.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn every_coordinate_arm_is_judged_through_its_typed_kernel_identity() {
    for (provider, rendered) in provider_arms() {
        let Some(rendered) = rendered else {
            continue;
        };
        assert_eq!(
            gate(Some(vec![rendered]), None).admit(&subject(provider.clone())),
            Ok(SelectorVerdict::Matched),
            "{provider} must match the exact spelling `{rendered}`"
        );
        assert_eq!(
            gate(Some(vec!["org.other/absent"]), None).admit(&subject(provider.clone())),
            Ok(SelectorVerdict::Skipped),
            "{provider} must not match an unrelated coordinate"
        );
    }
}

/// Rows 2 and 3, together, because the whole point is that they DIFFER.
///
/// `Unclaimed` answering "no match" is a CHOSEN verdict, not one inherited by
/// accident from the kernel's absent-value rule: no contribution row declared
/// this document, so no owner exists for a `packages` dimension to name, and
/// the address' authority — the package that OWNS the document — is not the
/// question that dimension asks. The gate expresses the choice by mapping the
/// arm onto an absent kernel provider, so the one glob authority still gives
/// the answer and no second copy of the rule exists to drift.
///
/// The same answer would be silently WRONG for `Undetermined`: a row DID
/// declare that document and its owner merely has no typed spelling yet, so
/// "matches nothing" would be a confident lie whose symptom is a transform
/// that quietly never applies. That arm refuses instead. Collapse either into
/// the other and exactly one of these two assertions goes red.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn the_two_absences_answer_an_authored_packages_dimension_differently() {
    // Maximally permissive members, so the verdict is about the ABSENT
    // provider and never about a pattern that failed to match.
    let permissive = gate(Some(vec!["*", "**", "org.demo/*"]), None);

    assert_eq!(
        permissive.admit(&subject(DocumentProvider::Unclaimed)),
        Ok(SelectorVerdict::Skipped),
        "an unclaimed document is out of scope, and that is a final verdict"
    );
    assert_eq!(
        permissive.admit(&subject(DocumentProvider::Undetermined)),
        Err(SelectorAdmissionError::UndeterminedProvider),
        "an undetermined provider is not an answer, so it refuses"
    );
}

/// Row 4: with no authored `packages` dimension the provider is irrelevant —
/// for every arm, including the one that otherwise refuses.
///
/// This is what keeps the surviving capability gap narrow. An `Undetermined`
/// provider only blocks a question that was actually asked; a selector that
/// scopes by path alone judges every document in the world, and the unknown
/// owner never enters the decision.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn without_an_authored_packages_dimension_the_provider_is_irrelevant() {
    let matching = gate(None, Some(vec!["boot/*.md"]));
    let failing = gate(None, Some(vec!["boot/other.md"]));
    for (provider, _) in provider_arms() {
        assert_eq!(
            matching.admit(&subject(provider.clone())),
            Ok(SelectorVerdict::Matched),
            "{provider}: the path decides alone"
        );
        assert_eq!(
            failing.admit(&subject(provider.clone())),
            Ok(SelectorVerdict::Skipped),
            "{provider}: the path decides alone"
        );
    }
}

/// A selector with no authored dimension at all judges every subject in
/// scope.
///
/// `TransformPlan::build` canonicalizes such a selector to outer absence, so
/// this row is unreachable through a plan — but the gate is a total function
/// over the values it accepts, and a partial one would be a trap for the
/// first caller that reaches it another way.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn an_unauthored_selector_matches_every_subject() {
    let unscoped = SelectorGate::new(&compiled_selector(SelectorShape::Absent));
    for (provider, _) in provider_arms() {
        assert_eq!(
            unscoped.admit(&subject(provider.clone())),
            Ok(SelectorVerdict::Matched),
            "{provider}: an unauthored selector scopes nothing out"
        );
    }
}

/// `BACKLOG.md` `B-117`, closed at this adapter: a path that cannot obey the
/// `paths` contract refuses BEFORE anything is matched.
///
/// The refusal is deliberately unconditional in the authored dimensions. A
/// backslashed path is malformed, not merely unknown — the three boundaries
/// that already refuse one (the artifact plan, the wire scalar gate, the
/// inter-pass verifier) do not first ask what will read it, and neither does
/// this one. That is exactly the difference from `Undetermined`, which is a
/// well-formed value whose answer is unknown and therefore only refuses when
/// something asks for it.
///
/// The last row freezes the precedence: a subject that is BOTH backslashed
/// and undetermined refuses on the path, because deciding a malformed value
/// against a glob answers a question that was never well posed.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn a_backslashed_declared_path_refuses_before_any_matching() {
    let backslashed = |provider| DocumentSubject::declared(provider, "boot\\alpha.md");
    let path_refusal = |subject: &DocumentSubject| {
        matches!(
            SelectorGate::new(&compiled_selector(SelectorShape::Absent)).admit(subject),
            Err(SelectorAdmissionError::BackslashedDeclaredPath { .. })
        )
    };
    assert!(
        path_refusal(&backslashed(DocumentProvider::Unclaimed)),
        "the contract holds even when no dimension reads the path"
    );

    for shape in [
        gate(None, Some(vec!["boot/*"])),
        gate(Some(vec!["org.demo/*"]), None),
        gate(Some(vec!["org.demo/*"]), Some(vec!["boot/*"])),
    ] {
        assert!(
            matches!(
                shape.admit(&backslashed(DocumentProvider::Unclaimed)),
                Err(SelectorAdmissionError::BackslashedDeclaredPath { .. })
            ),
            "every authored shape refuses a malformed path"
        );
    }

    // Precedence: the malformed path beats the undecidable provider.
    assert!(
        matches!(
            gate(Some(vec!["org.demo/*"]), None)
                .admit(&backslashed(DocumentProvider::Undetermined)),
            Err(SelectorAdmissionError::BackslashedDeclaredPath { .. })
        ),
        "the path contract is checked before the provider is consulted"
    );

    // The negative control: the same subject with the separator corrected
    // reaches the provider rule and answers with it.
    assert_eq!(
        gate(Some(vec!["org.demo/*"]), None).admit(&subject(DocumentProvider::Undetermined)),
        Err(SelectorAdmissionError::UndeterminedProvider),
        "only the separator differed"
    );
}
