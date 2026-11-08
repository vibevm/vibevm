//! Portable stop-file and publication-boundary integration.
//!
//! Every cancelled or rolled-back scenario here also runs the leftover
//! check (`common::leftovers`): the project including `.vibe/`, the
//! command's own HOME and TMPDIR, the settings and both caches are
//! snapshotted before and after, and nothing may differ but the entries of
//! [`ALLOWED`].
mod common;
use common::UserScratch;
use common::leftovers::{
    Allowed, Entry, Isolation, Snapshot, Watched, assert_complete_store, assert_no_leftovers,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn copy(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let output = target.join(entry.file_name());
        if path.is_dir() {
            copy(&path, &output);
        } else {
            fs::copy(path, output).unwrap();
        }
    }
}

fn is_hook_stop_file(path: &Path, _: Option<&Entry>) -> bool {
    path == Path::new(".vibe/cancel-request")
}

fn is_run_record(path: &Path, after: Option<&Entry>) -> bool {
    path == Path::new(".vibe/lifecycle.toml") && after.is_some()
}

fn is_empty_lease_lock(path: &Path, after: Option<&Entry>) -> bool {
    path == Path::new(".vibe/lifecycle.lock")
        && matches!(after, Some(Entry::File(bytes)) if bytes.is_empty())
}

/// An entry of a content store; [`assert_clean`] checks each store is
/// made of complete objects only.
fn is_store_object(path: &Path, after: Option<&Entry>) -> bool {
    after.is_some()
        && (path.starts_with("cache")
            || path == Path::new(".native")
            || path.starts_with(".native/metadata-store"))
}

/// What a cancelled or rolled-back project operation may leave changed.
/// Each entry cites where the record is documented; nothing else may
/// remain.
const ALLOWED: &[Allowed] = &[
    // The caller's stop control, written by the test's own hook. PROP-073
    // `#PROJECT-COOPERATIVE-STOP`: "vibe does not remove that caller
    // control file".
    Allowed {
        root: "project",
        matches: is_hook_stop_file,
        why: ".vibe/cancel-request — the caller-owned stop file (PROP-073 #PROJECT-COOPERATIVE-STOP)",
    },
    // The run record of the stopped or failed run: PROP-054 `#OBS-RUN-RECORD`
    // "Every run leaves a record", kept in `.vibe/lifecycle.toml`
    // (`#REF-LIFECYCLE-TOML`). A failed hook's `fail` row is how the next
    // run knows what happened.
    Allowed {
        root: "project",
        matches: is_run_record,
        why: ".vibe/lifecycle.toml — the durable run record (PROP-054 #OBS-RUN-RECORD, #REF-LIFECYCLE-TOML)",
    },
    // The coordination lease's lock file, empty. PROP-072
    // `#SOURCE-INSTALL-BOUNDARY`: "its empty lifecycle.lock … may remain".
    Allowed {
        root: "project",
        matches: is_empty_lease_lock,
        why: ".vibe/lifecycle.lock, empty — the lease lock (PROP-072 #SOURCE-INSTALL-BOUNDARY)",
    },
    // Verified, complete downloaded content: PROP-073
    // `#PROJECT-COOPERATIVE-STOP` "Completed cache objects may remain
    // reusable" (Need 5: downloaded content may be kept in the cache).
    Allowed {
        root: "settings",
        matches: is_store_object,
        why: "cache/<group>/<name>/v<version> with its .sha256 — a complete store object (PROP-073 #PROJECT-COOPERATIVE-STOP)",
    },
    Allowed {
        root: "registry-cache",
        matches: is_store_object,
        why: ".native/metadata-store/<group>/<name>/v<version> with its .sha256 — a complete metadata object (PROP-073 #PROJECT-COOPERATIVE-STOP)",
    },
];

/// The roots the cancelled command may write: its isolation's HOME and
/// TMPDIR, the user scratch, and the whole project.
fn watched(isolation: &Isolation, user: &UserScratch, project: &Path) -> Watched {
    isolation.watched(user).root("project", project)
}

