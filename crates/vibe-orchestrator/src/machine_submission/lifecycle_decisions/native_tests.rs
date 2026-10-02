//! Real filesystem oracles; compile here, runtime acceptance belongs to parent.
use super::*;
use std::fs;
const MANIFEST: &str = "[project]\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'once'\npoint = 'phase:validate'\nhandler = { kind = 'script', base = 'run' }\n";
struct Fixture {
    dir: tempfile::TempDir,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    os: TargetOs,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), MANIFEST).unwrap();
        let os = TargetOs::current().expect("native OS");
        let (name, script) = if os == TargetOs::Windows {
            ("run.ps1", "Add-Content counter 'effect'\n")
        } else {
            ("run.sh", "#!/bin/sh\nprintf 'effect\\n' >> counter\n")
        };
        fs::write(dir.path().join(name), script).unwrap();
        let scope = SubmissionScope {
            context: "native-decisions".into(),
            target: "root".into(),
            principal: "host".into(),
        };
        let namespace = {
            let lease = vibe_lifecycle::LifecycleLease::acquire(dir.path()).unwrap();
            SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap()
        };
        Self {
            dir,
            namespace,
            scope,
            os,
        }
    }
    fn host(&self, facts: HostPolicyFacts) -> FixedLifecycleDecisionHost {
        FixedLifecycleDecisionHost::from_host(
            self.dir.path().into(),
            self.namespace.clone(),
            self.scope.clone(),
            self.os,
            facts,
        )
        .unwrap()
    }
    fn script(&self) -> PathBuf {
        self.dir.path().join(if self.os == TargetOs::Windows {
            "run.ps1"
        } else {
            "run.sh"
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // RootScriptNeverInvoked across every native fixture, including refusals.
        assert!(
            !self.dir.path().join("counter").exists(),
            "provider counter must be absent"
        );
        assert!(!self.dir.path().join(".vibe/lifecycle.toml").exists());
        assert!(!self.dir.path().join(".vibe/lifecycle-outcomes").exists());
        assert!(!self.dir.path().join(".vibe/tmp").exists());
        let index: serde_json::Value = serde_json::from_slice(
            &fs::read(self.dir.path().join(".vibe/machine-submissions.json")).unwrap(),
        )
        .unwrap();
        assert!(
            index["namespaces"]
                .as_object()
                .unwrap()
                .values()
                .all(|n| n["keys"].as_object().unwrap().is_empty())
        );
    }
}
fn facts() -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: true,
        allow_detach: true,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: true,
    }
}
fn options(control: RequestedOptionsControl) -> RequestedOptions {
    RequestedOptions {
        control,
        force: false,
        trace: false,
        deploy_profile: None,
    }
}
fn selection(q: &InputDecisionMessage, id: &str) -> InputDecisionMessage {
    let mut a = q.clone();
    a.body = Body::SelectionAnswer(Box::new(BodySelectionAnswer {
        choice_ids: vec![id.into()],
    }));
    a
}
fn approval(q: &InputDecisionMessage, approved: bool) -> InputDecisionMessage {
    let mut a = q.clone();
    a.body = Body::ApprovalAnswer(Box::new(BodyApprovalAnswer { approved }));
    a
}
fn answer(
    host: &FixedLifecycleDecisionHost,
    a: &InputDecisionMessage,
) -> Result<DecisionOutcome, FixedDecisionError> {
    host.answer(&serde_json::to_vec(a).unwrap())
}
fn next(outcome: DecisionOutcome) -> InputDecisionMessage {
    match outcome {
        DecisionOutcome::Question(q) => q,
        _ => panic!("next question"),
    }
}
fn evidence(outcome: DecisionOutcome) -> ValidatedFixedPlanEvidence {
    match outcome {
        DecisionOutcome::Validated(e) => e,
        _ => panic!("opaque evidence"),
    }
}
fn refused(result: Result<DecisionOutcome, FixedDecisionError>, expected: FixedDecisionError) {
    assert!(matches!(result, Err(e) if e == expected));
}

