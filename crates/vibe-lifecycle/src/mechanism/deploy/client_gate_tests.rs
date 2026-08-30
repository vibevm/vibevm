//! The final R8-CLIENTS commissioning gate: real package records compose
//! with all six shipped user-client destinations in one isolated world.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;
use specmark::verifies;
use vibe_core::manifest::DeployTarget;
use vibe_extension_registry::SelectionStep;
use vibe_wire::generated::artifact_record::{ArtifactKind as RecordKind, ArtifactShape};
use vibe_wire::generated::deploy_receipt::ReceiptStatus;

use super::plugin::PluginClient;
use super::plugin::fixture::{World, target as plugin_target, write};
use super::skill::SkillClient;
use super::skill::support::{FailingAfterWrite, selection, target as skill_target};
use super::state::{DeployState, DeploymentHome};
use super::{
    DeployError, Selected, apply_selection, execute_deploy_targets, list_deployments,
    undeploy_targets,
};
use crate::mechanism::client_projection::support as projection;
use crate::mechanism::package::PackagedArtifact;
use crate::mechanism::package::support as package;

const STANDALONE_ENTRY: &str = concat!(
    "---\n",
    "name: standalone\n",
    "description: A standalone commissioning skill.\n",
    "---\n\n",
    "Standalone body.\n",
);
const RECOVERY_ENTRY: &str = concat!(
    "---\n",
    "name: recovery\n",
    "description: A recovery commissioning skill.\n",
    "---\n\n",
    "Recovery body.\n",
);
const COLLISION_ENTRY: &str = concat!(
    "---\n",
    "name: demo\n",
    "description: A deliberately colliding standalone skill.\n",
    "---\n\n",
    "Collision body.\n",
);

struct ProjectionArtifacts {
    claude: String,
    codex: String,
    opencode: String,
}

