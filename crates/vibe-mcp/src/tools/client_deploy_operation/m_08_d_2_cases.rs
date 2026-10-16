use crate as vibe_mcp;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use vibe_lifecycle::{
    ClientExecutable, client_commands,
    client_deploy::{Cancellation, Command, Host, Prepared},
    process::{ProcessError, ProcessOutput, ProcessRunner, ProcessSpec, SelectedFileWitness},
};
use vibe_wire::generated::client_deploy::e1::reply::{ReplyAction, ReplyStatus, StepStatus};
const CLOCK: &str = "2026-10-15T00:00:00Z";
fn hash(v: &[u8]) -> String {
    SelectedFileWitness::from_bytes(v.to_vec())
        .sha256
        .trim_start_matches("sha256:")
        .to_owned()
}
struct Control(Mutex<client_commands::Context>);
impl client_commands::Authority for Control {
    fn current(
        &self,
        _: &vibe_wire::generated::client_command::e1::request::Request,
    ) -> Result<client_commands::Context, &'static str> {
        Ok(self.0.lock().unwrap().clone())
    }
}
struct Observed {
    root: std::path::PathBuf,
    subject: Mutex<String>,
    unavailable: std::sync::atomic::AtomicBool,
}
impl vibe_lifecycle::client_deploy::Observer for Observed {
    fn subject(&self) -> Result<String, String> {
        Ok(self.subject.lock().unwrap().clone())
    }
    fn observe(&self, resources: &[String]) -> Result<BTreeMap<String, Option<String>>, String> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err("observer unavailable".into());
        }
        resources
            .iter()
            .map(|s| {
                let path = self.root.join(s);
                let v = match std::fs::read(path) {
                    Ok(v) => Some(hash(&v)),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => return Err(e.to_string()),
                };
                Ok((s.clone(), v))
            })
            .collect()
    }
}
struct Runner {
    root: std::path::PathBuf,
    calls: AtomicUsize,
    probes: AtomicUsize,
    fail_after: AtomicUsize,
    cancel: Mutex<Option<Cancellation>>,
}
impl ProcessRunner for Runner {
    fn run(&self, p: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        let args = p
            .args
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>();
        if args == ["--version"] {
            self.probes.fetch_add(1, Ordering::SeqCst);
            return Ok(ProcessOutput {
                code: Some(0),
                stdout: b"Claude Code 2.1.7\n".to_vec(),
                ..Default::default()
            });
        }
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let coordinate = args.last().unwrap();
        let (name, version) = coordinate.split_once('@').unwrap_or((coordinate, "v1"));
        if args.iter().any(|v| *v == "uninstall" || *v == "remove") {
            let _ = std::fs::remove_file(self.root.join(name));
        } else {
            std::fs::write(self.root.join(name), version).unwrap();
        }
        if let Some(c) = self.cancel.lock().unwrap().take() {
            c.request();
        }
        if self.fail_after.load(Ordering::SeqCst) == n {
            return Err(ProcessError::Wait(
                "fixture response loss after effect".into(),
            ));
        }
        Ok(ProcessOutput {
            code: Some(0),
            stdout: b"not state evidence".to_vec(),
            ..Default::default()
        })
    }
}
struct Fixture {
    _dir: tempfile::TempDir,
    host: Arc<Host>,
    commands: Arc<client_commands::Host>,
    runner: Arc<Runner>,
    control: Arc<Control>,
    observed: Arc<Observed>,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        for p in ["project", "settings", "arena"] {
            std::fs::create_dir(dir.path().join(p)).unwrap();
        }
        let root = dir.path().join("project");
        let executable = root.join("fixture-client");
        std::fs::write(&executable, b"inert").unwrap();
        std::fs::write(root.join("foreign"), b"untouched").unwrap();
        let control = Arc::new(Control(Mutex::new(client_commands::Context {
            executable: ClientExecutable::Resolved {
                command: "claude".into(),
                path: executable,
            },
            cwd: root.clone(),
            user_home: root.clone(),
            roots: vec![root.clone()],
            environment_ref: "env-1".into(),
            principal: "fixture".into(),
            policy: "policy-1".into(),
            evidence: "version-1".into(),
            revoked: false,
        })));
        let runner = Arc::new(Runner {
            root: root.clone(),
            calls: AtomicUsize::new(0),
            probes: AtomicUsize::new(0),
            fail_after: AtomicUsize::new(0),
            cancel: Mutex::new(None),
        });
        let commands = Arc::new(client_commands::Host::new(control.clone(), runner.clone()));
        let observed = Arc::new(Observed {
            root: root.clone(),
            subject: Mutex::new("descriptor-1".into()),
            unavailable: std::sync::atomic::AtomicBool::new(false),
        });
        let host = Arc::new(
            Host::new(
                vibe_safefs::Project::open(&root).unwrap(),
                vibe_safefs::Project::open(&dir.path().join("settings")).unwrap(),
                vibe_safefs::Project::open(&dir.path().join("arena")).unwrap(),
                "fixture-client".into(),
                "fixture-scope".into(),
                commands.clone(),
                observed.clone(),
            )
            .unwrap(),
        );
        Self {
            _dir: dir,
            host,
            commands,
            runner,
            control,
            observed,
        }
    }
    fn plan(&self, action: ReplyAction, version: &str, names: &[&str]) -> Prepared {
        let remove = action == ReplyAction::Remove;
        let commands=names.iter().map(|name|{
   let q=vibe_wire::behaviour::client_command::read_request(&serde_json::to_vec(&serde_json::json!({"schema":1,"client":"claude","version":"2.1.7","contract_version":1,"operation":if remove{"remove"}else{"install"},"coordinate":if remove{name.to_string()}else{format!("{name}@{version}")}})).unwrap()).unwrap();
   Command{prepared:self.commands.prepare(q).unwrap(),resource:name.to_string(),desired:if remove{None}else{Some(hash(version.as_bytes()))},retry_safe:false}
  }).collect();
        self.host.prepare(action, commands, None).unwrap()
    }
    fn apply(
        &self,
        p: &Prepared,
        key: &str,
    ) -> vibe_wire::generated::client_deploy::e1::reply::Reply {
        let g = self.host.approve(p).unwrap();
        let run = self.host.proposed_run(key);
        self.host.initialize(p, &run).unwrap();
        self.host
            .execute(p, Some(&g), &run, false, &Cancellation::default(), CLOCK)
            .unwrap()
    }
    fn count(&self) -> usize {
        self.runner.calls.load(Ordering::SeqCst)
    }
    fn foreign(&self) {
        assert_eq!(
            std::fs::read(self.host.root().join("foreign")).unwrap(),
            b"untouched"
        )
    }
}

