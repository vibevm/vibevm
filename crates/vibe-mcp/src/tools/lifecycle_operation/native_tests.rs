//! Native provider cases through the actual registered Server tools/call.
//! Compiled only in this packet; runtime evidence belongs to the parent.
use super::*;
use crate::{MemoryTransport, Server};
use serde_json::json;
use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};
use vibe_core::manifest::TargetOs;
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_orchestrator::machine_operation::{input_decision as d, lifecycle_view as v};
use vibe_orchestrator::machine_submission::{
    FixedLifecycleDecisionHost, FixedOperationStatus, HostPolicyFacts, ValidatedFixedPlanEvidence,
};
const WAIT: Duration = Duration::from_secs(20);
struct Fixture {
    dir: tempfile::TempDir,
    scope: SubmissionScope,
    ns: SubmissionNamespace,
}
fn facts(approve: bool) -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: approve,
        allow_detach: true,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: true,
    }
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"),"[project]\nname='demo'\nversion='1.1.0'\n[[extension]]\nid='once'\npoint='phase:validate'\nhandler={kind='script',base='run'}\n").unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        fs::write(dir.path().join("run.sh"),format!("#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\nprintf 'native-output\\n'\ntouch entered\ni=0\nwhile [ ! -f release ]; do i=$((i+1)); if [ \"$i\" -gt 400 ]; then exit 70; fi; sleep 0.05; done\n")).unwrap();
        let scope = SubmissionScope {
            context: "mcp-native".into(),
            target: "root".into(),
            principal: "trusted-host".into(),
        };
        let lease = LifecycleLease::acquire(dir.path()).unwrap();
        let ns = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
        drop(lease);
        Self { dir, scope, ns }
    }
    fn host(&self, approve: bool) -> Arc<FixedLifecycleDecisionHost> {
        Arc::new(
            FixedLifecycleDecisionHost::from_host(
                self.dir.path().into(),
                self.ns.clone(),
                self.scope.clone(),
                TargetOs::Linux,
                facts(approve),
            )
            .unwrap(),
        )
    }
    fn server(&self, h: Arc<FixedLifecycleDecisionHost>) -> Server<MemoryTransport> {
        Server::new(
            MemoryTransport::with_input(""),
            ServerContext::with_store_root(self.dir.path(), self.dir.path().join("unused-cache"))
                .with_fixed_lifecycle_host(h)
                .unwrap(),
        )
    }
    fn release(&self) {
        fs::write(self.dir.path().join("release"), "go").unwrap();
    }
    fn count(&self) {
        assert_eq!(
            fs::read(self.dir.path().join("counter")).unwrap(),
            b"effect\n"
        );
    }
    fn no_provider(&self) {
        assert!(!self.dir.path().join("counter").exists());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::write(self.dir.path().join("release"), "cleanup");
    }
}
fn call(s: &mut Server<MemoryTransport>, args: Value) -> Value {
    s.transport=MemoryTransport::with_input(json!({"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"name":"lifecycle_operation","arguments":args}}).to_string()+"\n");
    s.run().unwrap();
    let reply: Value = serde_json::from_str(s.transport.take_output().trim()).unwrap();
    assert!(reply["error"].is_null(), "{reply}");
    let r = reply["result"].clone();
    assert!(r["structuredContent"].is_object());
    assert_eq!(r["content"][0]["type"], "text");
    r
}
fn prepare(s: &mut Server<MemoryTransport>, control: &str) -> d::InputDecisionMessage {
    let r = call(
        s,
        json!({"schema":1,"action":{"kind":"prepare","phase":"validate","requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":control}}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    serde_json::from_str(
        r["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}
fn answer(s: &mut Server<MemoryTransport>, q: &d::InputDecisionMessage) -> Value {
    call(
        s,
        json!({"schema":1,"action":{"kind":"answer","message_json":serde_json::to_string(q).unwrap()}}),
    )
}
fn approve(s: &mut Server<MemoryTransport>, control: &str) -> String {
    let mut q = prepare(s, control);
    if matches!(q.body, d::Body::SelectionQuestion(_)) {
        q.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        let r = answer(s, &q);
        assert_eq!(r["isError"], false);
        q = serde_json::from_str(
            r["structuredContent"]["reply"]["payload_json"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
    }
    let id = q.question_id.clone().unwrap();
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    for _ in 0..2 {
        let r = answer(s, &q);
        assert_eq!(r["isError"], false, "{r}");
        assert_eq!(r["structuredContent"]["reply"]["accepted_ref"], id);
    }
    id
}
fn direct_approve(h: &FixedLifecycleDecisionHost) -> ValidatedFixedPlanEvidence {
    let mut q = h
        .prepare_requested_options(d::RequestedOptions {
            control: d::RequestedOptionsControl::Detach,
            force: false,
            trace: false,
            deploy_profile: None,
        })
        .unwrap();
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    match h.answer_typed(&q).unwrap() {
        DecisionOutcome::Validated(e) => e,
        _ => panic!("validated"),
    }
}
fn submit(s: &mut Server<MemoryTransport>, reference: &str, key: &str) -> Value {
    call(
        s,
        json!({"schema":1,"action":{"kind":"submit","accepted_ref":reference,"key":key}}),
    )
}
fn inspect(s: &mut Server<MemoryTransport>, key: &str) -> Value {
    call(s, json!({"schema":1,"action":{"kind":"inspect","key":key}}))
}
fn row(r: &Value) -> v::LifecycleSubmissionView {
    serde_json::from_str(
        r["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}
fn until(mut f: impl FnMut() -> bool) {
    let end = Instant::now() + WAIT;
    while !f() {
        assert!(Instant::now() < end, "native deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn actual_selection_approval_submit_same_key_direct_owner_inspect_and_changed_intent() {
    let f = Fixture::new();
    let h = f.host(true);
    let mut s = f.server(h.clone());
    let no_control = approve(&mut s, "none");
    let detach = approve(&mut s, "detach");
    let direct = direct_approve(&h);
    let before = Instant::now();
    let initial = submit(&mut s, &detach, "once");
    assert_eq!(initial["isError"], false, "{initial}");
    assert!(before.elapsed() < Duration::from_secs(2));
    until(|| f.dir.path().join("entered").exists());
    f.count();
    let begin = Instant::now();
    let running = inspect(&mut s, "once");
    assert!(begin.elapsed() < Duration::from_secs(2));
    let run = row(&running).run_id.unwrap();
    let repeated = submit(&mut s, &detach, "once");
    assert_eq!(repeated["isError"], false);
    assert_eq!(row(&repeated).run_id.as_deref(), Some(run.as_str()));
    let job = h
        .submit_scoped(
            &direct,
            "once",
            chrono::Utc::now().to_rfc3339().parse().unwrap(),
        )
        .unwrap();
    assert!(matches!(job.inspect().unwrap(),FixedOperationStatus::Running{run_id} if run_id==run));
    let refused = submit(&mut s, &no_control, "once");
    assert_eq!(refused["isError"], true);
    assert_eq!(refused["structuredContent"]["code"], "submission_refused");
    f.release();
    until(|| {
        matches!(
            job.inspect().unwrap(),
            FixedOperationStatus::ResultAvailable { .. }
        )
    });
    let terminal = row(&inspect(&mut s, "once"));
    assert_eq!(terminal.run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(terminal.status, v::Status::ResultAvailable(_)));
    f.count();
    let lease = LifecycleLease::acquire(f.dir.path()).unwrap();
    let c1 = SubmissionReservations::inspect_existing_key(&lease, &f.ns, &f.scope, "once")
        .unwrap()
        .unwrap();
    assert_eq!(c1.domain_run_id, run);
    drop(lease);
    for suffix in ["marker.json", "admitted-input.json"] {
        assert!(
            f.dir
                .path()
                .join(".vibe/lifecycle-outcomes")
                .join(format!("{run}.{suffix}"))
                .is_file()
        );
    }
    fs::write(f.dir.path().join("run.sh"), "broken source").unwrap();
    let mut cold = f.server(f.host(false));
    let coldrow = row(&inspect(&mut cold, "once"));
    assert_eq!(coldrow.run_id.as_deref(), Some(run.as_str()));
    f.count();
}
#[test]
fn forged_foreign_shapes_options_policy_and_context_roots_refuse_before_provider() {
    let f = Fixture::new();
    let h = f.host(true);
    let mut s = f.server(h.clone());
    for reference in ["forged", "sha256:pretend", "99"] {
        let r = submit(&mut s, reference, "once");
        assert_eq!(r["structuredContent"]["code"], "unknown_reference");
    }
    for args in [
        json!({"schema":1,"root":"/tmp","action":{"kind":"inspect","key":"once"}}),
        json!({"schema":1,"action":{"kind":"answer","message_json":"{"}}),
        json!({"schema":1,"action":{"kind":"answer","message_json":"x".repeat(65537)}}),
    ] {
        assert_eq!(
            call(&mut s, args)["structuredContent"]["code"],
            "invalid_request"
        );
    }
    for (phase, force, trace, profile) in [
        ("build", false, false, Value::Null),
        ("validate", true, false, Value::Null),
        ("validate", false, true, Value::Null),
        ("validate", false, false, json!("posix")),
    ] {
        let r = call(
            &mut s,
            json!({"schema":1,"action":{"kind":"prepare","phase":phase,"requested_options":{"force":force,"trace":trace,"deploy_profile":profile,"control":"none"}}}),
        );
        assert_eq!(r["structuredContent"]["code"], "implementation_gap");
    }
    let mut q = prepare(&mut s, "detach");
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    q.principal_id = Some("foreign".into());
    assert_eq!(answer(&mut s, &q)["isError"], true);
    let reference = approve(&mut s, "detach");
    let mut foreign = f.server(f.host(true));
    assert_eq!(
        submit(&mut foreign, &reference, "once")["structuredContent"]["code"],
        "unknown_reference"
    );
    h.set_host_policy(facts(false)).unwrap();
    assert_eq!(submit(&mut s, &reference, "once")["isError"], true);
    let other = tempfile::tempdir().unwrap();
    assert!(
        ServerContext::new(other.path())
            .with_fixed_lifecycle_host(h.clone())
            .is_err()
    );
    s.context.project_root = other.path().into();
    assert_eq!(
        inspect(&mut s, "once")["structuredContent"]["code"],
        "context_root_mismatch"
    );
    f.no_provider();
}
#[test]
fn physical_root_replacement_and_source_drift_do_not_execute() {
    let f = Fixture::new();
    let h = f.host(true);
    let mut s = f.server(h);
    let reference = approve(&mut s, "detach");
    let moved = f.dir.path().with_extension("original");
    fs::rename(f.dir.path(), &moved).unwrap();
    fs::create_dir(f.dir.path()).unwrap();
    let r = submit(&mut s, &reference, "once");
    assert_eq!(r["structuredContent"]["code"], "context_root_mismatch");
    fs::remove_dir(f.dir.path()).unwrap();
    fs::rename(&moved, f.dir.path()).unwrap();
    fs::write(f.dir.path().join("run.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    let _ = submit(&mut s, &reference, "once");
    until(|| inspect(&mut s, "once")["isError"] == true);
    f.no_provider();
}
#[test]
fn damaged_final_report_keeps_known_run_scope_phase_and_unavailable_effect_without_redispatch() {
    for missing in [false, true] {
        let f = Fixture::new();
        let h = f.host(true);
        let mut s = f.server(h);
        let reference = approve(&mut s, "detach");
        assert_eq!(submit(&mut s, &reference, "once")["isError"], false);
        until(|| f.dir.path().join("entered").exists());
        f.release();
        let mut run = String::new();
        until(|| {
            let r = inspect(&mut s, "once");
            if r["isError"] == false {
                let v = row(&r);
                if matches!(v.status, v::Status::ResultAvailable(_)) {
                    run = v.run_id.unwrap();
                    return true;
                }
            }
            false
        });
        let path = f
            .dir
            .path()
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run}.report.json"));
        if missing {
            fs::remove_file(path).unwrap();
        } else {
            fs::write(path, "corrupt").unwrap();
        }
        let result = inspect(&mut s, "once");
        assert_eq!(result["isError"], true, "{result}");
        let e: LifecycleActionError =
            serde_json::from_value(result["structuredContent"].clone()).unwrap();
        assert_eq!(e.known_run_id.as_deref(), Some(run.as_str()));
        assert_eq!(e.requested_phase.as_deref(), Some("validate"));
        assert!(e.requested_scope.is_some());
        assert!(e.view_json.is_some());
        assert_eq!(
            e.effect_observation,
            LifecycleActionErrorEffectObservation::Unavailable
        );
        let _ = submit(&mut s, &reference, "once");
        f.count();
    }
}

#[test]
fn retained_reference_and_job_capacities_are_typed_and_never_evict() {
    let f = Fixture::new();
    let h = f.host(true);
    let mut s = f.server(h);
    let mut references = Vec::new();
    for _ in 0..4 {
        references.push(approve(&mut s, "detach"));
    }
    let fifth = call(
        &mut s,
        json!({"schema":1,"action":{"kind":"prepare","phase":"validate","requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"detach"}}}),
    );
    assert_eq!(fifth["structuredContent"]["code"], "capacity");
    f.release();
    for index in 0..8 {
        let key = format!("job-{index}");
        assert_eq!(
            submit(&mut s, &references[index % 4], &key)["isError"],
            false
        );
        until(|| {
            let r = inspect(&mut s, &key);
            if r["isError"] == true {
                panic!(
                    "capacity job {index} inspection error: {}",
                    r["structuredContent"]
                );
            }
            let observed = row(&r);
            match observed.status {
                v::Status::ResultAvailable(_) => true,
                v::Status::Starting(_) | v::Status::Running(_) => false,
                other => panic!("capacity job {index} stopped before result: {other:?}"),
            }
        });
    }
    // All eight independently reserved runs complete, but a new key is not
    // force: unchanged inputs reuse the actual engine freshness record.
    for index in 0..8 {
        let result = inspect(&mut s, &format!("job-{index}"));
        let view = row(&result);
        assert_eq!(view.work_outcomes.len(), 1);
        assert_eq!(
            view.work_outcomes[0].status,
            if index == 0 { "ok" } else { "fresh" }
        );
    }
    let refused = submit(&mut s, &references[0], "ninth");
    assert_eq!(refused["structuredContent"]["code"], "capacity");
    assert_eq!(inspect(&mut s, "job-0")["isError"], false);
    assert_eq!(
        fs::read_to_string(f.dir.path().join("counter"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn cold_absent_and_granted_key_bounds_do_not_dispatch() {
    let f = Fixture::new();
    let mut s = f.server(f.host(false));
    let absent = inspect(&mut s, "unknown");
    assert_eq!(absent["structuredContent"]["reply"]["kind"], "absent");
    for key in [
        "".to_owned(),
        " padded".into(),
        "line\nbreak".into(),
        "x".repeat(257),
    ] {
        assert_eq!(
            inspect(&mut s, &key)["structuredContent"]["code"],
            "invalid_request"
        );
    }
    f.no_provider();
}

#[test]
fn concurrent_registered_mcp_calls_share_the_actual_owner_job() {
    let f = Fixture::new();
    let h = f.host(true);
    let mut a = f.server(h.clone());
    let mut b = f.server(h);
    let ar = approve(&mut a, "detach");
    let br = approve(&mut b, "detach");
    let ac = std::mem::replace(&mut a.context, ServerContext::new("unused"));
    let bc = std::mem::replace(&mut b.context, ServerContext::new("unused"));
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for (context, reference) in [(ac, ar), (bc, br)] {
        let start = barrier.clone();
        workers.push(std::thread::spawn(move || {
            let mut server = Server::new(MemoryTransport::with_input(""), context);
            start.wait();
            let result = submit(&mut server, &reference, "once");
            assert_eq!(result["isError"], false, "{result}");
            server.context
        }));
    }
    barrier.wait();
    let contexts: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    until(|| f.dir.path().join("entered").exists());
    let mut servers: Vec<_> = contexts
        .into_iter()
        .map(|c| Server::new(MemoryTransport::with_input(""), c))
        .collect();
    let run = row(&inspect(&mut servers[0], "once")).run_id.unwrap();
    assert_eq!(
        row(&inspect(&mut servers[1], "once")).run_id.as_deref(),
        Some(run.as_str())
    );
    f.count();
    f.release();
    until(|| {
        matches!(
            row(&inspect(&mut servers[0], "once")).status,
            v::Status::ResultAvailable(_)
        )
    });
    f.count();
}

#[path = "observers/native_tests.rs"]
mod observer_cases;
