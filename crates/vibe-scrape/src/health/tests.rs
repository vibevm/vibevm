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

#[test]
fn cargo_argv_is_canonical_and_tests_are_explicit() {
    let temp = tempfile::tempdir().unwrap();
    write_cargo_fixture(temp.path());
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "build"
workspace = true
locked = true
all_targets = true
tests = "required"
profile = "release"
features = ["z", "a"]
timeout_seconds = 10"#,
        "strict",
        "tool-offline",
    );
    let mut resolver = FakeResolver::new(TestPresence::Present);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    assert_eq!(health.checks[0].commands.len(), 2);
    assert_eq!(
        rendered(&health.checks[0].commands[0].argv),
        [
            "build",
            "--workspace",
            "--all-targets",
            "--features",
            "a,z",
            "--profile",
            "release",
            "--locked",
            "--offline",
        ]
    );
    assert_eq!(rendered(&health.checks[0].commands[1].argv)[0], "test");
}

#[test]
fn npm_uses_node_and_cli_assets_without_a_command_shell() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"scripts":{"build":"echo build","test":"echo test"}}"#,
    )
    .unwrap();
    std::fs::write(temp.path().join("package-lock.json"), "{}").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "web"
kind = "npm"
root = "."
manager = "npm"
lockfile = "package-lock.json"
install = "ci"
build_script = "build"
tests = "required"
test_script = "test"
timeout_seconds = 10"#,
        "strict",
        "tool-offline",
    );
    let mut resolver = FakeResolver::new(TestPresence::Present);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    let check = &health.checks[0];
    assert_eq!(check.assets.len(), 2);
    assert_eq!(
        rendered(&check.commands[0].argv),
        ["{asset:web/npm-cli}", "ci", "--offline"]
    );
    assert_eq!(
        rendered(&check.commands[1].argv),
        ["{asset:web/npm-cli}", "run", "build"]
    );
    assert_eq!(
        rendered(&check.commands[2].argv),
        ["{asset:web/npm-cli}", "run", "test"]
    );
}

#[test]
fn test_modes_refuse_required_absence_and_type_if_present_skip() {
    let temp = tempfile::tempdir().unwrap();
    write_cargo_fixture(temp.path());
    let (project, inventory) = observed(temp.path());
    let required = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "required"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let blocked = prepare(
        &project,
        &required,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert!(
        blocked
            .blockers
            .iter()
            .any(|blocker| blocker.code == "health-preparation-failed")
    );
    let optional = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "if-present"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let health = prepare(
        &project,
        &optional,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert_eq!(
        health.checks[0].tests,
        Some(TestDisposition::SkippedNotPresent)
    );
    assert_eq!(health.checks[0].commands.len(), 1);
}

#[test]
fn system_cargo_discovery_honors_disabled_auto_and_target_tests() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname='no-tests'\nversion='0.1.0'\nedition='2024'\nautotests=false\nautobins=false\n[lib]\ntest=false\n",
    )
    .unwrap();
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn product() {}\n").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "required"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let mut resolver = SystemHealthResolver::new(&project);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    assert!(health.blockers.iter().any(|blocker| {
        blocker.check_id.as_deref() == Some("rust") && blocker.message.contains("requires tests")
    }));
}

#[test]
fn system_maven_discovery_blocks_profile_dependent_required_tests() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("pom.xml"),
        "<project><profiles><profile><id>tests</id></profile></profiles></project>",
    )
    .unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "java"
kind = "maven"
root = "."
runner = "wrapper-first"
goal = "verify"
offline = true
tests = "required"
timeout_seconds = 10"#,
        "strict",
        "tool-offline",
    );
    let mut resolver = SystemHealthResolver::new(&project);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    assert!(health.blockers.iter().any(|blocker| {
        blocker.check_id.as_deref() == Some("java") && blocker.message.contains("indeterminate")
    }));
}

