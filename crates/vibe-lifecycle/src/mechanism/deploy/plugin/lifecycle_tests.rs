use super::fixture::{World, selection, target, write};
use super::*;
use crate::mechanism::deploy::state::{CheckpointLedger, DeployState, DeploymentHome};
use crate::mechanism::deploy::{Selected, apply_selection};
use crate::mechanism::deploy::{execute_deploy_targets, plan_deploy_targets, undeploy_targets};
use crate::process::{ProcessError, ProcessOutput, ProcessRunner, ProcessSpec};
use specmark::verifies;
use vibe_extension_registry::SelectionStep;

const CLAUDE_FILES: &[(&str, &str)] = &[
    (
        ".claude-plugin/plugin.json",
        "{\"name\":\"demo-plugin\",\"version\":\"1.4.2\"}\n",
    ),
    (
        "skills/demo/SKILL.md",
        "---\nname: demo\ndescription: Demo.\n---\n\nBody.\n",
    ),
    (
        ".mcp.json",
        "{\"mcpServers\":{\"demo\":{\"command\":\"${PLUGIN_ROOT}/demo\"}}}\n",
    ),
];

const CODEX_FILES: &[(&str, &str)] = &[
    (
        ".codex-plugin/plugin.json",
        "{\"name\":\"demo-plugin\",\"version\":\"1.4.2\"}\n",
    ),
    (
        "skills/demo/SKILL.md",
        "---\nname: demo\ndescription: Demo.\n---\n\nBody.\n",
    ),
];

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn claude_compiled_fake_drives_plan_apply_idempotence_and_inverse() {
    let world = World::new();
    let digest = world.record_projection("claude-projection", CLAUDE_FILES);
    let targets = [target(
        PluginClient::Claude,
        "claude-target",
        "claude-projection",
        "demo-plugin",
    )];
    let selected = selection("claude-target");
    let execution = world.execution(&targets, &selected);

    let first = execute_deploy_targets(&execution).expect("Claude plugin deploys through fake");
    assert_eq!(first[0].resources.len(), 1);
    let support = world
        .settings
        .path()
        .join("client-marketplaces/claude/claude-target")
        .join(&digest);
    assert!(support.join(".claude-plugin/marketplace.json").is_file());
    assert!(
        support
            .join("plugins/demo-plugin/.claude-plugin/plugin.json")
            .is_file()
    );
    let support_digest = crate::mechanism::contain::tree_digest(&support)
        .unwrap()
        .digest;

    execute_deploy_targets(&execution).expect("repeat is an idempotent local reconcile");
    assert_eq!(
        crate::mechanism::contain::tree_digest(&support)
            .unwrap()
            .digest,
        support_digest
    );
    let removed = undeploy_targets(&execution).expect("receipt-owned coordinate removes");
    assert_eq!(removed[0].removed.len(), 1);
    assert!(
        support.is_dir(),
        "immutable marketplace support survives inverse"
    );

    let trace = world.trace();
    assert!(
        trace
            .iter()
            .any(|line| line.contains("plugin\tmarketplace\tadd\t--scope\tuser"))
    );
    assert!(
        trace
            .iter()
            .any(|line| line.contains("plugin\tinstall\t--scope\tuser"))
    );
    assert!(
        trace
            .iter()
            .any(|line| line.contains("plugin\tuninstall\t--scope\tuser"))
    );
    assert!(
        trace.iter().all(|line| !line.contains("PATH")
            && !line.contains("TOKEN")
            && !line.contains("PROXY"))
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn codex_compiled_fake_observes_the_exact_json_argv_contract() {
    let world = World::new();
    world.record_projection("codex-projection", CODEX_FILES);
    let targets = [target(
        PluginClient::Codex,
        "codex-target",
        "codex-projection",
        "demo-plugin",
    )];
    let selected = selection("codex-target");
    let execution = world.execution(&targets, &selected);
    execute_deploy_targets(&execution).expect("Codex plugin deploys through fake");
    undeploy_targets(&execution).expect("Codex coordinate removes");
    let trace = world.trace();
    for exact in [
        "plugin\tmarketplace\tadd\t--json",
        "plugin\tadd\t--json",
        "plugin\tlist\t--json",
        "plugin\tremove\t--json",
    ] {
        assert!(
            trace.iter().any(|line| line.contains(exact)),
            "missing `{exact}` in {trace:?}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn read_only_plan_runs_only_version_and_list_and_creates_no_support_or_state() {
    let world = World::new();
    world.record_projection("claude-plan", CLAUDE_FILES);
    let targets = [target(
        PluginClient::Claude,
        "plan-target",
        "claude-plan",
        "demo-plugin",
    )];
    let selected = selection("plan-target");
    let execution = world.execution(&targets, &selected);
    let home_before = world.home_census();
    let plans = plan_deploy_targets(&execution).expect("plan is readable");
    assert_eq!(plans.len(), 1);
    assert_eq!(
        world.home_census(),
        home_before,
        "plan changed the exact recursive injected-home census/hashes"
    );
    assert!(!world.state_home.exists());
    assert!(!world.settings.path().join("client-marketplaces").exists());
    let trace = world.trace();
    let argv: Vec<&str> = trace
        .iter()
        .map(|line| line.split('|').nth(1).expect("trace carries argv"))
        .collect();
    assert_eq!(
        argv,
        ["--version", "plugin\tlist\t--json"],
        "external plan trace must be non-vacuous and exact: {trace:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn opencode_merges_and_removes_only_projected_members() {
    let world = World::new();
    let files = [
        (
            "skills/demo/SKILL.md",
            "---\nname: demo\ndescription: Demo.\n---\n\nBody.\n",
        ),
        ("skills/demo/reference.md", "Reference.\n"),
        (
            "opencode.json",
            "{\"mcp\":{\"demo\":{\"command\":[\"demo\",\"--stdio\"],\"enabled\":true,\"type\":\"local\"}}}\n",
        ),
    ];
    world.record_projection("opencode-projection", &files);
    let config = world.at(".config/opencode/opencode.json");
    write(
        world.home.path(),
        ".config/opencode/opencode.json",
        "{\"theme\":\"dark\",\"mcp\":{\"foreign\":{\"type\":\"remote\",\"url\":\"https://example.test\",\"enabled\":true}}}\n",
    );
    let targets = [target(
        PluginClient::OpenCode,
        "opencode-target",
        "opencode-projection",
        "portable-plugin",
    )];
    let selected = selection("opencode-target");
    let execution = world.execution(&targets, &selected);
    execute_deploy_targets(&execution).expect("OpenCode projection deploys");
    assert_eq!(
        std::fs::read_to_string(world.at(".config/opencode/skills/demo/reference.md")).unwrap(),
        "Reference.\n"
    );
    let merged: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    assert_eq!(merged["theme"], "dark");
    assert_eq!(merged["mcp"]["foreign"]["url"], "https://example.test");
    assert_eq!(merged["mcp"]["demo"]["type"], "local");
    let bytes = std::fs::read(&config).unwrap();
    execute_deploy_targets(&execution).expect("equal OpenCode document is a no-op");
    assert_eq!(std::fs::read(&config).unwrap(), bytes);

    undeploy_targets(&execution).expect("OpenCode inverse removes owned members");
    assert!(!world.at(".config/opencode/skills/demo/SKILL.md").exists());
    let remaining: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    assert_eq!(remaining["theme"], "dark");
    assert!(remaining["mcp"].get("demo").is_none());
    assert_eq!(remaining["mcp"]["foreign"]["url"], "https://example.test");
    assert!(
        world
            .trace()
            .iter()
            .all(|line| line.contains("|--version|"))
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn changed_artifact_requires_undeploy_then_deploy_before_new_support() {
    let world = World::new();
    world.record_projection("claude-update", CLAUDE_FILES);
    let targets = [target(
        PluginClient::Claude,
        "update-target",
        "claude-update",
        "demo-plugin",
    )];
    let selected = selection("update-target");
    let execution = world.execution(&targets, &selected);
    execute_deploy_targets(&execution).expect("first generation deploys");
    let mut changed = CLAUDE_FILES.to_vec();
    changed[0] = (
        ".claude-plugin/plugin.json",
        "{\"name\":\"demo-plugin\",\"version\":\"1.4.3\"}\n",
    );
    world.record_projection("claude-update", &changed);
    let error = execute_deploy_targets(&execution).unwrap_err().to_string();
    assert!(error.contains("undeploy, then deploy"), "{error}");
    let roots = world
        .settings
        .path()
        .join("client-marketplaces/claude/update-target");
    assert_eq!(std::fs::read_dir(roots).unwrap().count(), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn corrupted_present_marketplace_is_refused_and_never_repaired() {
    let world = World::new();
    let digest = world.record_projection("claude-corrupt", CLAUDE_FILES);
    let targets = [target(
        PluginClient::Claude,
        "corrupt-target",
        "claude-corrupt",
        "demo-plugin",
    )];
    let selected = selection("corrupt-target");
    let execution = world.execution(&targets, &selected);
    execute_deploy_targets(&execution).expect("first deploy creates support");
    let manifest = world
        .settings
        .path()
        .join("client-marketplaces/claude/corrupt-target")
        .join(digest)
        .join(".claude-plugin/marketplace.json");
    std::fs::write(&manifest, b"foreign damage\n").unwrap();
    let before = std::fs::read(&manifest).unwrap();
    let error = execute_deploy_targets(&execution).unwrap_err().to_string();
    assert!(error.contains("immutable marketplace support"), "{error}");
    assert_eq!(std::fs::read(&manifest).unwrap(), before);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn installed_coordinate_without_receipt_is_an_unowned_occupant() {
    let world = World::new();
    world.record_projection("claude-unowned", CLAUDE_FILES);
    let targets = [target(
        PluginClient::Claude,
        "unowned-target",
        "claude-unowned",
        "demo-plugin",
    )];
    let selected = selection("unowned-target");
    let execution = world.execution(&targets, &selected);
    execute_deploy_targets(&execution).expect("fixture coordinate installs");
    let deployment = std::fs::read_dir(&world.state_home)
        .unwrap()
        .flatten()
        .find(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .expect("deployment directory");
    std::fs::remove_file(deployment.path().join("receipt.json")).unwrap();
    let error = plan_deploy_targets(&execution).unwrap_err().to_string();
    assert!(error.contains("no owning prior receipt"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn apply_rechecks_opencode_occupancy_after_a_clean_plan() {
    let world = World::new();
    let files = [(
        "skills/demo/SKILL.md",
        "---\nname: demo\ndescription: Demo.\n---\n\nDesired.\n",
    )];
    world.record_projection("opencode-race", &files);
    let targets = [target(
        PluginClient::OpenCode,
        "race-target",
        "opencode-race",
        "portable-plugin",
    )];
    let selected = selection("race-target");
    let artifact =
        crate::mechanism::deploy::artifact::resolve_artifact(world.project.path(), &targets[0])
            .unwrap();
    let provider = ClientPluginProvider::new(PluginClient::OpenCode);
    let planning = DeployTargetRequest {
        target: &targets[0],
        profile: &selected.profile,
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: None,
        recovery_intent: None,
        artifact: Some(&artifact),
        staging: None,
    };
    let plan = provider.plan(&planning).expect("empty destination plans");
    write(
        world.home.path(),
        ".config/opencode/skills/demo/SKILL.md",
        "foreign occupant\n",
    );
    let state = DeployState::open(&world.state_home).unwrap();
    let home = DeploymentHome::new(
        &world.state_home,
        "org.example/plugin-test",
        None,
        "race-target",
    );
    let mut ledger = CheckpointLedger::open(&state, &home, "race-plan").unwrap();
    let staging = tempfile::tempdir().unwrap();
    let applying = DeployTargetRequest {
        staging: Some(staging.path()),
        ..planning
    };
    let error = provider
        .apply(&applying, &plan, &mut ledger)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unowned occupant"), "{error}");
    assert_eq!(
        std::fs::read_to_string(world.at(".config/opencode/skills/demo/SKILL.md")).unwrap(),
        "foreign occupant\n"
    );
}

struct CrashAfterApply(ClientPluginProvider);

impl DeployProvider for CrashAfterApply {
    fn descriptor(&self) -> DeployDescriptor {
        self.0.descriptor()
    }
    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        self.0.plan(request)
    }
    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        self.0.fingerprint(request, plan)
    }
    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.0.apply(request, plan, checkpoint)?;
        Err(DeployProviderError::ClientCommand {
            target: request.target.id.clone(),
            client: "claude",
            operation: "test crash",
            reason: "injected after exact install and checkpoint".to_owned(),
        }
        .into())
    }
    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        self.0.verify(request, resources)
    }
    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        self.0.remove(request, resources, handle)
    }
    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.0.recover(request, plan, observed, checkpoint)
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn interrupted_cli_apply_recovers_without_a_second_install() {
    let world = World::new();
    world.record_projection("claude-recovery", CLAUDE_FILES);
    let targets = [target(
        PluginClient::Claude,
        "recovery-target",
        "claude-recovery",
        "demo-plugin",
    )];
    let selection = selection("recovery-target");
    let execution = world.execution(&targets, &selection);
    let failed = [Selected {
        target: &targets[0],
        provider: Box::new(CrashAfterApply(ClientPluginProvider::new(
            PluginClient::Claude,
        ))),
        pin: PluginClient::Claude.pin().to_owned(),
        via: SelectionStep::BuiltinDefault,
        displaced: None,
    }];
    assert!(apply_selection(&execution, &failed).is_err());
    let intent = std::fs::read_dir(&world.state_home)
        .unwrap()
        .flatten()
        .find(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .unwrap()
        .path()
        .join("intent.json");
    assert!(intent.is_file(), "crash leaves durable intent");

    let resumed = [Selected {
        target: &targets[0],
        provider: Box::new(ClientPluginProvider::new(PluginClient::Claude)),
        pin: PluginClient::Claude.pin().to_owned(),
        via: SelectionStep::BuiltinDefault,
        displaced: None,
    }];
    let outcome =
        apply_selection(&execution, &resumed).expect("ordinary rerun settles by recovery");
    assert_eq!(outcome[0].settlement, "recovered");
    let installs = world
        .trace()
        .iter()
        .filter(|line| line.contains("plugin\tinstall\t--scope\tuser"))
        .count();
    assert_eq!(installs, 1, "recovery must not reinstall an exact witness");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_wrong_client_projection_refuses_before_state_or_support() {
    let world = World::new();
    world.record_projection("wrong-client", CODEX_FILES);
    let targets = [target(
        PluginClient::Claude,
        "wrong-client-target",
        "wrong-client",
        "demo-plugin",
    )];
    let selected = selection("wrong-client-target");
    let error = execute_deploy_targets(&world.execution(&targets, &selected))
        .unwrap_err()
        .to_string();
    assert!(error.contains(".claude-plugin/plugin.json"), "{error}");
    assert!(!world.state_home.exists());
    assert!(!world.settings.path().join("client-marketplaces").exists());
}

struct WrongMinor;

impl ProcessRunner for WrongMinor {
    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        assert_eq!(spec.args, ["--version"]);
        Ok(ProcessOutput {
            code: Some(0),
            stdout: b"Claude Code 2.2.0\n".to_vec(),
            ..ProcessOutput::default()
        })
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn wrong_minor_version_refuses_through_the_injected_process_seam() {
    let world = World::new();
    let targets = [target(
        PluginClient::Claude,
        "version-target",
        "unused",
        "demo-plugin",
    )];
    let selected = selection("version-target");
    let request = DeployTargetRequest {
        target: &targets[0],
        profile: &selected.profile,
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: None,
        recovery_intent: None,
        artifact: None,
        staging: None,
    };
    let error = wire::probe_version(&WrongMinor, PluginClient::Claude, &request).unwrap_err();
    assert!(matches!(
        error,
        DeployProviderError::ClientVersion {
            supported: "2.1.x",
            ..
        }
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn opencode_remove_rejects_a_foreign_receipt_listed_resource() {
    let world = World::new();
    let targets = [target(
        PluginClient::OpenCode,
        "remove-target",
        "unused",
        "portable-plugin",
    )];
    write(world.home.path(), "foreign/sentinel", "keep\n");
    let receipt = crate::mechanism::deploy::skill::support::receipt_owning(
        0,
        &[("home:foreign/sentinel", &"0".repeat(64))],
    );
    let request = DeployTargetRequest {
        target: &targets[0],
        profile: "local",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: Some(&receipt),
        recovery_intent: None,
        artifact: None,
        staging: None,
    };
    let resource = "home:foreign/sentinel".to_owned();
    assert!(
        ClientPluginProvider::new(PluginClient::OpenCode)
            .remove(&request, std::slice::from_ref(&resource), None)
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(world.at("foreign/sentinel")).unwrap(),
        "keep\n"
    );
}