#[test]
fn lifecycle_decisions_native_actual_selection_resulting_approval_repeats_and_conflicts() {
    let f = Fixture::new();
    let h = f.host(facts());
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::None))
        .unwrap();
    assert!(matches!(q.body, Body::SelectionQuestion(_)));
    assert_eq!(
        q.operation_id.as_deref(),
        Some("rust:vibe-lifecycle:validate")
    );
    assert_eq!(q.domain_owner.as_deref(), Some("rust:vibe-lifecycle"));
    let a = selection(&q, "cancel_deferred_boundary");
    let next_q = next(answer(&h, &a).unwrap());
    assert_ne!(next_q.plan_id, q.plan_id);
    assert_eq!(next_q.source_id, q.source_id);
    assert_eq!(
        next_q.requested_options.control,
        RequestedOptionsControl::CancelDeferredBoundary
    );
    assert_eq!(next(answer(&h, &a).unwrap()), next_q);
    refused(
        answer(&h, &selection(&q, "detach")),
        FixedDecisionError::AnswerConflict,
    );
    let e = evidence(answer(&h, &approval(&next_q, true)).unwrap());
    assert_eq!(Some(e.identity()), next_q.plan_id.as_deref());
    assert_eq!(e.options(), &next_q.requested_options);
    assert_eq!(
        evidence(answer(&h, &approval(&next_q, true)).unwrap()).identity(),
        e.identity()
    );
    refused(
        answer(&h, &approval(&next_q, false)),
        FixedDecisionError::AnswerConflict,
    );
    let other = h
        .prepare_requested_options(options(RequestedOptionsControl::CancelDeferredBoundary))
        .unwrap();
    assert_eq!(
        other.plan_id, next_q.plan_id,
        "slot nonce is not semantic plan identity"
    );
}
#[test]
fn lifecycle_decisions_native_no_control_is_a_real_selection_and_not_approval() {
    let f = Fixture::new();
    let h = f.host(facts());
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::None))
        .unwrap();
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::InvalidData,
    );
    let q = next(answer(&h, &selection(&q, "none")).unwrap());
    assert_eq!(q.requested_options.control, RequestedOptionsControl::None);
    evidence(answer(&h, &approval(&q, true)).unwrap());
}
#[test]
fn lifecycle_decisions_native_current_policy_denied_boolean_and_revisions() {
    let f = Fixture::new();
    let mut denied = facts();
    denied.approve_captured_script = false;
    let h = f.host(denied.clone());
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::Detach))
        .unwrap();
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::PolicyDenied,
    );
    h.set_host_policy(facts()).unwrap();
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::StalePolicy,
    );
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::Detach))
        .unwrap();
    h.set_host_policy(denied).unwrap();
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::StalePolicy,
    );
}
#[test]
fn lifecycle_decisions_native_false_and_changed_approval_conflict() {
    let f = Fixture::new();
    let h = f.host(facts());
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::Detach))
        .unwrap();
    refused(
        answer(&h, &approval(&q, false)),
        FixedDecisionError::PolicyDenied,
    );
    refused(
        answer(&h, &approval(&q, false)),
        FixedDecisionError::PolicyDenied,
    );
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::AnswerConflict,
    );
}
#[test]
fn lifecycle_decisions_native_foreign_bindings_options_shapes_and_unknown_slot() {
    let f = Fixture::new();
    let h = f.host(facts());
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::Detach))
        .unwrap();
    for field in 0..11 {
        let mut a = approval(&q, true);
        let expected = match field {
            0 => {
                a.context_id = Some("foreign".into());
                FixedDecisionError::ForeignScope
            }
            1 => {
                a.target_id = Some("foreign".into());
                FixedDecisionError::ForeignScope
            }
            2 => {
                a.principal_id = Some("foreign".into());
                FixedDecisionError::ForeignScope
            }
            3 => {
                a.plan_id = Some("foreign".into());
                FixedDecisionError::InvalidData
            }
            4 => {
                a.source_id = Some("foreign".into());
                FixedDecisionError::InvalidData
            }
            5 => {
                a.policy_generation = Some("foreign".into());
                FixedDecisionError::StalePolicy
            }
            6 => {
                a.question_id = Some("foreign".into());
                FixedDecisionError::UnknownQuestion
            }
            7 => {
                a.requested_options.control = RequestedOptionsControl::None;
                FixedDecisionError::InvalidData
            }
            8 => {
                a.operation_id = Some("foreign".into());
                FixedDecisionError::InvalidData
            }
            9 => {
                a.domain_owner = Some("foreign".into());
                FixedDecisionError::InvalidData
            }
            _ => {
                a.schema = 2;
                FixedDecisionError::InvalidData
            }
        };
        refused(answer(&h, &a), expected);
    }
    refused(
        answer(&h, &selection(&q, "detach")),
        FixedDecisionError::InvalidData,
    );
    let mut prose = q.clone();
    prose.body = Body::TextAnswer(Box::new(BodyTextAnswer {
        value: "assume yes".into(),
    }));
    refused(answer(&h, &prose), FixedDecisionError::InvalidData);
    refused(
        h.answer(&vec![b' '; MAX_FRAME_BYTES + 1]),
        FixedDecisionError::InvalidData,
    );
    refused(
        h.answer(b"{\"approved\":true}"),
        FixedDecisionError::InvalidData,
    );
}
#[test]
fn lifecycle_decisions_native_restart_issuer_and_foreign_namespace_scope() {
    let f = Fixture::new();
    let h = f.host(facts());
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::Detach))
        .unwrap();
    let restart = f.host(facts());
    refused(
        answer(&restart, &approval(&q, true)),
        FixedDecisionError::StalePolicy,
    );
    let mut scope = f.scope.clone();
    scope.principal = "foreign".into();
    assert!(matches!(
        FixedLifecycleDecisionHost::from_host(
            f.dir.path().into(),
            f.namespace.clone(),
            scope,
            f.os,
            facts()
        ),
        Err(FixedDecisionError::ForeignScope)
    ));
}
#[test]
fn lifecycle_decisions_native_manifest_lock_script_drift_and_fresh_hashed_basis() {
    for kind in 0..3 {
        let f = Fixture::new();
        let h = f.host(facts());
        let q = h
            .prepare_requested_options(options(RequestedOptionsControl::Detach))
            .unwrap();
        match kind {
            0 => fs::write(
                f.dir.path().join("vibe.toml"),
                MANIFEST.replace("demo", "changed"),
            )
            .unwrap(),
            1 => fs::write(f.dir.path().join("vibe.lock"), "invalid changed lock").unwrap(),
            _ => {
                let mut script = fs::read(f.script()).unwrap();
                script.extend_from_slice(b"\n# changed\n");
                fs::write(f.script(), script).unwrap();
            }
        }
        let Err(FixedDecisionError::SourceDrift(reason)) = answer(&h, &approval(&q, true)) else {
            panic!("changed manifest/lock/script must report SourceDrift");
        };
        assert!(
            !reason.is_empty(),
            "source drift must retain its diagnostic"
        );
        if kind == 2 {
            assert!(reason.contains("original plan=") && reason.contains("current plan="));
            assert!(reason.contains(q.source_id.as_deref().unwrap()));
        } else {
            assert!(reason.contains("fresh source preflight failed:"));
            assert!(reason.contains(if kind == 0 { "manifest" } else { "lock" }));
        }
    }
    let f = Fixture::new();
    let h = f.host(facts());
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::Detach))
        .unwrap();
    let bytes = fs::read(f.script()).unwrap();
    fs::write(f.script(), bytes).unwrap();
    evidence(answer(&h, &approval(&q, true)).unwrap());
}
#[test]
fn lifecycle_decisions_native_physical_root_replacement_refuses_same_labels() {
    let f = Fixture::new();
    let h = f.host(facts());
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::Detach))
        .unwrap();
    let moved = f.dir.path().with_extension("decision-moved");
    fs::rename(f.dir.path(), &moved).unwrap();
    fs::create_dir(f.dir.path()).unwrap();
    fs::copy(moved.join("vibe.toml"), f.dir.path().join("vibe.toml")).unwrap();
    fs::copy(
        moved.join(if f.os == TargetOs::Windows {
            "run.ps1"
        } else {
            "run.sh"
        }),
        f.script(),
    )
    .unwrap();
    let result = answer(&h, &approval(&q, true));
    fs::remove_dir_all(f.dir.path()).unwrap();
    fs::rename(&moved, f.dir.path()).unwrap();
    refused(result, FixedDecisionError::ForeignScope);
}
#[test]
fn lifecycle_decisions_native_gaps_before_io_and_host_control_permissions() {
    let f = Fixture::new();
    let mut policy = facts();
    policy.allow_detach = false;
    policy.allow_cancel_pre_dispatch = false;
    policy.allow_cancel_deferred_boundary = false;
    let h = f.host(policy);
    let q = h
        .prepare_requested_options(options(RequestedOptionsControl::None))
        .unwrap();
    let Body::SelectionQuestion(body) = &q.body else {
        panic!("selection")
    };
    assert_eq!(body.choices.len(), 1);
    refused(
        answer(&h, &selection(&q, "detach")),
        FixedDecisionError::InvalidData,
    );
    for c in [
        RequestedOptionsControl::Detach,
        RequestedOptionsControl::CancelPreDispatch,
        RequestedOptionsControl::CancelDeferredBoundary,
    ] {
        assert!(matches!(
            h.prepare_requested_options(options(c)),
            Err(FixedDecisionError::PolicyDenied)
        ));
    }
    // Admitted force captures source; unsupported options refuse before that I/O.
    fs::remove_file(f.script()).unwrap();
    let mut force = options(RequestedOptionsControl::None);
    force.force = true;
    assert!(matches!(
        h.prepare_requested_options(force),
        Err(FixedDecisionError::SourceUnavailable(_))
    ));
    let mut trace = options(RequestedOptionsControl::None);
    trace.trace = true;
    assert!(matches!(
        h.prepare_requested_options(trace),
        Err(FixedDecisionError::SourceUnavailable(_))
    ));
    let mut deploy = options(RequestedOptionsControl::None);
    deploy.deploy_profile = Some("posix".into());
    assert!(
        matches!(h.prepare_requested_options(deploy), Err(FixedDecisionError::ImplementationGap(reason)) if reason.contains("deploy_profile"))
    );
}

