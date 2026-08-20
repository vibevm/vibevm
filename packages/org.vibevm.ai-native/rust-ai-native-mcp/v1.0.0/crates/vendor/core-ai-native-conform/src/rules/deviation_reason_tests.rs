// B-053 — the three Rust deviation rules thread the fact's `reason` text
// onto `FindingStatus::DeviationAcknowledged { reason }`, so an
// acknowledged Rust finding reproduces the author's
// `#[spec(deviates = …, reason = "…")]` in the SARIF `justification` —
// the path TS/Go have always had. The three rules (`UnsafeGate`,
// `NoUnwrapInDomain`, `AmbientEnv`) share one shape: read `reason` off
// the fact, hand it to the status. The tests below prove each route
// threads the reason, and the unsafe one drives the SARIF renderer
// end-to-end (the renderer is generic over the status, proven
// source-agnostic by `acknowledged_finding_renders_with_in_source_
// suppression` in `sarif::tests`).

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use super::*;
use crate::Rule;
use crate::rules;

fn sf(file: &str, crate_name: &str, facts: Vec<Fact>) -> SourceFacts {
    SourceFacts {
        file: file.to_string(),
        crate_name: crate_name.to_string(),
        facts,
    }
}

/// `UnsafeGate` over `UnsafeUse` — the canonical route, driven
/// end-to-end through the SARIF renderer. Three sites in one file:
/// an acknowledged deviation WITH a reason (the text threads to the
/// status and, via the generic renderer, to `justification`), one
/// WITHOUT a reason (the degenerate case — `None`, falling back to the
/// fixed marker, the prior behaviour kept as a regression gate), and a
/// live violation (untouched by the reason plumbing).
#[test]
fn unsafe_deviation_threads_reason_into_status_and_justification() {
    let rule = rules::UnsafeGate {
        audit_crates: vec![],
    };
    let facts = vec![sf(
        "crates/a/src/lib.rs",
        "a",
        vec![
            Fact::UnsafeUse {
                context: "block".into(),
                line: 5,
                in_test: false,
                in_deviation: true,
                reason: Some("FFI boundary, audited".into()),
            },
            Fact::UnsafeUse {
                context: "block".into(),
                line: 9,
                in_test: false,
                in_deviation: true,
                reason: None,
            },
            Fact::UnsafeUse {
                context: "block".into(),
                line: 13,
                in_test: false,
                in_deviation: false,
                reason: None,
            },
        ],
    )];
    let found = rule.check(&facts);
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!(
        found.iter().find(|f| f.line == 5).unwrap().status,
        crate::FindingStatus::DeviationAcknowledged {
            reason: Some("FFI boundary, audited".into())
        }
    );
    // No reason text -> the status is None (the fixed marker is the
    // renderer's job, asserted below).
    assert_eq!(
        found.iter().find(|f| f.line == 9).unwrap().status,
        crate::FindingStatus::DeviationAcknowledged { reason: None }
    );
    // A live violation is untouched by the reason plumbing.
    assert_eq!(
        found.iter().find(|f| f.line == 13).unwrap().status,
        crate::FindingStatus::Live
    );
    // End-to-end: the generic renderer carries a present reason straight
    // to `justification`, and falls back to the fixed marker verbatim
    // when the reason is absent.
    let report = crate::sarif::render(&[&rule], &found);
    assert!(report.contains("\"justification\": \"FFI boundary, audited\""));
    assert!(report.contains("acknowledged in-source deviation"));
}

/// `NoUnwrapInDomain` over `UnwrapUse` — a separate rule method, the
/// SAME thread. Asserted at the rule layer; the SARIF render is generic
/// and proven end-to-end by the unsafe test above.
#[test]
fn unwrap_deviation_threads_reason_into_status() {
    let rule = rules::NoUnwrapInDomain {
        gated_crates: vec!["x".into()],
    };
    let facts = vec![sf(
        "crates/x/src/m.rs",
        "x",
        vec![
            Fact::UnwrapUse {
                method: "unwrap".into(),
                line: 5,
                in_test: false,
                in_deviation: true,
                reason: Some("infallible post-condition".into()),
            },
            Fact::UnwrapUse {
                method: "expect".into(),
                line: 9,
                in_test: false,
                in_deviation: true,
                reason: None,
            },
            Fact::UnwrapUse {
                method: "unwrap".into(),
                line: 13,
                in_test: false,
                in_deviation: false,
                reason: None,
            },
        ],
    )];
    let found = rule.check(&facts);
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!(
        found.iter().find(|f| f.line == 5).unwrap().status,
        crate::FindingStatus::DeviationAcknowledged {
            reason: Some("infallible post-condition".into())
        }
    );
    assert_eq!(
        found.iter().find(|f| f.line == 9).unwrap().status,
        crate::FindingStatus::DeviationAcknowledged { reason: None }
    );
    assert_eq!(
        found.iter().find(|f| f.line == 13).unwrap().status,
        crate::FindingStatus::Live
    );
}

/// `AmbientEnv` over `EnvRead` — the third route, the same thread.
#[test]
fn env_deviation_threads_reason_into_status() {
    let rule = rules::AmbientEnv {
        gated_crates: vec!["x".into()],
        audit_crates: vec![],
        roots: vec![],
    };
    let facts = vec![sf(
        "crates/x/src/deep.rs",
        "x",
        vec![
            Fact::EnvRead {
                method: "var".into(),
                line: 5,
                in_test: false,
                in_deviation: true,
                reason: Some("resolved once at a shim".into()),
            },
            Fact::EnvRead {
                method: "var_os".into(),
                line: 9,
                in_test: false,
                in_deviation: true,
                reason: None,
            },
            Fact::EnvRead {
                method: "var".into(),
                line: 13,
                in_test: false,
                in_deviation: false,
                reason: None,
            },
        ],
    )];
    let found = rule.check(&facts);
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!(
        found.iter().find(|f| f.line == 5).unwrap().status,
        crate::FindingStatus::DeviationAcknowledged {
            reason: Some("resolved once at a shim".into())
        }
    );
    assert_eq!(
        found.iter().find(|f| f.line == 9).unwrap().status,
        crate::FindingStatus::DeviationAcknowledged { reason: None }
    );
    assert_eq!(
        found.iter().find(|f| f.line == 13).unwrap().status,
        crate::FindingStatus::Live
    );
}
