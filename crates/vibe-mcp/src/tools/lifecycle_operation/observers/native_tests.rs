//! Native registered-tool cases: compile-only here, execution belongs to parent.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM");
use super::*;
fn attach(s: &mut Server<MemoryTransport>, key: &str) -> Value {
    let r = call(s, json!({"schema":1,"action":{"kind":"attach","key":key}}));
    assert_eq!(r["isError"], false, "{r}");
    let a = r["structuredContent"]["reply"].clone();
    assert_eq!(a["kind"], "attached");
    assert_eq!(a["observer_ref"].as_str().unwrap().len(), 32);
    a
}
fn frame(a: &Value, body: Value) -> Value {
    json!({"schema":1,"context_id":a["context_id"],"target_id":a["target_id"],
        "operation_id":a["operation_id"],"run_id":a["run_id"],"stream_id":a["stream_id"],"body":body})
}
fn stream(s: &mut Server<MemoryTransport>, a: &Value, kind: &str, message: Value) -> Value {
    call(
        s,
        json!({"schema":1,"action":{"kind":kind,"observer_ref":a["observer_ref"],
        "message_json":message.to_string()}}),
    )
}
fn response(r: &Value) -> Value {
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(
        r["structuredContent"]["reply"]["format_id"],
        "machine-operation-stream-message"
    );
    let text = r["structuredContent"]["reply"]["payload_json"]
        .as_str()
        .unwrap();
    let _: vibe_wire::generated::machine::e1::operation_stream_message::OperationStreamMessage =
        serde_json::from_str(text).unwrap();
    serde_json::from_str(text).unwrap()
}
fn cancel(a: &Value, id: &str) -> Value {
    frame(
        a,
        json!({"kind":"control_request","action":"cancel","request_id":id,"after_sequence":"0"}),
    )
}
fn replay(a: &Value) -> Value {
    frame(
        a,
        json!({"kind":"replay_request","after_sequence":"0","max_events":64,"max_bytes":65536}),
    )
}
fn detach(s: &mut Server<MemoryTransport>, a: &Value) -> Value {
    call(
        s,
        json!({"schema":1,"action":{"kind":"detach","observer_ref":a["observer_ref"]}}),
    )
}
fn start(f: &Fixture, mode: &str) -> (Server<MemoryTransport>, Value) {
    let mut s = f.server(f.host(true));
    let approved = approve(&mut s, mode);
    assert_eq!(submit(&mut s, &approved, "once")["isError"], false);
    until(|| f.dir.path().join("entered").exists());
    let a = attach(&mut s, "once");
    assert_eq!(
        a["run_id"].as_str(),
        row(&inspect(&mut s, "once")).run_id.as_deref()
    );
    (s, a)
}
#[test]
fn actual_entered_none_detach_and_pre_only_cancel_unavailable_detach_keeps_provider() {
    for mode in ["none", "detach", "cancel_pre_dispatch"] {
        let f = Fixture::new();
        let (mut s, a) = start(&f, mode);
        let r = response(&stream(&mut s, &a, "control", cancel(&a, "cancel")));
        assert_eq!(r["body"]["status"], "unavailable", "{r}");
        f.count();
        assert!(!f.dir.path().join("release").exists());
        let history = response(&stream(&mut s, &a, "replay", replay(&a)));
        assert!(
            history["body"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["payload"]["type"] == "phase")
        );
        let mut tiny = replay(&a);
        tiny["body"]["max_bytes"] = json!(1);
        let bounded = stream(&mut s, &a, "replay", tiny);
        assert_eq!(bounded["isError"], true);
        assert_eq!(
            bounded["structuredContent"]["code"],
            "inspection_unavailable"
        );
        assert!(
            bounded["structuredContent"]["message"]
                .as_str()
                .unwrap()
                .contains("ReplyBudget")
        );
        // An unrepresentable envelope is an owned refusal, not invented loss.
        let retained = response(&stream(&mut s, &a, "replay", replay(&a)));
        assert!(
            retained["body"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["payload"]["type"] == "phase")
        );
        assert_eq!(detach(&mut s, &a)["isError"], false);
        assert_eq!(
            detach(&mut s, &a)["structuredContent"]["code"],
            "unknown_reference"
        );
        assert_eq!(
            stream(&mut s, &a, "replay", replay(&a))["structuredContent"]["code"],
            "unknown_reference"
        );
        f.count();
        assert!(matches!(
            row(&inspect(&mut s, "once")).status,
            v::Status::Running(_)
        ));
        f.release();
        until(|| {
            matches!(
                row(&inspect(&mut s, "once")).status,
                v::Status::ResultAvailable(_)
            )
        });
        f.count();
    }
}
#[test]
fn actual_deferred_scope_run_schema_body_refusals_precede_latch_and_receipt_survives() {
    let f = Fixture::new();
    let (mut s, a) = start(&f, "cancel_deferred_boundary");
    for field in [
        "context_id",
        "target_id",
        "operation_id",
        "run_id",
        "stream_id",
    ] {
        let mut wrong = cancel(&a, "forbidden");
        wrong[field] = json!("foreign");
        assert_eq!(stream(&mut s, &a, "control", wrong)["isError"], true);
    }
    let mut extra = cancel(&a, "forbidden");
    extra["grant"] = json!(true);
    assert_eq!(
        stream(&mut s, &a, "control", extra)["structuredContent"]["code"],
        "invalid_request"
    );
    assert_eq!(
        stream(
            &mut s,
            &a,
            "control",
            cancel(&a, "body-only")["body"].clone()
        )["structuredContent"]["code"],
        "invalid_request"
    );
    assert_eq!(
        call(
            &mut s,
            json!({"schema":1,"action":{"kind":"control","observer_ref":a["observer_ref"],
        "message_json":" ".repeat(65537)}})
        )["structuredContent"]["code"],
        "invalid_request"
    );
    let mut wrong = cancel(&a, "forbidden");
    wrong["schema"] = json!(2);
    assert_eq!(stream(&mut s, &a, "control", wrong)["isError"], true);
    assert_eq!(stream(&mut s, &a, "control", replay(&a))["isError"], true);
    assert_eq!(
        stream(&mut s, &a, "replay", cancel(&a, "forbidden"))["isError"],
        true
    );
    assert_eq!(
        stream(&mut s, &a, "control", cancel(&a, "original"))["isError"],
        false
    );
    let r = response(&stream(&mut s, &a, "control", cancel(&a, "original")));
    assert_eq!(r["body"]["status"], "requested");
    assert_eq!(r["body"]["operation_effect_state"], "uncertain");
    f.count();
    f.release();
    until(|| {
        matches!(
            row(&inspect(&mut s, "once")).status,
            v::Status::ResultAvailable(_)
        )
    });
    let terminal = row(&inspect(&mut s, "once"));
    let v::Status::ResultAvailable(t) = terminal.status else {
        panic!("terminal result");
    };
    assert_eq!(
        t.effect_state,
        v::StatusResultAvailableEffectState::Uncertain
    );
    let reached = response(&stream(&mut s, &a, "control", cancel(&a, "original")));
    assert_eq!(reached["body"]["status"], "reached_safe_stop");
    assert_eq!(reached["body"]["operation_effect_state"], "uncertain");
    assert!(reached["body"]["result_ref"].is_string());
    let receipt: Value = serde_json::from_slice(
        &fs::read(
            f.dir
                .path()
                .join(".vibe/lifecycle-outcomes")
                .join(format!("{}.cancel.json", a["run_id"].as_str().unwrap())),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt, reached);
    let late = response(&stream(&mut s, &a, "control", cancel(&a, "late")));
    assert_eq!(late["body"]["status"], "already_terminal");
    assert_eq!(
        response(&stream(&mut s, &a, "control", cancel(&a, "original"))),
        reached
    );
    let history = response(&stream(&mut s, &a, "replay", replay(&a)));
    assert!(
        history["body"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["payload"]["type"] == "raw_output")
    );
    f.count();
}
#[test]
fn actual_observer_capacity_foreign_bridge_stale_refs_and_root_replacement() {
    let f = Fixture::new();
    let h = f.host(true);
    let mut s = f.server(h.clone());
    let mut foreign = f.server(h);
    assert_eq!(
        call(
            &mut s,
            json!({"schema":1,"action":{"kind":"attach","key":"unknown"}})
        )["isError"],
        true
    );
    let approved = approve(&mut s, "cancel_deferred_boundary");
    let foreign_approved = approve(&mut foreign, "cancel_deferred_boundary");
    assert_eq!(submit(&mut s, &approved, "once")["isError"], false);
    until(|| f.dir.path().join("entered").exists());
    assert_eq!(
        submit(&mut foreign, &foreign_approved, "once")["isError"],
        false
    );
    let observers: Vec<_> = (0..8).map(|_| attach(&mut s, "once")).collect();
    let refs: std::collections::BTreeSet<_> = observers
        .iter()
        .map(|a| a["observer_ref"].as_str().unwrap())
        .collect();
    assert_eq!(refs.len(), 8);
    assert_eq!(
        call(
            &mut s,
            json!({"schema":1,"action":{"kind":"attach","key":"once"}})
        )["structuredContent"]["code"],
        "capacity"
    );
    assert_eq!(
        stream(
            &mut foreign,
            &observers[0],
            "control",
            cancel(&observers[0], "foreign")
        )["structuredContent"]["code"],
        "unknown_reference"
    );
    assert_eq!(detach(&mut s, &observers[0])["isError"], false);
    let wire_detach = frame(
        &observers[1],
        json!({"kind":"control_request","action":"detach",
        "request_id":"wire-detach","after_sequence":"0"}),
    );
    assert_eq!(
        response(&stream(&mut s, &observers[1], "control", wire_detach))["body"]["status"],
        "detached"
    );
    assert_eq!(
        detach(&mut s, &observers[1])["structuredContent"]["code"],
        "unknown_reference"
    );
    let fresh = attach(&mut s, "once");
    assert_ne!(fresh["observer_ref"], observers[0]["observer_ref"]);
    let original = s.context.project_root.clone();
    let other = tempfile::tempdir().unwrap();
    s.context.project_root = other.path().into();
    assert_eq!(
        stream(&mut s, &fresh, "control", cancel(&fresh, "bad-root"))["structuredContent"]["code"],
        "context_root_mismatch"
    );
    s.context.project_root = original.clone();
    struct RestoreRoot {
        root: std::path::PathBuf,
        retained: std::path::PathBuf,
    }
    impl Drop for RestoreRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir(&self.root);
            let _ = fs::rename(&self.retained, &self.root);
        }
    }
    let retained = original.with_extension("retained");
    fs::rename(&original, &retained).unwrap();
    let restore = RestoreRoot {
        root: original.clone(),
        retained,
    };
    fs::create_dir(&original).unwrap();
    assert_eq!(
        stream(&mut s, &fresh, "control", cancel(&fresh, "replaced-root"))["structuredContent"]["code"],
        "context_root_mismatch"
    );
    drop(restore);
    // Root/connection loss detaches subscriptions while the separately held host continues.
    drop(s);
    assert!(!f.dir.path().join("release").exists());
    f.count();
    f.release();
    until(|| {
        matches!(
            row(&inspect(&mut foreign, "once")).status,
            v::Status::ResultAvailable(_)
        )
    });
    f.count();
}