/// The cancelled command, with its own HOME and TMPDIR.
fn isolated(user: &UserScratch, isolation: &Isolation) -> assert_cmd::Command {
    let mut command = user.vibe();
    isolation.apply(&mut command);
    command
}

fn assert_clean(watched: &Watched, before: &Snapshot) {
    assert_no_leftovers(before, &watched.snapshot(), ALLOWED);
    for store in watched.roots_named(&["settings", "registry-cache"]) {
        assert_complete_store(&store.join("cache"));
        assert_complete_store(&store.join(".native/metadata-store"));
    }
}

/// Every file under `root` except the `.vibe` operation state, byte for byte.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(root).unwrap();
            if rel
                .components()
                .next()
                .is_some_and(|part| part.as_os_str() == ".vibe")
            {
                continue;
            }
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(rel.into(), fs::read(path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

/// Arrange the node's agent files so the boot regeneration rewrites them,
/// and return the stop file that regeneration itself creates.
///
/// `CLAUDE.md` holds co-tenant bytes with no managed block, so the
/// regeneration appends one after them. `GEMINI.md` is absent, so the
/// regeneration creates it — the node publication's last write, after every
/// unit and node lane. Watching it as the stop file requests the stop exactly
/// once boot regeneration has finished and before the lock is published, the
/// window the boot inverse exists for; a restore that removes it again is
/// part of the byte-identical check.
fn stop_after_boot_regeneration(project: &Path) -> PathBuf {
    fs::write(
        project.join("CLAUDE.md"),
        "# Owner notes

Keep this.
",
    )
    .unwrap();
    let stop = project.join("GEMINI.md");
    if stop.exists() {
        fs::remove_file(&stop).unwrap();
    }
    stop
}

/// The project's files are exactly `before`; a failure names the paths.
fn assert_unchanged(project: &Path, before: &BTreeMap<PathBuf, Vec<u8>>) {
    let after = snapshot(project);
    let differing: Vec<&PathBuf> = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .collect();
    assert!(
        differing.is_empty(),
        "changed by the cancelled run: {differing:?}"
    );
}

fn assert_cancelled_and_restored(output: &std::process::Output) {
    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = common::hook_slot::documents(&output.stdout);
    assert_eq!(docs.last().unwrap()["outcome"], "cancelled");
    assert_eq!(docs.last().unwrap()["publication"], "restored");
}

#[test]
fn existing_stop_file_preserves_manifest_for_install_and_update() {
    for verb in ["install", "update"] {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        user.vibe()
            .args(["init", "--no-registry", "--author", "Owner"])
            .arg("--path")
            .arg(project.path())
            .assert()
            .success();
        let before = fs::read(project.path().join("vibe.toml")).unwrap();
        let outer = tempfile::tempdir().unwrap();
        let stop = outer.path().join("stop");
        fs::write(&stop, "").unwrap();
        let isolation = Isolation::new(outer.path());
        let watched = watched(&isolation, &user, project.path());
        let baseline = watched.snapshot();
        let output = isolated(&user, &isolation)
            .args([verb, "--json", "--assume-yes", "--stop-file"])
            .arg(&stop)
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(130));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["outcome"], "cancelled");
        assert_eq!(report["publication"], "not-started");
        assert_eq!(fs::read(project.path().join("vibe.toml")).unwrap(), before);
        assert!(stop.exists());
        assert_clean(&watched, &baseline);
    }
}

#[test]
fn stop_requested_by_preinstall_hook_rolls_back_owned_project_publication() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let package = registry.join("org.example/hooked/v0.1.0");
    let before = fs::read(project.path().join("vibe.toml")).unwrap();
    let stop = project.path().join(".vibe/cancel-request");
    fs::create_dir_all(stop.parent().unwrap()).unwrap();
    let shell = fs::read_to_string(package.join("hooks/pre.sh")).unwrap();
    fs::write(
        package.join("hooks/pre.sh"),
        format!("{shell}\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\n"),
    )
    .unwrap();
    let ps = fs::read_to_string(package.join("hooks/pre.ps1")).unwrap();
    fs::write(package.join("hooks/pre.ps1"), format!("{ps}\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/cancel-request') -Value stop\n")).unwrap();
    let isolation = Isolation::new(_outer.path());
    let watched = watched(&isolation, &user, project.path());
    let baseline = watched.snapshot();
    let output = isolated(&user, &isolation)
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = common::hook_slot::documents(&output.stdout);
    assert_eq!(docs.last().unwrap()["outcome"], "cancelled");
    assert_eq!(docs.last().unwrap()["publication"], "restored");
    assert_eq!(fs::read(project.path().join("vibe.toml")).unwrap(), before);
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .exists()
    );
    assert_clean(&watched, &baseline);
}

