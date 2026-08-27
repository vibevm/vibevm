//! The shared trace fixture: a temporary project, injected clocks, the real
//! two-document compiler world, and the on-disk readers every red asserts
//! through.
//!
//! Nothing here asserts on an elapsed number and nothing calls `now()`. Time
//! arrives as an argument, exactly as the production writer requires, so a
//! red is a statement about structure rather than about how fast the host is.

use std::collections::BTreeMap;
use std::path::Path;

use vibe_spec::{
    ArtifactInput, ArtifactPlan, ArtifactTarget as CompilerTarget, EmittedArtifact, SectionSource,
    SpecAddress, compile_artifact_traced,
};
use vibe_wire::behaviour::compiler_trace_index::validate;
use vibe_wire::generated::compiler_trace_index::e1::index::{
    ArtifactTarget, CompilerTraceIndex, ProjectIdentity, RunStatus, ScopeKind, Timestamp,
};

use super::super::{ScopeDescriptor, TraceLimits, TraceRun, TraceScope};

/// The run id most reds use. Exactly 32 lowercase hex.
pub(super) const RUN_A: &str = "0123456789abcdef0123456789abcdef";

/// An injected instant. Seconds since the epoch, so ordering is obvious in
/// the assertion that reads it.
pub(super) fn at(seconds: i64) -> Timestamp {
    Timestamp::from_timestamp(seconds, 0).expect("a fixture instant is representable")
}

/// A temporary project root. Absolute by construction, which is what the
/// writer requires.
pub(super) fn project() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary project root")
}

/// Open a fresh run with tiny limits, so budget and retention reds need no
/// production configuration.
pub(super) fn open(root: &Path, run_id: &str, limits: TraceLimits) -> TraceRun {
    TraceRun::open_with_limits(root, run_id, at(1_000), limits)
        .expect("a fresh run under a temporary root opens")
}

/// Generous limits: a budget no fixture reaches, and the production
/// retention count.
pub(super) fn roomy() -> TraceLimits {
    TraceLimits::for_test(u64::MAX, 9)
}

pub(super) fn node_scope(id: &str, label: &str) -> ScopeDescriptor {
    ScopeDescriptor {
        id: id.to_string(),
        kind: ScopeKind::Node,
        label: label.to_string(),
        artifact: "static-md".to_string(),
        target: ArtifactTarget::StaticMd,
    }
}

pub(super) fn unit_scope(id: &str, label: &str) -> ScopeDescriptor {
    ScopeDescriptor {
        id: id.to_string(),
        kind: ScopeKind::Unit,
        label: label.to_string(),
        artifact: "static-md".to_string(),
        target: ArtifactTarget::StaticMd,
    }
}

/// The absolute run directory of `run_id` under `root`.
pub(super) fn run_dir(root: &Path, run_id: &str) -> std::path::PathBuf {
    root.join(".vibe").join("trace").join(run_id)
}

/// Read the index EXACTLY as an outside reader would: off disk, through the
/// generated type, held to the epoch's own relational validator.
pub(super) fn read_index(directory: &Path) -> CompilerTraceIndex {
    let bytes = std::fs::read(directory.join("index.json"))
        .unwrap_or_else(|error| panic!("`{}` is readable: {error}", directory.display()));
    let index: CompilerTraceIndex =
        serde_json::from_slice(&bytes).expect("the index parses as the generated type");
    validate(&index).expect("the index obeys every relational law");
    index
}

