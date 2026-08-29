//! The behaviour cell's own laws, exercised against the same corpus the
//! wire-corpus integration test walks (`crates/vibe-wire/tests/
//! extensions_analyze_wire_corpus.rs`) — the cell half proves each typed
//! refusal by family, the integration half proves the corpus documents
//! route through the generated reader and this validator exactly as the
//! corpus docs promise.

use crate::behaviour::extensions_analyze::{
    COMMAND, ExtensionsAnalyzeError, REPORT_EPOCH, spell_bytes, validate,
};
use crate::generated::extensions_analyze::ExtensionsAnalyze;

/// A report with every law satisfied — the base every refusal below
/// breaks exactly one member of.
fn valid_report() -> ExtensionsAnalyze {
    let doc = serde_json::json!({
        "schema": 1,
        "command": "extensions-analyze",
        "artifacts": [
            {
                "lane": { "kind": "node", "node_rel": "." },
                "artifact_id": "static-md",
                "target": "static-md",
                "total_emitted_bytes": "1000",
                "occurrence_count": 2,
                "frame_overhead_bytes": "400",
                "contributions": [
                    {
                        "provider": { "kind": "dependency", "group": "org.vibevm.core", "name": "vibe" },
                        "kind": "normal",
                        "origin": "org.vibevm.core/vibe",
                        "path": "vibevm/vibedeps/org.vibevm.core/vibe/1.0.0/boot/10-flow.md",
                        "bytes": "500",
                        "occurrences": 1
                    },
                    {
                        "provider": { "kind": "host-virtual-workspace" },
                        "kind": "simple",
                        "origin": ".",
                        "path": "vibevm/vibespecs/boot/90-user.md",
                        "bytes": "100",
                        "occurrences": 1
                    }
                ],
                "deltas": [
                    {
                        "pass": "transform:emitted:org.vibevm/vibe#xml-minify",
                        "stage": "emitted",
                        "lane_byte_delta": null,
                        "artifact_byte_delta": { "before": "1040", "after": "1000" }
                    }
                ],
                "token_estimate": null,
                "estimator_id": null
            }
        ]
    });
    serde_json::from_value(doc).expect("the base report parses")
}

#[test]
fn the_base_report_validates() {
    validate(&valid_report()).expect("every law is satisfied");
}

#[test]
fn a_wrong_epoch_refuses_typed() {
    let mut report = valid_report();
    report.schema = REPORT_EPOCH + 1;
    assert_eq!(
        validate(&report),
        Err(ExtensionsAnalyzeError::SchemaEpoch {
            found: REPORT_EPOCH + 1,
        })
    );
}

#[test]
fn a_foreign_command_identity_refuses() {
    let mut report = valid_report();
    report.command = "extensions".to_string();
    assert!(matches!(
        validate(&report),
        Err(ExtensionsAnalyzeError::CommandIdentity { .. })
    ));
}

#[test]
fn an_artifact_id_its_target_disclaims_refuses() {
    let mut report = valid_report();
    report.artifacts[0].artifact_id = "static-xml".to_string();
    assert!(matches!(
        validate(&report),
        Err(ExtensionsAnalyzeError::ArtifactTargetMismatch { .. })
    ));
}

#[test]
fn non_canonical_byte_counts_refuse_naming_the_member() {
    for spelling in ["", "00", "01", "1_000", "+12", " 12", "12 ", "0x10", "１２"] {
        let mut report = valid_report();
        report.artifacts[0].total_emitted_bytes = spelling.to_string();
        let error = validate(&report)
            .err()
            .unwrap_or_else(|| panic!("{spelling:?} must refuse as non-canonical"));
        assert!(
            matches!(error, ExtensionsAnalyzeError::ByteCountNotCanonical { .. }),
            "{spelling:?} refused as {error}"
        );
    }
    // The one zero spelling that IS canonical: a contribution whose
    // material occupies nothing still reconciles, chain included.
    let mut report = valid_report();
    report.artifacts[0].total_emitted_bytes = "600".to_string();
    report.artifacts[0].frame_overhead_bytes = "100".to_string();
    report.artifacts[0].contributions[0].bytes = "500".to_string();
    report.artifacts[0].contributions[1].bytes = "0".to_string();
    let pair = report.artifacts[0].deltas[0]
        .artifact_byte_delta
        .as_mut()
        .expect("the base row carries the artifact pair");
    pair.before = "640".to_string();
    pair.after = "600".to_string();
    validate(&report).expect("a canonical `0` contribution is lawful");
}

#[test]
fn spell_bytes_is_the_canonical_spelling() {
    assert_eq!(spell_bytes(0), "0");
    assert_eq!(spell_bytes(7), "7");
    assert_eq!(spell_bytes(1 << 40), (1u128 << 40).to_string());
    // And the producer half and the reader half agree: everything the
    // helper spells, the law accepts.
    for count in [0u128, 1, 9, 10, 4_294_967_296, u64::MAX as u128] {
        let spelled = spell_bytes(count);
        assert!(
            crate::behaviour::scalars::is_canonical_decimal(&spelled),
            "{spelled} must be canonical"
        );
    }
}

#[test]
fn contributions_plus_frame_must_equal_the_total() {
    let mut report = valid_report();
    // Fold 40 frame bytes into the largest contribution: the row keeps
    // every scalar law and only the reconciliation breaks.
    report.artifacts[0].contributions[0].bytes = "540".to_string();
    assert!(matches!(
        validate(&report),
        Err(ExtensionsAnalyzeError::TotalsDoNotReconcile { .. })
    ));
    // The honest repair — attributing the same total with a smaller
    // frame — validates.
    report.artifacts[0].frame_overhead_bytes = "360".to_string();
    validate(&report).expect("the reconciled spelling validates");
}