#[test]
fn cancel_after_hook_replaces_manifest_preserves_the_foreign_file() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let package = registry.join("org.example/hooked/v0.1.0");
    let stop = project.path().join(".vibe/cancel-request");
    fs::create_dir_all(stop.parent().unwrap()).unwrap();
    let shell = fs::read_to_string(package.join("hooks/pre.sh")).unwrap();
    fs::write(package.join("hooks/pre.sh"),format!("{shell}\nrm \"$VIBE_PROJECT_ROOT/vibe.toml\"\nprintf foreign > \"$VIBE_PROJECT_ROOT/vibe.toml\"\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\n")).unwrap();
    let ps = fs::read_to_string(package.join("hooks/pre.ps1")).unwrap();
    fs::write(package.join("hooks/pre.ps1"),format!("{ps}\nRemove-Item -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT 'vibe.toml')\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT 'vibe.toml') -Value foreign -NoNewline\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/cancel-request') -Value stop\n")).unwrap();
    let output = user
        .vibe()
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let documents = common::hook_slot::documents(&output.stdout);
    assert_eq!(
        documents.last().unwrap()["publication"],
        "recovery-required"
    );
    assert_eq!(
        fs::read(project.path().join("vibe.toml")).unwrap(),
        b"foreign"
    );
}

