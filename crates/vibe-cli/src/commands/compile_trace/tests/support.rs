//! The shared fixture: a temporary workspace root, counting clocks, hand-built
//! run identities, and the real two-document compiler world.
//!
//! The clocks COUNT. Several of this atom's laws are of the form "that path
//! must not ask what time it is", and a clock that only answers cannot prove
//! one.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use vibe_lifecycle::{RunIdentity, SupersededTrace};
use vibe_spec::{
    ArtifactInput, ArtifactPlan, ArtifactTarget as CompilerTarget, SectionSource, SpecAddress,
    compile_artifact_traced,
};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    ArtifactTarget, CompilerTraceIndex, ScopeKind,
};
use vibe_wire::generated::shared::Timestamp;
use vibe_workspace::compile_trace::{ScopeDescriptor, TraceScope};

/// Two distinct, exactly-32-lowercase-hex lifecycle run ids.
pub(super) const RUN_A: &str = "0123456789abcdef0123456789abcdef";
pub(super) const RUN_B: &str = "fedcba9876543210fedcba9876543210";

/// The RFC 3339 spelling the lifecycle actually persists.
pub(super) const STARTED_A: &str = "2026-08-27T10:00:00Z";
pub(super) const STARTED_B: &str = "2026-08-27T09:00:00Z";

pub(super) fn project() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary workspace root")
}

pub(super) fn at(seconds: i64) -> Timestamp {
    Timestamp::from_timestamp(seconds, 0).expect("a fixture instant is representable")
}

/// The trace epoch's timestamp for one of the RFC 3339 starts above — derived,
/// never a hand-computed epoch second, because a reopen compares it exactly.
pub(super) fn started(text: &str) -> Timestamp {
    chrono::DateTime::parse_from_rfc3339(text)
        .expect("a fixture start is RFC 3339")
        .with_timezone(&chrono::Utc)
}

/// A fixture instant `seconds` after [`STARTED_A`].
///
/// Terminal instants MUST follow the run's own start — the trace index refuses
/// a `finished` before its `started`, and a fixture that hand-picked epoch
/// seconds would be quietly writing 1970 into a 2026 run.
pub(super) fn after(seconds: i64) -> Timestamp {
    started(STARTED_A) + chrono::Duration::seconds(seconds)
}

/// An injected clock that remembers how often it was asked.
pub(super) struct Ticks {
    instant: Timestamp,
    calls: Cell<usize>,
}

impl Ticks {
    pub(super) fn new(seconds: i64) -> Self {
        Self {
            instant: after(seconds),
            calls: Cell::new(0),
        }
    }

    pub(super) fn clock(&self) -> impl Fn() -> Timestamp + '_ {
        move || {
            self.calls.set(self.calls.get() + 1);
            self.instant
        }
    }

    pub(super) fn calls(&self) -> usize {
        self.calls.get()
    }
}

/// One selected lifecycle identity, spelled exactly as `select_run_identity`
/// would hand it over.
pub(super) fn identity(run_id: &str, adopted: bool, compile_trace: bool) -> RunIdentity {
    RunIdentity {
        run_id: run_id.to_string(),
        started: STARTED_A.to_string(),
        adopted,
        compile_trace,
        superseded_trace: None,
    }
}

/// The same, displacing a state-proven parked traced run.
pub(super) fn displacing(run_id: &str, superseded: &str) -> RunIdentity {
    RunIdentity {
        superseded_trace: Some(SupersededTrace {
            run_id: superseded.to_string(),
            started: STARTED_B.to_string(),
        }),
        ..identity(run_id, false, true)
    }
}

pub(super) fn trace_root(root: &Path) -> PathBuf {
    root.join(".vibe").join("trace")
}

pub(super) fn run_dir(root: &Path, run_id: &str) -> PathBuf {
    trace_root(root).join(run_id)
}

/// Read one run's index exactly as an outside reader would.
pub(super) fn read_index(root: &Path, run_id: &str) -> CompilerTraceIndex {
    let path = run_dir(root, run_id).join("index.json");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("`{}` is readable: {error}", path.display()));
    serde_json::from_slice(&bytes).expect("the index parses as the generated type")
}

/// Every 32-lowercase-hex run directory under the root, sorted.
pub(super) fn run_directories(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(trace_root(root)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| entry.expect("a readable entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| {
            name.len() == 32 && name.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
        .collect();
    names.sort();
    names
}

/// Every byte of every file under `.vibe/trace`, concatenated — the corpus a
/// leak red searches for its sentinel in.
pub(super) fn all_trace_bytes(root: &Path) -> String {
    let mut found = String::new();
    let mut stack = vec![trace_root(root)];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries {
            let entry = entry.expect("a readable entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = std::fs::read(&path) {
                found.push_str(&String::from_utf8_lossy(&bytes));
            }
        }
    }
    found
}

pub(super) fn node_scope(id: &str) -> ScopeDescriptor {
    ScopeDescriptor {
        id: id.to_string(),
        kind: ScopeKind::Node,
        label: ".".to_string(),
        artifact: "static-md".to_string(),
        target: ArtifactTarget::StaticMd,
    }
}

/// The smallest honest world whose ONE root pulls in a second addressed
/// document, so the real built-in schedule invokes `parse` twice and the run
/// really does accumulate aggregate timing rows.
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
}

impl SectionSource for World {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        self.0
            .get(&address.without_pin())
            .cloned()
            .ok_or_else(|| format!("missing {}", address.without_pin()))
    }
}

/// Run one real traced compilation into `scope` and resolve it, exactly as a
/// traced artifact compile does — so the run carries events, snapshots and
/// aggregate rows rather than a synthetic zero, and leaves no `pending` scope
/// to make a successful command's terminal `ok` index impossible.
pub(super) fn compile(scope: &TraceScope) {
    let plan = ArtifactPlan::static_lane(
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
    .unwrap();
    let artifact = compile_artifact_traced(plan, &World::two_documents(), scope)
        .expect("the two-document world compiles");
    scope.complete_lossy(&artifact.output_fingerprint());
}
