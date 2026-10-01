//! Linux native process evidence; Windows/macOS native cases remain unrun.
//! All IPC below is a bounded test fixture, not a product transport or schema.
use super::*;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Instant;

const ROLE: &str = "VIBEVM_LIVE_OWNER_PROCESS_TEST_ROLE";
const ROOT: &str = "VIBEVM_LIVE_OWNER_PROCESS_TEST_ROOT";
const PREFIX: &str = "machine_submission::lifecycle_adapter::live_owner::process_tests::";
const LIMIT: usize = 64 * 1024;
const WAIT: Duration = Duration::from_secs(15);

fn bounded_read(path: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= LIMIT, "fixture exceeds byte bound");
    bytes
}
fn wait_until(label: &str, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !predicate() {
        assert!(Instant::now() < deadline, "deadline: {label}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn wait_child(child: &mut Child) -> ExitStatus {
    let mut status = None;
    wait_until("owned child exit", || {
        status = child.try_wait().unwrap();
        status.is_some()
    });
    status.unwrap()
}

// Drop releases the real script even when an assertion unwinds. Only an owned
// Child is killed/waited. The provider has its own finite gate deadline too.
struct Fixture {
    dir: tempfile::TempDir,
    child: Option<Child>,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), ONE_SCRIPT_VALIDATE_MANIFEST).unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        let script = format!(
            "#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\nprintf 'entered\\n' > entered\ni=0\nwhile [ ! -f release ]; do\n  i=$((i + 1))\n  if [ \"$i\" -ge 600 ]; then printf 'timeout\\n' > finished; exit 70; fi\n  sleep 0.05\ndone\nprintf 'finished\\n' > finished\n"
        );
        fs::write(dir.path().join("run.sh"), script).unwrap();
        Self { dir, child: None }
    }
    fn root(&self) -> &Path {
        self.dir.path()
    }
    fn entered(&self) {
        wait_until("real script entered", || {
            self.root().join("entered").is_file()
        });
        assert_eq!(bounded_read(&self.root().join("entered")), b"entered\n");
        self.count_one();
    }
    fn count_one(&self) {
        assert_eq!(bounded_read(&self.root().join("counter")), b"effect\n");
    }
    fn release(&self) {
        fs::write(self.root().join("release"), b"release\n").unwrap();
    }
    fn finished(&self) {
        wait_until("real script finished", || {
            self.root().join("finished").is_file()
        });
        assert_eq!(bounded_read(&self.root().join("finished")), b"finished\n");
        self.count_one();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::write(self.root().join("release"), b"cleanup\n");
        if let Some(child) = self.child.as_mut() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        // No cleanup assertion during unwinding; keep the root until the
        // provider's explicit finish, or its independently bounded deadline.
        let deadline = Instant::now() + Duration::from_secs(35);
        while self.root().join("entered").exists()
            && !self.root().join("finished").exists()
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
fn coordinates(root: &Path) -> HostCoordinates {
    let scope = SubmissionScope {
        context: "process-proof-context".into(),
        target: "root".into(),
        principal: "host-owner".into(),
    };
    let namespace = {
        let lease = LifecycleLease::acquire(root).unwrap();
        SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap()
    };
    HostCoordinates {
        root: root.to_owned(),
        namespace,
        scope,
        key: "real-once".into(),
    }
}
fn begin(c: &HostCoordinates) -> (LiveOwner, Observer, Binding) {
    let (owner, observer) = start(
        c.root.clone(),
        c.namespace.clone(),
        c.scope.clone(),
        c.key.clone(),
        DefaultLifecycleRequest {
            requested: Phase::Validate,
            force: false,
            agent_mode: RunAgentMode::Cli,
            assume_yes: true,
            trace_flag: false,
            install_inputs: Default::default(),
            policy: Default::default(),
        },
        TargetOs::Linux,
        "2026-09-30T00:00:05Z".parse().unwrap(),
    )
    .unwrap();
    let binding = owner
        .wait_ready(WAIT)
        .expect("actual production Ready binding");
    assert_eq!(
        owner.policy(),
        ConnectionLossPolicy::ContinueCurrentDispatch
    );
    assert_eq!(
        observer.inspect().unwrap(),
        Observation::Running {
            run_id: binding.run_id.clone()
        }
    );
    let lease = owner.shared.record.lock().unwrap().lease.clone().unwrap();
    let row = SubmissionReservations::inspect_existing_key(&lease, &c.namespace, &c.scope, &c.key)
        .unwrap()
        .unwrap();
    assert_eq!(row.journal_admission, JournalAdmission::Ready);
    assert_eq!(row.domain_run_id, binding.run_id);
    assert_eq!(row.reservation_ref.as_ref(), Some(&binding.reservation_ref));
    assert_eq!(row.request_sha256, binding.request_sha256);
    assert_eq!(row.plan_sha256, binding.plan_sha256);
    (owner, observer, binding)
}
fn busy(root: &Path) {
    assert!(matches!(
        LifecycleLease::acquire(root),
        Err(vibe_lifecycle::LifecycleLeaseError::Busy { .. })
    ));
}
fn helper_command(name: &str, role: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &format!("{PREFIX}{name}"), "--nocapture"])
        .env(ROLE, role)
        .env_remove(ROOT)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[test]
