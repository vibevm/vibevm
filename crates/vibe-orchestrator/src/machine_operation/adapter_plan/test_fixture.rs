// Shared owned fixture for configured route/CLI/MCP consumer tests.
use super::adapter_owner::{
    AdapterDeployPreparation, AdapterPlanHost, AdapterPlanRoute, AdapterTarget,
};
struct ScriptStateOwner;
impl vibe_lifecycle::ClientStateOwner for ScriptStateOwner {
    fn observe(
        &self,
        binding: &vibe_lifecycle::ClientStateBinding,
    ) -> Result<vibe_lifecycle::ClientStateWitness, String> {
        if binding.client != "claude" || binding.scope != "user" {
            return Err("fixture scope".into());
        }
        // The script below reads exactly list.json, or emits [] on absence.
        vibe_lifecycle::ClientStateWitness::observe_files(
            "list-json-script-fixture/1",
            binding,
            &binding.user_home,
            &["list.json"],
        )
    }
}
pub fn deploy_route(
    qualified: bool,
) -> (
    tempfile::TempDir,
    tempfile::TempDir,
    std::sync::Arc<AdapterPlanRoute>,
    std::path::PathBuf,
) {
    use std::os::unix::fs::PermissionsExt;
    use vibe_lifecycle::{ClientExecutable, ClientExecutables, DeploySelection};
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[package]\ngroup='org.example'\nname='fixture'\nkind='tool'\nversion='1.0.0'\nauthors=['Fixture']\nlicense='MIT'\ndescription='Fixture'\nkeywords=['fixture']\n").unwrap();
    let owned = tempfile::tempdir().unwrap();
    let settings = owned.path().join("settings");
    let home = owned.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let artifact = root.path().join("projection");
    std::fs::create_dir_all(artifact.join(".claude-plugin")).unwrap();
    std::fs::create_dir_all(artifact.join("skills/demo")).unwrap();
    std::fs::write(
        artifact.join(".claude-plugin/plugin.json"),
        r#"{"name":"demo-plugin","version":"1.4.2"}"#,
    )
    .unwrap();
    std::fs::write(artifact.join("skills/demo/SKILL.md"), "body").unwrap();
    // Fixed inert fixture digest, independently checked by the actual artifact owner.
    let record = serde_json::json!({"schema":1,"id":"projection","kind":"directory","shape":"directory","created_at":"2026-10-15T00:00:00Z","digest":{"algorithm":"sha256-tree/1","value":"e1af14a7ce53620e6231a0d383ba1c885fbbff99bf890016debeee35324072e3"},"freshness":{},"path_absolute":artifact,"path_relative":{"root":"project","path":"projection"},"producer":{"target":"projection","mechanism":"package:claude-plugin","provider":{"key":"org.vibevm/vibe#fixture"}},"verification":{"status":"unverified"}});
    std::fs::create_dir_all(root.path().join(".vibe/state/artifacts")).unwrap();
    std::fs::write(
        root.path().join(".vibe/state/artifacts/projection.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    let executable = owned.path().join("client");
    let trace = owned.path().join("calls");
    std::fs::write(&executable, format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$1\" in --version) printf '2.1.0\\n';; plugin) if [ -f '{}' ]; then /bin/cat '{}'; else printf '[]\\n'; fi;; *) exit 9;; esac\n", trace.display(), home.join("list.json").display(), home.join("list.json").display())).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let clients = ClientExecutables {
        claude: ClientExecutable::Resolved {
            command: "claude".into(),
            path: executable.clone(),
        },
        codex: ClientExecutable::Missing {
            command: "codex".into(),
        },
        opencode: ClientExecutable::Missing {
            command: "opencode".into(),
        },
    };
    let targets = vec![vibe_core::manifest::DeployTarget {
        id: "demo".into(),
        artifact: "projection".into(),
        mechanism: "deploy:claude-plugin".parse().unwrap(),
        provider: None,
        when: None,
        depends_on: None,
        config: Some(vibe_core::manifest::ExtensionConfig::from_table(
            "name='demo-plugin'".parse().unwrap(),
        )),
    }];
    let selection = DeploySelection {
        profile: "local".into(),
        targets: vec!["demo".into()],
    };
    let manifest_path = root.path().join("vibe.toml");
    let mut manifest = std::fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str("\n[[artifacts.package]]\nid='produce-projection'\nmechanism='package:claude-plugin'\ninputs=[{path='skill'}]\noutputs=[{id='projection',kind='directory'}]\n[[deploy.target]]\nid='demo'\nartifact='projection'\nmechanism='deploy:claude-plugin'\nconfig={name='demo-plugin'}\n[deploy.profiles.local]\ntargets=['demo']\n");
    vibe_core::manifest::Manifest::parse_str(&manifest).unwrap();
    std::fs::write(manifest_path, manifest).unwrap();
    let host = AdapterPlanHost::new(
        root.path(),
        AdapterTarget::Deploy {
            profile: "local".into(),
        },
    )
    .unwrap();
    let capture = host
        .prepare_deploy_with_client_state_owner(
            true,
            AdapterDeployPreparation {
                targets: &targets,
                os: vibe_core::manifest::TargetOs::Linux,
                selection: &selection,
                state_home: &settings.join("state/deployments"),
                settings_root: &settings,
                user_home: &home,
                clients: &clients,
                project: "org.example/fixture",
            },
            qualified.then(|| {
                std::sync::Arc::new(ScriptStateOwner)
                    as std::sync::Arc<dyn vibe_lifecycle::ClientStateOwner>
            }),
        )
        .unwrap();
    let route = std::sync::Arc::new(AdapterPlanRoute::new(host, Some(capture)).unwrap());
    (root, owned, route, trace)
}
