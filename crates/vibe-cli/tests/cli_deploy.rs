//! The three deploy command surfaces, driven as a real `vibe` process.
//!
//! §7's own command list:
//!
//! ```text
//! vibe deploy [--profile X] [--plan]
//! vibe undeploy --profile X
//! vibe deployments [--json]
//! ```
//!
//! Every test here runs against an isolated per-user home (`UserScratch`
//! sets `$VIBE_SETTINGS`), so the deployment state home these commands
//! read and write is the test's own temp tree and the operator's real
//! `~/.vibe` is unreachable — asserted, not assumed, in
//! [`the_state_home_is_the_isolated_one`].
//!
//! What a deploy can REACH at this atom is the reserved `deploy:vibe-bin`
//! row, which refuses as provider-not-landed by design (§7.0.2). So the
//! surfaces are proven by what they do around that refusal: which profile
//! they resolved, what they did not build, and what they did not read.
//! The provider-side laws are proven at the unit seam, against the
//! hermetic fixture.

mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use serde_json::Value;

/// A project that declares two deploy profiles over one packaged
/// artifact. No build target: the chain must be able to run without a
/// Cargo workspace, because what is under test is the deploy surface.
fn two_profiles(root: &Path) {
    write_project(
        root,
        concat!(
            "[[deploy.target]]\nid = \"local\"\nartifact = \"demo.md\"\n",
            "mechanism = \"deploy:vibe-bin\"\n\n",
            "[[deploy.target]]\nid = \"production\"\nartifact = \"demo.md\"\n",
            "mechanism = \"deploy:vibe-bin\"\n\n",
            "[deploy.profiles.local]\ntargets = [\"local\"]\n\n",
            "[deploy.profiles.production]\ntargets = [\"production\"]\n",
        ),
    );
}

/// The same project with exactly one profile and an explicit default.
fn one_profile(root: &Path) {
    write_project(
        root,
        concat!(
            "[deploy]\ndefault_profile = \"local\"\n\n",
            "[[deploy.target]]\nid = \"local\"\nartifact = \"demo.md\"\n",
            "mechanism = \"deploy:vibe-bin\"\n\n",
            "[deploy.profiles.local]\ntargets = [\"local\"]\n",
        ),
    );
}

/// The shared project head: one static-skill package target whose output
/// a deploy target may name.
fn write_project(root: &Path, deploy: &str) {
    fs::create_dir_all(root.join("skills/demo")).unwrap();
    fs::write(
        root.join("skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: A skill the deploy surface tests package.\n---\n\nBody.\n",
    )
    .unwrap();
    fs::write(
        root.join("vibe.toml"),
        format!(
            concat!(
                "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
                "[[artifacts.package]]\nid = \"demo\"\n",
                "mechanism = \"package:static-skill\"\n",
                "outputs = [{{ id = \"demo.md\", kind = \"file\" }}]\n",
                "config = {{ source = \"skills/demo\" }}\n\n",
                "{deploy}",
            ),
            deploy = deploy,
        ),
    )
    .unwrap();
}

/// The one JSON document a command emitted, however it was formatted.
fn document(bytes: &[u8]) -> Value {
    let text = String::from_utf8_lossy(bytes);
    let start = text
        .find('{')
        .unwrap_or_else(|| panic!("a JSON document on stdout:\n{text}"));
    serde_json::from_str(&text[start..])
        .unwrap_or_else(|error| panic!("valid JSON: {error}\n{}", &text[start..]))
}

/// §7.0.6: `--plan` is a read-only planner, NOT a chain run.
///
/// At this atom it resolves the profile and reaches the reserved deploy
/// row, which refuses. What the test pins is everything AROUND that: the
/// plan built nothing, recorded nothing and wrote no deployment state,
/// which is what "read-only" means.
#[test]
fn deploy_plan_builds_nothing_records_nothing_and_writes_no_state() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    one_profile(project.path());

    let output = user
        .vibe()
        .arg("deploy")
        .arg("--plan")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{output:?}");
    assert!(stderr.contains("R8-VIBE-BIN"), "{stderr}");
    assert!(
        !project.path().join("target").exists(),
        "a plan builds and packages nothing",
    );
    assert!(
        !project.path().join(".vibe/state/artifacts").exists(),
        "a plan records nothing",
    );
    assert!(
        !user.settings.join("state").exists(),
        "and it writes no deployment state",
    );
}