fn real_script_survives_observation_client_process_exit_and_detach() {
    let mut f = Fixture::new();
    let c = coordinates(f.root());
    let (owner, observer, binding) = begin(&c);
    f.entered();
    let mut client_observer = owner.attach(&binding, &c).unwrap();
    let Observation::Running { run_id } = client_observer.inspect().unwrap() else {
        panic!("client observation must come from the actual private attach");
    };
    // The client gets observation metadata only, never root/scope/key or a
    // Binding. Exiting this distinct process disconnects its host subscription.
    let metadata = format!("Running\n{run_id}\n");
    assert!(metadata.len() < 4096);
    f.child = Some(
        helper_command("observation_client_helper", "client")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let child = f.child.as_mut().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(metadata.as_bytes())
        .unwrap();
    assert!(
        wait_child(child).success(),
        "actual observation client must exit successfully"
    );
    let mut ack = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_string(&mut ack)
        .unwrap();
    assert!(ack.len() <= 4096);
    assert!(
        ack.lines()
            .any(|line| line == format!("OBSERVATION-READ:{run_id}")),
        "exact helper must acknowledge the captured observation"
    );
    assert!(client_observer.detach());
    drop(observer);
    assert!(owner.shared.record.lock().unwrap().subscriptions.is_empty());
    assert_eq!(owner.state(), LiveState::Running);
    busy(f.root());
    f.count_one();
    assert!(!f.root().join("finished").exists());
    f.release();
    f.finished();
    let record = owner.shared.record.lock().unwrap();
    let (record, timeout) = owner
        .shared
        .changed
        .wait_timeout_while(record, WAIT, |r| {
            matches!(r.state, LiveState::Starting | LiveState::Running)
        })
        .unwrap();
    assert!(!timeout.timed_out(), "real owner must reach terminal state");
    drop(record);
    owner.worker.lock().unwrap().take().unwrap().join().unwrap();
    let attached = owner.attach(&binding, &c).unwrap();
    let Observation::ResultAvailable { run_id, result } = attached.inspect().unwrap() else {
        panic!("actual domain marker/report required");
    };
    assert_eq!(run_id, binding.run_id);
    assert_eq!(result.marker.outcome, MarkerOutcome::Completed);
    assert!(result.report_bytes.is_some());
    // Writer freedom is established while owner/binding/observers remain alive.
    let lease = LifecycleLease::acquire(f.root()).unwrap();
    let request = owner
        .shared
        .record
        .lock()
        .unwrap()
        .canonical_request
        .clone();
    assert_eq!(
        attach_existing_lifecycle(&lease, &c.namespace, &c.scope, &c.key, &request).unwrap(),
        Some(SubmissionOutcome::Replayed {
            run_id,
            outcome: result
        })
    );
    f.count_one();
}

#[test]
fn observation_client_helper() {
    if std::env::var(ROLE).as_deref() != Ok("client") {
        return;
    }
    assert!(
        std::env::var_os(ROOT).is_none(),
        "observation client receives no root"
    );
    let mut metadata = String::new();
    std::io::stdin()
        .take(4097)
        .read_to_string(&mut metadata)
        .unwrap();
    assert!(metadata.len() < 4096);
    let lines: Vec<_> = metadata.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Running");
    assert!(!lines[1].is_empty() && lines[1].len() <= 256);
    println!("OBSERVATION-READ:{}", lines[1]);
}

// Captured trusted coordinates/request, not a grant or public wire value.
struct Ticket {
    root: PathBuf,
    namespace: String,
    context: String,
    target: String,
    principal: String,
    key: String,
    run_id: String,
    reservation_ref: String,
    request: Vec<u8>,
}
#[test]
fn owning_host_helper() {
    if std::env::var(ROLE).as_deref() != Ok("host") {
        return;
    }
    let root = PathBuf::from(std::env::var_os(ROOT).expect("explicit isolated fixture root"));
    let c = coordinates(&root);
    let (owner, observer, binding) = begin(&c);
    wait_until("host sees counted provider", || {
        root.join("entered").is_file()
    });
    assert_eq!(bounded_read(&root.join("counter")), b"effect\n");
    let ticket = Ticket {
        root: c.root.clone(),
        namespace: c.namespace.id().into(),
        context: c.scope.context.clone(),
        target: c.scope.target.clone(),
        principal: c.scope.principal.clone(),
        key: c.key.clone(),
        run_id: binding.run_id.clone(),
        reservation_ref: binding.reservation_ref.clone(),
        request: owner
            .shared
            .record
            .lock()
            .unwrap()
            .canonical_request
            .clone(),
    };
    let bytes = serde_json::to_vec(&(
        &ticket.root,
        &ticket.namespace,
        &ticket.context,
        &ticket.target,
        &ticket.principal,
        &ticket.key,
        &ticket.run_id,
        &ticket.reservation_ref,
        &ticket.request,
    ))
    .unwrap();
    assert!(bytes.len() <= LIMIT);
    fs::write(root.join("ticket.pending"), bytes).unwrap();
    fs::rename(root.join("ticket.pending"), root.join("ticket")).unwrap();
    // Bounded park: parent kills this exact child before releasing the script.
    wait_until("host fixture stop", || root.join("host-stop").is_file());
    drop((owner, observer, binding));
}

#[test]
fn real_owning_host_death_retains_same_run_recovery_without_redispatch() {
    let mut f = Fixture::new();
    f.child = Some(
        helper_command("owning_host_helper", "host")
            .env(ROOT, f.root())
            .stdin(Stdio::null())
            .spawn()
            .unwrap(),
    );
    wait_until("host captured Ready ticket", || {
        f.root().join("ticket").is_file()
    });
    let (root, namespace, context, target, principal, key, run_id, reservation_ref, request): (
        PathBuf,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Vec<u8>,
    ) = serde_json::from_slice(&bounded_read(&f.root().join("ticket"))).unwrap();
    let ticket = Ticket {
        root,
        namespace,
        context,
        target,
        principal,
        key,
        run_id,
        reservation_ref,
        request,
    };
    assert_eq!(ticket.root, f.root());
    let namespace = SubmissionNamespace::from_host_token(ticket.namespace.clone()).unwrap();
    let scope = SubmissionScope {
        context: ticket.context,
        target: ticket.target,
        principal: ticket.principal,
    };
    f.entered();
    busy(f.root());
    let marker = f
        .root()
        .join(".vibe/lifecycle-outcomes")
        .join(format!("{}.marker.json", ticket.run_id));
    assert!(!marker.exists());
    assert!(!f.root().join("finished").exists());
    let host = f.child.as_mut().unwrap();
    assert!(
        host.try_wait().unwrap().is_none(),
        "exact owning host still alive"
    );
    host.kill().unwrap();
    let status = wait_child(host);
    assert_eq!(
        status.signal(),
        Some(9),
        "actual owned host killed, not a panic/EOF inference"
    );
    let lease =
        LifecycleLease::acquire(f.root()).expect("host death releases original writer lease");
    let row = SubmissionReservations::inspect_existing_key(&lease, &namespace, &scope, &ticket.key)
        .unwrap()
        .unwrap();
    assert_eq!(row.domain_run_id, ticket.run_id);
    assert_eq!(row.journal_admission, JournalAdmission::Ready);
    assert_eq!(
        row.reservation_ref.as_deref(),
        Some(ticket.reservation_ref.as_str())
    );
    assert!(
        matches!(inspect_reserved_lifecycle_run(&lease, &ticket.run_id).unwrap(),
        LifecycleRunJournalObservation::RecoveryRequired { state }
        if state.run.run_id.as_deref() == Some(ticket.run_id.as_str()))
    );
    let expected = Some(SubmissionOutcome::RecoveryRequired {
        run_id: ticket.run_id.clone(),
    });
    assert_eq!(
        attach_existing_lifecycle(&lease, &namespace, &scope, &ticket.key, &ticket.request)
            .unwrap(),
        expected
    );
    assert!(!marker.exists());
    f.count_one();
    // A provider may outlive the owning host; finish that real script explicitly.
    f.release();
    f.finished();
    assert_eq!(
        attach_existing_lifecycle(&lease, &namespace, &scope, &ticket.key, &ticket.request)
            .unwrap(),
        expected
    );
    assert!(
        !marker.exists(),
        "provider exit after host death cannot invent Completed"
    );
    f.count_one();
}
