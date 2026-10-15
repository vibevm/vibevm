use super::plugin::{
    PluginClient,
    fixture::{World, selection, target},
};
use super::*;
const FILES: &[(&str, &str)] = &[
    (
        ".claude-plugin/plugin.json",
        "{\"name\":\"demo-plugin\",\"version\":\"1.4.2\"}",
    ),
    ("skills/demo/SKILL.md", "body"),
];
#[test]
fn missing_client_capture_refuses_without_process_and_prepared_query_is_pure() {
    let world = World::new();
    world.record_projection("claude-prepared", FILES);
    let targets = [target(
        PluginClient::Claude,
        "demo",
        "claude-prepared",
        "demo-plugin",
    )];
    let selection = selection("demo");
    let execution = world.execution(&targets, &selection);
    let resolved = resolve_selection(&execution).unwrap();
    plugin::fixture::qualify(&resolved);
    let before = world.trace();
    assert!(
        plan::plan_resolved(&execution, &resolved)
            .unwrap_err()
            .to_string()
            .contains("client-capture-missing")
    );
    assert_eq!(world.trace(), before);
    let prepared = plan::prepare_resolved(&execution, &resolved).unwrap();
    let after = world.trace();
    assert_eq!(after.len() - before.len(), 2);
    let home = world.home_census();
    assert_eq!(
        plan::plan_resolved(&execution, &resolved).unwrap(),
        prepared
    );
    assert_eq!(
        plan::plan_resolved(&execution, &resolved).unwrap(),
        prepared
    );
    assert_eq!(world.trace(), after);
    assert_eq!(world.home_census(), home);
    assert!(!world.state_home.exists());
}

#[test]
fn prepared_three_clients_fingerprint_without_reprobe_and_changed_executable_refuses() {
    for client in PluginClient::ALL {
        let world = World::new();
        let (id, files): (&str, &[(&str, &str)]) = match client {
            PluginClient::Claude => ("claude-query", FILES),
            PluginClient::Codex => (
                "codex-query",
                &[
                    (
                        ".codex-plugin/plugin.json",
                        "{\"name\":\"demo-plugin\",\"version\":\"1.4.2\"}",
                    ),
                    ("skills/demo/SKILL.md", "body"),
                ],
            ),
            PluginClient::OpenCode => (
                "opencode-query",
                &[
                    ("skills/demo/SKILL.md", "body"),
                    ("opencode.json", "{\"mcp\":{}}"),
                ],
            ),
        };
        world.record_projection(id, files);
        let targets = [target(client, "demo", id, "demo-plugin")];
        let selection = selection("demo");
        let execution = world.execution(&targets, &selection);
        let prepared = prepare_deploy_query_with_client_state_owner(
            &execution,
            Some(std::sync::Arc::new(plugin::fixture::FakeStateOwner)),
        )
        .unwrap();
        let trace = world.trace();
        assert_eq!(
            trace.len(),
            if client == PluginClient::OpenCode {
                1
            } else {
                2
            }
        );
        let home = world.home_census();
        let first = prepared.plan().unwrap();
        assert!(!first.is_empty());
        assert_eq!(prepared.plan().unwrap(), first);
        assert_eq!(world.trace(), trace);
        assert_eq!(world.home_census(), home);
        let executable = match client {
            PluginClient::Claude => &world.clients.claude,
            PluginClient::Codex => &world.clients.codex,
            PluginClient::OpenCode => &world.clients.opencode,
        }
        .resolved_path()
        .unwrap();
        // Intentional tamper is outside the measured query interval.
        std::fs::write(executable, "changed executable").unwrap();
        assert!(
            prepared
                .plan()
                .unwrap_err()
                .to_string()
                .contains("client-capture-stale")
        );
        assert_eq!(world.trace(), trace);
        assert!(!world.state_home.exists());
    }
}

