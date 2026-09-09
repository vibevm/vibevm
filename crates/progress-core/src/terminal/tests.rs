use super::*;
use crate::evidence::{Evidence, EvidenceProvider, NoEvidence};
use crate::model::{Action, Granularity, MarkerForm};

fn marker(stage: Stage, state: State) -> Marker {
    Marker {
        stage,
        state,
        action: None,
        actionstage: None,
        audience: Vec::new(),
        comment: None,
        r#ref: None,
        form: MarkerForm::Point,
        granularity: Granularity::Paragraph,
        line: 1,
    }
}

fn requirements(csv: &str) -> ArtifactRequirements {
    ArtifactRequirements::parse_csv(csv).unwrap()
}

struct Typed(ProviderArtifactEvidence);

impl EvidenceProvider for Typed {
    fn evidence_for(&self, _unit_addr: &str) -> Option<Evidence> {
        None
    }

    fn artifact_evidence_for(
        &self,
        _unit_addr: &str,
        _artifact: ArtifactKind,
        _reference: Option<&str>,
    ) -> ProviderArtifactEvidence {
        self.0.clone()
    }
}

#[test]
fn absence_is_unclassified_not_vacuously_terminal() {
    let result = resolve_terminal(
        "a.md#A",
        &marker(Stage::Spec, State::Done),
        None,
        &NoEvidence,
    );
    assert_eq!(result.outcome, TerminalOutcome::Unclassified);
    assert!(result.artifacts.is_empty());
}

#[test]
fn self_carried_artifacts_close_only_at_done_and_action_keeps_pending() {
    let req = requirements("specification,decision,research,plan,disposition");
    let mut status = marker(Stage::Spec, State::Work);
    let result = resolve_terminal("a.md#A", &status, Some(&req), &NoEvidence);
    assert_eq!(result.outcome, TerminalOutcome::Pending);
    assert!(
        result
            .artifacts
            .iter()
            .all(|artifact| artifact.state == ArtifactState::Missing
                && artifact.locators == Some(vec!["a.md#A".into()]))
    );

    status.state = State::Done;
    let result = resolve_terminal("a.md#A", &status, Some(&req), &NoEvidence);
    assert_eq!(result.outcome, TerminalOutcome::Terminal);
    assert!(result.artifacts.iter().all(|artifact| {
        artifact.state == ArtifactState::Satisfied
            && artifact.locators.as_deref() == Some(["a.md#A".to_string()].as_slice())
    }));

    status.action = Some(Action::Continue);
    assert_eq!(
        resolve_terminal("a.md#A", &status, Some(&req), &NoEvidence).outcome,
        TerminalOutcome::Pending
    );
}

#[test]
fn counts_include_addressed_unclassified_facts_without_a_whole_unit_marker() {
    let mut doc = crate::parse::parse_document(
        "a.md",
        "# A {#a}\n\n@fact:PLAIN no marker\n\n@fact:WRAP <status stage=\"spec\" state=\"done\">fragment</status>\n",
    );
    assert_eq!(counts_for_doc(&doc, &NoEvidence).unclassified, 2);

    // The additive field is absent in an old sidecar. An unclassified fact
    // still counts; no status inference is needed for that outcome.
    for fact in doc.blocks.iter_mut().flat_map(|block| &mut block.facts) {
        fact.marker_index = None;
    }
    assert_eq!(counts_for_doc(&doc, &NoEvidence).unclassified, 2);
}