#[test]
fn update_cancel_restores_old_manifest_lock_and_installed_slot() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='fixture'\nurl='file:///{}'\n",
            registry.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    user.vibe()
        .args(["install", "org.example/hooked", "--assume-yes", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let manifest = fs::read(project.path().join("vibe.toml")).unwrap();
    let lock = fs::read(project.path().join("vibe.lock")).unwrap();
    let old = registry.join("org.example/hooked/v0.1.0");
    let next = registry.join("org.example/hooked/v0.1.1");
    copy(&old, &next);
    let declaration = fs::read_to_string(next.join("vibe.toml"))
        .unwrap()
        .replace("0.1.0", "0.1.1");
    fs::write(next.join("vibe.toml"), declaration).unwrap();
    let shell = fs::read_to_string(next.join("hooks/pre.sh")).unwrap();
    fs::write(
        next.join("hooks/pre.sh"),
        format!("{shell}\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\n"),
    )
    .unwrap();
    let ps = fs::read_to_string(next.join("hooks/pre.ps1")).unwrap();
    fs::write(next.join("hooks/pre.ps1"),format!("{ps}\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/cancel-request') -Value stop\n")).unwrap();
    let stop = project.path().join(".vibe/cancel-request");
    let isolation = Isolation::new(_outer.path());
    let watched = watched(&isolation, &user, project.path());
    let baseline = watched.snapshot();
    let output = isolated(&user, &isolation)
        .args([
            "update",
            // A named update keeps the test on one package's version move.
            "org.example/hooked",
            "--exact",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(stop)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = common::hook_slot::documents(&output.stdout);
    assert_eq!(docs.last().unwrap()["outcome"], "cancelled");
    assert_eq!(docs.last().unwrap()["publication"], "restored");
    assert_eq!(
        fs::read(project.path().join("vibe.toml")).unwrap(),
        manifest
    );
    assert_eq!(fs::read(project.path().join("vibe.lock")).unwrap(), lock);
    assert!(
        project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .is_dir()
    );
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.1"))
            .exists()
    );
    assert_clean(&watched, &baseline);
}

/// Desktop ApplyManifest is install with no coordinates, --offline and a
/// saved authored manifest. Cancelling must keep that saved file, not undo
/// an earlier editor save which is outside this invocation.
#[test]
fn apply_saved_manifest_cancel_keeps_saved_bytes_and_prior_installed_world() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='fixture'\nurl='file:///{}'\n",
            registry.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    let package = registry.join("org.example/hooked/v0.1.0");
    let shell = fs::read_to_string(package.join("hooks/pre.sh")).unwrap();
    fs::write(
        package.join("hooks/pre.sh"),
        format!("{shell}\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\n"),
    )
    .unwrap();
    let ps = fs::read_to_string(package.join("hooks/pre.ps1")).unwrap();
    fs::write(package.join("hooks/pre.ps1"),format!("{ps}\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/cancel-request') -Value stop\n")).unwrap();
    user.vibe()
        .args(["cache", "add", "org.example/hooked", "--path"])
        .arg(project.path())
        .assert()
        .success();
    // The UI's saved authored input exists before the operation's boundary.
    let manifest_path = project.path().join("vibe.toml");
    let mut authored: toml::Value =
        toml::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    let requires = authored
        .as_table_mut()
        .unwrap()
        .entry("requires")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    let packages = requires
        .as_table_mut()
        .unwrap()
        .entry("packages")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    packages.as_table_mut().unwrap().insert(
        "org.example/hooked".into(),
        toml::Value::String("^0.1.0".into()),
    );
    fs::write(&manifest_path, toml::to_string(&authored).unwrap()).unwrap();
    let before = snapshot(project.path());
    let saved = fs::read(&manifest_path).unwrap();
    let stop = project.path().join(".vibe/cancel-request");
    fs::create_dir_all(stop.parent().unwrap()).unwrap();
    let isolation = Isolation::new(_outer.path());
    let watched = watched(&isolation, &user, project.path());
    let baseline = watched.snapshot();
    let output = isolated(&user, &isolation)
        .args([
            "install",
            "--offline",
            "--json",
            "--assume-yes",
            "--agent-mode",
            "agent",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let documents = common::hook_slot::documents(&output.stdout);
    assert_eq!(documents.last().unwrap()["outcome"], "cancelled");
    assert_eq!(documents.last().unwrap()["publication"], "restored");
    assert_eq!(fs::read(&manifest_path).unwrap(), saved);
    assert_eq!(snapshot(project.path()), before);
    assert_clean(&watched, &baseline);
}

#[test]
fn install_stop_after_boot_regeneration_restores_boot_lanes_and_agent_files() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let stop = stop_after_boot_regeneration(project.path());
    let before = snapshot(project.path());
    let isolation = Isolation::new(_outer.path());
    let watched = watched(&isolation, &user, project.path());
    let baseline = watched.snapshot();
    let output = isolated(&user, &isolation)
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_cancelled_and_restored(&output);
    assert_unchanged(project.path(), &before);
    assert_clean(&watched, &baseline);
}

#[test]
fn update_stop_after_boot_regeneration_restores_boot_lanes_and_agent_files() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='fixture'\nurl='file:///{}'\n",
            registry.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    user.vibe()
        .args(["install", "org.example/hooked", "--assume-yes", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let next = registry.join("org.example/hooked/v0.1.1");
    copy(&registry.join("org.example/hooked/v0.1.0"), &next);
    let declaration = fs::read_to_string(next.join("vibe.toml"))
        .unwrap()
        .replace("0.1.0", "0.1.1");
    fs::write(next.join("vibe.toml"), declaration).unwrap();
    let stop = stop_after_boot_regeneration(project.path());
    let before = snapshot(project.path());
    let isolation = Isolation::new(_outer.path());
    let watched = watched(&isolation, &user, project.path());
    let baseline = watched.snapshot();
    let output = isolated(&user, &isolation)
        .args([
            "update",
            "org.example/hooked",
            "--exact",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_cancelled_and_restored(&output);
    assert_unchanged(project.path(), &before);
    assert_clean(&watched, &baseline);
}

/// A pre-install hook that fails rolls the install back (PROP-020 §2.5);
/// the rollback leaves nothing behind either.
#[test]
fn failed_preinstall_hook_rollback_leaves_no_leftovers() {
    use common::hook_slot::{Fixture, setup};
    let (outer, user, project, registry) = setup(Fixture::PreFail);
    let before = fs::read(project.path().join("vibe.toml")).unwrap();
    let isolation = Isolation::new(outer.path());
    let watched = watched(&isolation, &user, project.path());
    let baseline = watched.snapshot();
    let output = isolated(&user, &isolation)
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--json",
            "--assume-yes",
        ])
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_ne!(output.status.code(), Some(130));
    assert_eq!(fs::read(project.path().join("vibe.toml")).unwrap(), before);
    assert_clean(&watched, &baseline);
}

/// A cancelled preview publishes nothing and writes no cache.
#[test]
fn cancelled_preview_leaves_no_leftovers() {
    use common::hook_slot::{Fixture, setup};
    let (outer, user, project, registry) = setup(Fixture::Timing);
    let stop = outer.path().join("stop");
    fs::write(&stop, "").unwrap();
    let isolation = Isolation::new(outer.path());
    let watched = watched(&isolation, &user, project.path());
    let baseline = watched.snapshot();
    let output = isolated(&user, &isolation)
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--plan-only",
            "--json",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = common::hook_slot::documents(&output.stdout);
    assert_eq!(docs.last().unwrap()["outcome"], "cancelled");
    assert_eq!(docs.last().unwrap()["cache_writes"], "none");
    assert!(stop.exists());
    assert_clean(&watched, &baseline);
}

/// Ctrl-C on an attended terminal asks before it stops (PROP-073
/// `#PROJECT-COOPERATIVE-STOP`). Each run is driven through a real
/// pseudo-terminal by Python's `pty` module: the pre-install hook signals
/// that it runs and waits for a go file, the driver types Ctrl-C into the
/// terminal — so the terminal's own `SIGINT` reaches the whole foreground
/// group, hook included — waits for the question, answers it, and releases
/// the hook.
#[cfg(target_os = "linux")]
mod attended {
    use super::*;

    const DRIVER: &str = r#"
import os, pty, select, sys, time
ready, go, answer, transcript = sys.argv[1:5]
argv = sys.argv[5:]
pid, fd = pty.fork()
if pid == 0:
    os.environ["TERM"] = "xterm"
    os.execvp(argv[0], argv)
seen = b""
def pump(seconds):
    global seen
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                return False
            if not chunk:
                return False
            seen += chunk
    return True
def until(check, what):
    end = time.time() + 120
    while not check():
        if time.time() > end:
            open(transcript, "wb").write(seen)
            sys.exit("timed out waiting for " + what)
        pump(0.05)
until(lambda: os.path.exists(ready), "the hook")
os.write(fd, b"\x03")
until(lambda: b"Stop the operation?" in seen, "the question")
os.write(fd, answer.encode())
# The answer is read before the hook may finish.
until(lambda: seen.rfind(b"Stop the operation?") < max(seen.rfind(b"yes"), seen.rfind(b"no")), "the answer")
open(go, "w").close()
status = None
end = time.time() + 300
while status is None and time.time() < end:
    pump(0.05)
    done, code = os.waitpid(pid, os.WNOHANG)
    if done:
        status = code
while pump(0.2):
    pass
open(transcript, "wb").write(seen)
if status is None:
    sys.exit("vibe did not exit")
sys.exit(os.waitstatus_to_exitcode(status) % 256)
"#;

    /// The pre-install hook: say it runs, then wait to be released. It does
    /// not guard against Ctrl-C: it runs as a supervised lifecycle child in
    /// a process group of its own, so the terminal's Ctrl-C reaches vibe
    /// alone and the hook keeps running while the question is asked.
    fn hook(package: &Path, ready: &Path, go: &Path) {
        let shell = fs::read_to_string(package.join("hooks/pre.sh")).unwrap();
        fs::write(
            package.join("hooks/pre.sh"),
            format!(
                "{shell}\nprintf ready > '{}'\nwhile [ ! -f '{}' ]; do sleep 0.05; done\n",
                ready.display(),
                go.display()
            ),
        )
        .unwrap();
    }

    struct Run {
        code: Option<i32>,
        transcript: String,
    }

    fn drive(
        user: &UserScratch,
        isolation: &Isolation,
        outer: &Path,
        answer: &str,
        args: &[&std::ffi::OsStr],
    ) -> Run {
        let driver = outer.join("driver.py");
        fs::write(&driver, DRIVER).unwrap();
        let transcript = outer.join(format!("transcript-{answer}"));
        let output = std::process::Command::new("python3")
            .arg(&driver)
            .arg(outer.join("ready"))
            .arg(outer.join("go"))
            .arg(answer)
            .arg(&transcript)
            .arg(assert_cmd::cargo::cargo_bin("vibe"))
            .args(args)
            .envs(isolation.envs())
            .env(vibe_core::settings::SETTINGS_DIR_ENV, &user.settings)
            .env(vibe_test_support::REGISTRY_CACHE_ENV, &user.cache)
            .env(vibe_test_support::SEARCH_CACHE_ENV, &user.search_cache)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env_remove("VIBE_UNATTENDED")
            .env_remove("CI")
            .output()
            .unwrap();
        let transcript =
            String::from_utf8_lossy(&fs::read(&transcript).unwrap_or_default()).into_owned();
        assert!(
            output.stderr.is_empty(),
            "the driver failed: {}\n{transcript}",
            String::from_utf8_lossy(&output.stderr)
        );
        Run {
            code: output.status.code(),
            transcript,
        }
    }

    fn install_args<'a>(registry: &'a Path, project: &'a Path) -> Vec<&'a std::ffi::OsStr> {
        vec![
            "install".as_ref(),
            "org.example/hooked@=0.1.0".as_ref(),
            "--assume-yes".as_ref(),
            "--registry".as_ref(),
            registry.as_os_str(),
            "--path".as_ref(),
            project.as_os_str(),
        ]
    }

    #[test]
    fn yes_stops_with_130_and_restores_the_project() {
        use common::hook_slot::{Fixture, setup};
        let (outer, user, project, registry) = setup(Fixture::Timing);
        let package = registry.join("org.example/hooked/v0.1.0");
        hook(
            &package,
            &outer.path().join("ready"),
            &outer.path().join("go"),
        );
        let before = fs::read(project.path().join("vibe.toml")).unwrap();
        let isolation = Isolation::new(outer.path());
        let watched = watched(&isolation, &user, project.path());
        let baseline = watched.snapshot();
        let run = drive(
            &user,
            &isolation,
            outer.path(),
            "y",
            &install_args(&registry, project.path()),
        );
        assert_eq!(run.code, Some(130), "{}", run.transcript);
        assert!(
            run.transcript.contains("Stop the operation?"),
            "{}",
            run.transcript
        );
        assert_eq!(fs::read(project.path().join("vibe.toml")).unwrap(), before);
        assert!(
            !project
                .path()
                .join(common::slot_dir("org.example.hooked", "0.1.0"))
                .exists()
        );
        assert_clean(&watched, &baseline);
    }

    #[test]
    fn no_continues_the_operation_to_success() {
        use common::hook_slot::{Fixture, setup};
        let (outer, user, project, registry) = setup(Fixture::Timing);
        let package = registry.join("org.example/hooked/v0.1.0");
        hook(
            &package,
            &outer.path().join("ready"),
            &outer.path().join("go"),
        );
        let isolation = Isolation::new(outer.path());
        let run = drive(
            &user,
            &isolation,
            outer.path(),
            "n",
            &install_args(&registry, project.path()),
        );
        assert_eq!(run.code, Some(0), "{}", run.transcript);
        assert!(
            run.transcript.contains("Stop the operation?"),
            "{}",
            run.transcript
        );
        assert!(
            project
                .path()
                .join(common::slot_dir("org.example.hooked", "0.1.0"))
                .is_dir(),
            "{}",
            run.transcript
        );
        assert!(project.path().join("vibe.lock").is_file());
    }
}