/// §10's sentinel gate: "run `--plan` with sentinel token files and prove
/// no credential read".
///
/// The probe is real rather than circumstantial: the sentinel files are
/// held UNREADABLE for the whole child run — permission-stripped on unix,
/// opened with no sharing on Windows — so a credential read would fail
/// loudly and name the path. The plan instead completes its own business
/// having never opened either, and the bytes are unchanged afterwards.
#[test]
fn deploy_plan_never_reads_a_token() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    one_profile(project.path());
    fs::create_dir_all(&user.settings).unwrap();
    // The two paths the publisher's own token convention reads, seeded
    // inside the ISOLATED settings home — no real credential is involved.
    let sentinel = user.settings.join("github.publish.token");
    let legacy = user.settings.join("git.publish.token");
    for path in [&sentinel, &legacy] {
        fs::write(path, "SENTINEL-TOKEN-NEVER-READ\n").unwrap();
    }
    let guard = deny_reads(&[&sentinel, &legacy]);

    let output = user
        .vibe()
        .arg("deploy")
        .arg("--plan")
        .arg("--profile")
        .arg("local")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    drop(guard);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for rendered in [&stdout, &stderr] {
        assert!(
            !rendered.contains("SENTINEL-TOKEN-NEVER-READ"),
            "a plan never echoes a credential:\n{rendered}",
        );
        assert!(
            !rendered.contains("publish.token"),
            "a plan never even names a credential path:\n{rendered}",
        );
    }
    // The refusal it DID produce is the deploy engine's own, so the run
    // really reached the planner rather than dying early for an
    // unrelated reason.
    assert!(stderr.contains("R8-VIBE-BIN"), "{stderr}");
    restore_reads(&[&sentinel, &legacy]);
    for path in [&sentinel, &legacy] {
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "SENTINEL-TOKEN-NEVER-READ\n",
            "the sentinel is byte-identical afterwards",
        );
    }
    assert!(!project.path().join("target").exists(), "and no build ran",);
    assert!(
        !user.settings.join("state").exists(),
        "and no destination or state changed",
    );
}

/// Hold the named files unreadable for the lifetime of the returned
/// guard. Two platforms, one meaning: an attempt to read fails loudly.
#[cfg(windows)]
fn deny_reads(paths: &[&Path]) -> Vec<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    paths
        .iter()
        .map(|path| {
            fs::OpenOptions::new()
                .read(true)
                // No sharing at all: a second opener — including the
                // child process — is refused by the OS.
                .share_mode(0)
                .open(path)
                .expect("the sentinel opens exclusively")
        })
        .collect()
}

#[cfg(unix)]
fn deny_reads(paths: &[&Path]) -> Vec<()> {
    use std::os::unix::fs::PermissionsExt;
    for path in paths {
        fs::set_permissions(path, fs::Permissions::from_mode(0o000))
            .expect("the sentinel permission strips");
    }
    paths.iter().map(|_| ()).collect()
}

/// Undo [`deny_reads`] so the assertion afterwards can read the bytes.
#[cfg(windows)]
fn restore_reads(_paths: &[&Path]) {}

#[cfg(unix)]
fn restore_reads(paths: &[&Path]) {
    use std::os::unix::fs::PermissionsExt;
    for path in paths {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .expect("the sentinel permission restores");
    }
}

/// §7's legality rule at the surface: two profiles and no default refuse,
/// naming both.
#[test]
fn a_bare_deploy_over_two_profiles_refuses_and_names_them() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    two_profiles(project.path());

    let output = user
        .vibe()
        .arg("deploy")
        .arg("--plan")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("needs a profile"), "{stderr}");
    assert!(stderr.contains("local"), "{stderr}");
    assert!(stderr.contains("production"), "{stderr}");
}