use vibe_orchestrator::machine_operation::client_deploy::{
    ClientDeployRoute, DOMAIN_OWNER, OPERATION_ID,
};
fn route(f: &Fixture) -> Arc<ClientDeployRoute> {
    let scope = vibe_lifecycle::SubmissionScope {
        context: "fixture-scope".into(),
        target: "fixture-client".into(),
        principal: "fixture".into(),
    };
    let lease = vibe_lifecycle::LifecycleLease::acquire(f.host.root()).unwrap();
    let namespace =
        vibe_lifecycle::SubmissionReservations::issue_namespace(&lease, &scope, 60000).unwrap();
    drop(lease);
    Arc::new(ClientDeployRoute::new(f.host.clone(), namespace, scope, CLOCK.into()).unwrap())
}
fn request(
    action: &str,
    plan: &str,
    key: &str,
) -> vibe_wire::generated::client_deploy::e1::request::Request {
    vibe_wire::behaviour::client_deploy::read_request(
        &serde_json::to_vec(
            &serde_json::json!({"schema":1,"action":action,"plan_id":plan,"key":key}),
        )
        .unwrap(),
    )
    .unwrap()
}
fn fresh_recovery(f: &Fixture, route: &ClientDeployRoute, plan_id: &str, key: &str) {
    let retained = route.dispatch(&request("inspect", plan_id, key)).unwrap();
    let evidence = f.host.inspect_commands(&retained.run_id).unwrap();
    let commands = evidence
        .into_iter()
        .zip(&retained.steps)
        .map(|(evidence, step)| {
            let q = vibe_wire::behaviour::client_command::read_request(
                &serde_json::to_vec(&evidence.request).unwrap(),
            )
            .unwrap();
            let desired = &retained
                .resources
                .iter()
                .find(|r| r.resource == step.resource)
                .unwrap()
                .desired;
            Command {
                prepared: f.commands.prepare(q).unwrap(),
                resource: step.resource.clone(),
                desired: (desired != "absent").then(|| desired.clone()),
                retry_safe: step.retry_safe,
            }
        })
        .collect();
    let plan = f.host.prepare_recovery(&retained.run_id, commands).unwrap();
    let approval = f.host.approve(&plan).unwrap();
    assert_eq!(route.register(plan, Some(approval)).unwrap(), plan_id);
}
fn register(f: &Fixture, r: &ClientDeployRoute, action: ReplyAction, version: &str) -> String {
    let p = f.plan(action, version, &["a", "b"]);
    let g = f.host.approve(&p).unwrap();
    r.register(p, Some(g)).unwrap()
}
fn envelope(q: &vibe_wire::generated::client_deploy::e1::request::Request) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,"source_id":null,"evidence_id":null,"body":{"stage":"input","format_id":"client-deploy-request","payload_json":serde_json::to_string(q).unwrap()}})).unwrap()
}