#[test]
fn the_occurrence_count_is_the_contribution_sum() {
    let mut report = valid_report();
    report.artifacts[0].occurrence_count = 3;
    assert!(matches!(
        validate(&report),
        Err(ExtensionsAnalyzeError::OccurrenceCountMismatch { .. })
    ));
}

#[test]
fn the_occurrence_grammar_follows_the_kind() {
    let mut report = valid_report();
    report.artifacts[0].contributions[1].kind =
        crate::generated::extensions_analyze::ContributionKind::Elided;
    // The simple row became elided but still claims one occurrence.
    assert!(matches!(
        validate(&report),
        Err(ExtensionsAnalyzeError::OccurrenceGrammar { .. })
    ));
}

#[test]
fn a_lane_row_may_not_carry_the_artifact_member_and_vice_versa() {
    // Conflation both ways: a lane row carrying the artifact pair, and
    // the mirror image.
    let mut conflation = valid_report();
    let artifact = &mut conflation.artifacts[0];
    artifact.deltas[0].stage = crate::generated::extensions_analyze::Stage::Lane;
    assert!(matches!(
        validate(&conflation),
        Err(ExtensionsAnalyzeError::StageMemberMismatch { .. })
    ));

    let mut absence = valid_report();
    let artifact = &mut absence.artifacts[0];
    artifact.deltas[0].artifact_byte_delta = None;
    assert!(matches!(
        validate(&absence),
        Err(ExtensionsAnalyzeError::StageMemberMismatch { .. })
    ));
}

#[test]
fn the_emitted_chain_must_be_continuous_and_reach_the_total() {
    // A FIRST emitted row's `before` is the backend's raw output and is
    // unanchored — the discontinuity a chain can see needs a second row.
    let mut chained = valid_report();
    let artifact = &mut chained.artifacts[0];
    artifact.deltas[0]
        .artifact_byte_delta
        .as_mut()
        .expect("the base row carries the artifact pair")
        .after = "1020".to_string();
    artifact
        .deltas
        .push(crate::generated::extensions_analyze::DeltaRow {
            pass: "transform:emitted:org.vibevm/vibe#footer".to_string(),
            stage: crate::generated::extensions_analyze::Stage::Emitted,
            lane_byte_delta: None,
            artifact_byte_delta: Some(crate::generated::extensions_analyze::BytePair {
                before: "1020".to_string(),
                after: "1000".to_string(),
            }),
        });
    validate(&chained).expect("the continuous chain validates");

    // Break the join: the second row begins somewhere the first did not
    // leave the artifact.
    let mut broken = chained.clone();
    broken.artifacts[0].deltas[1]
        .artifact_byte_delta
        .as_mut()
        .expect("the second row carries the artifact pair")
        .before = "999".to_string();
    assert!(matches!(
        validate(&broken),
        Err(ExtensionsAnalyzeError::DeltaChainBroken { .. })
    ));

    let mut short = valid_report();
    short.artifacts[0].deltas[0]
        .artifact_byte_delta
        .as_mut()
        .expect("the base row carries the artifact pair")
        .after = "999".to_string();
    assert!(matches!(
        validate(&short),
        Err(ExtensionsAnalyzeError::DeltaChainDoesNotReachTotal { .. })
    ));
}

#[test]
fn an_estimate_without_an_estimator_refuses_and_vice_versa() {
    let mut orphan = valid_report();
    orphan.artifacts[0].token_estimate = Some(12_345);
    assert!(matches!(
        validate(&orphan),
        Err(ExtensionsAnalyzeError::EstimatorCoupling {
            estimate_is_some: true,
            ..
        })
    ));

    let mut headless = valid_report();
    headless.artifacts[0].estimator_id = Some("tiktoken-v1".to_string());
    assert!(matches!(
        validate(&headless),
        Err(ExtensionsAnalyzeError::EstimatorCoupling {
            estimate_is_some: false,
            ..
        })
    ));

    // Together they are lawful — the format carries an estimate the day
    // a named estimator produces it.
    let mut pair = orphan;
    pair.artifacts[0].estimator_id = Some("tiktoken-v1".to_string());
    validate(&pair).expect("an estimate beside its estimator validates");
}

#[test]
fn unsafe_scalars_and_backslashed_paths_refuse_naming_the_member() {
    let mut blank = valid_report();
    blank.artifacts[0].contributions[0].origin = "  ".to_string();
    assert!(matches!(
        validate(&blank),
        Err(ExtensionsAnalyzeError::UnsafeScalar { member, .. })
            if member == "contributions[0].origin"
    ));

    let mut backslashed = valid_report();
    backslashed.artifacts[0].contributions[0].path = "vibevm\\boot.md".to_string();
    assert!(matches!(
        validate(&backslashed),
        Err(ExtensionsAnalyzeError::BackslashedPath { member, .. })
            if member == "contributions[0].path"
    ));

    let mut node = valid_report();
    node.artifacts[0].lane = crate::generated::extensions_analyze::LaneIdentity::Node(Box::new(
        crate::generated::extensions_analyze::LaneIdentityNode {
            node_rel: "member\\node".to_string(),
        },
    ));
    assert!(matches!(
        validate(&node),
        Err(ExtensionsAnalyzeError::BackslashedPath { member, .. })
            if member == "lane.node.node_rel"
    ));
}

#[test]
fn the_empty_report_is_lawful_and_the_command_is_pinned() {
    let report: ExtensionsAnalyze = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "command": COMMAND,
        "artifacts": []
    }))
    .expect("the empty report parses");
    validate(&report).expect("a node with no static lane analyzes to an empty list");
    assert_eq!(COMMAND, "extensions-analyze");
}