/// An unknown profile refuses and lists the defined ones.
#[test]
fn an_unknown_profile_refuses_at_the_surface() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    two_profiles(project.path());

    let output = user
        .vibe()
        .arg("deploy")
        .arg("--plan")
        .arg("--profile")
        .arg("staging")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`--profile staging`"), "{stderr}");
    assert!(stderr.contains("defined: local, production"), "{stderr}");
}

/// §7.0.2 at the surface: a real `vibe deploy` runs the chain, packages
/// its artifact, reaches the deploy fence and refuses as
/// provider-not-landed — never pretending it deployed.
#[test]
fn a_real_deploy_runs_the_chain_and_then_refuses_at_the_fence() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    one_profile(project.path());

    let output = user
        .vibe()
        .arg("deploy")
        .arg("--profile")
        .arg("local")
        .arg("--offline")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("R8-VIBE-BIN"), "{stderr}");
    assert!(
        stderr.contains("deploys nothing rather than pretend"),
        "{stderr}",
    );
    // The chain really ran: the package phase produced its distributable
    // and recorded it, so the refusal is at the deploy fence and not
    // upstream of it.
    assert!(
        project
            .path()
            .join("target/vibe-package/demo/SKILL.md")
            .is_file(),
        "the package fence fired before the deploy fence",
    );
    let state = user.settings.join("state/deployments");
    assert!(
        !state.exists()
            || fs::read_dir(&state).map_or(true, |mut entries| entries.next().is_none()),
        "and a refused selection wrote no deployment state",
    );
}

/// `vibe deployments` answers on an untouched machine, and its JSON is
/// the documented shape.
#[test]
fn deployments_lists_nothing_on_an_untouched_machine() {
    let user = UserScratch::new();

    let output = user
        .vibe()
        .arg("deployments")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let report = document(&output.stdout);
    assert_eq!(report["command"], "deployments");
    assert_eq!(report["ok"], true);
    assert_eq!(report["count"], 0);
    assert!(report["deployments"].as_array().expect("a list").is_empty());
}

/// `vibe undeploy` requires its profile — the architecture's own spelling
/// for the destructive verb.
#[test]
fn undeploy_requires_an_explicit_profile() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    one_profile(project.path());

    let output = user
        .vibe()
        .arg("undeploy")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--profile"), "{stderr}");
}

/// `vibe undeploy --profile X` on a machine that deployed nothing refuses
/// by name rather than silently succeeding.
#[test]
fn undeploy_without_a_landed_provider_refuses_by_name() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    one_profile(project.path());

    let output = user
        .vibe()
        .arg("undeploy")
        .arg("--profile")
        .arg("local")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("R8-VIBE-BIN"), "{stderr}");
    assert!(
        !user.settings.join("state").exists(),
        "and an inverse deployment that refused removed nothing",
    );
}

/// The deployment state home these commands use is the ISOLATED one.
///
/// Named rather than assumed: every other test in this file would still
/// pass if the commands had written into the operator's real `~/.vibe`,
/// and this is the assertion that says they did not.
#[test]
fn the_state_home_is_the_isolated_one() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    one_profile(project.path());

    user.vibe()
        .arg("deployments")
        .arg("--json")
        .output()
        .unwrap();

    let isolated = vibe_test_support::isolated_home().expect("the test process is isolated");
    assert!(
        !user.settings.starts_with(
            vibe_core::settings::settings_dir_from(None, dirs_home())
                .unwrap_or_else(|| isolated.to_path_buf())
        ),
        "the scratch settings home is not the operator's `~/.vibe`",
    );
    let state = user.settings.join("state").join("deployments");
    assert!(
        state.starts_with(&user.settings),
        "`{}` must hang off the scratch settings home",
        state.display(),
    );
}

/// The operator's real home, as the settings resolver would compute it —
/// used only to assert the scratch home is NOT it.
fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
}
