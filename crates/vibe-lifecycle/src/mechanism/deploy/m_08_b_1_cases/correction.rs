use super::*;
use crate::process::{
    ProcessError, ProcessOutput, ProcessRunner, ProcessSpec, SystemProcessRunner,
};
use plugin::fixture::FakeStateOwner;
use std::sync::{Arc, Mutex};

struct Case {
    world: World,
    target: vibe_core::manifest::DeployTarget,
    artifact: protocol::ResolvedDeployArtifact,
}
impl Case {
    fn new() -> Self {
        let world = World::new();
        world.record_projection("claude-correction", FILES);
        let target = target(
            PluginClient::Claude,
            "demo",
            "claude-correction",
            "demo-plugin",
        );
        let artifact = artifact::resolve_artifact(world.project.path(), &target).unwrap();
        Self {
            world,
            target,
            artifact,
        }
    }
    fn request(&self) -> protocol::DeployTargetRequest<'_> {
        protocol::DeployTargetRequest {
            target: &self.target,
            profile: "local",
            project_root: self.world.project.path(),
            settings_root: self.world.settings.path(),
            user_home: self.world.home.path(),
            clients: &self.world.clients,
            prior_receipt: None,
            recovery_intent: None,
            artifact: Some(&self.artifact),
            staging: None,
        }
    }
    fn launcher(&self) -> &std::path::Path {
        self.world.clients.claude.resolved_path().unwrap()
    }
    fn provider(&self, runner: Arc<dyn ProcessRunner>) -> plugin::ClientPluginProvider {
        let p = plugin::ClientPluginProvider::with_runner(PluginClient::Claude, runner);
        p.set_client_state_owner(Arc::new(FakeStateOwner));
        p
    }
}
#[derive(Default)]
struct Inert(Mutex<Vec<ProcessSpec>>);
impl ProcessRunner for Inert {
    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        self.0.lock().unwrap().push(spec.clone());
        Ok(ProcessOutput {
            code: Some(0),
            stdout: if spec.args == ["--version"] {
                b"Claude Code 2.1.299\n".to_vec()
            } else {
                b"[]\n".to_vec()
            },
            ..Default::default()
        })
    }
}
fn pure_interval(world: &World, query: impl FnOnce()) {
    use std::{cell::Cell, collections::BTreeMap, path::Path, rc::Rc};
    fn census(root: &Path) -> Option<BTreeMap<String, Vec<u8>>> {
        fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for e in std::fs::read_dir(at).unwrap() {
                let p = e.unwrap().path();
                let meta = std::fs::symlink_metadata(&p).unwrap();
                let key = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                if meta.file_type().is_symlink() {
                    out.insert(
                        format!("link:{key}"),
                        std::fs::read_link(&p)
                            .unwrap()
                            .as_os_str()
                            .as_encoded_bytes()
                            .to_vec(),
                    );
                } else if meta.is_dir() {
                    out.insert(format!("dir:{key}"), vec![]);
                    walk(root, &p, out);
                } else {
                    out.insert(format!("file:{key}"), std::fs::read(&p).unwrap());
                }
            }
        }
        if !root.exists() {
            return None;
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        Some(out)
    }
    let roots = [
        world.project.path(),
        world.settings.path(),
        world.home.path(),
        world.fake.path(),
        world.state_home.as_path(),
    ];
    let before: Vec<_> = roots.iter().map(|r| census(r)).collect();
    let mutations = Rc::new(Cell::new(0));
    for arm in [
        vibe_safefs::arm_before_create_dir,
        vibe_safefs::arm_before_link,
        vibe_safefs::arm_before_proved_removal,
        vibe_safefs::arm_before_lock,
    ] {
        let counter = mutations.clone();
        arm(Some(Box::new(move |_, _| counter.set(counter.get() + 1))));
    }
    struct Disarm;
    impl Drop for Disarm {
        fn drop(&mut self) {
            vibe_safefs::arm_before_create_dir(None);
            vibe_safefs::arm_before_link(None);
            vibe_safefs::arm_before_proved_removal(None);
            vibe_safefs::arm_before_lock(None);
        }
    }
    let _guard = Disarm;
    query();
    assert_eq!(mutations.get(), 0);
    assert_eq!(roots.iter().map(|r| census(r)).collect::<Vec<_>>(), before);
    println!(
        "correction query interval: safefs_mutation_attempts=0 finite_roots=5 membership_and_bytes_equal=true"
    );
}
fn assert_refusal(
    world: &World,
    p: &plugin::ClientPluginProvider,
    req: &protocol::DeployTargetRequest<'_>,
    plan: &protocol::DeployPlan,
    reason: &str,
) {
    pure_interval(world, || {
        for error in [
            p.plan(req).unwrap_err(),
            p.fingerprint(req, plan).unwrap_err(),
            p.prepared_query_observation()
                .unwrap()
                .current(req)
                .unwrap_err(),
        ] {
            assert!(error.to_string().contains(reason), "{error}");
        }
    });
}
#[test]
#[cfg(unix)]
fn admitted_parent_and_leaf_symlink_preserves_launcher_execution() {
    use std::os::unix::fs::symlink;
    let mut c = Case::new();
    let image = c.launcher().to_owned();
    let store = c.world.fake.path().join("store");
    std::fs::create_dir(&store).unwrap();
    let target = store.join("image");
    std::fs::rename(&image, &target).unwrap();
    let parent = c.world.fake.path().join("current");
    symlink(&store, &parent).unwrap();
    symlink("image", store.join("launcher")).unwrap();
    let launcher = parent.join("launcher");
    c.world.clients.claude = ClientExecutable::Resolved {
        command: "claude".into(),
        path: launcher.clone(),
    };
    let runner = Arc::new(Inert::default());
    let p = c.provider(runner.clone());
    p.prepare_plan(&c.request()).unwrap();
    let first = p.plan(&c.request()).unwrap();
    assert_eq!(p.plan(&c.request()).unwrap(), first);
    let calls = runner.0.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|s| s.program == launcher));
    println!("F1 symlinks: preparation_calls=2 query_calls=0 launcher_spelling_preserved=true");
}
#[test]
fn large_inert_executable_uses_size_independent_observation() {
    let c = Case::new();
    let file = std::fs::File::create(c.launcher()).unwrap();
    file.set_len(265_342_136).unwrap();
    drop(file);
    let runner = Arc::new(Inert::default());
    let p = c.provider(runner.clone());
    p.prepare_plan(&c.request()).unwrap();
    let plan = p.plan(&c.request()).unwrap();
    p.fingerprint(&c.request(), &plan).unwrap();
    assert_eq!(runner.0.lock().unwrap().len(), 2);
    println!(
        "F1 sparse fixture: bytes=265342136 retained_executable_bytes=0 preparation_calls=2 query_calls=0"
    );
}
#[test]
#[cfg(unix)]
fn equal_byte_launcher_retarget_refuses_for_parent_and_leaf() {
    use std::os::unix::fs::symlink;
    for parent_link in [false, true] {
        let mut c = Case::new();
        let base = c.world.fake.path();
        let a = base.join("a");
        let b = base.join("b");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        std::fs::copy(c.launcher(), a.join("image")).unwrap();
        std::fs::copy(c.launcher(), b.join("image")).unwrap();
        let link = base.join("link");
        symlink(
            if parent_link {
                a.clone()
            } else {
                a.join("image")
            },
            &link,
        )
        .unwrap();
        c.world.clients.claude = ClientExecutable::Resolved {
            command: "claude".into(),
            path: if parent_link {
                link.join("image")
            } else {
                link.clone()
            },
        };
        let runner = Arc::new(Inert::default());
        let p = c.provider(runner.clone());
        p.prepare_plan(&c.request()).unwrap();
        let plan = p.plan(&c.request()).unwrap();
        std::fs::remove_file(&link).unwrap();
        symlink(
            if parent_link {
                b.clone()
            } else {
                b.join("image")
            },
            &link,
        )
        .unwrap();
        assert_refusal(&c.world, &p, &c.request(), &plan, "client-capture-stale");
        assert_eq!(runner.0.lock().unwrap().len(), 2);
    }
}
#[test]
#[cfg(unix)]
fn executable_mode_content_and_object_drift_refuse() {
    use std::os::unix::fs::PermissionsExt;
    for change in ["mode", "content", "object"] {
        let c = Case::new();
        let runner = Arc::new(Inert::default());
        let p = c.provider(runner.clone());
        p.prepare_plan(&c.request()).unwrap();
        let plan = p.plan(&c.request()).unwrap();
        match change {
            "mode" => {
                std::fs::set_permissions(c.launcher(), std::fs::Permissions::from_mode(0o600))
                    .unwrap()
            }
            "content" => std::fs::write(c.launcher(), b"changed").unwrap(),
            _ => {
                let next = c.world.fake.path().join("replacement");
                std::fs::copy(c.launcher(), &next).unwrap();
                std::fs::rename(next, c.launcher()).unwrap();
            }
        }
        assert_refusal(&c.world, &p, &c.request(), &plan, "client-capture-stale");
        assert_eq!(runner.0.lock().unwrap().len(), 2);
    }
}
#[test]
fn owned_client_disable_remove_scope_and_version_drift_refuse() {
    for client in [PluginClient::Claude, PluginClient::Codex] {
        for change in ["disable", "remove", "scope", "version"] {
            let world = World::new();
            let files = if client == PluginClient::Claude {
                FILES.to_vec()
            } else {
                vec![(".codex-plugin/plugin.json", FILES[0].1), FILES[1]]
            };
            let id = format!("{}-owned", client.as_str());
            world.record_projection(&id, &files);
            let targets = [target(client, "demo", &id, "demo-plugin")];
            let selection = selection("demo");
            let execution = world.execution(&targets, &selection);
            execute_deploy_targets(&execution).unwrap();
            let prepared = prepare_deploy_query_with_client_state_owner(
                &execution,
                Some(Arc::new(FakeStateOwner)),
            )
            .unwrap();
            assert!(!prepared.plan().unwrap().is_empty());
            let state = world.at(&format!(".{}/fake-plugin-state.txt", client.as_str()));
            let mut fields: Vec<String> = std::fs::read_to_string(&state)
                .unwrap()
                .split('|')
                .map(str::to_owned)
                .collect();
            match change {
                "disable" => fields[4] = "false".into(),
                "scope" => fields[5] = "false".into(),
                "version" => fields[2] = "9.9.9".into(),
                _ => {}
            }
            if change == "remove" {
                std::fs::remove_file(&state).unwrap();
            } else {
                std::fs::write(&state, fields.join("|")).unwrap();
            }
            let before = world.trace();
            let home = world.home_census();
            pure_interval(&world, || {
                assert!(
                    prepared
                        .plan()
                        .unwrap_err()
                        .to_string()
                        .contains("client-capture-stale")
                );
            });
            assert_eq!(world.trace(), before);
            assert_eq!(world.home_census(), home);
        }
    }
    println!(
        "F2 owned fixture: clients=2 drift_axes=4 query_process_calls=0 home_membership_and_bytes_unchanged=true"
    );
}
#[test]
fn absent_state_then_foreign_installation_refuses_all_query_views() {
    let c = Case::new();
    let p = c.provider(Arc::new(SystemProcessRunner));
    p.prepare_plan(&c.request()).unwrap();
    let plan = p.plan(&c.request()).unwrap();
    plugin::fixture::write(
        c.world.home.path(),
        ".claude/fake-plugin-state.txt",
        "foreign|demo-plugin|1.4.2|true|true|true",
    );
    let trace = c.world.trace();
    let home = c.world.home_census();
    assert_refusal(&c.world, &p, &c.request(), &plan, "client-capture-stale");
    assert_eq!(c.world.trace(), trace);
    assert_eq!(c.world.home_census(), home);
}
#[test]
fn client_state_change_during_list_refuses_capture() {
    struct Mutator;
    impl ProcessRunner for Mutator {
        fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
            let output = SystemProcessRunner.run(spec)?;
            if spec.args.first().is_some_and(|s| s == "plugin") {
                let root =
                    std::path::PathBuf::from(spec.env.get(std::ffi::OsStr::new("HOME")).unwrap());
                plugin::fixture::write(
                    &root,
                    ".claude/fake-plugin-state.txt",
                    "foreign|demo-plugin|1.4.2|true|true|true",
                );
            }
            Ok(output)
        }
    }
    let c = Case::new();
    let p = c.provider(Arc::new(Mutator));
    assert!(
        p.prepare_plan(&c.request())
            .unwrap_err()
            .to_string()
            .contains("client-capture-stale")
    );
    assert!(p.prepared_query_observation().is_none());
    assert_eq!(c.world.trace().len(), 2);
}
#[test]
fn unqualified_state_keeps_historical_probes_but_all_queries_refuse() {
    let c = Case::new();
    let p = plugin::ClientPluginProvider::new(PluginClient::Claude);
    p.prepare_plan(&c.request()).unwrap();
    let (plan, fingerprint) = p.preparation_result(&c.request()).unwrap();
    assert_eq!(fingerprint.digest.len(), 64);
    assert_eq!(c.world.trace().len(), 2);
    let trace = c.world.trace();
    let home = c.world.home_census();
    assert_refusal(
        &c.world,
        &p,
        &c.request(),
        &plan,
        "client-state-unavailable",
    );
    assert_eq!(c.world.trace(), trace);
    assert_eq!(c.world.home_census(), home);
    assert!(!c.world.state_home.exists());
}
#[test]
fn absent_and_unsupported_executables_have_distinct_refusals() {
    let c = Case::new();
    let runner = Arc::new(Inert::default());
    let p = c.provider(runner.clone());
    std::fs::remove_file(c.launcher()).unwrap();
    assert!(
        p.prepare_plan(&c.request())
            .unwrap_err()
            .to_string()
            .contains("client-capture-missing")
    );
    std::fs::create_dir(c.launcher()).unwrap();
    assert!(
        p.prepare_plan(&c.request())
            .unwrap_err()
            .to_string()
            .contains("client-executable-unavailable")
    );
    assert!(runner.0.lock().unwrap().is_empty());
}
