use crate as vibe_orchestrator;

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

#[test]
fn response_loss_reconnects_same_operation_without_duplicate_effect() {
    let f = Fixture::new();
    let r = route(&f);
    let p = register(&f, &r, ReplyAction::Apply, "v1");
    let q = request("apply", &p, "key");
    let first = r.dispatch(&q).unwrap();
    assert_eq!(first.status, ReplyStatus::Completed);
    assert_eq!(r.dispatch(&q).unwrap(), first);
    assert_eq!(r.dispatch(&request("inspect", &p, "key")).unwrap(), first);
    assert_eq!(f.count(), 2);
}
#[test]
fn changed_payload_same_key_and_foreign_scope_refuse() {
    let f = Fixture::new();
    let r = route(&f);
    let p = register(&f, &r, ReplyAction::Apply, "v1");
    r.dispatch(&request("apply", &p, "key")).unwrap();
    let update = register(&f, &r, ReplyAction::Update, "v2");
    assert!(r.dispatch(&request("update", &update, "key")).is_err());
    let foreign = tempfile::tempdir().unwrap();
    assert!(r.compare_context_root(foreign.path()).is_err());
    let mut q = request("inspect", &p, "key");
    q.run_id = Some("foreign".into());
    assert!(r.dispatch(&q).is_err());
    assert_eq!(f.count(), 2);
}
#[test]
fn all_verbs_publish_exact_owner_resource_authority_and_partial_status() {
    let f = Fixture::new();
    let r = route(&f);
    let unprepared = r.dispatch(&request("plan", "unprepared", "key")).unwrap();
    assert_eq!(unprepared.status, ReplyStatus::NeedsPreparation);
    assert!(unprepared.resources.is_empty());
    assert_eq!(f.count(), 0);
    let p = register(&f, &r, ReplyAction::Apply, "v1");
    assert_eq!(
        r.dispatch(&request("plan", &p, "key"))
            .unwrap()
            .resources
            .len(),
        2
    );
    f.runner.fail_after.store(1, Ordering::SeqCst);
    let partial = r.dispatch(&request("apply", &p, "key")).unwrap();
    assert_eq!(partial.status, ReplyStatus::Partial);
    assert_eq!(partial.resources.len(), 2);
    assert_eq!(
        r.dispatch(&request("recover", &p, "key")).unwrap().status,
        ReplyStatus::NeedsAuthority
    );
    fresh_recovery(&f, &r, &p, "key");
    assert_eq!(
        r.dispatch(&request("recover", &p, "key")).unwrap().status,
        ReplyStatus::Completed
    );
    assert_eq!(
        r.dispatch(&request("verify", &p, "key")).unwrap().status,
        ReplyStatus::Completed
    );
    for (action, verb, version, key) in [
        (ReplyAction::Update, "update", "v2", "update"),
        (ReplyAction::Remove, "remove", "", "remove"),
    ] {
        let p = register(&f, &r, action, version);
        assert_eq!(
            r.dispatch(&request(verb, &p, key)).unwrap().status,
            ReplyStatus::Completed
        );
    }
    f.foreign();
}
