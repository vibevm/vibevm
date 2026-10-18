use super::preparation::*;
fn plan() -> PreparationPlan {
    PreparationPlan {
        context: "ctx".into(),
        capture: "capture".into(),
        cache: "/preview".into(),
        stages: vec![
            PreparationStage {
                kind: StageKind::Fetch,
                input: "source@hash".into(),
                effects: vec!["cache insertion".into()],
            },
            PreparationStage {
                kind: StageKind::Native,
                input: "owner@epoch".into(),
                effects: vec!["scratch compile".into()],
            },
        ],
    }
}
#[test]
fn partial_evidence_retains_exact_bytes_and_failure() {
    let p = plan();
    let o = PreparationOutcome {
        plan: p.clone(),
        stages: vec![
            StageOutcome::Captured {
                evidence: StageEvidence {
                    files: [("boot.md".into(), b"retained".to_vec())].into(),
                },
            },
            StageOutcome::Failed {
                reason: "revoked native provider".into(),
            },
        ],
        terminal: true,
    };
    o.validate(&p).unwrap();
    assert!(!o.complete());
    let bytes = serde_json::to_vec(&o).unwrap();
    assert_eq!(
        o,
        serde_json::from_slice::<PreparationOutcome>(&bytes).unwrap()
    );
}
#[test]
fn foreign_or_fabricated_completion_refuses() {
    let p = plan();
    let mut o = PreparationOutcome {
        plan: p.clone(),
        stages: vec![],
        terminal: true,
    };
    assert!(o.validate(&p).is_err());
    o.terminal = false;
    o.validate(&p).unwrap();
    assert!(!o.complete());
    o.plan.capture = "foreign".into();
    assert!(o.validate(&p).is_err());
}
#[test]
fn no_evidence_may_follow_a_failed_stage() {
    let p = plan();
    let o = PreparationOutcome {
        plan: p.clone(),
        stages: vec![
            StageOutcome::Failed {
                reason: "fetch failed".into(),
            },
            StageOutcome::Captured {
                evidence: StageEvidence {
                    files: Default::default(),
                },
            },
        ],
        terminal: true,
    };
    assert!(o.validate(&p).is_err());
}
