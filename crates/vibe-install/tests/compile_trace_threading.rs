//! R3.4's vibe-install seam: the observed apply's traced sibling carries ONE
//! borrowed compile-trace run through the whole plan→apply pipeline, and the
//! untraced wrapper leaves no trace state at all.
//!
//! Integration-grain because the crate sets `[lib] test = false` (Windows UAC
//! installer detection, PROP-007 §9.5): the binary name carries no `install`
//! substring.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{
    InstallRequest, InstallSource, NullObserver, Plan, SlotLifecycleSeams,
    apply_with_spec_format_and_lifecycle_observed,
    apply_with_spec_format_and_lifecycle_observed_traced,
};
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_registry::{CachedPackage, RegistryError, ResolvedPackage, compute_content_hash};
use vibe_resolver::{FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};
use vibe_wire::behaviour::compiler_trace_index::validate;
use vibe_wire::generated::compiler_trace_index::e1::index::{
    CompilerTraceIndex, RunStatus, ScopeStatus, Timestamp,
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_workspace::compile_trace::{TraceLimits, TraceRun};

/// One immutable fixture package carrying a boot snippet, served from a temp
/// tree with a real content hash.
struct FixtureSource {
    fixtures: PathBuf,
}

impl InstallSource for FixtureSource {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        _store_root: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let dir = self.fixtures.join(pkgref.name.to_string());
        let manifest = Manifest::read(dir.join(Manifest::FILENAME)).map_err(|e| {
            RegistryError::MalformedMeta {
                path: dir.join(Manifest::FILENAME),
                reason: e.to_string(),
            }
        })?;
        let version = manifest
            .package
            .as_ref()
            .expect("fixture [package]")
            .version
            .clone();
        let content_hash = compute_content_hash(&dir)?;
        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group: Group::parse("org.vibevm").unwrap(),
                name: pkgref.name.to_string(),
                version,
                source_dir: dir.clone(),
            },
            cache_dir: dir.clone(),
            manifest,
            content_hash,
            source_uri: "https://example.test/bootlib.git".to_string(),
            registry_name: Some("test".to_string()),
            source_ref: Some("v1.0.0".to_string()),
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
        })
    }

    fn solve(&self, _roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(ResolvedGraph {
            packages: vec![ResolvedNode {
                group: Group::parse("org.vibevm").unwrap(),
                name: "bootlib".to_string(),
                version: semver::Version::parse("1.0.0").unwrap(),
                dependencies: vec![],
                is_root: true,
            }],
        })
    }

    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
        let path = self
            .fixtures
            .join(pkg.name.to_string())
            .join(Manifest::FILENAME);
        Manifest::read(path)
            .map_err(|error| vibe_resolver::DepProviderError::Other(error.to_string()).into())
    }

    fn solve_masked(
        &self,
        _roots: &[PackageRef],
        _blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.solve(_roots)
    }

    fn materialise_in_place(
        &self,
        _pkgref: &PackageRef,
        _slot: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        panic!("no in-place packages in this fixture");
    }
}

/// A project whose root STATICALLY links `bootlib` (so the root node really
/// compiles a static lane), plus the fixture package that feeds it.
fn project_with_static_lane() -> (TempDir, PathBuf, FixtureSource) {
    let outer = TempDir::new().unwrap();
    let fixtures = outer.path().to_path_buf();
    fs::create_dir_all(fixtures.join("bootlib/boot")).unwrap();
    fs::write(
        fixtures.join("bootlib/vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"bootlib\"\nkind = \"flow\"\n\
         version = \"1.0.0\"\n\n[boot_snippet]\nsource = \"boot/bootlib.md\"\n",
    )
    .unwrap();
    fs::create_dir_all(fixtures.join("bootlib/boot")).unwrap();
    fs::write(fixtures.join("bootlib/boot/bootlib.md"), "# bootlib boot\n").unwrap();

    let project = outer.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n\
         [requires.packages]\n\"org.vibevm/bootlib\" = { version = \"^1\", link = \"static\" }\n",
    )
    .unwrap();
    (outer, project, FixtureSource { fixtures })
}