#[test]
fn projected_cargo_model_reuses_autotests_and_target_rules() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("tests")).unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname='tests'\nversion='0.1.0'\nedition='2024'\nautolib=false\nautobins=false\nautotests=true\n",
    )
    .unwrap();
    std::fs::write(temp.path().join("tests/it.rs"), "#[test] fn it() {}\n").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "required"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let mut resolver = FakeResolver::new(TestPresence::Present);
    let prepared = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    let projected = vec![
        crate::rewrite::ProjectedEntry {
            path: "Cargo.toml".to_owned(),
            kind: crate::model::EntryKind::File,
            bytes: Some(b"[package]\nname='tests'\nversion='0.1.0'\nedition='2024'\nautolib=false\nautobins=false\nautotests=false\n".to_vec()),
            unix_mode: None,
        },
        crate::rewrite::ProjectedEntry {
            path: "tests".to_owned(),
            kind: crate::model::EntryKind::Directory,
            bytes: None,
            unix_mode: None,
        },
        crate::rewrite::ProjectedEntry {
            path: "tests/it.rs".to_owned(),
            kind: crate::model::EntryKind::File,
            bytes: Some(b"#[test] fn it() {}\n".to_vec()),
            unix_mode: None,
        },
    ];
    let blockers = validate_projected_final(&contract, &prepared, &projected);
    assert!(
        blockers
            .iter()
            .any(|blocker| { blocker.code == "health-projected-test-applicability-changed" })
    );
}

#[test]
fn projected_maven_model_reuses_profile_indeterminacy() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("pom.xml"), "<project/>").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "java"
kind = "maven"
root = "."
runner = "explicit"
goal = "verify"
offline = true
tests = "required"
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let mut resolver = FakeResolver::new(TestPresence::Present);
    let prepared = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    let projected = vec![crate::rewrite::ProjectedEntry {
        path: "pom.xml".to_owned(),
        kind: crate::model::EntryKind::File,
        bytes: Some(
            b"<project><profiles><profile><id>tests</id></profile></profiles></project>".to_vec(),
        ),
        unix_mode: None,
    }];
    let blockers = validate_projected_final(&contract, &prepared, &projected);
    assert!(
        blockers
            .iter()
            .any(|blocker| { blocker.code == "health-projected-test-indeterminate" })
    );
}

#[test]
fn python_and_maven_argv_are_structured() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("src")).unwrap();
    std::fs::write(temp.path().join("src/app.py"), "pass\n").unwrap();
    std::fs::write(temp.path().join("pyproject.toml"), "[build-system]").unwrap();
    std::fs::write(temp.path().join("pom.xml"), "<project/>").unwrap();
    let (project, inventory) = observed(temp.path());
    let python = contract(
        r#"[[healthcheck]]
id = "py"
kind = "python-pip"
root = "."
interpreter = "python"
source_roots = ["src"]
dependency_check = true
build = true
tests = "skip"
timeout_seconds = 10"#,
        "strict",
        "tool-offline",
    );
    let health = prepare(
        &project,
        &python,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert_eq!(health.checks[0].commands.len(), 3);
    assert_eq!(
        rendered(&health.checks[0].commands[2].argv),
        [
            "-s",
            "-m",
            "build",
            "--no-isolation",
            "--outdir",
            "{scratch}"
        ]
    );

    let maven = contract(
        r#"[[healthcheck]]
id = "java"
kind = "maven"
root = "."
runner = "explicit"
goal = "verify"
offline = true
tests = "skip"
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let health = prepare(
        &project,
        &maven,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert_eq!(
        rendered(&health.checks[0].commands[0].argv),
        [
            "--batch-mode",
            "--no-transfer-progress",
            "--offline",
            "-DskipTests",
            "verify",
        ]
    );
}

#[test]
fn custom_bundle_is_stable_and_placeholders_are_whole_arguments() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("tools")).unwrap();
    std::fs::write(temp.path().join("tools/health.py"), b"print('ok')\n").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "domain"
kind = "custom"
root = "."
source = "tools/health.py"
snapshot = ["tools", "tools/**"]
interpreter = "python"
argv = ["--phase", "{phase}", "--root", "{root}", "--result", "{result}"]
protocol = "vibe-health-json-v1"
reads = ["**"]
writes = []
spawn = false
network = "deny"
timeout_seconds = 10"#,
        "strict",
        "deny",
    );
    let mut resolver = FakeResolver::new(TestPresence::Absent);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    let check = &health.checks[0];
    let bundle = check.custom_bundle.as_ref().unwrap();
    assert_eq!(bundle.entries.len(), 2);
    assert!(bundle.entries[1].content.is_some());
    assert_eq!(
        rendered(&check.commands[0].argv),
        [
            "{bundle:tools/health.py}",
            "--phase",
            "{phase}",
            "--root",
            "{root}",
            "--result",
            "{result}",
        ]
    );
    assert!(check.sandbox.spawn_prevention);
    assert!(check.sandbox.network_deny);
}