struct ForeignSnapshot {
    files: Vec<(PathBuf, Vec<u8>)>,
    root_value: Value,
    mcp_value: Value,
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn real_packages_project_and_commission_all_six_client_destinations() {
    let world = World::new();
    assert_fake_executables(&world);
    let skill = package_skill(&world, "standalone", "standalone", STANDALONE_ENTRY);
    let projected = package_all_projections(&world);
    let foreign = plant_foreign_neighbours(&world);

    let targets = vec![
        skill_target(SkillClient::Claude, "claude-skill", &skill.id, "standalone"),
        skill_target(SkillClient::Codex, "codex-skill", &skill.id, "standalone"),
        skill_target(
            SkillClient::OpenCode,
            "opencode-skill",
            &skill.id,
            "standalone",
        ),
        plugin_target(
            PluginClient::Claude,
            "claude-plugin",
            &projected.claude,
            "demo-plugin",
        ),
        plugin_target(
            PluginClient::Codex,
            "codex-plugin",
            &projected.codex,
            "demo-plugin",
        ),
        plugin_target(
            PluginClient::OpenCode,
            "opencode-plugin",
            &projected.opencode,
            "demo-plugin",
        ),
    ];
    let selected = selection(&[
        "claude-skill",
        "codex-skill",
        "opencode-skill",
        "claude-plugin",
        "codex-plugin",
        "opencode-plugin",
    ]);
    let execution = world.execution(&targets, &selected);

    let deployed = execute_deploy_targets(&execution).expect("all six destinations deploy");
    assert_eq!(deployed.len(), 6);
    assert_eq!(
        deployed
            .iter()
            .map(|item| item.target.as_str())
            .collect::<Vec<_>>(),
        selected
            .targets
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    assert!(
        deployed
            .iter()
            .all(|item| item.via == "the shipped builtin default")
    );
    assert_foreign_neighbours(&world, &foreign);
    assert_eq!(
        std::fs::read(world.at(".claude/skills/standalone/SKILL.md")).unwrap(),
        STANDALONE_ENTRY.as_bytes()
    );
    assert_eq!(
        std::fs::read(world.at(".agents/skills/standalone/SKILL.md")).unwrap(),
        STANDALONE_ENTRY.as_bytes()
    );
    assert_eq!(
        std::fs::read(world.at(".config/opencode/skills/standalone/SKILL.md")).unwrap(),
        STANDALONE_ENTRY.as_bytes()
    );
    assert_eq!(
        std::fs::read(world.at(".config/opencode/skills/demo/SKILL.md")).unwrap(),
        b"---\nname: demo\ndescription: A packaged skill.\n---\n\nBody.\n"
    );
    assert_eq!(
        std::fs::read(world.at(".config/opencode/skills/demo/reference.md")).unwrap(),
        b"Reference.\n"
    );
    let merged = read_json(&world.at(".config/opencode/opencode.json"));
    assert!(merged["mcp"].get("alpha").is_some());
    assert!(merged["mcp"].get("zeta").is_some());

    let listed = list_deployments(&world.state_home).expect("six receipt facts list");
    assert_eq!(listed.len(), 6);
    assert!(listed.iter().all(|row| row.status.as_str() == "verified"));
    assert_eq!(listed.iter().map(|row| row.resources).sum::<usize>(), 9);

    let claude_support = marketplace_support(&world, "claude", "claude-plugin");
    let codex_support = marketplace_support(&world, "codex", "codex-plugin");
    let claude_digest = tree_digest(&claude_support);
    let codex_digest = tree_digest(&codex_support);

    let removed = undeploy_targets(&execution).expect("all six destinations undeploy");
    assert_eq!(removed.len(), 6);
    assert_foreign_neighbours(&world, &foreign);
    for relative in [
        ".claude/skills/standalone/SKILL.md",
        ".agents/skills/standalone/SKILL.md",
        ".config/opencode/skills/standalone/SKILL.md",
        ".config/opencode/skills/demo/SKILL.md",
        ".config/opencode/skills/demo/reference.md",
    ] {
        assert!(!world.at(relative).exists(), "owned `{relative}` survived");
    }
    let inverse = read_json(&world.at(".config/opencode/opencode.json"));
    assert!(inverse["mcp"].get("alpha").is_none());
    assert!(inverse["mcp"].get("zeta").is_none());
    assert_eq!(tree_digest(&claude_support), claude_digest);
    assert_eq!(tree_digest(&codex_support), codex_digest);
    let reversed = list_deployments(&world.state_home).expect("inverse receipts list");
    assert_eq!(reversed.len(), 6);
    assert!(reversed.iter().all(|row| row.resources == 0));
    assert_trace(&world, &claude_support, &codex_support);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn packaged_skill_crash_after_write_recovers_under_the_interrupted_generation() {
    let world = World::new();
    let skill = package_skill(&world, "recovery", "recovery", RECOVERY_ENTRY);
    let row = skill_target(
        SkillClient::OpenCode,
        "recovery-skill",
        &skill.id,
        "recovery",
    );
    let selected = selection(&["recovery-skill"]);
    let execution = world.execution(std::slice::from_ref(&row), &selected);
    write(
        world.home.path(),
        ".config/opencode/skills/foreign/SKILL.md",
        "foreign recovery neighbour\n",
    );
    let foreign = std::fs::read(world.at(".config/opencode/skills/foreign/SKILL.md")).unwrap();

    let interrupted = Selected {
        target: &row,
        provider: Box::new(FailingAfterWrite(SkillClient::OpenCode)),
        pin: SkillClient::OpenCode.pin().to_owned(),
        via: SelectionStep::BuiltinDefault,
        displaced: None,
    };
    let error = apply_selection(&execution, &[interrupted])
        .expect_err("the injected crash fires after publication");
    assert!(error.to_string().contains("sentinel-after-write"));

    let entry = world.at(".config/opencode/skills/recovery/SKILL.md");
    let stranded = std::fs::read(&entry).expect("the interrupted bytes are durable");
    assert_eq!(stranded, RECOVERY_ENTRY.as_bytes());
    assert!(list_deployments(&world.state_home).unwrap().is_empty());
    let (state, home) = deployment_state(&world, &row);
    let intent = state
        .read_intent(&home)
        .unwrap()
        .expect("the interrupted generation has durable intent");
    assert_eq!(intent.target.generation, 0);
    assert_eq!(intent.resources.len(), 1);
    assert!(state.read_receipt(&home).unwrap().is_none());

    let recovered = execute_deploy_targets(&execution).expect("the normal executor recovers");
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].settlement, "recovered");
    assert_eq!(recovered[0].generation, 0);
    assert_eq!(std::fs::read(&entry).unwrap(), stranded);
    assert_eq!(
        std::fs::read(world.at(".config/opencode/skills/foreign/SKILL.md")).unwrap(),
        foreign
    );
    let receipt = state
        .read_receipt(&home)
        .unwrap()
        .expect("the interrupted generation finalises");
    assert_eq!(receipt.generation, 0);
    assert_eq!(receipt.status, ReceiptStatus::Verified);
    assert!(
        receipt
            .evidence
            .as_deref()
            .is_some_and(|evidence| evidence.contains("was already desired and stayed"))
    );
    assert!(state.read_intent(&home).unwrap().is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn opencode_skill_and_plugin_collision_refuses_in_preplan_without_mutation() {
    let world = World::new();
    let skill = package_skill(&world, "collision", "demo", COLLISION_ENTRY);
    let projected = package_all_projections(&world);
    let foreign = plant_foreign_neighbours(&world);
    let targets = vec![
        skill_target(SkillClient::OpenCode, "collision-skill", &skill.id, "demo"),
        plugin_target(
            PluginClient::OpenCode,
            "collision-plugin",
            &projected.opencode,
            "demo-plugin",
        ),
    ];
    let selected = selection(&["collision-skill", "collision-plugin"]);
    let home_before = census(world.home.path());
    let settings_before = census(world.settings.path());
    assert!(!world.state_home.exists());

    let error = execute_deploy_targets(&world.execution(&targets, &selected))
        .expect_err("duplicate physical ownership refuses in preplan");
    match error {
        DeployError::DuplicateOwnedResource {
            first,
            second,
            resource,
            alias,
        } => {
            assert_eq!(first, "collision-skill");
            assert_eq!(second, "collision-plugin");
            assert_eq!(resource, "home:.config/opencode/skills/demo/SKILL.md");
            assert_eq!(alias, resource);
        }
        other => panic!("expected duplicate ownership, got {other}"),
    }
    assert_eq!(census(world.home.path()), home_before);
    assert_eq!(census(world.settings.path()), settings_before);
    assert!(!world.state_home.exists());
    assert!(!world.settings.path().join("client-marketplaces").exists());
    assert_foreign_neighbours(&world, &foreign);
}

fn package_skill(world: &World, id: &str, source: &str, document: &str) -> PackagedArtifact {
    let source_root = format!("skills/{source}");
    package::write(
        world.project.path(),
        &format!("{source_root}/SKILL.md"),
        document,
    );
    let target = package::skill_target(id, &source_root, &[]);
    let outcomes = package::run_default(world.project.path(), &[target])
        .expect("the real static-skill provider packages");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].mechanism, "package:static-skill");
    assert_eq!(outcomes[0].produced.len(), 1);
    let produced = outcomes[0].produced[0].clone();
    assert_eq!(produced.id, format!("{id}.md"));
    let record = projection::record(world.project.path(), &produced);
    assert_eq!(record.kind, RecordKind::Skill);
    assert_eq!(record.shape, ArtifactShape::File);
    produced
}

fn package_all_projections(world: &World) -> ProjectionArtifacts {
    projection::write_full_plugin(world.project.path());
    let outcomes = projection::all_three(world.project.path());
    assert_eq!(outcomes.len(), 4);
    let canonical = projection::outcome(&outcomes, projection::CANONICAL);
    let canonical_record = projection::record(world.project.path(), &canonical.produced[0]);
    assert_eq!(canonical.produced[0].id, projection::CANONICAL_ARTIFACT);
    assert_eq!(canonical_record.kind, RecordKind::AgentPlugin);
    assert_eq!(canonical_record.shape, ArtifactShape::Directory);

    let mut artifacts = BTreeMap::new();
    for (target, mechanism, client) in projection::CLIENTS {
        let outcome = projection::outcome(&outcomes, target);
        assert_eq!(outcome.mechanism, mechanism);
        assert_eq!(outcome.produced.len(), 1);
        let produced = &outcome.produced[0];
        assert_eq!(produced.id, format!("{target}.dir"));
        let record = projection::record(world.project.path(), produced);
        assert_eq!(record.kind, RecordKind::Directory);
        assert_eq!(record.shape, ArtifactShape::Directory);
        artifacts.insert(client, produced.id.clone());
    }
    ProjectionArtifacts {
        claude: artifacts.remove("claude").unwrap(),
        codex: artifacts.remove("codex").unwrap(),
        opencode: artifacts.remove("opencode").unwrap(),
    }
}

fn plant_foreign_neighbours(world: &World) -> ForeignSnapshot {
    for (relative, bytes) in [
        (
            ".claude/skills/foreign/SKILL.md",
            "foreign Claude skill\r\n",
        ),
        (".agents/skills/foreign/SKILL.md", "foreign Codex skill\n"),
        (
            ".config/opencode/skills/foreign/SKILL.md",
            "foreign OpenCode skill\n",
        ),
        (".codex/private/client.keep", "client-private bytes\r\n"),
    ] {
        write(world.home.path(), relative, bytes);
    }
    write(
        world.home.path(),
        ".config/opencode/opencode.json",
        concat!(
            "{\"theme\":{\"name\":\"foreign-dark\",\"palette\":[1,true,null]},",
            "\"mcp\":{\"foreign\":{\"type\":\"remote\",",
            "\"url\":\"https://foreign.invalid/mcp\",",
            "\"enabled\":false,\"headers\":{\"X-Foreign\":\"exact\"}}}}\n",
        ),
    );
    let config = read_json(&world.at(".config/opencode/opencode.json"));
    ForeignSnapshot {
        files: [
            ".claude/skills/foreign/SKILL.md",
            ".agents/skills/foreign/SKILL.md",
            ".config/opencode/skills/foreign/SKILL.md",
            ".codex/private/client.keep",
        ]
        .into_iter()
        .map(|relative| {
            let path = world.at(relative);
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect(),
        root_value: config["theme"].clone(),
        mcp_value: config["mcp"]["foreign"].clone(),
    }
}

fn assert_foreign_neighbours(world: &World, snapshot: &ForeignSnapshot) {
    for (path, bytes) in &snapshot.files {
        assert_eq!(std::fs::read(path).unwrap(), *bytes, "{}", path.display());
    }
    let config = read_json(&world.at(".config/opencode/opencode.json"));
    assert_eq!(config["theme"], snapshot.root_value);
    assert_eq!(config["mcp"]["foreign"], snapshot.mcp_value);
}

fn assert_fake_executables(world: &World) {
    for executable in [
        &world.clients.claude,
        &world.clients.codex,
        &world.clients.opencode,
    ] {
        match executable {
            super::ClientExecutable::Resolved { path, .. } => {
                assert!(path.is_absolute());
                assert!(path.starts_with(world.fake.path()), "{}", path.display());
                assert!(path.is_file());
            }
            missing => panic!("compiled fake client was not injected: {missing:?}"),
        }
    }
}

fn marketplace_support(world: &World, client: &str, target: &str) -> PathBuf {
    let root = world
        .settings
        .path()
        .join("client-marketplaces")
        .join(client)
        .join(target);
    let entries: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1);
    entries[0].clone()
}

fn assert_trace(world: &World, claude_support: &Path, codex_support: &Path) {
    let rows: Vec<(String, Vec<String>, String)> = world
        .trace()
        .into_iter()
        .map(|line| {
            let mut fields = line.splitn(3, '|');
            let client = fields.next().unwrap().to_owned();
            let argv = fields
                .next()
                .unwrap()
                .split('\t')
                .filter(|arg| !arg.is_empty())
                .map(str::to_owned)
                .collect();
            let environment = fields.next().unwrap().to_owned();
            (client, argv, environment)
        })
        .collect();
    assert!(rows.iter().all(|(_, _, environment)| {
        !environment.contains("PATH")
            && !environment.contains("TOKEN")
            && !environment.contains("PROXY")
    }));

    let manifest_name = |support: &Path, relative: &str| {
        read_json(&support.join(relative))["name"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let claude_coordinate = format!(
        "demo-plugin@{}",
        manifest_name(claude_support, ".claude-plugin/marketplace.json")
    );
    let codex_coordinate = format!(
        "demo-plugin@{}",
        manifest_name(codex_support, "marketplace.json")
    );
    let actual = |client: &str| {
        rows.iter()
            .filter(|(seen, _, _)| seen == client)
            .map(|(_, argv, _)| argv.clone())
            .collect::<Vec<_>>()
    };
    let words = |items: &[&str]| {
        items
            .iter()
            .map(|item| (*item).to_owned())
            .collect::<Vec<_>>()
    };
    let claude_root = claude_support.to_string_lossy().into_owned();
    let codex_root = codex_support.to_string_lossy().into_owned();
    assert_eq!(
        actual("claude"),
        vec![
            words(&["--version"]),
            words(&["plugin", "list", "--json"]),
            words(&["--version"]),
            words(&["plugin", "list", "--json"]),
            words(&[
                "plugin",
                "marketplace",
                "add",
                "--scope",
                "user",
                &claude_root,
            ]),
            words(&["plugin", "install", "--scope", "user", &claude_coordinate]),
            words(&["plugin", "list", "--json"]),
            words(&["plugin", "list", "--json"]),
            words(&["plugin", "list", "--json"]),
            words(&["--version"]),
            words(&["plugin", "list", "--json"]),
            words(&["plugin", "uninstall", "--scope", "user", &claude_coordinate]),
            words(&["plugin", "list", "--json"]),
        ]
    );
    assert_eq!(
        actual("codex"),
        vec![
            words(&["--version"]),
            words(&["plugin", "list", "--json"]),
            words(&["--version"]),
            words(&["plugin", "list", "--json"]),
            words(&["plugin", "marketplace", "add", "--json", &codex_root]),
            words(&["plugin", "add", "--json", &codex_coordinate]),
            words(&["plugin", "list", "--json"]),
            words(&["plugin", "list", "--json"]),
            words(&["plugin", "list", "--json"]),
            words(&["--version"]),
            words(&["plugin", "list", "--json"]),
            words(&["plugin", "remove", "--json", &codex_coordinate]),
            words(&["plugin", "list", "--json"]),
        ]
    );
    assert_eq!(
        actual("opencode"),
        vec![words(&["--version"]), words(&["--version"])]
    );
}

fn deployment_state(world: &World, row: &DeployTarget) -> (DeployState, DeploymentHome) {
    let state = DeployState::open(&world.state_home).expect("the deployment state opens");
    let home = DeploymentHome::new(&world.state_home, "org.example/plugin-test", None, &row.id);
    (state, home)
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn tree_digest(path: &Path) -> String {
    crate::mechanism::contain::tree_digest(path)
        .expect("the support tree digests")
        .digest
}

fn census(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    fn descend(root: &Path, at: &Path, entries: &mut Vec<(String, Option<Vec<u8>>)>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if path.is_dir() {
                entries.push((relative, None));
                descend(root, &path, entries);
            } else {
                entries.push((relative, Some(std::fs::read(path).unwrap())));
            }
        }
    }
    let mut entries = Vec::new();
    descend(root, root, &mut entries);
    entries.sort();
    entries
}