/// Every entry of a run directory, sorted, so a red can name the whole set
/// rather than probing one file at a time.
pub(super) fn entries(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .expect("the run directory is listable")
        .map(|entry| {
            entry
                .expect("a readable entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// The `n`-th fixture run id: 32 lowercase hex by construction.
pub(super) fn run_id(n: u128) -> String {
    format!("{n:032x}")
}

/// The REAL project identity of a root — the same digest the writer derives,
/// through the same single canonicalisation. A seeded run that carried
/// anything else would be another project's run, which retention refuses to
/// touch, so a fixture that faked it would silently stop testing deletion.
pub(super) fn identity_of(root: &Path) -> ProjectIdentity {
    let canonical = std::fs::canonicalize(root).expect("a temporary root canonicalises");
    super::super::identity::project_identity(&canonical)
}

/// A plausible identity that is NOT this root's — another project's run,
/// spelled exactly as that project's writer would have spelled it.
pub(super) fn foreign_identity() -> ProjectIdentity {
    ProjectIdentity {
        display: ".".to_string(),
        root_digest: format!("sha256:{}", "b7".repeat(32)),
    }
}

/// A minimal but genuinely valid index for one seeded run, owned by `root`.
pub(super) fn seeded_index(
    root: &Path,
    run_id: &str,
    started: i64,
    status: RunStatus,
) -> CompilerTraceIndex {
    let terminal = status != RunStatus::Running;
    CompilerTraceIndex {
        aggregates: Vec::new(),
        events: Vec::new(),
        project: identity_of(root),
        run_id: run_id.to_string(),
        schema: 1,
        scopes: Vec::new(),
        started: at(started),
        failure: (status == RunStatus::Failed).then(|| "seeded failure".to_string()),
        finished: terminal.then(|| at(started + 1)),
        status,
    }
}

/// Plant one complete owned run directory that retention is entitled to
/// delete, and return its path.
pub(super) fn seed_run(root: &Path, run_id: &str, started: i64) -> std::path::PathBuf {
    seed_index_at(
        root,
        run_id,
        &seeded_index(root, run_id, started, RunStatus::Ok),
    )
}

/// Plant a run directory carrying exactly `index`, however (in)eligible that
/// makes it.
pub(super) fn seed_index_at(
    root: &Path,
    run_id: &str,
    index: &CompilerTraceIndex,
) -> std::path::PathBuf {
    let directory = run_dir(root, run_id);
    std::fs::create_dir_all(&directory).expect("a seeded run directory");
    let mut bytes = serde_json::to_vec_pretty(index).expect("a seeded index serialises");
    bytes.push(b'\n');
    std::fs::write(directory.join("index.json"), bytes).expect("a seeded index");
    directory
}

/// Every 32-lowercase-hex direct child of `.vibe/trace`, sorted.
pub(super) fn run_directories(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = entries(&root.join(".vibe").join("trace"))
        .into_iter()
        .filter(|name| {
            name.len() == 32
                && name
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
        .collect();
    names.sort();
    names
}

/// The smallest honest world whose ONE root pulls in a second addressed
/// document, so the real built-in schedule invokes `parse` exactly twice.
pub(super) struct World(BTreeMap<String, String>);

impl World {
    pub(super) fn two_documents() -> Self {
        let mut map = BTreeMap::new();
        map.insert(
            "spec://org.demo/alpha/boot/entry#root".to_string(),
            "# Alpha {#root}\n#use spec://org.demo/shared/boot/base#root\nALPHA\n".to_string(),
        );
        map.insert(
            "spec://org.demo/shared/boot/base#root".to_string(),
            "# Shared {#root}\n##SHARED shared\n".to_string(),
        );
        Self(map)
    }

    /// The same world with the `#use` target missing, so a real built-in pass
    /// fails inside the artifact segment and the compiler returns an error.
    pub(super) fn dangling_use() -> Self {
        let mut world = Self::two_documents();
        world.0.remove("spec://org.demo/shared/boot/base#root");
        world
    }
}

impl SectionSource for World {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        self.0
            .get(&address.without_pin())
            .cloned()
            .ok_or_else(|| format!("missing {}", address.without_pin()))
    }
}

/// The real static-lane plan the boot artifacts themselves compile through.
pub(super) fn plan() -> ArtifactPlan {
    ArtifactPlan::static_lane(
        CompilerTarget::StaticMarkdown,
        "vibevm/vibespecs/boot/STATIC.md",
        "vibevm/vibespecs",
        vec![
            ArtifactInput::normal(
                "org.demo/alpha",
                "boot/entry.md",
                SpecAddress::parse("spec://org.demo/alpha/boot/entry#root").unwrap(),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

/// Run one real traced compilation into `scope`.
pub(super) fn compile(scope: &TraceScope, world: &World) -> Option<EmittedArtifact> {
    compile_artifact_traced(plan(), world, scope).ok()
}

/// The same, asserting the compile really did succeed — the property every
/// observer red depends on.
pub(super) fn compile_ok(scope: &TraceScope, world: &World) -> EmittedArtifact {
    compile_artifact_traced(plan(), world, scope).expect("the two-document world compiles")
}