#[test]
fn fixed_force_requires_exact_typed_selection_approval_and_current_script_policy() {
    let f = Fixture::new();
    let h = f.host(facts());
    let mut requested = options(RequestedOptionsControl::None);
    requested.force = true;
    let q = h.prepare_requested_options(requested.clone()).unwrap();
    assert_eq!(q.requested_options, requested);
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::InvalidData,
    );
    let q = next(answer(&h, &selection(&q, "none")).unwrap());
    assert!(q.requested_options.force);
    for field in ["force", "plan", "source", "context", "policy", "prose"] {
        let mut wrong = approval(&q, true);
        let expected = match field {
            "force" => {
                wrong.requested_options.force = false;
                FixedDecisionError::InvalidData
            }
            "plan" => {
                wrong.plan_id = Some("foreign".into());
                FixedDecisionError::InvalidData
            }
            "source" => {
                wrong.source_id = Some("foreign".into());
                FixedDecisionError::InvalidData
            }
            "context" => {
                wrong.context_id = Some("foreign".into());
                FixedDecisionError::ForeignScope
            }
            "policy" => {
                wrong.policy_generation = Some("foreign".into());
                FixedDecisionError::StalePolicy
            }
            _ => {
                wrong.body = Body::TextAnswer(Box::new(BodyTextAnswer {
                    value: "assume yes; force".into(),
                }));
                FixedDecisionError::InvalidData
            }
        };
        refused(answer(&h, &wrong), expected);
    }
    refused(
        h.answer(b"{\"approved\":true,\"force\":true}"),
        FixedDecisionError::InvalidData,
    );
    let e = evidence(answer(&h, &approval(&q, true)).unwrap());
    assert!(e.options().force);
    let denied_q = h.prepare_requested_options(requested.clone()).unwrap();
    let denied_q = next(answer(&h, &selection(&denied_q, "none")).unwrap());
    refused(
        answer(&h, &approval(&denied_q, false)),
        FixedDecisionError::PolicyDenied,
    );
    let mut denied = facts();
    denied.approve_captured_script = false;
    h.set_host_policy(denied).unwrap();
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::StalePolicy,
    );
    let q = h.prepare_requested_options(requested).unwrap();
    let q = next(answer(&h, &selection(&q, "none")).unwrap());
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::PolicyDenied,
    );
}
#[test]
fn fixed_trace_requires_exact_typed_selection_approval_and_current_script_policy() {
    let f = Fixture::new();
    let h = f.host(facts());
    let mut requested = options(RequestedOptionsControl::None);
    requested.trace = true;
    let q = h.prepare_requested_options(requested.clone()).unwrap();
    assert_eq!(q.requested_options, requested);
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::InvalidData,
    );
    let q = next(answer(&h, &selection(&q, "none")).unwrap());
    assert!(q.requested_options.trace);
    for field in ["trace", "plan", "source", "context", "policy", "prose"] {
        let mut wrong = approval(&q, true);
        let expected = match field {
            "trace" => {
                wrong.requested_options.trace = false;
                FixedDecisionError::InvalidData
            }
            "plan" => {
                wrong.plan_id = Some("foreign".into());
                FixedDecisionError::InvalidData
            }
            "source" => {
                wrong.source_id = Some("foreign".into());
                FixedDecisionError::InvalidData
            }
            "context" => {
                wrong.context_id = Some("foreign".into());
                FixedDecisionError::ForeignScope
            }
            "policy" => {
                wrong.policy_generation = Some("foreign".into());
                FixedDecisionError::StalePolicy
            }
            _ => {
                wrong.body = Body::TextAnswer(Box::new(BodyTextAnswer {
                    value: "assume yes; trace".into(),
                }));
                FixedDecisionError::InvalidData
            }
        };
        refused(answer(&h, &wrong), expected);
    }
    refused(
        h.answer(b"{\"approved\":true,\"trace\":true}"),
        FixedDecisionError::InvalidData,
    );
    let e = evidence(answer(&h, &approval(&q, true)).unwrap());
    assert!(e.options().trace);
    let denied_q = h.prepare_requested_options(requested.clone()).unwrap();
    let denied_q = next(answer(&h, &selection(&denied_q, "none")).unwrap());
    refused(
        answer(&h, &approval(&denied_q, false)),
        FixedDecisionError::PolicyDenied,
    );
    let mut denied = facts();
    denied.approve_captured_script = false;
    h.set_host_policy(denied).unwrap();
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::StalePolicy,
    );
    let q = h.prepare_requested_options(requested).unwrap();
    let q = next(answer(&h, &selection(&q, "none")).unwrap());
    refused(
        answer(&h, &approval(&q, true)),
        FixedDecisionError::PolicyDenied,
    );
}
#[test]
fn lifecycle_decisions_native_four_sequences_pair_counts_once_without_eviction() {
    let f = Fixture::new();
    let h = f.host(facts());
    let mut first = None;
    for _ in 0..MAX_SEQUENCES {
        let q = h
            .prepare_requested_options(options(RequestedOptionsControl::None))
            .unwrap();
        let a = selection(&q, "none");
        let next_q = next(answer(&h, &a).unwrap());
        first.get_or_insert((a, next_q));
    }
    assert!(matches!(
        h.prepare_requested_options(options(RequestedOptionsControl::None)),
        Err(FixedDecisionError::Capacity)
    ));
    let (a, q) = first.unwrap();
    assert_eq!(next(answer(&h, &a).unwrap()), q);
    evidence(answer(&h, &approval(&q, true)).unwrap());
}