#[test]
fn known_zero_unavailable_and_positive_without_locators_stay_distinct() {
    let req = requirements("implementation");
    let status = marker(Stage::Impl, State::Done);
    let unavailable = resolve_terminal("a.md#A", &status, Some(&req), &NoEvidence);
    assert_eq!(unavailable.artifacts[0].state, ArtifactState::Unavailable);
    assert_eq!(unavailable.artifacts[0].count, None);

    let zero = Typed(ProviderArtifactEvidence::Known {
        count: 0,
        locators: Some(Vec::new()),
    });
    let missing = resolve_terminal("a.md#A", &status, Some(&req), &zero);
    assert_eq!(missing.artifacts[0].state, ArtifactState::Missing);
    assert_eq!(missing.artifacts[0].count, Some(0));

    let positive = Typed(ProviderArtifactEvidence::Known {
        count: 2,
        locators: None,
    });
    let satisfied = resolve_terminal("a.md#A", &status, Some(&req), &positive);
    assert_eq!(satisfied.outcome, TerminalOutcome::Terminal);
    assert_eq!(satisfied.artifacts[0].count, Some(2));
    assert_eq!(satisfied.artifacts[0].locators, None);
}

#[test]
fn provider_locators_are_canonical_without_changing_the_observed_count() {
    let req = requirements("implementation");
    let provider = Typed(ProviderArtifactEvidence::Known {
        count: 3,
        locators: Some(vec!["z.rs:9".into(), "a.rs:1".into(), "z.rs:9".into()]),
    });
    let result = resolve_terminal(
        "a.md#A",
        &marker(Stage::Impl, State::Done),
        Some(&req),
        &provider,
    );
    assert_eq!(result.artifacts[0].count, Some(3));
    assert_eq!(
        result.artifacts[0].locators,
        Some(vec!["a.rs:1".into(), "z.rs:9".into()])
    );
}

#[test]
fn documentation_self_carries_only_at_doc_done() {
    let req = requirements("documentation");
    let doc = resolve_terminal(
        "a.md#A",
        &marker(Stage::Doc, State::Done),
        Some(&req),
        &NoEvidence,
    );
    assert_eq!(doc.outcome, TerminalOutcome::Terminal);

    let spec = resolve_terminal(
        "a.md#A",
        &marker(Stage::Spec, State::Done),
        Some(&req),
        &NoEvidence,
    );
    assert_eq!(spec.artifacts[0].state, ArtifactState::Unavailable);

    let observed = Typed(ProviderArtifactEvidence::Known {
        count: 1,
        locators: Some(vec!["docs/guide.md:7".into()]),
    });
    let spec = resolve_terminal(
        "a.md#A",
        &marker(Stage::Spec, State::Done),
        Some(&req),
        &observed,
    );
    assert_eq!(spec.outcome, TerminalOutcome::Terminal);
    assert_eq!(
        spec.artifacts[0].locators,
        Some(vec!["docs/guide.md:7".into()])
    );
}

#[test]
fn external_keeps_authored_locator_when_observer_is_unavailable() {
    let req = requirements("external");
    let mut status = marker(Stage::Impl, State::Done);
    status.r#ref = Some("receipt:7".into());
    let result = resolve_terminal("a.md#A", &status, Some(&req), &NoEvidence);
    assert_eq!(result.outcome, TerminalOutcome::Pending);
    assert_eq!(result.artifacts[0].state, ArtifactState::Unavailable);
    assert_eq!(result.artifacts[0].locators, Some(vec!["receipt:7".into()]));
}

#[test]
fn external_passes_the_authored_reference_to_its_observer() {
    struct External;
    impl EvidenceProvider for External {
        fn evidence_for(&self, _unit_addr: &str) -> Option<Evidence> {
            None
        }

        fn artifact_evidence_for(
            &self,
            _unit_addr: &str,
            artifact: ArtifactKind,
            reference: Option<&str>,
        ) -> ProviderArtifactEvidence {
            assert_eq!(artifact, ArtifactKind::External);
            assert_eq!(reference, Some("receipt:7"));
            ProviderArtifactEvidence::Known {
                count: 1,
                locators: None,
            }
        }
    }
    let req = requirements("external");
    let mut status = marker(Stage::Impl, State::Done);
    status.r#ref = Some("receipt:7".into());
    let result = resolve_terminal("a.md#A", &status, Some(&req), &External);
    assert_eq!(result.outcome, TerminalOutcome::Terminal);
    assert_eq!(result.artifacts[0].locators, Some(vec!["receipt:7".into()]));
}
