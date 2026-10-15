use super::*;
use std::path::Path;
use std::sync::Mutex;
use vibe_agent_projection::client_capabilities::ClientCapability;
use vibe_core::manifest::*;

const CLOCK: &str = "2026-10-15T00:00:00Z";
pub(super) struct Authority {
    pub(super) selected: Mutex<SelectedObservation>,
    pub(super) control: Mutex<ControlObservation>,
}
impl GenericAuthority for Authority {
    fn selected_current(&self, _: &str) -> Result<SelectedObservation, String> {
        Ok(self.selected.lock().unwrap().clone())
    }
    fn current_control(&self) -> Result<ControlObservation, String> {
        Ok(self.control.lock().unwrap().clone())
    }
}
pub(super) struct Fixture {
    pub(super) dir: tempfile::TempDir,
    pub(super) host: GenericHost,
    pub(super) authority: Arc<Authority>,
}
pub(super) fn binding(
    scope: ClientScope,
    shape: ClientShape,
    path: &str,
    keys: &[&str],
) -> ClientBinding {
    ClientBinding {
        scope,
        shape,
        destination: path.into(),
        key: keys.iter().map(|s| s.to_string()).collect(),
        operations: vec![match shape {
            ClientShape::File => ClientOperation::ProjectFile,
            ClientShape::Tree => ClientOperation::ProjectTree,
            ClientShape::JsonMember => ClientOperation::ReconcileJsonMember,
            ClientShape::TomlMember => ClientOperation::ReconcileTomlMember,
        }],
    }
}
pub(super) fn request(bindings: Vec<ClientBinding>) -> ClientRequest {
    ClientRequest {
        schema: 1,
        client_id: "org.example/new-client".into(),
        version: "*".into(),
        bindings,
    }
}
fn protections() -> ProtectedControls {
    let category = |name: &str| {
        [ClientScope::Project, ClientScope::User]
            .into_iter()
            .map(|scope| ProtectedResource {
                scope,
                relative: format!("controls/{name}"),
            })
            .collect()
    };
    ProtectedControls {
        grants: category("grants"),
        policy: category("policy"),
        producer_trust: category("trust"),
        publication_mode: category("publication"),
    }
}
impl Fixture {
    pub(super) fn new(r: ClientRequest) -> Self {
        let dir = tempfile::tempdir().unwrap();
        for name in ["project", "user", "settings"] {
            std::fs::create_dir(dir.path().join(name)).unwrap();
        }
        let authority = Arc::new(Authority {
            selected: Mutex::new(SelectedObservation {
                identity: SourceIdentity {
                    provider: "org.example/provider#client".into(),
                    provider_version: "1.0.0".into(),
                    provider_digest: "b".repeat(64),
                    declaration: "client".into(),
                    declaration_digest: "c".repeat(64),
                    descriptor: "client-descriptor".into(),
                    source: "fixture-capture".into(),
                    source_digest: "a".repeat(64),
                    descriptor_digest: digest(&encoded(&r).unwrap()),
                },
                request: r,
                selected: true,
                enabled: true,
            }),
            control: Mutex::new(ControlObservation {
                principal: "fixture-owner".into(),
                policy_generation: "generation-1".into(),
            }),
        });
        let host = GenericHost::new(
            Project::open(&dir.path().join("project")).unwrap(),
            Project::open(&dir.path().join("user")).unwrap(),
            Project::open(&dir.path().join("settings")).unwrap(),
            authority.clone(),
            protections(),
        )
        .unwrap();
        Self {
            dir,
            host,
            authority,
        }
    }
    pub(super) fn map(&self) -> ClientCapabilities {
        let o = self.authority.selected.lock().unwrap().clone();
        ClientCapabilities::admit([ClientCapability {
            provider: o.identity.provider,
            descriptor_digest: o.identity.descriptor_digest,
            request: o.request,
            disabled: !o.enabled,
        }])
        .unwrap()
    }
    pub(super) fn admitted(&self, payloads: Vec<GenericPayload>) -> AdmittedClient {
        let map = self.map();
        let id = map.rows().keys().next().unwrap();
        self.host
            .admit_payload(self.host.capture(&map, id).unwrap(), payloads)
            .unwrap()
    }
    pub(super) fn plan(&self, payloads: Vec<GenericPayload>) -> GenericPlan {
        self.host.plan(&self.admitted(payloads)).unwrap()
    }
    pub(super) fn write(&self, scope: ClientScope, path: &str, bytes: &[u8]) {
        self.host.root(scope).write_atomic(path, bytes).unwrap();
    }
    pub(super) fn bytes(&self, scope: ClientScope, path: &str) -> Vec<u8> {
        self.host
            .root(scope)
            .read_file_bounded(path, 4 * 1024 * 1024)
            .unwrap()
            .unwrap()
    }
    pub(super) fn census(&self) -> BTreeMap<String, String> {
        census(self.dir.path())
    }
}
fn census(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, String>) {
        for e in std::fs::read_dir(at).unwrap() {
            let p = e.unwrap().path();
            let metadata = std::fs::symlink_metadata(&p).unwrap();
            let relative = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            if metadata.is_dir() {
                out.insert(relative, "directory".into());
                walk(root, &p, out);
            } else if metadata.file_type().is_symlink() {
                out.insert(
                    relative,
                    format!("link:{:?}", std::fs::read_link(p).unwrap()),
                );
            } else {
                out.insert(relative, digest(&std::fs::read(p).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[track_caller]
fn unchanged(f: &Fixture, before: &BTreeMap<String, String>) {
    let after = f.census();
    if let Some(directory) = std::env::var_os("M08C2_EVIDENCE_DIR") {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let record = serde_json::json!({"source_line":std::panic::Location::caller().line(),"before":before,"after":after,"equal":*before==after});
        std::fs::write(
            Path::new(&directory).join(format!("census-{sequence}.json")),
            serde_json::to_vec_pretty(&record).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(*before, after);
}
pub(super) fn file_fixture() -> Fixture {
    Fixture::new(request(vec![binding(
        ClientScope::Project,
        ClientShape::File,
        "new/file",
        &[],
    )]))
}
fn payload() -> Vec<GenericPayload> {
    vec![GenericPayload::File(b"owned".to_vec())]
}

#[test]
fn four_shapes_exact_grant_preserves_foreign_files_and_members() {
    let f = Fixture::new(request(vec![
        binding(ClientScope::Project, ClientShape::File, "new/output", &[]),
        binding(ClientScope::User, ClientShape::Tree, "bundle", &[]),
        binding(
            ClientScope::User,
            ClientShape::JsonMember,
            "client.json",
            &["mcpServers", "new.key/segment"],
        ),
        binding(
            ClientScope::Project,
            ClientShape::TomlMember,
            "client.toml",
            &["mcp", "literal.dot/slash"],
        ),
    ]));
    f.write(ClientScope::User, "bundle/foreign", b"untouched");
    f.write(
        ClientScope::User,
        "client.json",
        br#"{"foreign":true,"mcpServers":{"neighbor":{"command":"inert"}}}"#,
    );
    f.write(
        ClientScope::Project,
        "client.toml",
        b"foreign = 7\n[mcp.neighbor]\ncommand = 'inert'\n",
    );
    let before = f.census();
    let plan = f.plan(vec![
        GenericPayload::File(b"file".to_vec()),
        GenericPayload::Tree(BTreeMap::from([
            ("a".into(), b"A".to_vec()),
            ("nested/b".into(), b"B".to_vec()),
        ])),
        GenericPayload::JsonMember(
            serde_json::json!({"command":"never-executed","args":["--inert"]}),
        ),
        GenericPayload::TomlMember(toml::Value::String("config".into())),
    ]);
    unchanged(&f, &before);
    let grant = f.host.approve(&plan).unwrap();
    unchanged(&f, &before);
    assert_eq!(
        f.host.apply(&plan, None, CLOCK).unwrap_err().effect,
        GenericEffect::NoEffects
    );
    unchanged(&f, &before);
    let outcome = f.host.apply(&plan, Some(&grant), CLOCK).unwrap();
    assert_eq!(outcome.resources.len(), 5);
    assert_eq!(outcome.effect, GenericEffect::Destinations);
    assert_eq!(f.bytes(ClientScope::Project, "new/output"), b"file");
    assert_eq!(f.bytes(ClientScope::User, "bundle/a"), b"A");
    assert_eq!(f.bytes(ClientScope::User, "bundle/nested/b"), b"B");
    assert_eq!(f.bytes(ClientScope::User, "bundle/foreign"), b"untouched");
    let json: serde_json::Value =
        serde_json::from_slice(&f.bytes(ClientScope::User, "client.json")).unwrap();
    assert_eq!(json["foreign"], true);
    assert_eq!(json["mcpServers"]["neighbor"]["command"], "inert");
    assert_eq!(
        json["mcpServers"]["new.key/segment"]["command"],
        "never-executed"
    );
    let toml: toml::Value =
        toml::from_str(std::str::from_utf8(&f.bytes(ClientScope::Project, "client.toml")).unwrap())
            .unwrap();
    assert_eq!(toml["foreign"].as_integer(), Some(7));
    assert_eq!(toml["mcp"]["neighbor"]["command"].as_str(), Some("inert"));
    assert_eq!(toml["mcp"]["literal.dot/slash"].as_str(), Some("config"));
    let state = super::super::state::DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let home = f.host.home(&plan.target);
    let receipt = state.read_receipt(&home).unwrap().unwrap();
    assert_eq!(receipt.resources.len(), 5);
    assert!(state.read_intent(&home).unwrap().is_none());
    let locks = state.read_lock_resources(&home).unwrap().unwrap();
    assert!(locks.pending.is_none());
    // Five physical documents plus three incumbent User compatibility keys.
    assert_eq!(locks.committed.unwrap().resources.len(), 8);
}
#[test]
fn absent_wrong_broadened_and_changed_plans_have_zero_effects() {
    let f = file_fixture();
    let plan = f.plan(payload());
    let grant = f.host.approve(&plan).unwrap();
    let other = f.plan(vec![GenericPayload::File(b"different".to_vec())]);
    let wrong = f.host.approve(&other).unwrap();
    let before = f.census();
    assert!(f.host.apply(&plan, None, CLOCK).is_err());
    assert!(f.host.apply(&plan, Some(&wrong), CLOCK).is_err());
    for change in 0..6 {
        let mut p = plan.clone();
        match change {
            0 => p.resources.push(p.resources[0].clone()),
            1 => p.resources[0].operation = ClientOperation::ProjectTree,
            2 => p.admitted.capture.observation.identity.descriptor_digest = "b".repeat(64),
            3 => p.documents[0].after.as_mut().unwrap().push(1),
            4 => p.target.push('x'),
            _ => p.roots[0].push('x'),
        };
        assert_eq!(
            f.host.apply(&p, Some(&grant), CLOCK).unwrap_err().effect,
            GenericEffect::NoEffects
        );
        unchanged(&f, &before);
    }
    let f2 = file_fixture();
    assert_eq!(
        f2.host
            .apply(&plan, Some(&grant), CLOCK)
            .unwrap_err()
            .effect,
        GenericEffect::NoEffects
    );
    unchanged(&f, &before);
}
#[test]
fn current_selection_source_principal_policy_and_root_drift_refuse() {
    let f = file_fixture();
    let plan = f.plan(payload());
    let grant = f.host.approve(&plan).unwrap();
    let original = f.authority.selected.lock().unwrap().clone();
    let control = f.authority.control.lock().unwrap().clone();
    let before = f.census();
    for change in 0..13 {
        {
            let mut o = f.authority.selected.lock().unwrap();
            *o = original.clone();
            match change {
                0 => o.selected = false,
                1 => o.enabled = false,
                2 => o.identity.provider.push('x'),
                3 => o.identity.declaration.push('x'),
                4 => o.identity.source.push('x'),
                5 => o.identity.source_digest.push('x'),
                6 => o.identity.descriptor_digest.push('x'),
                9 => o.identity.provider_digest.push('x'),
                10 => o.identity.declaration_digest.push('x'),
                11 => o.identity.descriptor.push('x'),
                12 => o.identity.provider_version.push('x'),
                _ => {}
            }
        }
        {
            let mut c = f.authority.control.lock().unwrap();
            *c = control.clone();
            if change == 7 {
                c.principal.push('x')
            }
            if change == 8 {
                c.policy_generation.push('x')
            }
        }
        assert!(f.host.approve(&plan).is_err());
        assert_eq!(
            f.host.apply(&plan, Some(&grant), CLOCK).unwrap_err().effect,
            GenericEffect::NoEffects
        );
        unchanged(&f, &before);
    }
    *f.authority.selected.lock().unwrap() = original;
    *f.authority.control.lock().unwrap() = control;
    let state = super::super::state::DeployState::open_bound(&f.host.settings).unwrap();
    drop(state);
    let state_plan = f.plan(payload());
    let state_grant = f.host.approve(&state_plan).unwrap();
    let state_path = f
        .host
        .settings
        .root_path()
        .join(super::super::model::DEPLOY_STATE_DIR);
    let saved = f.dir.path().join("old-state");
    std::fs::rename(&state_path, &saved).unwrap();
    std::fs::create_dir(&state_path).unwrap();
    let state_changed = f.census();
    assert_eq!(
        f.host
            .apply(&state_plan, Some(&state_grant), CLOCK)
            .unwrap_err()
            .effect,
        GenericEffect::NoEffects
    );
    unchanged(&f, &state_changed);
    let project = f.dir.path().join("project");
    std::fs::rename(&project, f.dir.path().join("old-project")).unwrap();
    std::fs::create_dir(&project).unwrap();
    let drifted = f.census();
    assert!(f.host.apply(&plan, Some(&grant), CLOCK).is_err());
    unchanged(&f, &drifted);
}
#[test]
fn caller_map_is_not_selection_or_payload_authority() {
    let f = file_fixture();
    let map = f.map();
    let before = f.census();
    f.authority.selected.lock().unwrap().selected = false;
    assert!(f.host.capture(&map, "org.example/new-client").is_err());
    f.authority.selected.lock().unwrap().selected = true;
    let capture = f.host.capture(&map, "org.example/new-client").unwrap();
    assert!(
        f.host
            .admit_payload(
                capture,
                vec![GenericPayload::JsonMember(serde_json::json!({}))]
            )
            .is_err()
    );
    let mut row = map.rows().values().next().unwrap().clone();
    row.request.bindings[0].destination = "changed".into();
    let map = ClientCapabilities::admit([row]).unwrap();
    assert!(f.host.capture(&map, "org.example/new-client").is_err());
    unchanged(&f, &before);
}
#[test]
fn traversal_reserved_alias_ancestor_and_member_conflicts_refuse() {
    for path in [
        "../escape",
        "/absolute",
        "a//b",
        "a\\b",
        "CON",
        ".vibe-stage-forbidden",
    ] {
        let f = Fixture::new(request(vec![binding(
            ClientScope::Project,
            ClientShape::File,
            path,
            &[],
        )]));
        let before = f.census();
        let o = f.authority.selected.lock().unwrap().clone();
        let map = ClientCapabilities::admit([ClientCapability {
            provider: o.identity.provider,
            descriptor_digest: o.identity.descriptor_digest,
            request: o.request,
            disabled: false,
        }]);
        if let Ok(map) = map {
            let c = f.host.capture(&map, "org.example/new-client").unwrap();
            let a = f.host.admit_payload(c, payload()).unwrap();
            assert!(f.host.plan(&a).is_err(), "{path}");
        }
        unchanged(&f, &before);
    }
    for (a, b) in [("same", "SAME"), ("a", "a/b")] {
        let f = Fixture::new(request(vec![
            binding(ClientScope::Project, ClientShape::File, a, &[]),
            binding(ClientScope::Project, ClientShape::File, b, &[]),
        ]));
        let before = f.census();
        assert!(
            f.host
                .plan(&f.admitted(vec![
                    GenericPayload::File(vec![]),
                    GenericPayload::File(vec![])
                ]))
                .is_err()
        );
        unchanged(&f, &before);
    }
    for keys in [vec!["x"], vec!["x", "child"]] {
        let f = Fixture::new(request(vec![
            binding(ClientScope::Project, ClientShape::JsonMember, "doc", &["x"]),
            binding(ClientScope::Project, ClientShape::JsonMember, "doc", &keys),
        ]));
        if f.authority
            .selected
            .lock()
            .unwrap()
            .request
            .validate()
            .is_ok()
        {
            assert!(
                f.host
                    .plan(&f.admitted(vec![GenericPayload::JsonMember(serde_json::json!(1)); 2]))
                    .is_err()
            );
        }
    }
    let f = Fixture::new(request(vec![
        binding(ClientScope::Project, ClientShape::Tree, "tree", &[]),
        binding(ClientScope::Project, ClientShape::File, "tree/child", &[]),
    ]));
    assert!(
        f.host
            .plan(&f.admitted(vec![
                GenericPayload::Tree(BTreeMap::from([("a".into(), vec![])])),
                GenericPayload::File(vec![])
            ]))
            .is_err()
    );
    let f = Fixture::new(request(vec![
        binding(ClientScope::Project, ClientShape::File, "doc", &[]),
        binding(ClientScope::Project, ClientShape::JsonMember, "doc", &["x"]),
    ]));
    assert!(
        f.host
            .plan(&f.admitted(vec![
                GenericPayload::File(vec![]),
                GenericPayload::JsonMember(serde_json::json!(1))
            ]))
            .is_err()
    );
    let f = Fixture::new(request(vec![binding(
        ClientScope::Project,
        ClientShape::Tree,
        "tree",
        &[],
    )]));
    assert!(
        f.host
            .plan(&f.admitted(vec![GenericPayload::Tree(BTreeMap::from([
                ("a".into(), vec![]),
                ("a/b".into(), vec![])
            ]))]))
            .is_err()
    );
}
#[test]
fn all_host_protected_categories_and_aliases_refuse() {
    for scope in [ClientScope::Project, ClientScope::User] {
        for path in [
            "controls/grants",
            "controls/policy",
            "controls/trust",
            "controls/publication",
            "Controls/GRANTS",
            "controls",
            "controls/policy/child",
        ] {
            let f = Fixture::new(request(vec![binding(scope, ClientShape::File, path, &[])]));
            let before = f.census();
            assert!(f.host.plan(&f.admitted(payload())).is_err(), "{path}");
            unchanged(&f, &before);
        }
    }
    let f = file_fixture();
    let mut protected = protections();
    protected.grants.clear();
    assert!(
        GenericHost::new(
            Project::open(f.host.project.root_path()).unwrap(),
            Project::open(f.host.user.root_path()).unwrap(),
            Project::open(f.host.settings.root_path()).unwrap(),
            f.authority.clone(),
            protected
        )
        .is_err()
    );
}
#[cfg(unix)]
#[test]
fn physical_links_and_protected_aliases_refuse_without_writes() {
    let f = file_fixture();
    f.write(ClientScope::Project, "controls/policy", b"policy");
    std::fs::create_dir(f.host.project.root_path().join("new")).unwrap();
    std::fs::hard_link(
        f.host.project.root_path().join("controls/policy"),
        f.host.project.root_path().join("new/file"),
    )
    .unwrap();
    let before = f.census();
    assert!(f.host.plan(&f.admitted(payload())).is_err());
    unchanged(&f, &before);
    let f = file_fixture();
    std::os::unix::fs::symlink(
        f.host.user.root_path(),
        f.host.project.root_path().join("new"),
    )
    .unwrap();
    let before = f.census();
    assert!(f.host.plan(&f.admitted(payload())).is_err());
    unchanged(&f, &before);
}
#[test]
fn launch_content_is_inspectable_changes_identity_and_never_executes() {
    let f = Fixture::new(request(vec![binding(
        ClientScope::User,
        ClientShape::JsonMember,
        "mcp.json",
        &["mcp", "server"],
    )]));
    let marker = f.dir.path().join("process-must-not-run");
    let value = serde_json::json!({"command":"sh","args":["-c",format!("touch {}",marker.display())],"enabled":true});
    let p = f.plan(vec![GenericPayload::JsonMember(value.clone())]);
    let grant = f.host.approve(&p).unwrap();
    assert!(p.documents().any(|(_, _, bytes)| {
        std::str::from_utf8(bytes)
            .unwrap()
            .contains("process-must-not-run")
    }));
    assert_eq!(p.payloads(), &[GenericPayload::JsonMember(value)]);
    let other = f.plan(vec![GenericPayload::JsonMember(
        serde_json::json!({"command":"other"}),
    )]);
    assert_ne!(p.identity(), other.identity());
    assert!(f.host.apply(&other, Some(&grant), CLOCK).is_err());
    f.host.apply(&p, Some(&grant), CLOCK).unwrap();
    assert!(!marker.exists());
}
#[test]
fn physical_document_contention_and_literal_member_coexistence() {
    let f = Fixture::new(request(vec![
        binding(
            ClientScope::User,
            ClientShape::JsonMember,
            "shared.json",
            &["mcp", "A/B"],
        ),
        binding(
            ClientScope::User,
            ClientShape::JsonMember,
            "shared.json",
            &["mcp", "a/b"],
        ),
    ]));
    let state = super::super::state::DeployState::open_bound(&f.host.settings).unwrap();
    let plan = f.plan(vec![
        GenericPayload::JsonMember(serde_json::json!(1)),
        GenericPayload::JsonMember(serde_json::json!(2)),
    ]);
    // One document: a common absolute lease plus its incumbent User lease.
    assert_eq!(plan.locks(&f.host).len(), 2);
    let grant = f.host.approve(&plan).unwrap();
    let held = state.lock_destinations(&plan.locks(&f.host)).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            super::apply::LOCKED_HOOK.with(|h| {
                *h.borrow_mut() = Some(Box::new(move || {
                    tx.send(()).unwrap();
                }))
            });
            f.host.apply(&plan, Some(&grant), CLOCK)
        });
        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(100))
                .is_err()
        );
        drop(held);
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        worker.join().unwrap().unwrap();
    });
    let doc: serde_json::Value =
        serde_json::from_slice(&f.bytes(ClientScope::User, "shared.json")).unwrap();
    assert_eq!(doc["mcp"]["A/B"], 1);
    assert_eq!(doc["mcp"]["a/b"], 2);
    {
        let mut o = f.authority.selected.lock().unwrap();
        o.request.client_id = "org.example/second-client".into();
        o.request.bindings = vec![binding(
            ClientScope::User,
            ClientShape::JsonMember,
            "shared.json",
            &["mcp", "C"],
        )];
    }
    let p = f.plan(vec![GenericPayload::JsonMember(serde_json::json!(3))]);
    assert_eq!(p.locks(&f.host), plan.locks(&f.host));
    let g = f.host.approve(&p).unwrap();
    f.host.apply(&p, Some(&g), CLOCK).unwrap();
    let doc: serde_json::Value =
        serde_json::from_slice(&f.bytes(ClientScope::User, "shared.json")).unwrap();
    assert_eq!(doc["mcp"]["A/B"], 1);
    assert_eq!(doc["mcp"]["C"], 3);
    {
        let mut o = f.authority.selected.lock().unwrap();
        o.request.client_id = "org.example/third-client".into();
        o.request.bindings[0].key = vec!["mcp".into()];
    }
    let before = f.census();
    assert!(
        f.host
            .plan(&f.admitted(vec![GenericPayload::JsonMember(serde_json::json!({}))]))
            .is_err()
    );
    unchanged(&f, &before);
}
#[test]
fn stale_locked_destination_and_ownership_refuse_without_partial_merge() {
    let raced = file_fixture();
    raced.write(ClientScope::Project, "new/neighbor", b"foreign");
    let raced_plan = raced.plan(payload());
    let raced_grant = raced.host.approve(&raced_plan).unwrap();
    vibe_safefs::arm_before_link(Some(Box::new(|directory, name| {
        std::fs::write(directory.path().join(name), b"late-foreign").unwrap();
    })));
    let race_result = raced.host.apply(&raced_plan, Some(&raced_grant), CLOCK);
    vibe_safefs::arm_before_link(None);
    assert_eq!(
        race_result.unwrap_err().effect,
        GenericEffect::Infrastructure
    );
    assert_eq!(
        raced.bytes(ClientScope::Project, "new/file"),
        b"late-foreign"
    );

    let f = file_fixture();
    let p = f.plan(payload());
    let g = f.host.approve(&p).unwrap();
    let path = f.host.project.root_path().join("new/file");
    super::apply::LOCKED_HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move || {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"foreign-race").unwrap();
        }))
    });
    let error = f.host.apply(&p, Some(&g), CLOCK).unwrap_err();
    assert_eq!(error.effect, GenericEffect::Infrastructure);
    assert_eq!(f.bytes(ClientScope::Project, "new/file"), b"foreign-race");
    let state = super::super::state::DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    assert!(
        state
            .read_intent(&f.host.home(&p.target))
            .unwrap()
            .is_none()
    );
    let source_fixture = file_fixture();
    let source_plan = source_fixture.plan(payload());
    let source_grant = source_fixture.host.approve(&source_plan).unwrap();
    let authority = source_fixture.authority.clone();
    super::apply::LOCKED_HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move || {
            authority.selected.lock().unwrap().identity.source_digest = "d".repeat(64);
        }))
    });
    assert_eq!(
        source_fixture
            .host
            .apply(&source_plan, Some(&source_grant), CLOCK)
            .unwrap_err()
            .effect,
        GenericEffect::Infrastructure
    );
    assert!(
        !source_fixture
            .host
            .project
            .root_path()
            .join("new/file")
            .exists()
    );
    let f = file_fixture();
    let p = f.plan(payload());
    let g = f.host.approve(&p).unwrap();
    f.host.apply(&p, Some(&g), CLOCK).unwrap();
    let p = f.plan(payload());
    let g = f.host.approve(&p).unwrap();
    let settings = f.host.settings.root_path().to_owned();
    let target = p.target.clone();
    let root = f.host.roots[0].clone();
    super::apply::LOCKED_HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move || {
            let settings = Project::open(&settings).unwrap();
            let state = super::super::state::DeployState::observe_bound(&settings)
                .unwrap()
                .unwrap();
            let home = super::super::state::DeploymentHome::new(
                &settings
                    .root_path()
                    .join(super::super::model::DEPLOY_STATE_DIR),
                &root,
                None,
                &target,
            );
            let mut r = state.read_receipt(&home).unwrap().unwrap();
            r.evidence = Some("changed ownership witness".into());
            state.write_receipt(&home, &r).unwrap();
        }))
    });
    assert_eq!(
        f.host.apply(&p, Some(&g), CLOCK).unwrap_err().effect,
        GenericEffect::Infrastructure
    );
    assert_eq!(f.bytes(ClientScope::Project, "new/file"), b"owned");
}
#[test]
fn publication_failure_reports_destination_effect_and_retained_intent() {
    let preparation = file_fixture();
    let preparation_plan = preparation.plan(payload());
    let preparation_grant = preparation.host.approve(&preparation_plan).unwrap();
    vibe_safefs::fail_before_publish(Some("file"));
    let prepared = preparation
        .host
        .apply(&preparation_plan, Some(&preparation_grant), CLOCK);
    vibe_safefs::fail_before_publish(None);
    let error = prepared.unwrap_err();
    assert_eq!(error.effect, GenericEffect::Destinations);
    assert!(error.reason.contains("BeforePublication"));
    assert!(preparation.host.project.root_path().join("new").is_dir());
    assert!(
        !preparation
            .host
            .project
            .root_path()
            .join("new/file")
            .exists()
    );

    let f = file_fixture();
    let p = f.plan(payload());
    let g = f.host.approve(&p).unwrap();
    vibe_safefs::fail_after_publish(Some("file"));
    let result = f.host.apply(&p, Some(&g), CLOCK);
    vibe_safefs::fail_after_publish(None);
    let error = result.unwrap_err();
    assert_eq!(error.effect, GenericEffect::Destinations);
    assert!(error.reason.contains("PossiblyPublished"));
    assert_eq!(f.bytes(ClientScope::Project, "new/file"), b"owned");
    let state = super::super::state::DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let home = f.host.home(&p.target);
    assert!(state.read_intent(&home).unwrap().is_some());
    assert!(state.read_receipt(&home).unwrap().is_none());
    assert!(
        state
            .read_lock_resources(&home)
            .unwrap()
            .unwrap()
            .pending
            .is_some()
    );
    let before = f.census();
    assert!(f.host.apply(&p, Some(&g), CLOCK).is_err());
    unchanged(&f, &before);
}
#[test]
fn scalar_ancestors_and_equal_foreign_values_do_not_transfer_ownership() {
    for bytes in [b"".as_slice(), br#"{"foreign":1,"foreign":2}"#] {
        let f = Fixture::new(request(vec![binding(
            ClientScope::Project,
            ClientShape::JsonMember,
            "doc",
            &["new"],
        )]));
        f.write(ClientScope::Project, "doc", bytes);
        let before = f.census();
        assert!(
            f.host
                .plan(&f.admitted(vec![GenericPayload::JsonMember(serde_json::json!(1))]))
                .is_err()
        );
        unchanged(&f, &before);
    }

    for shape in [ClientShape::JsonMember, ClientShape::TomlMember] {
        let f = Fixture::new(request(vec![binding(
            ClientScope::Project,
            shape,
            "doc",
            &["a", "b"],
        )]));
        let (bytes, value) = match shape {
            ClientShape::JsonMember => (
                br#"{"a":1}"#.to_vec(),
                GenericPayload::JsonMember(serde_json::json!(1)),
            ),
            _ => (
                b"a=1\n".to_vec(),
                GenericPayload::TomlMember(toml::Value::Integer(1)),
            ),
        };
        f.write(ClientScope::Project, "doc", &bytes);
        let before = f.census();
        assert!(f.host.plan(&f.admitted(vec![value])).is_err());
        unchanged(&f, &before);
    }
    let f = file_fixture();
    f.write(ClientScope::Project, "new/file", b"owned");
    let before = f.census();
    assert!(f.host.plan(&f.admitted(payload())).is_err());
    unchanged(&f, &before);
}

