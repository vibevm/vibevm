use super::*;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, sync::Arc};
use vibe_core::manifest::{DeployTarget, ExtensionConfig, ExtensionsControl, MechanismRoutes};
use vibe_extension_registry::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_mechanisms,
};
use vibe_lifecycle::{
    ClientExecutable, ClientExecutables, LifecycleLease, SubmissionReservations, SubmissionScope,
};
use vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployHost;
use vibe_safefs::Project;
use vibe_wire::{behaviour::artifact_record::validate, generated::artifact_record::ArtifactRecord};

fn action(kind: &str, key: &str) -> Value {
    json!({"schema":1,"action":{"kind":kind,"key":key}})
}
fn call(ctx: ServerContext, args: Value) -> Value {
    let request = json!({"jsonrpc":"2.0","id":31,"method":"tools/call",
        "params":{"name":"fixed_deploy_operation","arguments":args}});
    let response = crate::dispatch_one(ctx, &request.to_string()).unwrap();
    serde_json::from_str::<Value>(&response).unwrap()["result"].clone()
}

fn registry() -> Arc<vibe_extension_registry::MechanismRegistry> {
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("fixed-deploy-mcp"),
                root: ".".into(),
                version: "1.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    Arc::new(collect_mechanisms(&world).unwrap())
}
fn clients() -> ClientExecutables {
    ClientExecutables {
        claude: ClientExecutable::Missing {
            command: "claude".into(),
        },
        codex: ClientExecutable::Missing {
            command: "codex".into(),
        },
        opencode: ClientExecutable::Missing {
            command: "opencode".into(),
        },
    }
}
struct Fixture {
    project: tempfile::TempDir,
    settings: tempfile::TempDir,
    home: tempfile::TempDir,
    scope: SubmissionScope,
    namespace: vibe_lifecycle::SubmissionNamespace,
}
impl Fixture {
    fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let settings = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let manifest = concat!(
            "[project]\nname = 'demo'\nversion = '1.1.0'\n\n",
            "[[artifacts.build]]\nid = 'helper'\nmechanism = 'build:cargo'\n",
            "outputs = [{ id = 'helper.exe', kind = 'executable' }]\n\n",
            "[[deploy.target]]\nid = 'local-helper'\nartifact = 'helper.exe'\n",
            "mechanism = 'deploy:vibe-bin'\ndepends_on = []\n",
            "config = { command = 'local-helper' }\n\n",
            "[deploy.profiles.local]\ntargets = ['local-helper']\n",
        );
        fs::write(project.path().join("vibe.toml"), manifest).unwrap();
        let artifact = project.path().join("target/debug/helper.exe");
        fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        fs::write(&artifact, b"helper executable").unwrap();
        record(project.path(), &artifact);
        let scope = SubmissionScope {
            context: "trusted-deploy-mcp".into(),
            target: "instance-a".into(),
            principal: "owner".into(),
        };
        let lease = LifecycleLease::acquire(project.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        Self {
            project,
            settings,
            home,
            scope,
            namespace,
        }
    }
    fn route(&self) -> Arc<FixedDeployRoute> {
        let target = DeployTarget {
            id: "local-helper".into(),
            artifact: "helper.exe".into(),
            mechanism: "deploy:vibe-bin".parse().unwrap(),
            provider: None,
            when: None,
            depends_on: Some(Vec::new()),
            config: Some(ExtensionConfig::from_table(
                "command = 'local-helper'".parse::<toml::Table>().unwrap(),
            )),
        };
        let host = FixedDeployHost::from_trusted_host(
            Arc::new(Project::open(self.project.path()).unwrap()),
            Arc::new(Project::open(self.settings.path()).unwrap()),
            Arc::new(Project::open(self.home.path()).unwrap()),
            self.namespace.clone(),
            self.scope.clone(),
            "local".into(),
            target,
            registry(),
            Arc::new(MechanismRoutes::default()),
            clients(),
            "org.example/demo".into(),
            Some("generation-a".into()),
            "nonce-a".into(),
            1,
            Arc::new(|| Ok(Some("generation-a".into()))),
        )
        .unwrap();
        Arc::new(FixedDeployRoute::new(Arc::new(host)))
    }
    fn context(&self, route: Arc<FixedDeployRoute>) -> ServerContext {
        ServerContext::with_store_root(self.project.path(), self.project.path().join("unused"))
            .with_fixed_deploy_route(route)
            .unwrap()
    }
    fn no_key(&self, key: &str) {
        let lease = LifecycleLease::acquire(self.project.path()).unwrap();
        assert!(
            SubmissionReservations::inspect_existing_key(&lease, &self.namespace, &self.scope, key)
                .unwrap()
                .is_none()
        );
    }
}
fn record(root: &Path, artifact: &Path) {
    let digest = format!("{:x}", Sha256::digest(fs::read(artifact).unwrap()));
    let absolute = artifact.to_string_lossy().replace('\\', "/");
    let record: ArtifactRecord = serde_json::from_value(json!({
        "schema":1,"id":"helper.exe","kind":"executable","shape":"file",
        "path_absolute":absolute,
        "path_relative":{"root":"project","path":"target/debug/helper.exe"},
        "digest":{"algorithm":"sha256","value":digest},
        "producer":{"target":"helper","mechanism":"build:cargo","provider":{"key":"org.vibevm/vibe#cargo"}},
        "freshness":{},"created_at":"2026-10-03T12:00:00Z",
        "verification":{"status":"verified","evidence":"fixture artifact"}
    })).unwrap();
    validate(&record).unwrap();
    let home = root.join(".vibe/state/artifacts");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join("helper.exe.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
}

#[test]
fn registered_but_ungranted_deploy_refuses_without_state() {
    let root = tempfile::tempdir().unwrap();
    let descriptor = FixedDeployOperationMcpTool.descriptor();
    assert_eq!(descriptor.name, "fixed_deploy_operation");
    assert_eq!(descriptor.input_schema["additionalProperties"], false);
    let listed: Value = serde_json::from_str(
        &crate::dispatch_one(
            ServerContext::with_store_root(root.path(), root.path().join("unused")),
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "fixed_deploy_operation")
    );
    let result = call(
        ServerContext::with_store_root(root.path(), root.path().join("unused")),
        action("inspect", "once"),
    );
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "unavailable_host");
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn forged_deploy_fields_refuse_before_owner_dispatch() {
    let fixture = Fixture::new();
    let route = fixture.route();
    for args in [
        json!({"schema":2,"action":{"kind":"submit","key":"once"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","settings_root":"/tmp"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","profile":"foreign"}}),
        json!({"schema":1,"action":{"kind":"submit","key":7}}),
        json!({"schema":1,"action":{"kind":"submit","key":"x".repeat(130_000)}}),
    ] {
        let result = call(fixture.context(route.clone()), args);
        assert_eq!(result["isError"], true, "{result}");
        assert_eq!(
            result["structuredContent"]["code"], "invalid_request",
            "{result}"
        );
    }
    fixture.no_key("once");
}

#[test]
fn fixed_deploy_submit_and_cold_inspect_share_one_owner_run() {
    let fixture = Fixture::new();
    let route = fixture.route();
    let absent = call(fixture.context(route.clone()), action("inspect", "once"));
    assert_eq!(
        absent["structuredContent"]["reply"]["kind"], "absent",
        "{absent}"
    );
    let first = call(fixture.context(route.clone()), action("submit", "once"));
    assert_eq!(first["isError"], false, "{first}");
    assert_eq!(
        first["structuredContent"]["reply"]["kind"], "completed",
        "{first}"
    );
    let run = first["structuredContent"]["reply"]["run_id"]
        .as_str()
        .unwrap();
    let result = fixture.project.path().join(format!(
        ".vibe/deploy-operations/{run}.bound-vibe-bin-result.json"
    ));
    assert!(result.is_file());
    let cold = call(fixture.context(fixture.route()), action("submit", "once"));
    assert_eq!(cold["structuredContent"], first["structuredContent"]);
    let index: Value = serde_json::from_slice(
        &fs::read(
            fixture
                .project
                .path()
                .join(".vibe/machine-submissions.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
}
