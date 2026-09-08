specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use std::collections::{BTreeMap, VecDeque};
use std::io::Cursor;

use sha2::{Digest as _, Sha256};
use vibe_safefs::Project;

use super::*;

struct FakeResolver {
    presence: TestPresence,
    custom_style: CustomLaunchStyle,
    requests: Vec<ResolveAssetRequest>,
}

impl FakeResolver {
    fn new(presence: TestPresence) -> Self {
        Self {
            presence,
            custom_style: CustomLaunchStyle::Interpreter,
            requests: Vec::new(),
        }
    }

    fn asset(request: &ResolveAssetRequest) -> AssetIdentity {
        AssetIdentity {
            id: request.id.clone(),
            role: request.role.clone(),
            display_path: format!("C:/sealed/{}", request.id.replace('/', "-")),
            sha256: format!("sha256:{}", "1".repeat(64)),
            bytes: 7,
            mode: None,
            platform_identity: format!("identity/{}", request.id),
            version: "fake 1".to_owned(),
            version_kind: VersionKind::Content,
            source: AssetSource::Resolved,
            live_identity: None,
        }
    }
}

impl HealthResolver for FakeResolver {
    fn resolve_asset(
        &mut self,
        request: ResolveAssetRequest,
    ) -> Result<AssetIdentity, HealthError> {
        let asset = Self::asset(&request);
        self.requests.push(request);
        Ok(asset)
    }

    fn resolve_custom_launch(
        &mut self,
        check_id: &str,
        interpreter: &str,
        _source: &str,
    ) -> Result<ResolvedCustomLaunch, HealthError> {
        let role = if self.custom_style == CustomLaunchStyle::Direct {
            AssetRole::CustomNative
        } else {
            AssetRole::CustomInterpreter
        };
        let request = ResolveAssetRequest {
            id: format!("{check_id}/custom-launch"),
            role,
            selector: interpreter.to_owned(),
        };
        let asset = Self::asset(&request);
        self.requests.push(request);
        Ok(ResolvedCustomLaunch {
            asset,
            style: self.custom_style,
        })
    }

    fn discover_tests(
        &mut self,
        _project: &Project,
        _inventory: &crate::model::Inventory,
        _request: &TestDiscoveryRequest,
    ) -> Result<TestPresence, HealthError> {
        Ok(self.presence)
    }
}

fn contract(row: &str, baseline: &str, network: &str) -> crate::contract::Contract {
    crate::contract::Contract::parse(
        format!(
            r#"schema = 1
id = "org.example.health"
[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"
[scope]
closed_roots = ["vibevm"]
outside = "implicit-keep"
[commit]
contract = "delete-last"
[[classify]]
id = "delete"
kind = "delete"
patterns = ["vibevm", "vibevm/**"]
owner = "vibe"
proof = "contract-assertion-v1"
modified = "delete"
require_match = false
[[assert]]
id = "absent"
kind = "paths-absent-v1"
patterns = ["vibevm", "vibevm/**"]
[health]
baseline = "{baseline}"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "{network}"
max_stdout_bytes = 8
max_stderr_bytes = 8
max_result_bytes = 1024
termination_grace_seconds = 1
{row}
"#
        )
        .as_bytes(),
    )
    .unwrap()
}

fn observed(root: &std::path::Path) -> (Project, crate::model::Inventory) {
    let project = Project::open(root).unwrap();
    let inventory = crate::inventory::collect(&project).unwrap();
    (project, inventory)
}

fn rendered(args: &[PreparedArg]) -> Vec<String> {
    args.iter()
        .map(|arg| match arg {
            PreparedArg::Literal(value) => value.clone(),
            PreparedArg::Root => "{root}".to_owned(),
            PreparedArg::Scratch => "{scratch}".to_owned(),
            PreparedArg::Result => "{result}".to_owned(),
            PreparedArg::Phase => "{phase}".to_owned(),
            PreparedArg::AssetPath(value) => format!("{{asset:{value}}}"),
            PreparedArg::BundlePath(value) => format!("{{bundle:{value}}}"),
        })
        .collect()
}

fn write_cargo_fixture(root: &std::path::Path) {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='health-fixture'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
}

include!("tests/preparation.rs");
include!("tests/protocol_judgment.rs");
include!("tests/process.rs");

#[derive(Default)]
struct FakeBackend {
    capabilities: BackendCapabilities,
    executions: VecDeque<CommandExecution>,
    observed: Option<tree::TreeSeal>,
    calls: usize,
}

impl super::backend::sealed::Sealed for FakeBackend {}

impl HealthBackend for FakeBackend {
    fn capabilities(&self) -> BackendCapabilities {
        self.capabilities
    }

    fn execute(
        &mut self,
        _request: BackendCommandRequest<'_>,
    ) -> Result<CommandExecution, HealthError> {
        self.calls += 1;
        self.executions
            .pop_front()
            .ok_or_else(|| HealthError::Execution("missing fake execution".to_owned()))
    }

    fn reprove_tree(&mut self, _context: &PhaseContext) -> Result<tree::TreeSeal, HealthError> {
        self.observed
            .clone()
            .ok_or_else(|| HealthError::Tree("missing fake tree".to_owned()))
    }
}

fn full_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        exact_executable_identity: true,
        filesystem_isolation: true,
        read_policy_enforcement: true,
        process_tree_containment: true,
        graceful_termination: true,
        forced_tree_termination: true,
        spawn_prevention: true,
        network_deny: true,
        bounded_output: true,
        atomic_result: true,
        bundle_materialization: true,
        same_display_path_view: true,
    }
}