#[test]
fn protocol_rejects_duplicate_keys_ids_and_contradictory_status() {
    let duplicate_key =
        br#"{"protocol":1,"protocol":1,"status":"pass","summary":"ok","findings":[],"metrics":{}}"#;
    assert!(parse_health_result(duplicate_key, 1024).is_err());
    let duplicate_id = br#"{"protocol":1,"status":"warn","summary":"x","findings":[{"id":"x","severity":"warning","message":"a"},{"id":"x","severity":"warning","message":"b"}],"metrics":{}}"#;
    assert!(parse_health_result(duplicate_id, 1024).is_err());
    let contradiction = br#"{"protocol":1,"status":"pass","summary":"x","findings":[{"id":"x","severity":"warning","message":"a"}],"metrics":{}}"#;
    assert!(parse_health_result(contradiction, 1024).is_err());
    let valid = br#"{"protocol":1,"status":"fail","summary":"x","findings":[{"id":"x","severity":"error","message":"a"}],"metrics":{"tests":2}}"#;
    assert_eq!(
        parse_health_result(valid, 1024).unwrap().status,
        HealthStatus::Fail
    );
}

#[test]
fn bounded_stream_keeps_full_digest_and_split_utf8_state() {
    let mut accumulator = StreamAccumulator::new(5);
    accumulator.push(b"ab\xf0\x9f").unwrap();
    accumulator.push(b"\x98\x80cdef").unwrap();
    let evidence = accumulator.finish();
    assert_eq!(evidence.total_bytes, 10);
    assert!(evidence.truncated);
    assert!(evidence.head.is_empty());
    assert!(evidence.tail.is_empty());
    assert_eq!(evidence.utf8, Utf8State::Valid);
    let (out, err) = drain_concurrently(
        Cursor::new(vec![b'x'; 64]),
        Cursor::new(vec![0xff; 64]),
        4,
        4,
    )
    .unwrap();
    assert_eq!(out.total_bytes, 64);
    assert_eq!(err.utf8, Utf8State::Invalid);
}

#[test]
fn stream_evidence_never_persists_secret_excerpts() {
    let secret = b"token=super-secret-value";
    let mut accumulator = StreamAccumulator::new(1024);
    accumulator.push(secret).unwrap();
    let evidence = accumulator.finish();
    assert_eq!(evidence.total_bytes, secret.len() as u64);
    assert_eq!(
        evidence.sha256,
        format!("sha256:{:x}", Sha256::digest(secret))
    );
    assert!(evidence.head.is_empty());
    assert!(evidence.tail.is_empty());
}

fn finding(id: &str, severity: Severity) -> Finding {
    Finding {
        id: id.to_owned(),
        severity,
        message: String::new(),
        evidence: None,
    }
}

fn structured(status: HealthStatus, findings: Vec<Finding>) -> CheckState {
    CheckState::Completed(HealthVerdict::Structured(StructuredVerdict {
        status,
        summary: String::new(),
        findings,
        metrics: BTreeMap::new(),
    }))
}

fn phase(phase: HealthPhase, state: CheckState) -> PhaseHealthResult {
    PhaseHealthResult {
        phase,
        plan_id: "sha256:plan".to_owned(),
        checks: vec![CheckResult {
            id: "domain".to_owned(),
            state,
            commands: Vec::new(),
        }],
        assurance_reduced: false,
    }
}

#[test]
fn no_regression_accepts_subset_and_rejects_new_or_worse_finding() {
    let before = phase(
        HealthPhase::Before,
        structured(
            HealthStatus::Fail,
            vec![
                finding("a", Severity::Error),
                finding("b", Severity::Warning),
            ],
        ),
    );
    let improved = phase(
        HealthPhase::After,
        structured(HealthStatus::Warn, vec![finding("b", Severity::Warning)]),
    );
    assert_eq!(
        judge(BaselinePolicy::NoRegression, &before, &improved),
        BaselineDecision::AcceptReduced
    );
    let worse = phase(
        HealthPhase::After,
        structured(
            HealthStatus::Fail,
            vec![
                finding("b", Severity::Error),
                finding("new", Severity::Info),
            ],
        ),
    );
    assert_eq!(
        judge(BaselinePolicy::NoRegression, &before, &worse),
        BaselineDecision::RollbackAfter
    );
}

