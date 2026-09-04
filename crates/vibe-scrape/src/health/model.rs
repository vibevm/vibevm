//! Typed, serializable health plan and evidence values.

use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreparedHealth {
    pub plan_id: String,
    pub baseline: BaselinePolicy,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
    pub max_result_bytes: u64,
    pub termination_grace_seconds: u64,
    pub checks: Vec<PreparedHealthcheck>,
    pub blockers: Vec<HealthBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthBlocker {
    pub code: String,
    pub check_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BaselinePolicy {
    Strict,
    NoRegression,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreparedHealthcheck {
    pub id: String,
    pub kind: HealthcheckKind,
    pub root: String,
    pub applicability: Applicability,
    pub tests: Option<TestDisposition>,
    pub network: NetworkMode,
    pub assets: Vec<AssetIdentity>,
    pub commands: Vec<PreparedCommand>,
    pub effects: EffectPlan,
    pub sandbox: SandboxRequirement,
    pub protocol: ResultProtocol,
    pub custom_bundle: Option<CustomBundle>,
    pub assurance_reductions: Vec<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthcheckKind {
    Cargo,
    Npm,
    Maven,
    PythonPip,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Applicability {
    Applicable,
    SkippedWhenMissing { path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestDisposition {
    SkippedByContract,
    SkippedNotPresent,
    RunIfPresent,
    RunRequired,
}

impl TestDisposition {
    #[must_use]
    pub const fn runs(self) -> bool {
        matches!(self, Self::RunIfPresent | Self::RunRequired)
    }

    #[must_use]
    pub const fn reduces_assurance(self) -> bool {
        matches!(self, Self::SkippedByContract | Self::SkippedNotPresent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPresence {
    Present,
    Absent,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestDiscoveryRequest {
    pub check_id: String,
    pub kind: HealthcheckKind,
    pub root: String,
    pub selector: Option<String>,
    pub workspace: bool,
    pub all_targets: bool,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssetIdentity {
    pub id: String,
    pub role: AssetRole,
    pub display_path: String,
    pub sha256: String,
    pub bytes: u64,
    pub mode: Option<u32>,
    pub platform_identity: String,
    pub version: String,
    pub source: AssetSource,
    #[serde(skip)]
    pub live_identity: Option<vibe_safefs::FileIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetRole {
    Cargo,
    Node,
    NpmCli,
    MavenLauncher,
    Python,
    CustomInterpreter,
    CustomNative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AssetSource {
    Resolved,
    Bundle { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveAssetRequest {
    pub id: String,
    pub role: AssetRole,
    /// A command selector or adapter-owned asset selector. It is never a shell
    /// fragment and is resolved to one sealed identity by the resolver.
    pub selector: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomLaunchStyle {
    Interpreter,
    Direct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCustomLaunch {
    pub asset: AssetIdentity,
    pub style: CustomLaunchStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreparedCommand {
    pub step: CommandStep,
    pub executable_asset_id: String,
    pub argv: Vec<PreparedArg>,
    pub environment: BTreeMap<String, EnvironmentValue>,
    pub accepted_exit_codes: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandStep {
    Install,
    Build,
    Test,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum PreparedArg {
    Literal(String),
    Root,
    Scratch,
    Result,
    Phase,
    AssetPath(String),
    BundlePath(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum EnvironmentValue {
    Literal(String),
    ScratchPath(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EffectPlan {
    pub reads: Vec<String>,
    pub writes: Vec<String>,
    pub spawn: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkMode {
    Deny,
    ToolOffline,
    Inherit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SandboxRequirement {
    pub exact_executable_identity: bool,
    pub filesystem_isolation: bool,
    pub read_policy_enforcement: bool,
    pub process_tree_containment: bool,
    pub graceful_termination: bool,
    pub spawn_prevention: bool,
    pub network_deny: bool,
    pub bounded_output: bool,
    pub atomic_result: bool,
    pub bundle_materialization: bool,
}

impl SandboxRequirement {
    #[must_use]
    pub const fn for_check(network: NetworkMode, custom: bool, spawn: bool) -> Self {
        Self {
            exact_executable_identity: true,
            filesystem_isolation: true,
            read_policy_enforcement: custom,
            process_tree_containment: true,
            graceful_termination: true,
            spawn_prevention: custom && !spawn,
            network_deny: matches!(network, NetworkMode::Deny),
            bounded_output: true,
            atomic_result: false,
            bundle_materialization: custom,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BackendCapabilities {
    pub exact_executable_identity: bool,
    pub filesystem_isolation: bool,
    pub read_policy_enforcement: bool,
    pub process_tree_containment: bool,
    pub graceful_termination: bool,
    pub spawn_prevention: bool,
    pub network_deny: bool,
    pub bounded_output: bool,
    pub atomic_result: bool,
    pub bundle_materialization: bool,
    pub same_display_path_view: bool,
}

impl BackendCapabilities {
    #[must_use]
    pub const fn satisfies(self, required: SandboxRequirement, same_path: bool) -> bool {
        (!required.exact_executable_identity || self.exact_executable_identity)
            && (!required.filesystem_isolation || self.filesystem_isolation)
            && (!required.read_policy_enforcement || self.read_policy_enforcement)
            && (!required.process_tree_containment || self.process_tree_containment)
            && (!required.graceful_termination || self.graceful_termination)
            && (!required.spawn_prevention || self.spawn_prevention)
            && (!required.network_deny || self.network_deny)
            && (!required.bounded_output || self.bounded_output)
            && (!required.atomic_result || self.atomic_result)
            && (!required.bundle_materialization || self.bundle_materialization)
            && (!same_path || self.same_display_path_view)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ResultProtocol {
    BuiltIn,
    ExitCode,
    VibeHealthJsonV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CustomBundle {
    pub sha256: String,
    pub source: String,
    pub entries: Vec<BundleEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BundleEntry {
    pub path: String,
    pub kind: BundleEntryKind,
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
    pub mode: Option<u32>,
    #[serde(skip)]
    pub content: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BundleEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthPhase {
    Before,
    After,
}

impl HealthPhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhaseContext {
    pub phase: HealthPhase,
    pub root: String,
    pub protected_root: String,
    pub scratch: String,
    pub result: String,
    pub same_display_path_required: bool,
    pub expected_tree: crate::health::tree::TreeSeal,
    pub cancellation: CancellationToken,
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationDisposition {
    RefuseBefore,
    RollbackAfter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedCommand {
    pub step: CommandStep,
    pub executable: AssetIdentity,
    pub argv: Vec<ExpandedArg>,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpandedArg {
    Value(String),
    /// The backend substitutes the exact path of this already sealed asset.
    AssetPath(String),
    /// The backend substitutes this member's path in the materialized verifier
    /// snapshot, never its former project-tree path.
    BundlePath(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StreamEvidence {
    pub total_bytes: u64,
    pub sha256: String,
    pub truncated: bool,
    pub utf8: Utf8State,
    pub head: Vec<u8>,
    pub tail: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Utf8State {
    Valid,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandExecution {
    pub exit_code: i32,
    pub stdout: StreamEvidence,
    pub stderr: StreamEvidence,
    /// Exact bounded result bytes obtained through the backend's atomic-result
    /// protocol. `None` for commands without a structured result.
    pub result: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct BackendCommandRequest<'a> {
    pub check_id: String,
    pub phase: HealthPhase,
    pub root: String,
    pub protected_root: String,
    pub scratch: String,
    pub result: String,
    pub command: ExpandedCommand,
    pub assets: &'a [AssetIdentity],
    pub effects: EffectPlan,
    pub network: NetworkMode,
    pub custom_bundle: Option<&'a CustomBundle>,
    pub expected_tree: &'a crate::health::tree::TreeSeal,
    pub cancellation: CancellationToken,
    pub timeout_seconds: u64,
    pub termination_grace_seconds: u64,
    pub max_result_bytes: u64,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthVerdict {
    Pass,
    Structured(StructuredVerdict),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredVerdict {
    pub status: HealthStatus,
    pub summary: String,
    pub findings: Vec<Finding>,
    pub metrics: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub message: String,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckState {
    Skipped { reason: String },
    Completed(HealthVerdict),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub id: String,
    pub state: CheckState,
    pub commands: Vec<CommandExecution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseHealthResult {
    pub phase: HealthPhase,
    pub plan_id: String,
    pub checks: Vec<CheckResult>,
    pub assurance_reduced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineDecision {
    AcceptFull,
    AcceptReduced,
    RefuseBefore,
    RollbackAfter,
}

#[derive(Debug, thiserror::Error)]
pub enum HealthError {
    #[error("health preparation failed: {0}")]
    Preparation(String),
    #[error("health protocol failed: {0}")]
    Protocol(String),
    #[error("health execution failed: {0}")]
    Execution(String),
    #[error("health backend is unsupported: {0}")]
    Unsupported(String),
    #[error("health tree proof failed: {0}")]
    Tree(String),
    #[error(
        "healthcheck `{check_id}` was cancelled during {phase:?}; disposition is {disposition:?}"
    )]
    Cancelled {
        phase: HealthPhase,
        check_id: String,
        disposition: CancellationDisposition,
    },
}
