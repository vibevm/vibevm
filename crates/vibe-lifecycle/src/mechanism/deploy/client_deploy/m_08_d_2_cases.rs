use crate as vibe_lifecycle;

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
                let (path, keys) = if let Some(encoded) = s.strip_prefix("generic:") {
                    let (root, path, keys): (String, String, Vec<String>) =
                        serde_json::from_str(encoded).unwrap();
                    (std::path::Path::new(&root).join(path), keys)
                } else {
                    (self.root.join(s), Vec::new())
                };
                let v = match std::fs::read(path) {
                    Ok(v) => {
                        if keys.is_empty() {
                            Some(hash(&v))
                        } else {
                            let mut value: Option<serde_json::Value> =
                                Some(serde_json::from_slice(&v).unwrap());
                            for key in &keys {
                                value = value.and_then(|v| v.get(key).cloned())
                            }
                            value.map(|v| hash(&serde_json::to_vec(&v).unwrap()))
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => return Err(e.to_string()),
                };
                Ok((s.clone(), v))
            })
            .collect()
    }
}
struct Runner {
    unavailable_after: Mutex<Option<Arc<Observed>>>,
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
        if let Some(observer) = self.unavailable_after.lock().unwrap().take() {
            observer.unavailable.store(true, Ordering::SeqCst);
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
            unavailable_after: Mutex::new(None),
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

#[test]
fn full_apply_verify_update_remove_preserves_foreign_state() {
    composed_files_lifecycle();
    let f = Fixture::new();
    let p = f.plan(ReplyAction::Apply, "v1", &["a", "b"]);
    let probes = f.runner.probes.load(Ordering::SeqCst);
    assert_eq!(f.host.plan(&p).status, ReplyStatus::Planned);
    assert_eq!(f.runner.probes.load(Ordering::SeqCst), probes);
    let applied = f.apply(&p, "first");
    assert_eq!(applied.status, ReplyStatus::Completed);
    assert_eq!(
        f.host.verify(&applied.run_id).unwrap().status,
        ReplyStatus::Completed
    );
    let update = f.plan(ReplyAction::Update, "v2", &["a", "b"]);
    assert_eq!(f.apply(&update, "second").status, ReplyStatus::Completed);
    let remove = f.plan(ReplyAction::Remove, "", &["a", "b"]);
    assert_eq!(f.apply(&remove, "third").status, ReplyStatus::Completed);
    assert!(!f.host.root().join("a").exists());
    f.foreign();
}
#[test]
fn interrupted_command_prefix_recovers_with_fresh_authority() {
    uncertain_observed_prefix_and_unsupported_retry();
    let f = Fixture::new();
    let p = f.plan(ReplyAction::Apply, "v1", &["a", "b"]);
    f.runner.fail_after.store(1, Ordering::SeqCst);
    let partial = f.apply(&p, "interrupted");
    assert_eq!(partial.status, ReplyStatus::Partial);
    assert_eq!(partial.steps[0].status, StepStatus::Completed);
    assert_eq!(f.count(), 1);
    assert_eq!(
        f.host
            .execute(
                &p,
                None,
                &partial.run_id,
                true,
                &Cancellation::default(),
                CLOCK
            )
            .unwrap()
            .status,
        ReplyStatus::NeedsAuthority
    );
    let (cold, p) = cold_recovery(&f, &p, &partial.run_id);
    let grant = cold.approve(&p).unwrap();
    let resumed = cold
        .execute(
            &p,
            Some(&grant),
            &partial.run_id,
            true,
            &Cancellation::default(),
            CLOCK,
        )
        .unwrap();
    assert_eq!(resumed.status, ReplyStatus::Completed);
    assert_eq!(resumed.run_id, partial.run_id);
    assert_eq!(f.count(), 2);
    f.foreign();
}
#[test]
fn changed_executable_descriptor_and_expanded_plan_refuse_stale_grant() {
    stale_file_approval_blocks_commands();
    let foreign = Fixture::new();
    std::fs::write(foreign.host.root().join("a"), b"foreign-owned").unwrap();
    let plan = foreign.plan(ReplyAction::Apply, "v1", &["a"]);
    assert_eq!(foreign.apply(&plan, "foreign").status, ReplyStatus::Refused);
    assert_eq!(foreign.count(), 0);
    let g = foreign.host.approve(&plan).unwrap();
    assert_eq!(
        foreign
            .host
            .execute(
                &plan,
                Some(&g),
                &foreign.host.proposed_run("foreign"),
                true,
                &Cancellation::default(),
                CLOCK
            )
            .unwrap()
            .status,
        ReplyStatus::Refused
    );
    assert_eq!(foreign.count(), 0);
    assert_eq!(
        std::fs::read(foreign.host.root().join("a")).unwrap(),
        b"foreign-owned"
    );
    for changed in ["executable", "descriptor", "expansion", "policy"] {
        let f = Fixture::new();
        let p = f.plan(ReplyAction::Apply, "v1", &["a"]);
        let g = f.host.approve(&p).unwrap();
        let mut candidate = None;
        match changed {
            "executable" => {
                std::fs::write(f.host.root().join("fixture-client"), b"changed").unwrap()
            }
            "descriptor" => *f.observed.subject.lock().unwrap() = "descriptor-2".into(),
            "expansion" => candidate = Some(f.plan(ReplyAction::Apply, "v1", &["a", "b"])),
            _ => f.control.0.lock().unwrap().policy = "policy-2".into(),
        }
        let selected = candidate.as_ref().unwrap_or(&p);
        let run = f.host.proposed_run(changed);
        f.host.initialize(selected, &run).unwrap();
        let r = f
            .host
            .execute(
                selected,
                Some(&g),
                &run,
                false,
                &Cancellation::default(),
                CLOCK,
            )
            .unwrap();
        assert_eq!(r.status, ReplyStatus::NeedsAuthority);
        assert_eq!(f.count(), 0);
    }
}
#[test]
fn cancel_boundaries_and_post_effect_error_retain_owned_prefix() {
    for before in [true, false] {
        let f = Fixture::new();
        let p = f.plan(ReplyAction::Apply, "v1", &["a", "b"]);
        let c = Cancellation::default();
        if before {
            c.request()
        } else {
            *f.runner.cancel.lock().unwrap() = Some(c.clone())
        }
        let g = f.host.approve(&p).unwrap();
        let run = f.host.proposed_run("cancel");
        f.host.initialize(&p, &run).unwrap();
        let r = f
            .host
            .execute(&p, Some(&g), &run, false, &c, CLOCK)
            .unwrap();
        assert_eq!(r.status, ReplyStatus::Cancelled);
        assert_eq!(f.count(), if before { 0 } else { 1 });
        assert_eq!(f.host.inspect(&run).unwrap().unwrap().0, r);
        f.foreign();
    }
    let f = Fixture::new();
    let p = f.plan(ReplyAction::Apply, "v1", &["a", "b"]);
    f.runner.fail_after.store(1, Ordering::SeqCst);
    let r = f.apply(&p, "failure");
    assert_eq!(r.status, ReplyStatus::Partial);
    assert_eq!(r.steps[0].status, StepStatus::Completed);
    assert_eq!(r.steps[1].status, StepStatus::Pending);
}

struct FileControl {
    selected: vibe_lifecycle::generic::SelectedObservation,
    policy: Mutex<String>,
}
impl vibe_lifecycle::generic::GenericAuthority for FileControl {
    fn selected_current(
        &self,
        _: &str,
    ) -> Result<vibe_lifecycle::generic::SelectedObservation, String> {
        Ok(self.selected.clone())
    }
    fn current_control(&self) -> Result<vibe_lifecycle::generic::ControlObservation, String> {
        Ok(vibe_lifecycle::generic::ControlObservation {
            principal: "fixture".into(),
            policy_generation: self.policy.lock().unwrap().clone(),
        })
    }
}
fn file_host(f: &Fixture) -> (Arc<vibe_lifecycle::generic::GenericHost>, Arc<FileControl>) {
    use vibe_core::manifest::*;
    use vibe_lifecycle::generic::*;
    f.host.prepare_storage().unwrap();
    let user = f._dir.path().join("user");
    if !user.exists() {
        std::fs::create_dir(&user).unwrap();
        std::fs::write(f.host.root().join("owned.json"), br#"{"foreign":7}"#).unwrap();
    }
    let request = ClientRequest {
        schema: 1,
        client_id: "org.example/fixture".into(),
        version: "*".into(),
        bindings: vec![ClientBinding {
            scope: ClientScope::Project,
            shape: ClientShape::JsonMember,
            destination: "owned.json".into(),
            key: vec!["owned".into()],
            operations: vec![ClientOperation::ReconcileJsonMember],
        }],
    };
    let control = Arc::new(FileControl {
        selected: SelectedObservation {
            identity: SourceIdentity {
                provider: "org.example/provider#client".into(),
                provider_version: "1.0.0".into(),
                provider_digest: "a".repeat(64),
                declaration: "fixture".into(),
                declaration_digest: "b".repeat(64),
                descriptor: "fixture".into(),
                descriptor_digest: "c".repeat(64),
                source: "fixture".into(),
                source_digest: "d".repeat(64),
            },
            request,
            selected: true,
            enabled: true,
        },
        policy: Mutex::new("policy-1".into()),
    });
    let category = |name: &str| {
        vec![
            ProtectedResource {
                scope: ClientScope::Project,
                relative: format!("controls/{name}"),
            },
            ProtectedResource {
                scope: ClientScope::User,
                relative: format!("controls/{name}"),
            },
        ]
    };
    let host = GenericHost::new(
        vibe_safefs::Project::open(f.host.root()).unwrap(),
        vibe_safefs::Project::open(&user).unwrap(),
        vibe_safefs::Project::open(&f._dir.path().join("settings")).unwrap(),
        control.clone(),
        ProtectedControls {
            grants: category("grants"),
            policy: category("policy"),
            producer_trust: category("trust"),
            publication_mode: category("publication"),
        },
    )
    .unwrap()
    .with_coordination(vibe_safefs::Project::open(&f._dir.path().join("arena")).unwrap())
    .unwrap();
    (Arc::new(host), control)
}
fn file_admitted(
    host: &vibe_lifecycle::generic::GenericHost,
    control: &FileControl,
    value: &str,
) -> vibe_lifecycle::generic::AdmittedClient {
    use vibe_agent_projection::client_capabilities::{ClientCapabilities, ClientCapability};
    let row = &control.selected;
    let map = ClientCapabilities::admit([ClientCapability {
        provider: row.identity.provider.clone(),
        descriptor_digest: row.identity.descriptor_digest.clone(),
        request: row.request.clone(),
        disabled: false,
    }])
    .unwrap();
    host.admit_payload(
        host.capture(&map, "org.example/fixture").unwrap(),
        vec![vibe_lifecycle::generic::GenericPayload::JsonMember(
            serde_json::json!(value),
        )],
    )
    .unwrap()
}
fn composed_files_lifecycle() {
    let f = Fixture::new();
    let (files, control) = file_host(&f);
    let mut target = String::new();
    for (action, version, key) in [
        (ReplyAction::Apply, "v1", "files-1"),
        (ReplyAction::Update, "v2", "files-2"),
        (ReplyAction::Remove, "", "files-3"),
    ] {
        let fp = if action == ReplyAction::Remove {
            super::Files::Retained {
                host: files.clone(),
                plan: files
                    .prepare_retained(&target, vibe_lifecycle::generic::RetainedOperation::Cleanup)
                    .unwrap(),
            }
        } else {
            let plan = files
                .plan(&file_admitted(&files, &control, version))
                .unwrap();
            target = plan.target().into();
            super::Files::Apply {
                host: files.clone(),
                plan,
            }
        };
        let cp = f.plan(action.clone(), version, &["a", "b"]);
        let p = f.host.prepare(action, cp.commands, Some(fp)).unwrap();
        let r = f.apply(&p, key);
        assert_eq!(r.status, ReplyStatus::Completed, "{r:?}");
        assert_eq!(r.resources.len(), 3);
        let actual: serde_json::Value =
            serde_json::from_slice(&std::fs::read(f.host.root().join("owned.json")).unwrap())
                .unwrap();
        assert_eq!(actual["foreign"], 7);
        if key == "files-3" {
            assert!(actual.get("owned").is_none())
        } else {
            assert_eq!(actual["owned"], version)
        }
        f.foreign();
    }
}
fn stale_file_approval_blocks_commands() {
    for mode in ["missing", "wrong", "stale"] {
        let f = Fixture::new();
        let (files, control) = file_host(&f);
        let fp = files.plan(&file_admitted(&files, &control, "v1")).unwrap();
        let cp = f.plan(ReplyAction::Apply, "v1", &["a"]);
        let p = f
            .host
            .prepare(
                ReplyAction::Apply,
                cp.commands,
                Some(super::Files::Apply {
                    host: files.clone(),
                    plan: fp,
                }),
            )
            .unwrap();
        let mut g = f.host.approve(&p).unwrap();
        match mode {
            "missing" => g.files = None,
            "wrong" => {
                let wrong = files.plan(&file_admitted(&files, &control, "v2")).unwrap();
                g.files = Some(super::FileGrant::Apply(files.approve(&wrong).unwrap()));
            }
            _ => *control.policy.lock().unwrap() = "policy-2".into(),
        }
        let run = f.host.proposed_run("stale-files");
        f.host.initialize(&p, &run).unwrap();
        let r = f
            .host
            .execute(&p, Some(&g), &run, false, &Cancellation::default(), CLOCK)
            .unwrap();
        assert_eq!(r.status, ReplyStatus::NeedsAuthority);
        assert_eq!(f.count(), 0);
        assert_eq!(
            std::fs::read(f.host.root().join("owned.json")).unwrap(),
            br#"{"foreign":7}"#
        );
    }
}

fn cold_recovery(f: &Fixture, p: &Prepared, run: &str) -> (Host, Prepared) {
    cold_recovery_files(f, p, run, None)
}
fn cold_recovery_files(
    f: &Fixture,
    p: &Prepared,
    run: &str,
    files: Option<super::Files>,
) -> (Host, Prepared) {
    let probes = f.runner.probes.load(Ordering::SeqCst);
    assert_eq!(
        f.host.inspect_commands(run).unwrap().len(),
        p.commands.len()
    );
    assert_eq!(f.runner.probes.load(Ordering::SeqCst), probes);
    let commands = Arc::new(client_commands::Host::new(
        f.control.clone(),
        f.runner.clone(),
    ));
    let fresh = p
        .commands
        .iter()
        .map(|c| {
            let request = vibe_wire::behaviour::client_command::read_request(
                &serde_json::to_vec(&c.prepared.inspect().request).unwrap(),
            )
            .unwrap();
            Command {
                prepared: commands.prepare(request).unwrap(),
                resource: c.resource.clone(),
                desired: c.desired.clone(),
                retry_safe: c.retry_safe,
            }
        })
        .collect();
    let host = Host::new(
        vibe_safefs::Project::open(f.host.root()).unwrap(),
        vibe_safefs::Project::open(&f._dir.path().join("settings")).unwrap(),
        vibe_safefs::Project::open(&f._dir.path().join("arena")).unwrap(),
        "fixture-client".into(),
        "fixture-scope".into(),
        commands,
        f.observed.clone(),
    )
    .unwrap();
    let plan = host.prepare_recovery_with_files(run, fresh, files).unwrap();
    (host, plan)
}

#[test]
fn identical_dto_foreign_prepared_grant_refuses_before_files() {
    let f = Fixture::new();
    let (files, control) = file_host(&f);
    let prepare = || {
        let cp = f.plan(ReplyAction::Apply, "v1", &["a", "b"]);
        f.host
            .prepare(
                ReplyAction::Apply,
                cp.commands,
                Some(super::Files::Apply {
                    host: files.clone(),
                    plan: files.plan(&file_admitted(&files, &control, "v1")).unwrap(),
                }),
            )
            .unwrap()
    };
    let first = prepare();
    let stale = f.host.approve(&first).unwrap();
    let second = prepare();
    assert_eq!(first.id, second.id);
    let run = f.host.proposed_run("opaque-grant");
    f.host.initialize(&second, &run).unwrap();
    let refused = f
        .host
        .execute(
            &second,
            Some(&stale),
            &run,
            false,
            &Cancellation::default(),
            CLOCK,
        )
        .unwrap();
    assert_eq!(refused.status, ReplyStatus::NeedsAuthority);
    assert_eq!(f.count(), 0);
    assert_eq!(
        std::fs::read(f.host.root().join("owned.json")).unwrap(),
        br#"{"foreign":7}"#
    );
    let fresh = f.host.approve(&second).unwrap();
    assert_eq!(
        f.host
            .execute(
                &second,
                Some(&fresh),
                &run,
                false,
                &Cancellation::default(),
                CLOCK
            )
            .unwrap()
            .status,
        ReplyStatus::Completed
    );
    assert_eq!(f.count(), 2);
    f.foreign();
}

#[test]
fn repeated_cold_combined_recovery_preserves_original_c3_subject() {
    let f = Fixture::new();
    let (files, control) = file_host(&f);
    let fp = files.plan(&file_admitted(&files, &control, "v1")).unwrap();
    let subject = fp.identity();
    let target = fp.target().to_owned();
    let cp = f.plan(ReplyAction::Apply, "v1", &["a", "b"]);
    let p = f
        .host
        .prepare(
            ReplyAction::Apply,
            cp.commands,
            Some(super::Files::Apply {
                host: files,
                plan: fp,
            }),
        )
        .unwrap();
    f.runner.fail_after.store(1, Ordering::SeqCst);
    let partial = f.apply(&p, "combined-recovery");
    assert_eq!(partial.status, ReplyStatus::Partial);
    for cancel in [true, false] {
        let (files, control) = file_host(&f);
        let admitted = file_admitted(&files, &control, "v1");
        let fp = files.prepare_recovery(&target, Some(&admitted)).unwrap();
        assert_ne!(fp.identity(), subject);
        let (cold, plan) = cold_recovery_files(
            &f,
            &p,
            &partial.run_id,
            Some(super::Files::Retained {
                host: files,
                plan: fp,
            }),
        );
        let grant = cold.approve(&plan).unwrap();
        let cancellation = Cancellation::default();
        if cancel {
            cancellation.request();
        }
        let reply = cold
            .execute(
                &plan,
                Some(&grant),
                &partial.run_id,
                true,
                &cancellation,
                CLOCK,
            )
            .unwrap();
        assert_eq!(
            reply.status,
            if cancel {
                ReplyStatus::Cancelled
            } else {
                ReplyStatus::Completed
            }
        );
        assert_eq!(reply.run_id, partial.run_id);
        let state = super::DeployState::observe_bound(&cold.settings)
            .unwrap()
            .unwrap();
        assert_eq!(
            state
                .read_client(&cold.home(), &partial.run_id)
                .unwrap()
                .unwrap()
                .file_subject,
            Some(subject.clone())
        );
        assert_eq!(f.count(), if cancel { 1 } else { 2 });
    }
    let actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.host.root().join("owned.json")).unwrap()).unwrap();
    assert_eq!(actual, serde_json::json!({"foreign":7,"owned":"v1"}));
    f.foreign();
}

#[test]
fn receipt_publication_crash_recovers_same_run_and_refuses_foreign_join() {
    for stage in [1, 2, 3] {
        for partial in [false, true] {
            for foreign in ["none", "receipt", "sidecar", "intent"] {
                let f = Fixture::new();
                if partial {
                    f.runner.fail_after.store(1, Ordering::SeqCst);
                }
                let p = f.plan(ReplyAction::Apply, "v1", &["a", "b"]);
                let grant = f.host.approve(&p).unwrap();
                let run = f.host.proposed_run("publication-crash");
                f.host.initialize(&p, &run).unwrap();
                super::journal::FINALIZATION_FAULT.with(|fault| fault.set(stage));
                assert!(
                    f.host
                        .execute(
                            &p,
                            Some(&grant),
                            &run,
                            false,
                            &Cancellation::default(),
                            CLOCK
                        )
                        .unwrap_err()
                        .contains("interrupted finalization")
                );
                assert_eq!(f.count(), if partial { 1 } else { 2 });
                let (cold, plan) = cold_recovery(&f, &p, &run);
                let (observed, published) = cold.inspect(&run).unwrap().unwrap();
                assert!(!published);
                assert_eq!(
                    observed.status,
                    if partial {
                        ReplyStatus::Partial
                    } else {
                        ReplyStatus::Completed
                    }
                );
                if partial && foreign == "none" {
                    assert!(cold.cancel_at_boundary(&run).unwrap().cancel_pending);
                    assert!(cold.inspect(&run).unwrap().unwrap().0.cancel_pending);
                }
                if foreign != "none" {
                    let state = super::DeployState::observe_bound(&cold.settings)
                        .unwrap()
                        .unwrap();
                    match foreign {
                        "receipt" => {
                            let mut receipt = state.read_receipt(&cold.home()).unwrap().unwrap();
                            receipt.evidence = Some("foreign receipt".into());
                            state.write_receipt(&cold.home(), &receipt).unwrap();
                        }
                        "sidecar" => {
                            let mut sidecar =
                                state.read_lock_resources(&cold.home()).unwrap().unwrap();
                            sidecar
                                .pending
                                .as_mut()
                                .or(sidecar.committed.as_mut())
                                .unwrap()
                                .plan_hash = "f".repeat(64);
                            state.write_lock_resources(&cold.home(), &sidecar).unwrap();
                        }
                        _ => {
                            let intent = serde_json::from_value(serde_json::json!({"schema":1,"plan_hash":p.id,"resources":[],"target":{"project":cold.roots[0],"profile":"client-deploy","target":cold.target,"generation":1},"started_at":CLOCK})).unwrap();
                            state.write_intent(&cold.home(), &intent).unwrap();
                        }
                    }
                }
                let grant = cold.approve(&plan).unwrap();
                let result = cold
                    .execute(
                        &plan,
                        Some(&grant),
                        &run,
                        true,
                        &Cancellation::default(),
                        CLOCK,
                    )
                    .unwrap();
                assert_eq!(
                    result.status,
                    if foreign != "none" {
                        ReplyStatus::Refused
                    } else {
                        ReplyStatus::Completed
                    }
                );
                assert_eq!(result.run_id, run);
                assert_eq!(f.count(), if partial && foreign != "none" { 1 } else { 2 });
                assert_eq!(cold.inspect(&run).unwrap().unwrap().1, foreign == "none");
                f.foreign();
            }
        }
    }
}
fn uncertain_observed_prefix_and_unsupported_retry() {
    for landed in [true, false] {
        let f = Fixture::new();
        let p = f.plan(ReplyAction::Apply, "v1", &["a", "b"]);
        *f.runner.unavailable_after.lock().unwrap() = Some(f.observed.clone());
        f.runner.fail_after.store(1, Ordering::SeqCst);
        let partial = f.apply(&p, "uncertain");
        assert_eq!(partial.status, ReplyStatus::Unknown);
        assert_eq!(partial.steps[0].status, StepStatus::Uncertain);
        assert_eq!(f.count(), 1);
        f.observed.unavailable.store(false, Ordering::SeqCst);
        if !landed {
            std::fs::remove_file(f.host.root().join("a")).unwrap();
        }
        let (cold, p) = cold_recovery(&f, &p, &partial.run_id);
        let approval = cold.approve(&p).unwrap();
        let result = cold
            .execute(
                &p,
                Some(&approval),
                &partial.run_id,
                true,
                &Cancellation::default(),
                CLOCK,
            )
            .unwrap();
        assert_eq!(
            result.status,
            if landed {
                ReplyStatus::Completed
            } else {
                ReplyStatus::Refused
            }
        );
        assert_eq!(f.count(), if landed { 2 } else { 1 });
        f.foreign();
    }
}