#[test]
fn lifecycle_decisions_native_explicit_controls_have_exact_resulting_approval() {
    let f = Fixture::new();
    let h = f.host(facts());
    let mut identities = std::collections::BTreeSet::new();
    for control in [
        RequestedOptionsControl::Detach,
        RequestedOptionsControl::CancelPreDispatch,
        RequestedOptionsControl::CancelDeferredBoundary,
    ] {
        let requested = options(control);
        let q = h.prepare_requested_options(requested.clone()).unwrap();
        assert!(matches!(q.body, Body::ApprovalQuestion(_)));
        assert!(identities.insert(q.plan_id.clone()));
        let e = evidence(answer(&h, &approval(&q, true)).unwrap());
        assert_eq!(e.options(), &requested);
    }
}

#[test]
fn lifecycle_decisions_native_initial_invalid_manifest_retains_diagnostic() {
    let f = Fixture::new();
    let h = f.host(facts());
    fs::write(
        f.dir.path().join("vibe.toml"),
        "[project\nname = 'broken'\n",
    )
    .unwrap();
    let Err(FixedDecisionError::SourceUnavailable(reason)) =
        h.prepare_requested_options(options(RequestedOptionsControl::Detach))
    else {
        panic!("initial invalid manifest must report SourceUnavailable, not drift");
    };
    assert!(reason.contains("manifest"), "owning diagnostic: {reason}");
    assert!(!reason.is_empty());
    let mut forced = options(RequestedOptionsControl::None);
    forced.force = true;
    assert!(matches!(
        h.prepare_requested_options(forced),
        Err(FixedDecisionError::SourceUnavailable(reason)) if reason.contains("manifest")
    ));
}