use super::*;
#[test]
fn mcp_and_api_all_verbs_have_equal_typed_outcomes() {
    let f = Fixture::new();
    let r = route(&f);
    let (_runner, _, _, _) = connect(&f, r.clone());
    let ctx = crate::ServerContext::with_store_root(f.host.root(), f.host.root().join("unused"))
        .with_client_deploy_route(r.clone())
        .unwrap();
    for (action, verb, version, key) in [
        (ReplyAction::Apply, "apply", "v1", "a"),
        (ReplyAction::Update, "update", "v2", "b"),
        (ReplyAction::Remove, "remove", "", "c"),
    ] {
        let p = register(&f, &r, action, version);
        let plan = request("plan", &p, key);
        let pure = r.dispatch(&plan).unwrap();
        let out = ClientDeployOperationMcpTool
            .run(&serde_json::to_value(plan).unwrap(), &ctx)
            .unwrap();
        assert_eq!(out.structured(), &serde_json::to_value(pure).unwrap());
        let q = request(verb, &p, key);
        let api = r.dispatch(&q).unwrap();
        let out = ClientDeployOperationMcpTool
            .run(&serde_json::to_value(&q).unwrap(), &ctx)
            .unwrap();
        assert_eq!(out.structured(), &serde_json::to_value(api).unwrap());
        for verb in ["verify", "inspect", "recover", "cancel"] {
            let q = request(verb, &p, key);
            let api = r.dispatch(&q).unwrap();
            let out = ClientDeployOperationMcpTool
                .run(&serde_json::to_value(q).unwrap(), &ctx)
                .unwrap();
            assert_eq!(out.structured(), &serde_json::to_value(api).unwrap());
        }
    }
    f.foreign();
    assert!(
        crate::machine_session::client_deploy::ClientDeployMachineSessionRunner::new(ctx).is_ok()
    );
}
#[test]
fn ungranted_and_post_effect_refusals_keep_distinct_boundaries() {
    let f = Fixture::new();
    let r = route(&f);
    let p = register(&f, &r, ReplyAction::Apply, "v1");
    let q = serde_json::to_value(request("apply", &p, "partial")).unwrap();
    let ctx = crate::ServerContext::with_store_root(f.host.root(), f.host.root().join("unused"));
    assert!(ClientDeployOperationMcpTool.run(&q, &ctx).is_err());
    assert_eq!(f.count(), 0);
    let ctx = ctx.with_client_deploy_route(r.clone()).unwrap();
    f.runner.fail_after.store(1, Ordering::SeqCst);
    let out = ClientDeployOperationMcpTool.run(&q, &ctx).unwrap();
    assert!(out.is_error());
    assert_eq!(out.structured()["status"], "partial");
    assert!(!out.structured()["run_id"].as_str().unwrap().is_empty());
    assert_eq!(f.count(), 1);
    let recover = serde_json::to_value(request("recover", &p, "partial")).unwrap();
    let refused = ClientDeployOperationMcpTool.run(&recover, &ctx).unwrap();
    assert_eq!(refused.structured()["status"], "needs_authority");
    let cancelled = ClientDeployOperationMcpTool
        .run(
            &serde_json::to_value(request("cancel", &p, "partial")).unwrap(),
            &ctx,
        )
        .unwrap();
    assert_eq!(cancelled.structured()["status"], "cancelled");
    fresh_recovery(&f, &r, &p, "partial");
    let resumed = ClientDeployOperationMcpTool.run(&recover, &ctx).unwrap();
    assert_eq!(resumed.structured()["status"], "completed");
    assert_eq!(f.count(), 2);
}