#[test]
fn strict_refuses_a_red_before_and_rolls_back_a_red_after() {
    let red_before = phase(
        HealthPhase::Before,
        structured(
            HealthStatus::Warn,
            vec![finding("existing", Severity::Warning)],
        ),
    );
    let green_after = phase(
        HealthPhase::After,
        structured(HealthStatus::Pass, Vec::new()),
    );
    assert_eq!(
        judge(BaselinePolicy::Strict, &red_before, &green_after),
        BaselineDecision::RefuseBefore
    );

    let green_before = phase(
        HealthPhase::Before,
        structured(HealthStatus::Pass, Vec::new()),
    );
    let red_after = phase(
        HealthPhase::After,
        structured(HealthStatus::Fail, vec![finding("new", Severity::Error)]),
    );
    assert_eq!(
        judge(BaselinePolicy::Strict, &green_before, &red_after),
        BaselineDecision::RollbackAfter
    );
}

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

#[test]
fn backend_missing_capability_refuses_before_execution_and_tree_drift_refuses() {
    let temp = tempfile::tempdir().unwrap();
    write_cargo_fixture(temp.path());
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "skip"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "deny",
    );
    let health = prepare(
        &project,
        &contract,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    let seal = tree::TreeSeal::from_inventory(&inventory);
    let context = PhaseContext {
        phase: HealthPhase::Before,
        root: temp.path().display().to_string(),
        protected_root: temp.path().display().to_string(),
        scratch: "C:/scratch".to_owned(),
        result: "C:/result".to_owned(),
        same_display_path_required: false,
        expected_tree: seal.clone(),
        cancellation: CancellationToken::new(),
    };
    let mut unsupported = FakeBackend::default();
    assert!(run_phase(&mut unsupported, &health, &context).is_err());
    assert_eq!(unsupported.calls, 0);

    let execution = CommandExecution {
        exit_code: 0,
        stdout: empty_stream(8),
        stderr: empty_stream(8),
        result: None,
    };
    let mut changed = seal.clone();
    changed.tree_digest = "sha256:changed".to_owned();
    let mut backend = FakeBackend {
        capabilities: full_capabilities(),
        executions: VecDeque::from([execution]),
        observed: Some(changed),
        calls: 0,
    };
    assert!(matches!(
        run_phase(&mut backend, &health, &context),
        Err(HealthError::Tree(_))
    ));
}

#[test]
fn native_platform_stub_advertises_no_unimplemented_capability() {
    let backend = platform::native_backend();
    assert_eq!(backend.capabilities(), BackendCapabilities::default());
}

#[test]
fn tree_seal_reports_extra_missing_and_changed_entries() {
    let expected = tree::TreeSeal {
        tree_digest: "a".to_owned(),
        entries: vec![tree::TreeSealEntry {
            path: "a".to_owned(),
            kind: tree::TreeEntryKind::File,
            sha256: Some("one".to_owned()),
            bytes: Some(1),
            mode: None,
        }],
    };
    let observed = tree::TreeSeal {
        tree_digest: "b".to_owned(),
        entries: vec![tree::TreeSealEntry {
            path: "b".to_owned(),
            kind: tree::TreeEntryKind::File,
            sha256: Some("two".to_owned()),
            bytes: Some(1),
            mode: None,
        }],
    };
    let diff = expected.compare(&observed);
    assert!(
        diff.iter()
            .any(|item| matches!(item, tree::TreeDifference::Missing(path) if path == "a"))
    );
    assert!(
        diff.iter()
            .any(|item| matches!(item, tree::TreeDifference::Extra(path) if path == "b"))
    );
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

fn local_context(phase: &tempfile::TempDir, protected: &tempfile::TempDir) -> PhaseContext {
    let project = Project::open(protected.path()).unwrap();
    let inventory = crate::inventory::collect(&project).unwrap();
    PhaseContext {
        phase: HealthPhase::Before,
        root: phase.path().display().to_string(),
        protected_root: protected.path().display().to_string(),
        scratch: phase.path().join("scratch").display().to_string(),
        result: phase.path().join("results").display().to_string(),
        same_display_path_required: false,
        expected_tree: tree::TreeSeal::from_inventory(&inventory),
        cancellation: CancellationToken::new(),
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

#[test]
fn local_process_primitive_runs_a_sealed_group_in_an_isolated_view() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut backend = LocalProcessBackend::new();
    let mut plan = local_process_plan(&["--help"], 10);
    plan.checks[0].sandbox.graceful_termination = false;
    let result = run_phase(&mut backend, &plan, &local_context(&phase, &protected)).unwrap();
    assert_eq!(result.checks.len(), 1);
    assert_eq!(result.checks[0].commands[0].exit_code, 0);
}

#[test]
fn local_backend_child_sleeps_only_when_invoked_as_the_timeout_fixture() {
    if std::env::var_os("VIBE_HEALTH_TIMEOUT_FIXTURE").is_some() {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

#[test]
#[allow(clippy::zombie_processes)] // Deliberate orphan: the outer command-group must own/reap it.
fn local_backend_pipe_descendant_fixture() {
    match std::env::var("VIBE_HEALTH_PIPE_FIXTURE").as_deref() {
        Ok("leader") => {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "health::tests::local_backend_pipe_descendant_fixture",
                    "--nocapture",
                ])
                .env("VIBE_HEALTH_PIPE_FIXTURE", "descendant")
                .spawn()
                .unwrap();
        }
        Ok("descendant") => std::thread::sleep(std::time::Duration::from_secs(10)),
        _ => {}
    }
}

#[test]
fn local_process_primitive_times_out_and_terminates_its_group() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut plan = local_process_plan(
        &[
            "--exact",
            "health::tests::local_backend_child_sleeps_only_when_invoked_as_the_timeout_fixture",
            "--nocapture",
        ],
        1,
    );
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].commands[0].environment.insert(
        "VIBE_HEALTH_TIMEOUT_FIXTURE".to_owned(),
        EnvironmentValue::Literal("1".to_owned()),
    );
    let error = run_phase(
        &mut LocalProcessBackend::new(),
        &plan,
        &local_context(&phase, &protected),
    )
    .unwrap_err();
    assert!(error.to_string().contains("timed out"));
}