fn request() -> InstallRequest {
    InstallRequest {
        roots: vec![PackageRef::parse("org.vibevm/bootlib").unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "vibe test".to_string(),
    }
}

/// One injected lifecycle run id (exactly 32 lowercase hex) — injected rather
/// than allocated so the untraced twin can prove NO `.vibe` state appears:
/// `allocate_run_id` would itself create a scratch directory.
const RUN: &str = "0123456789abcdef0123456789abcdef";

fn metadata(trace_compile: bool) -> RunMetadata {
    RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile,
        run_id: RUN.to_string(),
        started: "2026-08-27T00:00:00Z".into(),
    }
}

/// One traced apply through the real plan pipeline: the borrowed run crosses
/// vibe-install → workspace apply → package units and nodes, and the run
/// directory ends with the root node's compiled occurrence in it.
#[test]
fn the_observed_apply_carries_one_borrowed_run_to_the_boot_compiles() {
    let (_outer, project, source) = project_with_static_lane();
    let metadata = metadata(true);
    let run = metadata.run_id.clone();
    let plan =
        vibe_install::plan(&source, &project, request(), &NullObserver).expect("plan succeeds");
    let planned = match plan {
        Plan::Ready(planned) => planned,
        Plan::Fresh => panic!("an explicit-root install is never Fresh"),
    };
    let trace = TraceRun::open_with_limits(
        &project,
        &run,
        Timestamp::from_timestamp(1_000, 0).unwrap(),
        TraceLimits::production(),
    )
    .expect("the recorder opens beside the lifecycle run");

    let report = apply_with_spec_format_and_lifecycle_observed_traced(
        &source,
        *planned,
        SlotIntegrity::TrustPresence,
        SpecFormat::Mixed,
        metadata,
        StreamMode::Capture,
        SlotLifecycleSeams::refusing(),
        Some(&trace),
    )
    .expect("the traced apply succeeds");

    assert_eq!(report.outcome.nodes_regenerated, vec!["."]);
    let bytes = fs::read(
        project
            .join(".vibe")
            .join("trace")
            .join(&run)
            .join("index.json"),
    )
    .expect("the run directory landed under the project");
    let index: CompilerTraceIndex =
        serde_json::from_slice(&bytes).expect("the index parses as the generated type");
    validate(&index).expect("the index obeys every relational law");
    assert_eq!(index.run_id, run, "the trace IS the lifecycle run");
    assert_eq!(
        index.status,
        RunStatus::Running,
        "apply borrows, never finishes"
    );
    // The base carries the artifact target; the attempt suffix is the
    // workspace's own closed grammar.
    let scope = index
        .scopes
        .iter()
        .find(|scope| scope.id == "node:.#static-md::attempt:1")
        .expect("the root node's occurrence is recorded");
    assert_eq!(scope.status, ScopeStatus::Compiled);
    assert!(
        index.events.iter().any(|event| event.scope == scope.id),
        "the compile's pass events are in the one run"
    );
}

/// The untraced wrapper is byte-compatible AND trace-silent: no run
/// directory, no `.vibe` at all.
#[test]
fn the_untraced_wrapper_leaves_no_trace_state() {
    let (_outer, project, source) = project_with_static_lane();
    let metadata = metadata(false);
    let plan =
        vibe_install::plan(&source, &project, request(), &NullObserver).expect("plan succeeds");
    let planned = match plan {
        Plan::Ready(planned) => planned,
        Plan::Fresh => panic!("an explicit-root install is never Fresh"),
    };
    apply_with_spec_format_and_lifecycle_observed(
        &source,
        *planned,
        SlotIntegrity::TrustPresence,
        SpecFormat::Mixed,
        metadata,
        StreamMode::Capture,
        SlotLifecycleSeams::refusing(),
    )
    .expect("the untraced apply succeeds");
    // The slot lifecycle's own `.vibe` state is not the trace's; the trace's
    // two side effects are the run directory and the cooperative lock.
    assert!(
        !project.join(".vibe").join("trace").exists(),
        "off mode opens no recorder and writes no run directory"
    );
    assert!(
        !project.join(".vibe").join("compile-trace.lock").exists(),
        "off mode contends for no trace lock"
    );
}