fn empty_stream(cap: usize) -> StreamEvidence {
    StreamAccumulator::new(cap).finish()
}

fn local_process_plan(arguments: &[&str], timeout_seconds: u64) -> PreparedHealth {
    let executable = std::env::current_exe().unwrap();
    let bytes = std::fs::read(&executable).unwrap();
    let live_identity = opaque_identity(&executable);
    let asset = AssetIdentity {
        id: "local/exe".to_owned(),
        role: AssetRole::CustomNative,
        display_path: executable.display().to_string(),
        sha256: format!("sha256:{:x}", Sha256::digest(&bytes)),
        bytes: bytes.len() as u64,
        mode: None,
        platform_identity: "test-current-exe".to_owned(),
        version: "test".to_owned(),
        version_kind: VersionKind::Content,
        source: AssetSource::Resolved,
        live_identity: Some(live_identity),
    };
    PreparedHealth {
        plan_id: "sha256:local".to_owned(),
        baseline: BaselinePolicy::Strict,
        max_stdout_bytes: 4096,
        max_stderr_bytes: 4096,
        max_result_bytes: 1024,
        termination_grace_seconds: 1,
        blockers: Vec::new(),
        checks: vec![PreparedHealthcheck {
            id: "local".to_owned(),
            kind: HealthcheckKind::Cargo,
            root: ".".to_owned(),
            applicability: Applicability::Applicable,
            tests: None,
            network: NetworkMode::Inherit,
            assets: vec![asset],
            commands: vec![PreparedCommand {
                step: CommandStep::Verify,
                executable_asset_id: "local/exe".to_owned(),
                argv: arguments
                    .iter()
                    .map(|arg| PreparedArg::Literal((*arg).to_owned()))
                    .collect(),
                environment: BTreeMap::new(),
                accepted_exit_codes: vec![0],
            }],
            effects: EffectPlan {
                reads: vec!["**".to_owned()],
                writes: vec!["**".to_owned()],
                spawn: true,
            },
            sandbox: SandboxRequirement::for_check(NetworkMode::Inherit, false, true),
            protocol: ResultProtocol::BuiltIn,
            custom_bundle: None,
            assurance_reductions: vec!["network-inherited".to_owned()],
            timeout_seconds,
        }],
    }
}

struct LocalContext {
    context: PhaseContext,
    _scratch: tempfile::TempDir,
}

impl std::ops::Deref for LocalContext {
    type Target = PhaseContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl std::ops::DerefMut for LocalContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

fn local_context(phase: &tempfile::TempDir, protected: &tempfile::TempDir) -> LocalContext {
    let project = Project::open(protected.path()).unwrap();
    let inventory = crate::inventory::collect(&project).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    LocalContext {
        context: PhaseContext {
            phase: HealthPhase::Before,
            root: phase.path().display().to_string(),
            protected_root: protected.path().display().to_string(),
            scratch: scratch.path().join("scratch").display().to_string(),
            result: scratch.path().join("results").display().to_string(),
            same_display_path_required: false,
            transactional_tree_reproof: false,
            expected_tree: tree::TreeSeal::from_inventory(&inventory),
            cancellation: CancellationToken::new(),
        },
        _scratch: scratch,
    }
}

fn opaque_identity(path: &std::path::Path) -> vibe_safefs::FileIdentity {
    let anchor = tempfile::tempdir().unwrap();
    let project = Project::open(anchor.path()).unwrap();
    Project::pin_absolute_file(path)
        .unwrap()
        .read_snapshot_bounded(&project, 64 * 1024 * 1024)
        .unwrap()
        .identity
}