#[test]
fn timeout_covers_descendant_held_pipes_without_hanging() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut plan = local_process_plan(
        &[
            "--exact",
            "health::tests::local_backend_pipe_descendant_fixture",
            "--nocapture",
        ],
        1,
    );
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].commands[0].environment.insert(
        "VIBE_HEALTH_PIPE_FIXTURE".to_owned(),
        EnvironmentValue::Literal("leader".to_owned()),
    );
    let started = std::time::Instant::now();
    let error = run_phase(
        &mut LocalProcessBackend::new(),
        &plan,
        &local_context(&phase, &protected),
    )
    .unwrap_err();
    assert!(error.to_string().contains("timed out"));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn cancellation_terminates_live_descendant_group_and_is_phase_typed() {
    let phase = tempfile::tempdir().unwrap();
    let protected = tempfile::tempdir().unwrap();
    let mut plan = local_process_plan(
        &[
            "--exact",
            "health::tests::local_backend_pipe_descendant_fixture",
            "--nocapture",
        ],
        30,
    );
    plan.checks[0].sandbox.graceful_termination = false;
    plan.checks[0].commands[0].environment.insert(
        "VIBE_HEALTH_PIPE_FIXTURE".to_owned(),
        EnvironmentValue::Literal("leader".to_owned()),
    );
    let mut context = local_context(&phase, &protected);
    let cancellation = context.cancellation.clone();
    let trigger = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        cancellation.cancel();
    });
    let started = std::time::Instant::now();
    let error = run_phase(&mut LocalProcessBackend::new(), &plan, &context).unwrap_err();
    trigger.join().unwrap();
    assert!(matches!(
        error,
        HealthError::Cancelled {
            phase: HealthPhase::Before,
            disposition: CancellationDisposition::RefuseBefore,
            ..
        }
    ));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));

    context.phase = HealthPhase::After;
    context.cancellation = CancellationToken::new();
    context.cancellation.cancel();
    let error = run_phase(&mut LocalProcessBackend::new(), &plan, &context).unwrap_err();
    assert!(matches!(
        error,
        HealthError::Cancelled {
            phase: HealthPhase::After,
            disposition: CancellationDisposition::RollbackAfter,
            ..
        }
    ));
}

#[cfg(windows)]
#[test]
fn windows_sealed_asset_handle_blocks_replacement_and_write() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("tool.exe");
    let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let asset = AssetIdentity {
        id: "tool".to_owned(),
        role: AssetRole::CustomNative,
        display_path: path.display().to_string(),
        sha256: format!("sha256:{:x}", Sha256::digest(&bytes)),
        bytes: bytes.len() as u64,
        mode: None,
        platform_identity: "test".to_owned(),
        version: "test".to_owned(),
        source: AssetSource::Resolved,
        live_identity: Some(opaque_identity(&path)),
    };
    let anchor = tempfile::tempdir().unwrap();
    let identity_project = Project::open(anchor.path()).unwrap();
    let held = super::local::verify_asset(&asset, &identity_project).unwrap();
    assert!(std::fs::rename(&path, temp.path().join("replacement.exe")).is_err());
    assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
    drop(held);
    assert!(std::fs::rename(&path, temp.path().join("replacement.exe")).is_ok());
}
