use super::*;

#[test]
fn strict_outer_and_nested_refusals_precede_copy() {
    let f = Fixture::new();
    let route = f.route();
    let good: Value = serde_json::from_slice(&frame("submit", "copy")).unwrap();
    let mut cases = vec![json!({}), json!(null)];
    for (field, value) in [
        ("schema", json!(2)),
        ("operation_id", json!("foreign")),
        ("domain_owner", json!("foreign")),
        ("source_id", json!("caller")),
        ("evidence_id", json!("caller")),
        ("unknown", json!(true)),
    ] {
        let mut v = good.clone();
        v[field] = value;
        cases.push(v);
    }
    for field in ["operation_id", "domain_owner", "source_id", "evidence_id"] {
        let mut v = good.clone();
        v.as_object_mut().unwrap().remove(field);
        cases.push(v);
    }
    for (field, value) in [
        ("stage", json!("prepared")),
        ("format_id", json!("machine-lifecycle-action-request")),
        ("format_id", json!(null)),
        ("payload_json", json!(null)),
        ("extra", json!(true)),
    ] {
        let mut v = good.clone();
        v["body"][field] = value;
        cases.push(v);
    }
    for request in [
        json!({"schema":2,"action":{"kind":"submit","key":"copy"}}),
        json!({"schema":1,"action":{"kind":"execute","key":"copy"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"copy","source":"source.bin"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"copy"},"timestamp":"now"}),
        json!({"schema":1,"action":{"kind":"submit","key":null}}),
        json!({"schema":1,"action":{"kind":"submit"}}),
    ] {
        let mut v = good.clone();
        v["body"]["payload_json"] = json!(request.to_string());
        cases.push(v);
    }
    for key in [
        "".to_owned(),
        " ".into(),
        " copy".into(),
        "copy\n".into(),
        "x".repeat(257),
        "é".repeat(129),
    ] {
        cases.push(serde_json::from_slice(&frame("submit", &key)).unwrap());
    }
    assert_eq!(cases.len(), 29);
    for v in cases {
        let response = FixedPackageRoute::exchange_local(
            Some(&route),
            &serde_json::to_vec(&v).unwrap(),
            f.root.path(),
        )
        .unwrap();
        assert_eq!(
            error(&response).code,
            err::FixedPackageActionErrorCode::InvalidRequest
        );
        let e::Body::Error(b) = response.body else {
            unreachable!()
        };
        assert_eq!(b.effect_state, e::BodyErrorEffectState::NoEffects);
        assert!(!f.output().exists());
    }
    for bytes in [vec![b' '; PUBLICATION_CAP + 1], {
        let mut v = good;
        v["body"]["payload_json"] = json!(" ".repeat(REQUEST_CAP + 1));
        serde_json::to_vec(&v).unwrap()
    }] {
        assert_eq!(
            error(&FixedPackageRoute::exchange_local(Some(&route), &bytes, f.root.path()).unwrap())
                .code,
            err::FixedPackageActionErrorCode::InvalidRequest
        );
    }
    assert!(!f.output().exists());
}
#[test]
fn no_grant_foreign_scope_and_foreign_root_refuse_without_copy() {
    let f = Fixture::new();
    let route = f.route();
    let r =
        FixedPackageRoute::exchange_local(None, &frame("submit", "copy"), f.root.path()).unwrap();
    let e = error(&r);
    assert_eq!(e.code, err::FixedPackageActionErrorCode::UnavailableHost);
    assert!(e.requested_scope.is_none() && e.scope_unavailable_reason.is_some());
    let foreign = tempfile::tempdir().unwrap();
    let r =
        FixedPackageRoute::exchange_local(Some(&route), &frame("submit", "copy"), foreign.path())
            .unwrap();
    assert_eq!(
        error(&r).code,
        err::FixedPackageActionErrorCode::ContextRootMismatch
    );
    let altered = SubmissionScope {
        principal: "other".into(),
        ..f.scope.clone()
    };
    assert!(matches!(
        FixedPackageHost::from_trusted_host(f.project.clone(), f.namespace.clone(), altered),
        Err(FixedPackageHostError::Namespace(_))
    ));
    assert!(!f.output().exists() && !foreign.path().join("target").exists());
}
#[test]
fn source_drift_error_retains_completed_owner_view_run_and_effect() {
    let f = Fixture::new();
    let route = f.route();
    let first = f.exchange(&route, "submit", "copy");
    let run = completed(&first);
    let modified = std::fs::metadata(f.output()).unwrap().modified().unwrap();
    std::fs::write(f.root.path().join("source.bin"), b"changed").unwrap();
    let response = f.exchange(&route, "submit", "copy");
    let err = error(&response);
    assert_eq!(
        err.code,
        err::FixedPackageActionErrorCode::SubmissionRefused
    );
    assert_eq!(
        err.effect_observation,
        err::FixedPackageActionErrorEffectObservation::Landed
    );
    assert_eq!(err.known_run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(err.view, Some(err::Reply::Completed(_))));
    let e::Body::Error(b) = response.body else {
        unreachable!()
    };
    assert_eq!(b.effect_state, e::BodyErrorEffectState::Completed);
    assert_eq!(b.completeness, e::BodyErrorCompleteness::Complete);
    assert_eq!(
        std::fs::metadata(f.output()).unwrap().modified().unwrap(),
        modified
    );
}
#[test]
fn owner_error_without_retained_inspection_remains_uncertain_without_run() {
    let f = Fixture::new();
    let route = f.route();
    // Missing source causes an owner refusal; no retained key is available.
    std::fs::remove_file(f.root.path().join("source.bin")).unwrap();
    let response = f.exchange(&route, "submit", "copy");
    let err = error(&response);
    assert!(err.known_run_id.is_none() && err.view.is_none());
    let e::Body::Error(b) = response.body else {
        unreachable!()
    };
    assert_eq!(b.effect_state, e::BodyErrorEffectState::Uncertain);
    assert_eq!(b.completeness, e::BodyErrorCompleteness::Unavailable);
    assert!(!f.output().exists());
}
#[test]
fn damaged_missing_completion_capsule_output_and_a2_keep_partial_truth_without_recopy() {
    for damage in [
        "result-corrupt",
        "result-missing",
        "capsule",
        "output",
        "a2",
        "intent",
    ] {
        let f = Fixture::new();
        let route = f.route();
        let run = completed(&f.exchange(&route, "submit", "copy"));
        let original = std::fs::metadata(f.output()).unwrap().modified().unwrap();
        let home = f.root.path().join(".vibe/package-operations");
        match damage {
            "result-corrupt" => {
                std::fs::write(home.join(format!("{run}.static-file-result.json")), b"{}").unwrap()
            }
            "result-missing" => {
                std::fs::remove_file(home.join(format!("{run}.static-file-result.json"))).unwrap()
            }
            "capsule" => {
                std::fs::write(home.join(format!("{run}.static-file-input.json")), b"{}").unwrap()
            }
            "output" => std::fs::write(f.output(), b"drift").unwrap(),
            "a2" => std::fs::write(
                f.root
                    .path()
                    .join(".vibe/state/artifacts/launcher.bin.json"),
                b"{}",
            )
            .unwrap(),
            "intent" => {
                std::fs::remove_file(home.join(format!("{run}.static-file-intent.json"))).unwrap()
            }
            _ => unreachable!(),
        }
        drop(route);
        let cold = f.route();
        for action in ["inspect", "submit"] {
            let expected = if action == "inspect" {
                cold.host.inspect("copy").unwrap().unwrap()
            } else {
                cold.host.submit("copy").unwrap()
            };
            let response = f.exchange(&cold, action, "copy");
            let projected = reply(&response);
            assert_eq!(projected.reply, contract::project(&expected).unwrap());
            assert_eq!(contract::run(&projected.reply), Some(run.as_str()));
            let e::Body::Result(b) = response.body else {
                unreachable!()
            };
            assert_eq!(b.effect_state, e::BodyResultEffectState::Uncertain);
            assert_eq!(
                b.completeness,
                if matches!(
                    expected,
                    StaticFileJournalInspection::RecoveryRequired { .. }
                ) {
                    e::BodyResultCompleteness::Unavailable
                } else {
                    e::BodyResultCompleteness::Partial
                }
            );
        }
        if damage != "output" {
            assert_eq!(
                std::fs::metadata(f.output()).unwrap().modified().unwrap(),
                original
            );
        }
        assert_eq!(
            std::fs::read(f.output()).unwrap(),
            if damage == "output" {
                b"drift".as_slice()
            } else {
                b"copy once".as_slice()
            }
        );
    }
}
#[test]
fn retired_and_expired_namespaces_preserve_old_run_and_refuse_new_copy() {
    for expired in [false, true] {
        let f = Fixture::new();
        let namespace = if expired {
            let lease = LifecycleLease::acquire(f.root.path()).unwrap();
            SubmissionReservations::issue_namespace(&lease, &f.scope, 1).unwrap()
        } else {
            f.namespace.clone()
        };
        let host = Arc::new(
            FixedPackageHost::from_trusted_host(
                f.project.clone(),
                namespace.clone(),
                f.scope.clone(),
            )
            .unwrap(),
        );
        let route = FixedPackageRoute::new(host);
        if expired {
            std::thread::sleep(std::time::Duration::from_millis(10));
        } else {
            let first = f.exchange(&route, "submit", "old");
            completed(&first);
            let lease = LifecycleLease::acquire(f.root.path()).unwrap();
            SubmissionReservations::retire_namespace(&lease, &namespace, &f.scope).unwrap();
            drop(lease);
            assert_eq!(f.exchange(&route, "inspect", "old"), first);
        }
        let response = f.exchange(&route, "submit", "new");
        assert!(matches!(reply(&response).reply, rep::Reply::Retired(r) if r.run_id.is_none()));
        let e::Body::Result(b) = response.body else {
            unreachable!()
        };
        assert_eq!(b.effect_state, e::BodyResultEffectState::Uncertain);
        assert_eq!(b.completeness, e::BodyResultCompleteness::Unavailable);
        if expired {
            assert!(!f.output().exists());
        }
    }
}
#[test]
fn owner_error_and_cold_inspection_failure_never_fabricate_absence_or_run() {
    let f = Fixture::new();
    let route = f.route();
    completed(&f.exchange(&route, "submit", "copy"));
    let before = std::fs::metadata(f.output()).unwrap().modified().unwrap();
    std::fs::write(f.root.path().join(".vibe/machine-submissions.json"), b"{}").unwrap();
    for action in ["submit", "inspect"] {
        let response = f.exchange(&route, action, "copy");
        let row = error(&response);
        assert!(row.known_run_id.is_none() && row.view.is_none());
        assert_eq!(
            row.effect_observation,
            err::FixedPackageActionErrorEffectObservation::Unavailable
        );
        let e::Body::Error(b) = response.body else {
            unreachable!()
        };
        assert_eq!(b.effect_state, e::BodyErrorEffectState::Uncertain);
        assert_eq!(b.completeness, e::BodyErrorCompleteness::Unavailable);
    }
    assert_eq!(
        std::fs::metadata(f.output()).unwrap().modified().unwrap(),
        before
    );
}
#[test]
fn physical_root_replacement_is_a_typed_pre_owner_refusal() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    let old = parent.path().join("old");
    std::fs::create_dir(&root).unwrap();
    let project = Arc::new(Project::open(&root).unwrap());
    let scope = SubmissionScope {
        context: "context".into(),
        target: "launcher".into(),
        principal: "principal".into(),
    };
    let lease = LifecycleLease::acquire(&root).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    drop(lease);
    let route = FixedPackageRoute::new(Arc::new(
        FixedPackageHost::from_trusted_host(project, namespace, scope).unwrap(),
    ));
    std::fs::rename(&root, &old).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("source.bin"), b"foreign").unwrap();
    let response =
        FixedPackageRoute::exchange_local(Some(&route), &frame("submit", "copy"), &root).unwrap();
    assert_eq!(
        error(&response).code,
        err::FixedPackageActionErrorCode::ContextRootMismatch
    );
    let e::Body::Error(b) = response.body else {
        unreachable!()
    };
    assert_eq!(b.effect_state, e::BodyErrorEffectState::NoEffects);
    assert!(!root.join("target").exists() && !old.join("target").exists());
}
