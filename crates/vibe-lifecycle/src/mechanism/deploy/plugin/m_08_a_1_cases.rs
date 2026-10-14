//! Plugin characterization below the process boundary. No process is launched.
use super::artifact::{AdmittedProjection, ProjectionFile};
use super::*;
use crate::mechanism::deploy::skill::{client::SkillClient, support::*};
use crate::mechanism::deploy::state::{DeployState, DeploymentHome};
use crate::process::{ProcessError, ProcessOutput, ProcessSpec};
use specmark::verifies;

const FIRST: &str = include_str!("../../../../../../fixtures/next/adapters/skill-v1.md");
const FOREIGN: &[u8] = include_bytes!("../../../../../../fixtures/next/adapters/foreign.json");
fn projection(world: &World) -> AdmittedProjection {
    let artifact = world.skill_artifact("fixture", FIRST);
    AdmittedProjection {
        identity: None,
        files: vec![ProjectionFile {
            relative: "skills/demo/SKILL.md".into(),
            absolute: artifact.absolute,
            digest: artifact.digest,
        }],
        opencode_mcp: serde_json::from_value(serde_json::json!({
            "demo": {"type":"local","command":["inert-fixture"],"enabled":true}
        }))
        .unwrap(),
    }
}
fn apply_projection(
    world: &World,
    request: &DeployTargetRequest<'_>,
    projection: &AdmittedProjection,
) {
    let state = DeployState::open(world.state.path()).unwrap();
    let home = DeploymentHome::new(
        world.state.path(),
        "org.example/demo",
        None,
        &request.target.id,
    );
    let mut ledger = CheckpointLedger::open(&state, &home, "fixture-plan").unwrap();
    opencode::apply(request, projection, &mut ledger).unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-OWNERSHIP")]
fn opencode_logical_members_physical_lock_exact_bytes_and_scoped_remove() {
    let world = World::new();
    let row = target(SkillClient::OpenCode, "plugin-target", "fixture", "demo");
    let projection = projection(&world);
    let config = ".config/opencode/opencode.json";
    write_home(&world, config, std::str::from_utf8(FOREIGN).unwrap());
    let req = request(&world, &row, None, None, true);
    let plan = opencode::plan(&req, &projection, "fixture".into()).unwrap();
    let resources: Vec<_> = plan.resources.iter().map(|r| r.resource.clone()).collect();
    assert_eq!(
        resources,
        [
            "home:.config/opencode/opencode.json#mcp/demo",
            "home:.config/opencode/skills/demo/SKILL.md"
        ]
    );
    assert_eq!(
        plan.lock_resources,
        [
            "home:.config/opencode/opencode.json",
            "home:.config/opencode/skills/demo/SKILL.md"
        ]
    );
    assert_eq!(
        std::fs::read(world.at(config)).unwrap(),
        FOREIGN,
        "planning preserves exact bytes"
    );
    apply_projection(&world, &req, &projection);
    let expected = "{\n  \"mcp\": {\n    \"demo\": {\n      \"command\": [\n        \"inert-fixture\"\n      ],\n      \"enabled\": true,\n      \"type\": \"local\"\n    },\n    \"foreign\": {\n      \"command\": [\n        \"never-executed\"\n      ],\n      \"enabled\": false,\n      \"type\": \"local\"\n    }\n  },\n  \"theme\": \"dark\"\n}\n";
    assert_eq!(
        std::fs::read(world.at(config)).unwrap(),
        expected.as_bytes()
    );
    assert_eq!(
        std::fs::read(world.at(".config/opencode/skills/demo/SKILL.md")).unwrap(),
        FIRST.as_bytes()
    );
    let owned: Vec<_> = plan
        .resources
        .iter()
        .map(|r| (r.resource.as_str(), r.desired_digest.as_str()))
        .collect();
    let receipt = receipt_owning(0, &owned);
    let req = request(&world, &row, None, Some(&receipt), true);
    apply_projection(&world, &req, &projection);
    assert_eq!(
        std::fs::read(world.at(config)).unwrap(),
        expected.as_bytes()
    );
    opencode::remove(&req, &resources).unwrap();
    let remaining: serde_json::Value =
        serde_json::from_slice(&std::fs::read(world.at(config)).unwrap()).unwrap();
    let foreign: serde_json::Value = serde_json::from_slice(FOREIGN).unwrap();
    assert_eq!(remaining, foreign);
    assert_eq!(
        std::fs::read(world.at(config)).unwrap(),
        include_bytes!("../../../../../../fixtures/next/adapters/foreign-after-remove.json")
    );
    assert!(!world.at(".config/opencode/skills/demo/SKILL.md").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-OWNERSHIP")]
fn opencode_equal_byte_foreign_members_and_unsupported_resources_refuse() {
    for skill in [true, false] {
        let world = World::new();
        let row = target(SkillClient::OpenCode, "plugin-target", "fixture", "demo");
        let projection = projection(&world);
        if skill {
            write_home(&world, ".config/opencode/skills/demo/SKILL.md", FIRST);
        } else {
            write_home(
                &world,
                ".config/opencode/opencode.json",
                "{\"mcp\":{\"demo\":{\"type\":\"local\",\"command\":[\"inert-fixture\"],\"enabled\":true}}}\n",
            );
        }
        assert!(
            opencode::plan(
                &request(&world, &row, None, None, false),
                &projection,
                "fixture".into()
            )
            .unwrap_err()
            .to_string()
            .contains("unowned occupant")
        );
        write_home(&world, "foreign/sentinel", "keep\n");
        let receipt = receipt_owning(0, &[("home:foreign/sentinel", &"0".repeat(64))]);
        assert!(
            opencode::remove(
                &request(&world, &row, None, Some(&receipt), false),
                &["home:foreign/sentinel".into()]
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(world.at("foreign/sentinel")).unwrap(),
            b"keep\n"
        );
    }
    let world = World::new();
    let row = target(SkillClient::OpenCode, "plugin-target", "fixture", "demo");
    write_home(&world, ".config/opencode/opencode.json", "{\"mcp\":[]}\n");
    assert!(
        opencode::plan(
            &request(&world, &row, None, None, false),
            &projection(&world),
            "fixture".into()
        )
        .is_err()
    );
}

#[derive(Default)]
struct Capture(Mutex<Vec<ProcessSpec>>);
impl ProcessRunner for Capture {
    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        self.0.lock().unwrap().push(spec.clone());
        Ok(ProcessOutput {
            code: Some(0),
            ..ProcessOutput::default()
        })
    }
}
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-SURFACES")]
fn cli_plugin_wire_is_inert_user_scope_install_remove_and_update_refusal() {
    for (client, add, install, remove, private, lock) in [
        (
            PluginClient::Claude,
            vec!["plugin", "marketplace", "add", "--scope", "user"],
            vec!["plugin", "install", "--scope", "user", "demo@fixture"],
            vec!["plugin", "uninstall", "--scope", "user", "demo@fixture"],
            ".claude",
            "client-state:claude:plugins",
        ),
        (
            PluginClient::Codex,
            vec!["plugin", "marketplace", "add", "--json"],
            vec!["plugin", "add", "--json", "demo@fixture"],
            vec!["plugin", "remove", "--json", "demo@fixture"],
            ".codex",
            "client-state:codex:plugins",
        ),
    ] {
        let mut world = World::new();
        let executable = crate::mechanism::deploy::ClientExecutable::Resolved {
            command: client.as_str().into(),
            path: world.project.path().join("does-not-exist"),
        };
        match client {
            PluginClient::Claude => world.clients.claude = executable,
            PluginClient::Codex => world.clients.codex = executable,
            _ => unreachable!(),
        }
        let row = target(SkillClient::Claude, "plugin-target", "fixture", "demo");
        let req = request(&world, &row, None, None, false);
        let capture = Capture::default();
        let root = world.settings.path().join("marketplace");
        wire::marketplace_add(&capture, client, &req, &root).unwrap();
        wire::install(&capture, client, &req, "demo@fixture").unwrap();
        wire::remove(&capture, client, &req, "demo@fixture").unwrap();
        let calls = capture.0.lock().unwrap();
        let mut expected_add: Vec<std::ffi::OsString> = add.iter().map(Into::into).collect();
        expected_add.push(root.into_os_string());
        assert_eq!(calls[0].args, expected_add);
        assert_eq!(calls[1].args, install);
        assert_eq!(calls[2].args, remove);
        for call in calls.iter() {
            assert_eq!(call.cwd, world.project.path());
            assert_eq!(call.program, world.project.path().join("does-not-exist"));
            assert!(!call.program.exists());
            let (key, path) = client.private_root(world.home.path()).unwrap();
            assert_eq!(path, world.home.path().join(private));
            assert_eq!(
                call.env.get(std::ffi::OsStr::new(key)),
                Some(&path.into_os_string())
            );
        }
        assert_eq!(client.logical_lock(), lock);
        let provider = ClientPluginProvider {
            client,
            runner: Arc::new(Capture::default()),
            planned_version: Mutex::new(None),
        };
        let receipt = receipt_owning(0, &[]);
        let req = request(&world, &row, None, Some(&receipt), false);
        assert!(
            provider
                .refuse_artifact_update(&req, &"1".repeat(64))
                .unwrap_err()
                .to_string()
                .contains("undeploy, then deploy")
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-OWNERSHIP")]
fn plugin_admission_rejects_file_and_unsupported_tree_shapes() {
    for client in PluginClient::ALL {
        let world = World::new();
        let mut file = world.skill_artifact("fixture", FIRST);
        assert!(artifact::admit("fixture", client.pin(), client, &file).is_err());
        let root = world.project.path().join("tree");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("unsupported.js"), b"inert; never execute\n").unwrap();
        file.absolute = root.clone();
        file.kind = ArtifactKind::Directory;
        file.shape = vibe_wire::generated::artifact_record::ArtifactShape::Directory;
        file.digest = crate::mechanism::contain::tree_digest(&root)
            .unwrap()
            .digest;
        assert!(artifact::admit("fixture", client.pin(), client, &file).is_err());
        assert_eq!(
            std::fs::read(root.join("unsupported.js")).unwrap(),
            b"inert; never execute\n"
        );
    }
    // The artifact-update refusal applies to every plugin client, including
    // OpenCode; direct member reconciliation does not expand that contract.
    for client in PluginClient::ALL {
        let world = World::new();
        let row = target(SkillClient::OpenCode, "plugin-target", "fixture", "demo");
        let receipt = receipt_owning(0, &[]);
        let req = request(&world, &row, None, Some(&receipt), false);
        let provider = ClientPluginProvider {
            client,
            runner: Arc::new(Capture::default()),
            planned_version: Mutex::new(None),
        };
        assert!(
            provider
                .refuse_artifact_update(&req, &"1".repeat(64))
                .is_err()
        );
    }
}