#[test]
fn two_hosts_opposite_scopes_share_document_lock_and_replan() {
    use std::sync::mpsc::{RecvTimeoutError, channel};
    use std::time::Duration;
    let wait = Duration::from_secs(5);
    let blocked = Duration::from_millis(100);
    // Both directions, with different retained root boundaries and accepted
    // root `.` spellings. No links or case-insensitive filesystem assumption.
    for first_scope in [ClientScope::User, ClientScope::Project] {
        let second_scope = match first_scope {
            ClientScope::User => ClientScope::Project,
            ClientScope::Project => ClientScope::User,
        };
        let a = Fixture::new(request(vec![binding(
            first_scope,
            ClientShape::JsonMember,
            "nested/shared.json",
            &["first"],
        )]));
        a.write(first_scope, "nested/shared.json", br#"{"foreign":7}"#);
        let mut b = Fixture::new(request(vec![binding(
            second_scope,
            ClientShape::JsonMember,
            "shared.json",
            &["second"],
        )]));
        let shared = a.host.root(first_scope).root_path().join("nested/.");
        let (project, user) = match second_scope {
            ClientScope::Project => (shared, b.host.user.root_path().to_path_buf()),
            ClientScope::User => (b.host.project.root_path().to_path_buf(), shared),
        };
        b.host = GenericHost::new(
            Project::open(&project).unwrap(),
            Project::open(&user).unwrap(),
            Project::open(a.host.settings.root_path()).unwrap(),
            b.authority.clone(),
            protections(),
        )
        .unwrap();
        let state = super::super::state::DeployState::open_bound(&a.host.settings).unwrap();
        let pa = a.plan(vec![GenericPayload::JsonMember(serde_json::json!(1))]);
        let pb = b.plan(vec![GenericPayload::JsonMember(serde_json::json!(2))]);
        let ga = a.host.approve(&pa).unwrap();
        let gb = b.host.approve(&pb).unwrap();
        assert_ne!(a.host.home(&pa.target).id(), b.host.home(&pb.target).id());
        assert_eq!(pa.documents[0].before, pb.documents[0].before);
        // Incumbent providers name User locks as home:relative and Project
        // locks as absolute. Acquire that key alone to prove compatibility.
        let incumbent = match first_scope {
            ClientScope::User => "home:nested/shared.json".to_owned(),
            ClientScope::Project => a
                .host
                .project
                .root_path()
                .join("nested/shared.json")
                .to_string_lossy()
                .into_owned(),
        };
        let legacy = state.lock_destinations(&[incumbent]).unwrap();
        let (a_attempt_tx, a_attempt_rx) = channel();
        let (a_enter_tx, a_enter_rx) = channel();
        let (release_tx, release_rx) = channel();
        let (b_attempt_tx, b_attempt_rx) = channel();
        let (b_enter_tx, b_enter_rx) = channel();
        std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                super::apply::BEFORE_DOCUMENT_LOCK_HOOK.with(|h| {
                    *h.borrow_mut() = Some(Box::new(move || a_attempt_tx.send(()).unwrap()));
                });
                super::apply::LOCKED_HOOK.with(|h| {
                    *h.borrow_mut() = Some(Box::new(move || {
                        a_enter_tx.send(()).unwrap();
                        release_rx.recv_timeout(wait).unwrap();
                    }));
                });
                a.host.apply(&pa, Some(&ga), CLOCK)
            });
            a_attempt_rx.recv_timeout(wait).unwrap();
            let legacy_blocked = a_enter_rx.recv_timeout(blocked);
            drop(legacy);
            a_enter_rx.recv_timeout(wait).unwrap();
            let second = scope.spawn(|| {
                super::apply::BEFORE_DOCUMENT_LOCK_HOOK.with(|h| {
                    *h.borrow_mut() = Some(Box::new(move || b_attempt_tx.send(()).unwrap()));
                });
                super::apply::LOCKED_HOOK.with(|h| {
                    *h.borrow_mut() = Some(Box::new(move || b_enter_tx.send(()).unwrap()));
                });
                b.host.apply(&pb, Some(&gb), CLOCK)
            });
            b_attempt_rx.recv_timeout(wait).unwrap();
            let document_blocked = b_enter_rx.recv_timeout(blocked);
            // No publication has happened while A holds the critical section.
            let while_held = a.bytes(first_scope, "nested/shared.json");
            release_tx.send(()).unwrap();
            first.join().unwrap().unwrap();
            b_enter_rx.recv_timeout(wait).unwrap();
            let stale = second.join().unwrap().unwrap_err();
            assert_eq!(legacy_blocked, Err(RecvTimeoutError::Timeout));
            assert_eq!(document_blocked, Err(RecvTimeoutError::Timeout));
            assert_eq!(while_held, br#"{"foreign":7}"#);
            assert_eq!(stale.effect, GenericEffect::Infrastructure);
            assert!(stale.reason.contains("ownership census changed"));
        });
        let landed: serde_json::Value =
            serde_json::from_slice(&a.bytes(first_scope, "nested/shared.json")).unwrap();
        assert_eq!(landed, serde_json::json!({"foreign":7,"first":1}));
        let fresh = b.plan(vec![GenericPayload::JsonMember(serde_json::json!(2))]);
        let fresh_grant = b.host.approve(&fresh).unwrap();
        b.host.apply(&fresh, Some(&fresh_grant), CLOCK).unwrap();
        let final_doc: serde_json::Value =
            serde_json::from_slice(&a.bytes(first_scope, "nested/shared.json")).unwrap();
        assert_eq!(
            final_doc,
            serde_json::json!({"foreign":7,"first":1,"second":2})
        );
    }
}