use std::io::Cursor;
use vibe_mcp::machine_session::{
    client_deploy::{ClientDeployMachineSessionRunner, ClientDeployRunnerError},
    framing::{read_frame, write_frame},
};
use vibe_wire::generated::machine::e1::{operation_envelope as env, session_frame as frame};
fn send_frame(
    runner: &mut ClientDeployMachineSessionRunner,
    q: &frame::SessionFrame,
) -> frame::SessionFrame {
    let mut input = Vec::new();
    write_frame(&mut input, q).unwrap();
    let mut output = Vec::new();
    runner.step(&mut Cursor::new(input), &mut output).unwrap();
    read_frame(&mut Cursor::new(output)).unwrap().unwrap()
}
fn connect(
    f: &Fixture,
    r: Arc<ClientDeployRoute>,
) -> (ClientDeployMachineSessionRunner, String, String, String) {
    let ctx = vibe_mcp::ServerContext::with_store_root(f.host.root(), f.host.root().join("unused"))
        .with_client_deploy_route(r)
        .unwrap();
    let mut runner = ClientDeployMachineSessionRunner::new(ctx).unwrap();
    let formats = [
        "machine-session-frame",
        "machine-operation-envelope",
        "client-deploy-request",
        "client-deploy-reply",
        "client-deploy-error",
    ];
    let q = frame::SessionFrame {
        schema: 1,
        session_id: None,
        request_id: Some("init".into()),
        body: frame::Body::InitializeOffer(Box::new(frame::BodyInitializeOffer {
            api_versions: vec!["1".into()],
            position_encodings: vec!["raw-byte-offsets".into()],
            schema_offers: formats
                .iter()
                .map(|id| frame::SchemaOffer {
                    format_id: (*id).into(),
                    epoch: 1,
                })
                .collect(),
            requested_capabilities: frame::Capabilities {
                streaming: true,
                replay: true,
                control: true,
            },
        })),
    };
    let reply = send_frame(&mut runner, &q);
    let frame::Body::Initialized(row) = reply.body else {
        panic!("{reply:?}")
    };
    assert_eq!(row.selected_schemas.len(), 5);
    assert_eq!(row.contexts.len(), 1);
    assert_eq!(row.contexts[0].target_ids.len(), 1);
    (
        runner,
        reply.session_id.unwrap(),
        row.contexts[0].context_id.clone(),
        row.contexts[0].target_ids[0].clone(),
    )
}
fn request_frame(
    session: &str,
    context: &str,
    target: &str,
    id: &str,
    q: &vibe_wire::generated::client_deploy::e1::request::Request,
) -> frame::SessionFrame {
    frame::SessionFrame {
        schema: 1,
        session_id: Some(session.into()),
        request_id: Some(id.into()),
        body: frame::Body::Request(Box::new(frame::BodyRequest {
            context_id: context.into(),
            target_id: Some(target.into()),
            envelope_json: String::from_utf8(envelope(q)).unwrap(),
        })),
    }
}
fn published(frame: frame::SessionFrame) -> env::OperationEnvelope {
    let frame::Body::Complete(row) = frame.body else {
        panic!("{frame:?}")
    };
    serde_json::from_str(row.envelope_json.as_deref().unwrap()).unwrap()
}
fn domain(envelope: env::OperationEnvelope) -> serde_json::Value {
    let env::Body::Result(row) = envelope.body else {
        panic!("{envelope:?}")
    };
    serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap()
}