#[test]
fn missing_native_result_refuses_before_image_admission() {
    use crate::mechanism::package::support::{empty_world, key, pin};
    use vibe_core::manifest::{ExtensionHandler, MechanismDecl, MechanismFreshness, MechanismRole};
    let fixture = support::Fixture::new("inert");
    let mut world = empty_world();
    world.installed.push(tests::installed(MechanismDecl {
        client_request: None,
        effects: None,
        id: "installer".into(),
        role: MechanismRole::Deploy,
        name: "vibe-bin".into(),
        handler: ExtensionHandler::Native {
            crate_dir: Some("absent-provider".into()),
            prebuilt: None,
        },
        protocol: 1,
        config_schema: "absent-schema".into(),
        freshness: MechanismFreshness::Provider,
    }));
    let registry = vibe_extension_registry::collect_mechanisms(&world).unwrap();
    let mut routes = vibe_core::manifest::MechanismRoutes::default();
    routes.insert(
        key("deploy:vibe-bin"),
        pin("org.example/deployers#installer"),
    );
    let targets = [vibe_core::manifest::DeployTarget {
        id: "native".into(),
        artifact: "helper.exe".into(),
        mechanism: key("deploy:vibe-bin"),
        provider: None,
        when: None,
        depends_on: None,
        config: None,
    }];
    let selection = DeploySelection {
        profile: "local".into(),
        targets: vec!["native".into()],
    };
    let state = fixture.state_home();
    let execution = DeployExecution {
        project_root: fixture.project.path(),
        targets: &targets,
        selection: &selection,
        registry: &registry,
        routes: &routes,
        state_home: &state,
        settings_root: fixture.settings.path(),
        user_home: fixture.home.path(),
        clients: &fixture.clients,
        project: "org.example/demo",
        package: None,
        created_at: "",
    };
    let error = plan_deploy_targets(&execution).unwrap_err();
    assert!(matches!(error, DeployError::Provider(crate::mechanism::MechanismError::Deploy(crate::mechanism::error::DeployProviderError::NeedsPreparation { reason: vibe_wire::generated::adapter_plan::e1::error::ErrorReason::NativePlanCaptureMissing, .. }))));
    assert!(!state.exists());
}

#[test]
fn injected_process_and_mutation_attempts_are_zero_during_plan_and_fingerprint() {
    use crate::process::{
        ProcessError, ProcessOutput, ProcessRunner, ProcessSpec, SystemProcessRunner,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::{cell::Cell, collections::BTreeMap, path::Path, rc::Rc};
    struct Spy(Arc<AtomicUsize>);
    impl ProcessRunner for Spy {
        fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            SystemProcessRunner.run(spec)
        }
    }
    fn census(root: &Path) -> Option<BTreeMap<String, Option<Vec<u8>>>> {
        fn walk(root: &Path, dir: &Path, result: &mut BTreeMap<String, Option<Vec<u8>>>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let p = entry.unwrap().path();
                let key = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                if p.is_dir() {
                    result.insert(key, None);
                    walk(root, &p, result);
                } else {
                    result.insert(key, Some(std::fs::read(p).unwrap()));
                }
            }
        }
        if !root.exists() {
            return None;
        }
        let mut result = BTreeMap::new();
        walk(root, root, &mut result);
        Some(result)
    }
    let world = World::new();
    world.record_projection("claude-spy", FILES);
    let target = target(PluginClient::Claude, "demo", "claude-spy", "demo-plugin");
    let artifact = artifact::resolve_artifact(world.project.path(), &target).unwrap();
    let request = protocol::DeployTargetRequest {
        target: &target,
        profile: "local",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: None,
        recovery_intent: None,
        artifact: Some(&artifact),
        staging: None,
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = plugin::ClientPluginProvider::with_runner(
        PluginClient::Claude,
        Arc::new(Spy(calls.clone())),
    );
    provider.set_client_state_owner(Arc::new(plugin::fixture::FakeStateOwner));
    assert!(provider.plan(&request).is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    provider.prepare_plan(&request).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let roots = [
        world.project.path(),
        world.settings.path(),
        world.home.path(),
        world.fake.path(),
        world.state_home.as_path(),
    ];
    let before: Vec<_> = roots.iter().map(|p| census(p)).collect();
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
    let guard = Disarm;
    let first = provider.plan(&request).unwrap();
    let fingerprint = provider.fingerprint(&request, &first).unwrap();
    assert_eq!(provider.plan(&request).unwrap(), first);
    assert_eq!(provider.fingerprint(&request, &first).unwrap(), fingerprint);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(mutations.get(), 0);
    assert_eq!(roots.iter().map(|p| census(p)).collect::<Vec<_>>(), before);
    println!(
        "B1 measured: preparation_process_attempts=2 query_process_attempts=0 safefs_mutation_attempts=0 finite_roots=5 state_absent=true"
    );
    drop(guard);
}

mod correction;
