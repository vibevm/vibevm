//! Shared fixtures for the R3.4 trace integration targets.
//!
//! Deliberately thin: the interesting assertions belong in the tests, and a
//! helper that decided what "one run" meant would be asserting the property it
//! is supposed to expose.

#![allow(dead_code, reason = "each trace target uses a different subset")]

use std::path::{Path, PathBuf};

use serde_json::Value;
use vibe_wire::generated::compiler_trace_index::e1::index::CompilerTraceIndex;

use super::common::UserScratch;

pub fn trace_dir(project: &Path) -> PathBuf {
    project.join(".vibe").join("trace")
}

/// Every 32-lowercase-hex run directory under a project, sorted.
///
/// The hex filter is the point: it refuses to count a stray file as a run, the
/// same way the writer refuses to infer ownership from a directory name.
pub fn run_directories(project: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(trace_dir(project)) else {
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

pub fn index_of(project: &Path, run_id: &str) -> CompilerTraceIndex {
    let path = trace_dir(project).join(run_id).join("index.json");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("`{}` is readable: {error}", path.display()));
    serde_json::from_slice(&bytes).expect("the index parses as the generated type")
}

pub fn trace_member(report: &Value) -> Option<&Value> {
    report.get("trace")
}

/// The run id this report claims, proved to be a real directory on disk.
///
/// Both halves matter: a report naming a run nobody created, and a directory
/// no report mentions, are the two ways "one command-owned run" fails.
pub fn sole_run(project: &Path, report: &Value) -> String {
    let trace = trace_member(report).expect("a traced command reports its trace");
    let run_id = trace["run_id"]
        .as_str()
        .expect("the member names its run")
        .to_string();
    assert!(
        trace_dir(project).join(&run_id).is_dir(),
        "the reported run `{run_id}` really exists on disk",
    );
    run_id
}

pub fn documents(bytes: &[u8]) -> Vec<Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| {
            panic!(
                "stdout is a JSON document stream: {error}\n{}",
                String::from_utf8_lossy(bytes)
            )
        })
}

/// The one registered root of `command`, refusing a second.
pub fn sole_root(bytes: &[u8], command: &str) -> Value {
    let docs = documents(bytes);
    let mut roots = docs.iter().filter(|doc| doc["command"] == command);
    let root = roots
        .next()
        .unwrap_or_else(|| panic!("one `{command}` root: {docs:#?}"))
        .clone();
    assert!(
        roots.next().is_none(),
        "exactly one `{command}` root: {docs:#?}",
    );
    root
}

/// A bare `vibe install --json` over an empty project, plus extra flags.
pub fn install_json(user: &UserScratch, project: &Path, extra: &[&str]) -> Value {
    let output = user
        .vibe()
        .args(["install", "--json", "--offline", "--assume-yes"])
        .args(extra)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    sole_root(&output.stdout, "install")
}

/// The whole of a quiet install's stdout — which is supposed to be one line.
pub fn quiet_install(user: &UserScratch, project: &Path, extra: &[&str]) -> String {
    let output = user
        .vibe()
        .args(["install", "--quiet", "--offline", "--assume-yes"])
        .args(extra)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A dependency that traces its OWN builds. Installing it must not switch
/// tracing on for the project that consumes it.
pub fn publish_tracing_package(registry: &Path) {
    let package = registry.join("org.trace").join("dep").join("v0.1.0");
    std::fs::create_dir_all(&package).expect("a fixture package directory");
    std::fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup = 'org.trace'\nname = 'dep'\nkind = 'tool'\nversion = '0.1.0'\n\n\
         [compile]\ntrace = true\n",
    )
    .expect("a fixture package manifest");
}

/// Turn the selected project's own `[compile] trace` on.
pub fn declare_trace(project: &Path) {
    let path = project.join("vibe.toml");
    let mut text = std::fs::read_to_string(&path).expect("the project manifest");
    text.push_str("\n[compile]\ntrace = true\n");
    std::fs::write(&path, text).expect("writing the project manifest");
}

/// Declare a STATIC dependency edge on the selected project.
///
/// The static lane is what makes a node compile at all: a dynamically linked
/// dependency contributes an `INDEX.md` entry and no compiled artifact, so a
/// traced run over one records nothing and proves nothing.
pub fn declare_static_dependency(project: &Path, pkgref: &str, version: &str) {
    let path = project.join("vibe.toml");
    let mut text = std::fs::read_to_string(&path).expect("the project manifest");
    text.push_str(&format!(
        "\n[requires]\npackages = {{ \"{pkgref}\" = {{ version = \"{version}\", \
         link = \"static\" }} }}\n"
    ));
    std::fs::write(&path, text).expect("writing the project manifest");
}

/// Corrupt the selected project's manifest — the mutation that proves a
/// command read it once and kept what it read.
pub fn corrupt_manifest(project: &Path) {
    std::fs::write(project.join("vibe.toml"), "[project\nname = broken\n")
        .expect("writing a malformed manifest");
}

/// Every allocated lifecycle run directory under a project.
pub fn lifecycle_run_dirs(project: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(project.join(".vibe").join("lifecycle")) else {
        return Vec::new();
    };
    entries
        .map(|entry| entry.expect("a readable entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect()
}

/// A project whose one dependency is STATICALLY linked, so installing it
/// really compiles a boot artifact and the trace has something to record. A
/// dynamic edge contributes an `INDEX.md` line and no compiled artifact, so a
/// run over one records nothing and would prove nothing.
pub fn static_project(user: &UserScratch) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    declare_static_dependency(project.path(), "flow:org.vibevm/integration-alpha", "^0.1");
    project
}

/// Clap refused the arguments — as opposed to the command running and failing.
pub fn clap_rejected(output: &std::process::Output) -> bool {
    String::from_utf8_lossy(&output.stderr).contains("unexpected argument")
}

/// Two runs in two temp directories differ in their `project` path; that is
/// never the difference under test.
pub fn normalise_project(document: &mut Value) {
    if let Some(object) = document.as_object_mut() {
        object.insert("project".into(), Value::String("<root>".into()));
    }
}

/// Whether the trace's cooperative lock file exists anywhere under `.vibe`.
///
/// Searched rather than hard-coded: the point of the assertion is that
/// disabled mode leaves NO trace artifact behind, and a test that named one
/// exact path would pass the day the writer moved it.
pub fn trace_lock_exists(project: &Path) -> bool {
    fn walk(dir: &Path) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if walk(&path) {
                    return true;
                }
            } else if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().contains("compile-trace"))
            {
                return true;
            }
        }
        false
    }
    walk(&project.join(".vibe"))
}

/// Every byte of every file under `.vibe/trace`, concatenated — the corpus a
/// leak red searches its sentinel in.
///
/// Whole-tree rather than index-only on purpose: a leak that reached an event
/// or snapshot file and not the index would still be a leak.
pub fn all_trace_bytes(project: &Path) -> String {
    let mut found = String::new();
    let mut stack = vec![trace_dir(project)];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
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
