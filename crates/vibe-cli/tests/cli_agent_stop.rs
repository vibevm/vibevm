//! A stop acknowledged while a paid agent request is in flight.
//!
//! The provider is a loopback fixture that reads the request and never
//! answers, so the request is genuinely in flight when the stop file appears.
//! The install must exit 130 well inside the provider's hold, the provider
//! must see its connection closed and exactly one request (no retry), the
//! report must name the provider-side effect `unknown` — the request may have
//! been accepted and billed — and no declared output may be written.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::UserScratch;
use common::agent_provider::{MockProvider, configure_provider};

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

#[test]
fn stop_during_an_in_flight_agent_request_closes_it_and_reports_an_unknown_effect() {
    if !common::git_available() {
        eprintln!("skipping in-flight agent stop e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::holding();
    let outer = tempfile::tempdir().unwrap();
    let published =
        common::hosted_slot::publish_slot_agent(outer.path(), "slot:pre-install", "0.1.0");
    let user = UserScratch::new();
    let project = common::hosted_slot::project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    let before = snapshot(project.path());
    let stop = outer.path().join("stop");

    let stopped_at = std::sync::Mutex::new(None::<Instant>);
    let output = std::thread::scope(|scope| {
        scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(120);
            while provider.hits() == 0 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            // Let the request settle into its held wait before stopping.
            std::thread::sleep(Duration::from_millis(200));
            fs::write(&stop, "").unwrap();
            *stopped_at.lock().unwrap() = Some(Instant::now());
        });
        user.vibe()
            .args([
                "install",
                "org.demo/tools",
                "--json",
                "--assume-yes",
                "--agent-mode",
                "cli",
                "--stop-file",
            ])
            .arg(&stop)
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap()
    });
    let returned = Instant::now();
    let stopped_at = stopped_at
        .into_inner()
        .unwrap()
        .expect("the provider received the paid request");
    let latency = returned.duration_since(stopped_at);

    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        latency < Duration::from_secs(2),
        "the in-flight request was not interrupted promptly: {latency:?}"
    );
    let report = common::hook_slot::documents(&output.stdout).pop().unwrap();
    assert_eq!(report["outcome"], "cancelled");
    assert_eq!(report["external_effects"]["llm"], "unknown", "{report}");
    assert_eq!(report["publication"], "restored", "{report}");
    let typed: vibe_wire::generated::project_cancellation::ProjectCancellation =
        serde_json::from_value(report.clone()).unwrap();
    assert_eq!(
        typed.external_effects.llm,
        Some(vibe_wire::generated::project_cancellation::Effect::Unknown)
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    while !provider.closed_by_client() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        provider.closed_by_client(),
        "the held request was closed by the client"
    );
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(provider.hits(), 1, "a paid request is never retried");
    assert!(!project.path().join("docs/slot.md").exists());
    assert_eq!(
        snapshot(project.path()),
        before,
        "the stopped install changed the project"
    );
}

fn is_stop_control(path: &Path, _: Option<&common::leftovers::Entry>) -> bool {
    path == Path::new("stop")
}

fn is_run_record(path: &Path, after: Option<&common::leftovers::Entry>) -> bool {
    path == Path::new(".vibe/lifecycle.toml") && after.is_some()
}

fn is_empty_lease_lock(path: &Path, after: Option<&common::leftovers::Entry>) -> bool {
    path == Path::new(".vibe/lifecycle.lock")
        && matches!(after, Some(common::leftovers::Entry::File(bytes)) if bytes.is_empty())
}

fn is_store_object(path: &Path, after: Option<&common::leftovers::Entry>) -> bool {
    after.is_some()
        && (path.starts_with("cache")
            || path == Path::new(".native")
            || path.starts_with(".native/metadata-store"))
}

/// The Git registry's per-package clone cache, `<registry-key>/packages/
/// <group>.<name>/clone` and the directories holding it: the acquisition
/// completed (the install was already materialising), so it is a completed,
/// reusable cache object, not operation garbage.
fn is_registry_clone_cache(path: &Path, after: Option<&common::leftovers::Entry>) -> bool {
    let parts: Vec<_> = path.components().collect();
    after.is_some()
        && match parts.as_slice() {
            [_] => true,
            [_, packages, rest @ ..] if packages.as_os_str() == "packages" => match rest {
                [] | [_] => true,
                [_, clone, ..] => clone.as_os_str() == "clone",
            },
            _ => false,
        }
}

