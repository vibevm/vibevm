use super::*;
use crate::process::ProcessError;
use std::sync::Mutex;
struct Control(Mutex<Context>);
impl Authority for Control {
    fn current(&self, _: &Request) -> Result<Context, &'static str> {
        Ok(self.0.lock().unwrap().clone())
    }
}
#[derive(Default)]
struct Runner(Mutex<Vec<ProcessSpec>>);
impl ProcessRunner for Runner {
    fn run(&self, s: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        self.0.lock().unwrap().push(s.clone());
        Ok(ProcessOutput {
            code: Some(0),
            stdout: b"Claude Code 2.1.7\n".to_vec(),
            ..Default::default()
        })
    }
}
struct Fixture {
    _dir: tempfile::TempDir,
    host: Host,
    control: Arc<Control>,
    runner: Arc<Runner>,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("client");
        std::fs::write(&path, b"inert fixture executable").unwrap();
        let control = Arc::new(Control(Mutex::new(Context {
            executable: ClientExecutable::Resolved {
                command: "claude".into(),
                path,
            },
            cwd: dir.path().into(),
            user_home: dir.path().into(),
            roots: vec![dir.path().into()],
            environment_ref: "fixture-env".into(),
            principal: "owner".into(),
            policy: "1".into(),
            evidence: "version-generation-1".into(),
            revoked: false,
        })));
        let runner = Arc::new(Runner::default());
        let host = Host::new(control.clone(), runner.clone());
        Self {
            _dir: dir,
            host,
            control,
            runner,
        }
    }
    fn request(&self) -> Request {
        client_command::read_request(br#"{"schema":1,"client":"claude","contract_version":1,"version":"2.1.7","operation":"install","coordinate":"literal/\"quote\"/$(data)"}"#).unwrap()
    }
    fn count(&self) -> usize {
        self.runner.0.lock().unwrap().len()
    }
}
#[test]
fn exact_approval_runs_only_inspected_spec_and_query_is_pure() {
    let f = Fixture::new();
    let p = f.host.prepare(f.request()).unwrap();
    assert_eq!(f.count(), 1);
    let wire = p.inspect();
    assert_eq!(wire.argv.last().unwrap(), "literal/\"quote\"/$(data)");
    assert!(wire.effects.iter().any(|s| s.contains("active content")));
    for _ in 0..5 {
        assert_eq!(p.inspect().argv, wire.argv);
    }
    assert_eq!(f.count(), 1);
    assert!(f.host.execute(&p, None).is_err());
    assert_eq!(f.count(), 1);
    let g = f.host.approve(&p).unwrap();
    assert_eq!(f.count(), 1);
    f.host.execute(&p, Some(&g)).unwrap();
    let seen = f.runner.0.lock().unwrap();
    let s = &seen[1];
    assert_eq!(s.program.to_str().unwrap(), wire.executable);
    assert_eq!(s.cwd.to_str().unwrap(), wire.cwd);
    assert_eq!(
        s.args
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>(),
        wire.argv
    );
    assert_eq!(
        s.env
            .iter()
            .map(|(k, v)| (
                k.to_str().unwrap().to_owned(),
                v.to_str().unwrap().to_owned()
            ))
            .collect::<std::collections::BTreeMap<_, _>>(),
        wire.environment
    );
    assert!(!s.env.contains_key(std::ffi::OsStr::new("PATH")));
    assert!(
        !s.env
            .contains_key(std::ffi::OsStr::new("AWS_SECRET_ACCESS_KEY"))
    );
}
#[test]
fn stale_control_version_environment_cwd_roots_and_revocation_refuse() {
    for mutate in [
        |c: &mut Context| c.evidence.push('2'),
        |c: &mut Context| c.environment_ref.push('2'),
        |c: &mut Context| c.cwd.push("other"),
        |c: &mut Context| c.roots.push("/".into()),
        |c: &mut Context| c.policy.push('2'),
        |c: &mut Context| c.principal.push('2'),
        |c: &mut Context| c.revoked = true,
        |c: &mut Context| c.user_home.push("other"),
    ] {
        let f = Fixture::new();
        let p = f.host.prepare(f.request()).unwrap();
        let g = f.host.approve(&p).unwrap();
        mutate(&mut f.control.0.lock().unwrap());
        assert!(f.host.execute(&p, Some(&g)).is_err());
        assert!(f.host.approve(&p).is_err());
        assert_eq!(f.count(), 1);
    }
}
#[test]
fn wrong_grants_and_unknown_contracts_refuse_before_operation() {
    let f = Fixture::new();
    let p = f.host.prepare(f.request()).unwrap();
    let g = f.host.approve(&p).unwrap();
    let mut q = f.request();
    q.coordinate = Some("other@m".into());
    let other = f.host.prepare(q).unwrap();
    assert!(f.host.execute(&other, Some(&g)).is_err());
    assert_eq!(f.count(), 2);
    let foreign = Fixture::new();
    assert!(foreign.host.execute(&p, Some(&g)).is_err());
    assert_eq!(foreign.count(), 0);
    for (version, contract) in [("2.2.0", 1), ("2.1.7", 2), ("2.1.8", 1)] {
        let mut q = f.request();
        q.version = version.into();
        q.contract_version = contract;
        assert!(f.host.prepare(q).is_err());
    }
    let mut q = f.request();
    q.client = RequestClient::Opencode;
    q.version = "1.17.1".into();
    assert!(f.host.prepare(q).is_err());
    f.control.0.lock().unwrap().executable = ClientExecutable::Missing {
        command: "claude".into(),
    };
    let before = f.count();
    assert!(f.host.prepare(f.request()).is_err());
    assert_eq!(before, f.count());
}
#[test]
fn executable_and_root_replacement_refuse() {
    let f = Fixture::new();
    let root = f._dir.path().join("target");
    std::fs::create_dir(&root).unwrap();
    f.control.0.lock().unwrap().roots.push(root.clone());
    let p = f.host.prepare(f.request()).unwrap();
    let g = f.host.approve(&p).unwrap();
    std::fs::rename(&root, root.with_extension("old")).unwrap();
    std::fs::create_dir(&root).unwrap();
    assert_eq!(observe(&p.executable.launcher).unwrap(), p.executable);
    assert!(f.host.execute(&p, Some(&g)).is_err());
    assert_eq!(f.count(), 1);

    let f = Fixture::new();
    let p = f.host.prepare(f.request()).unwrap();
    let g = f.host.approve(&p).unwrap();
    std::fs::write(&p.spec.program, b"changed").unwrap();
    assert!(f.host.execute(&p, Some(&g)).is_err());
    assert_eq!(f.count(), 1);
    let f = Fixture::new();
    let p = f.host.prepare(f.request()).unwrap();
    let g = f.host.approve(&p).unwrap();
    let old = p.spec.cwd.with_extension("old");
    std::fs::rename(&p.spec.cwd, &old).unwrap();
    std::fs::create_dir(&p.spec.cwd).unwrap();
    std::fs::write(&p.spec.program, b"inert fixture executable").unwrap();
    assert!(f.host.execute(&p, Some(&g)).is_err());
    assert_eq!(f.count(), 1);
    std::fs::remove_dir_all(&old).unwrap();
}
#[test]
fn quoted_path_is_data_and_nonutf8_wire_refuses() {
    let f = Fixture::new();
    let mut q = f.request();
    q.operation = RequestOperation::MarketplaceAdd;
    q.coordinate = None;
    q.path = Some(
        f._dir
            .path()
            .join("a \"quote\"/$(data)")
            .to_str()
            .unwrap()
            .into(),
    );
    std::fs::create_dir_all(q.path.as_ref().unwrap()).unwrap();
    let p = f.host.prepare(q).unwrap();
    assert_eq!(p.inspect().argv.len(), 6);
    assert_eq!(p.inspect().argv.last(), p.request.path.as_ref());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        f.control.0.lock().unwrap().user_home =
            f._dir.path().join(std::ffi::OsString::from_vec(vec![255]));
        std::fs::create_dir(&f.control.0.lock().unwrap().user_home).unwrap();
        let before = f.count();
        assert!(f.host.prepare(f.request()).is_err());
        assert_eq!(before, f.count());
    }
}
#[test]
fn real_generic_file_approval_confers_no_process_authority() {
    use super::super::generic::*;
    use sha2::{Digest, Sha256};
    use vibe_agent_projection::client_capabilities::{ClientCapabilities, ClientCapability};
    use vibe_core::manifest::{
        ClientBinding, ClientOperation, ClientRequest, ClientScope, ClientShape,
    };
    struct Files(SelectedObservation);
    impl GenericAuthority for Files {
        fn selected_current(&self, _: &str) -> Result<SelectedObservation, String> {
            Ok(self.0.clone())
        }
        fn current_control(&self) -> Result<ControlObservation, String> {
            Ok(ControlObservation {
                principal: "owner".into(),
                policy_generation: "1".into(),
            })
        }
    }
    let dir = tempfile::tempdir().unwrap();
    for name in ["project", "user", "settings"] {
        std::fs::create_dir(dir.path().join(name)).unwrap();
    }
    let request = ClientRequest {
        schema: 1,
        client_id: "org.example/file-client".into(),
        version: "*".into(),
        bindings: vec![ClientBinding {
            scope: ClientScope::User,
            shape: ClientShape::File,
            destination: "config.txt".into(),
            key: vec![],
            operations: vec![ClientOperation::ProjectFile],
        }],
    };
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&request).unwrap())
    );
    let source = SelectedObservation {
        request: request.clone(),
        selected: true,
        enabled: true,
        identity: SourceIdentity {
            provider: "org.example/provider#client".into(),
            provider_version: "1.0.0".into(),
            provider_digest: "b".repeat(64),
            declaration: "client".into(),
            declaration_digest: "c".repeat(64),
            descriptor: "descriptor".into(),
            source: "capture".into(),
            source_digest: "a".repeat(64),
            descriptor_digest: digest.clone(),
        },
    };
    let map = ClientCapabilities::admit([ClientCapability {
        provider: source.identity.provider.clone(),
        descriptor_digest: digest,
        request,
        disabled: false,
    }])
    .unwrap();
    let protected = || {
        vec![ProtectedResource {
            scope: ClientScope::User,
            relative: "controls".into(),
        }]
    };
    let files = GenericHost::new(
        vibe_safefs::Project::open(&dir.path().join("project")).unwrap(),
        vibe_safefs::Project::open(&dir.path().join("user")).unwrap(),
        vibe_safefs::Project::open(&dir.path().join("settings")).unwrap(),
        Arc::new(Files(source)),
        ProtectedControls {
            grants: protected(),
            policy: protected(),
            producer_trust: protected(),
            publication_mode: protected(),
        },
    )
    .unwrap();
    let capture = files.capture(&map, "org.example/file-client").unwrap();
    let admitted = files
        .admit_payload(
            capture,
            vec![GenericPayload::File(b"inert config".to_vec())],
        )
        .unwrap();
    let plan = files.plan(&admitted).unwrap();
    let _file_grant = files.approve(&plan).unwrap();
    let f = Fixture::new();
    let p = f.host.prepare(f.request()).unwrap();
    assert!(f.host.execute(&p, None).is_err());
    assert_eq!(f.count(), 1);
    // ExactGrant is a distinct opaque type: no conversion or process overload.
    assert_ne!(
        std::any::TypeId::of::<ExactGrant>(),
        std::any::TypeId::of::<ProcessGrant>()
    );
    assert!(client_command::read_request(br#"{"schema":1,"client":"native","contract_version":1,"version":"1","operation":"execute"}"#).is_err());
}