/// What the stopped install may leave changed; nothing else.
const ALLOWED: &[common::leftovers::Allowed] = &[
    common::leftovers::Allowed {
        root: "registry-cache",
        matches: is_registry_clone_cache,
        why: "<registry>/packages/<group>.<name>/clone — the completed Git registry clone cache (PROP-073 #PROJECT-COOPERATIVE-STOP: completed cache objects may remain reusable)",
    },
    common::leftovers::Allowed {
        root: "outer",
        matches: is_stop_control,
        why: "stop — the caller-owned stop file (PROP-073 #PROJECT-COOPERATIVE-STOP)",
    },
    common::leftovers::Allowed {
        root: "project",
        matches: is_run_record,
        why: ".vibe/lifecycle.toml — the durable run record (PROP-054 #OBS-RUN-RECORD)",
    },
    common::leftovers::Allowed {
        root: "project",
        matches: is_empty_lease_lock,
        why: ".vibe/lifecycle.lock, empty — the lease lock (PROP-072 #SOURCE-INSTALL-BOUNDARY)",
    },
    common::leftovers::Allowed {
        root: "settings",
        matches: is_store_object,
        why: "a complete store object (PROP-073 #PROJECT-COOPERATIVE-STOP)",
    },
    common::leftovers::Allowed {
        root: "registry-cache",
        matches: is_store_object,
        why: "a complete metadata object (PROP-073 #PROJECT-COOPERATIVE-STOP)",
    },
];

/// An agent step that COMPLETED earlier in an install is part of the
/// operation's pre-commit rollback: a later stop — here, while the second
/// agent step's paid request is in flight — puts the first step's published
/// output back together with the slots and boot files, and the install
/// reports `restored`, not `recovery-required`.
#[test]
fn a_later_stop_restores_the_outputs_of_a_completed_agent_step() {
    if !common::git_available() {
        eprintln!("skipping completed agent stop e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::answering_then_holding(common::hosted_slot::PAID_RESULT, 1);
    let outer = tempfile::tempdir().unwrap();
    let published =
        common::hosted_slot::publish_two_slot_agents(outer.path(), "slot:pre-install", "0.1.0");
    let user = UserScratch::new();
    let project = common::hosted_slot::project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    let isolation = common::leftovers::Isolation::new(outer.path());
    let control = outer.path().join("control");
    fs::create_dir_all(&control).unwrap();
    let watched = isolation
        .watched(&user)
        .root("project", project.path())
        .root("outer", &control);
    let before = watched.snapshot();
    let stop = control.join("stop");

    let first_output = project.path().join("docs/slot.md");
    let saw_first_output = std::sync::atomic::AtomicBool::new(false);
    let output = std::thread::scope(|scope| {
        scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(120);
            while provider.hits() < 2 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            // The first step answered and published before the second asked.
            saw_first_output.store(first_output.is_file(), std::sync::atomic::Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(200));
            fs::write(&stop, "").unwrap();
        });
        let mut command = user.vibe();
        isolation.apply(&mut command);
        command
            .args([
                "install",
                "org.demo/tools",
                "--json",
                "--assume-yes",
                "--agent-mode",
                "cli",
                "--stop-file",
            ])
            .arg(&stop)
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap()
    });

    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        saw_first_output.load(std::sync::atomic::Ordering::SeqCst),
        "the first agent step completed and published its output before the stop",
    );
    assert_eq!(provider.hits(), 2, "one answered request, one interrupted");
    let report = common::hook_slot::documents(&output.stdout).pop().unwrap();
    assert_eq!(report["outcome"], "cancelled", "{report}");
    assert_eq!(
        report["publication"], "restored",
        "the completed step's output is owned by the rollback: {report}"
    );
    assert_eq!(report["external_effects"]["llm"], "unknown", "{report}");
    assert!(
        !first_output.exists(),
        "the completed step's output was rolled back"
    );
    assert!(!project.path().join("docs/slot-second.md").exists());
    common::leftovers::assert_no_leftovers(&before, &watched.snapshot(), ALLOWED);
}

/// A completed step's output that something else replaced before the later
/// stop is foreign state: it is preserved, named, and the install reports
/// `recovery-required`.
#[test]
fn a_later_stop_preserves_a_replaced_completed_output_and_reports_recovery() {
    if !common::git_available() {
        eprintln!("skipping completed agent stop e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::answering_then_holding(common::hosted_slot::PAID_RESULT, 1);
    let outer = tempfile::tempdir().unwrap();
    let published =
        common::hosted_slot::publish_two_slot_agents(outer.path(), "slot:pre-install", "0.1.0");
    let user = UserScratch::new();
    let project = common::hosted_slot::project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    let stop = outer.path().join("stop");
    let first_output = project.path().join("docs/slot.md");

    let output = std::thread::scope(|scope| {
        scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(120);
            while provider.hits() < 2 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            // Someone else replaces the completed output before the stop.
            fs::remove_file(&first_output).unwrap();
            fs::write(&first_output, "foreign body\n").unwrap();
            std::thread::sleep(Duration::from_millis(200));
            fs::write(&stop, "").unwrap();
        });
        user.vibe()
            .args([
                "install",
                "org.demo/tools",
                "--json",
                "--assume-yes",
                "--agent-mode",
                "cli",
                "--stop-file",
            ])
            .arg(&stop)
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap()
    });

    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = common::hook_slot::documents(&output.stdout).pop().unwrap();
    assert_eq!(report["outcome"], "cancelled", "{report}");
    assert_eq!(report["publication"], "recovery-required", "{report}");
    let error = report["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("docs/slot.md") && error.contains("preserved"),
        "the preserved foreign replacement is named: {report}"
    );
    assert_eq!(
        fs::read_to_string(&first_output).unwrap(),
        "foreign body\n",
        "the foreign replacement is left untouched"
    );
}
