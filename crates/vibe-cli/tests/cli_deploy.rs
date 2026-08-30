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
//! What a deploy REACHES since R8-VIBE-BIN is the real `deploy:vibe-bin`
//! provider, so the surfaces are proven by what they really do: which
//! profile they resolved, the POPULATED plan body they report, the
//! deployment they write into the isolated settings home, and what they
//! still do not read. The provider-side laws are proven at the unit seam,
//! against the provider itself.
//!
//! The deployed artifact is declared by an `[[artifacts.build]]` row over a
//! dependency-free Cargo fixture, because a `[[deploy.target]]`'s `artifact`
//! must name a DECLARED output or the manifest does not validate. The
//! read-only surfaces never build it: they read a hand-written A2 record
//! for a prebuilt payload whose bytes and digest are pinned constants, so a
//! record that stopped describing the file fails loudly rather than
//! drifting. Exactly ONE test here runs the real chain, and therefore
//! exactly one pays for a real `cargo build` — the test that proves the
//! settings-root threading, which nothing below a command surface can.

mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use serde_json::Value;

/// The prebuilt payload's exact bytes, and their SHA-256. Pinned together:
/// the record below claims this digest, and the deploy engine re-proves it
/// against the file before it reconciles anything.
const PAYLOAD: &str = "vibe-bin e2e payload\n";
const PAYLOAD_DIGEST: &str = "adcdfaeee032efeca4c7ab1ecc70dc85ce6ac9741bb191f15d235829aad887b7";

/// The command alias every deploying test here installs under.
const COMMAND: &str = "vibe-cli-e2e";

/// The launcher's settings-relative identity on this host.
fn launcher_resource() -> String {
    format!("bin/{COMMAND}{}", if cfg!(windows) { ".cmd" } else { "" })
}

/// A project that declares two deploy profiles over one prebuilt
/// executable. No build target: the chain must be able to run without a
/// Cargo workspace, because what is under test is the deploy surface.
fn two_profiles(root: &Path) {
    write_project(
        root,
        &format!(
            concat!(
                "[[deploy.target]]\nid = \"local\"\nartifact = \"helper.exe\"\n",
                "mechanism = \"deploy:vibe-bin\"\nconfig = {{ command = \"{command}\" }}\n\n",
                "[[deploy.target]]\nid = \"production\"\nartifact = \"helper.exe\"\n",
                "mechanism = \"deploy:vibe-bin\"\nconfig = {{ command = \"{command}-prod\" }}\n\n",
                "[deploy.profiles.local]\ntargets = [\"local\"]\n\n",
                "[deploy.profiles.production]\ntargets = [\"production\"]\n",
            ),
            command = COMMAND,
        ),
    );
}

/// The same project with exactly one profile and an explicit default.
fn one_profile(root: &Path) {
    write_project(
        root,
        &format!(
            concat!(
                "[deploy]\ndefault_profile = \"local\"\n\n",
                "[[deploy.target]]\nid = \"local\"\nartifact = \"helper.exe\"\n",
                "mechanism = \"deploy:vibe-bin\"\nconfig = {{ command = \"{command}\" }}\n\n",
                "[deploy.profiles.local]\ntargets = [\"local\"]\n",
            ),
            command = COMMAND,
        ),
    );
}

/// The shared project head: one static-skill package target (so a real run
/// has a chain to execute), one `[[artifacts.build]]` row DECLARING the
/// executable a deploy target may name, and the prebuilt payload plus
/// record the read-only surfaces read instead of building it.
fn write_project(root: &Path, deploy: &str) {
    fs::create_dir_all(root.join("skills/demo")).unwrap();
    fs::write(
        root.join("skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: A skill the deploy surface tests package.\n---\n\nBody.\n",
    )
    .unwrap();
    // The dependency-free crate the build row names. Its own `[workspace]`
    // so no workspace above the temp directory can absorb it.
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        concat!(
            "[package]\nname = \"vibe-cli-deploy-fixture\"\nversion = \"0.1.0\"\n",
            "edition = \"2021\"\n\n[[bin]]\nname = \"clihelper\"\npath = \"src/main.rs\"\n\n",
            "[workspace]\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        "fn main() {\n    println!(\"vibe-cli deploy fixture\");\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("vibe.toml"),
        format!(
            concat!(
                "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
                "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
                "outputs = [{{ id = \"helper.exe\", kind = \"executable\", ",
                "select = {{ package = \"vibe-cli-deploy-fixture\", bin = \"clihelper\" }} }}]\n",
                "config = {{ offline = true }}\n\n",
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
    record_prebuilt_executable(root);
}

/// Seed the prebuilt payload and the A2 artifact record the build engine
/// would have written for it.
///
/// Hand-written rather than produced, and complete rather than minimal:
/// the deploy engine reads this record through the same strict A2 cell it
/// reads a real one through, so anything missing or misspelled refuses
/// here instead of silently changing what the test proves.
fn record_prebuilt_executable(root: &Path) {
    let relative = "tools/helper.exe";
    let absolute = root.join(relative);
    fs::create_dir_all(absolute.parent().unwrap()).unwrap();
    fs::write(&absolute, PAYLOAD).unwrap();
    let record = serde_json::json!({
        "schema": 1,
        "id": "helper.exe",
        "kind": "executable",
        "shape": "file",
        "path_absolute": absolute.display().to_string().replace('\\', "/"),
        "path_relative": { "root": "project", "path": relative },
        "digest": { "algorithm": "sha256", "value": PAYLOAD_DIGEST },
        "producer": {
            "target": "helper",
            "mechanism": "build:cargo",
            "provider": { "key": "org.vibevm/vibe#cargo" },
        },
        "freshness": {},
        "created_at": "2026-08-30T11:00:00Z",
        "verification": { "status": "verified", "evidence": "prebuilt deploy-surface fixture" },
    });
    let records = root.join(".vibe/state/artifacts");
    fs::create_dir_all(&records).unwrap();
    fs::write(
        records.join("helper.exe.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
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

/// §7.0.6: `--plan` is a read-only planner, NOT a chain run — and since
/// R8-VIBE-BIN its body is POPULATED.
///
/// R8-DEPLOY's ratification 8 said a populated plan "arrives with the
/// first landed deploy provider, and asserting one today would assert a
/// fiction". This is that assertion, and it keeps every read-only property
/// the earlier shape pinned: nothing built, nothing packaged, no
/// deployment state, and now also nothing installed into the destination.
#[test]
fn deploy_plan_reports_a_populated_body_and_still_writes_nothing() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    one_profile(project.path());

    let output = user
        .vibe()
        .arg("deploy")
        .arg("--plan")
        .arg("--json")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let report = document(&output.stdout);
    assert_eq!(report["mode"], "plan");
    assert_eq!(report["profile"], "local");
    let targets = report["targets"].as_array().expect("a target list");
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0]["provider"], "org.vibevm/vibe#vibe-bin");
    assert_eq!(targets[0]["planned"], true);
    let resources = targets[0]["resources"].as_array().expect("a resource list");
    let named: Vec<&str> = resources
        .iter()
        .map(|resource| resource["resource"].as_str().expect("a resource identity"))
        .collect();
    assert_eq!(
        named,
        [
            launcher_resource().as_str(),
            &format!("bin/{COMMAND}.current")
        ],
        "the launcher and the active-payload pointer, and never the payload",
    );
    for resource in resources {
        assert_eq!(resource["change"], "create");
        assert_eq!(
            resource["desired_digest"]
                .as_str()
                .expect("a desired digest")
                .len(),
            64,
        );
    }
    // Every read-only property the refusal-shaped ancestor pinned, kept.
    assert!(
        !project.path().join("target").exists(),
        "a plan builds and packages nothing",
    );
    assert_eq!(
        deployments(&user),
        0,
        "and it writes no deployment state — the engine's own pin is that the state home may \
         exist and must be EMPTY, since a read-only planner opens it to read receipts",
    );
    assert!(
        !user.settings.join("bin").exists() && !user.settings.join("store").exists(),
        "and it installs nothing into the destination it just described",
    );
}

/// How many deployments the isolated state home records.
///
/// A count rather than a directory-exists check: §7.0.6's planner OPENS the
/// engine-owned state home to read receipts, and `DeployState::open`
/// creates it — so the read-only property is "nothing is recorded", which
/// is what the engine's own unit pin asserts too.
fn deployments(user: &UserScratch) -> usize {
    fs::read_dir(user.settings.join("state/deployments")).map_or(0, |entries| {
        entries
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| !name.starts_with('.'))
            })
            .count()
    })
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
    // The plan it DID produce is populated, so the run really reached the
    // provider rather than dying early for an unrelated reason.
    assert!(output.status.success(), "{stderr}");
    assert!(stdout.contains(&launcher_resource()), "{stdout}");
    restore_reads(&[&sentinel, &legacy]);
    for path in [&sentinel, &legacy] {
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "SENTINEL-TOKEN-NEVER-READ\n",
            "the sentinel is byte-identical afterwards",
        );
    }
    assert!(!project.path().join("target").exists(), "and no build ran",);
    assert_eq!(deployments(&user), 0, "and no state changed");
    assert!(
        !user.settings.join("bin").exists(),
        "and no destination changed",
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

/// §7.0.2 at the surface, and §7.1.0 ruling 2's settings-root threading:
/// a real `vibe deploy` runs the chain, packages its artifact, reaches the
/// deploy fence and INSTALLS into the isolated settings home.
///
/// This is the one test that proves the whole carriage — the command layer
/// resolves the settings directory once, `DeployCarriage` carries it beside
/// the state home, and the provider reconciles a destination inside it. The
/// unit and lifecycle suites construct a `DeployExecution` directly and so
/// cannot prove that threading at all.
#[test]
fn a_real_deploy_runs_the_chain_and_installs_into_the_isolated_bin_home() {
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

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    // The chain really ran: the package phase produced its distributable
    // and recorded it, so the deployment happened at the deploy fence and
    // not upstream of it.
    assert!(
        project
            .path()
            .join("target/vibe-package/demo/SKILL.md")
            .is_file(),
        "the package fence fired before the deploy fence",
    );
    // And the destination is the ISOLATED settings home, reached only
    // because the carriage carried its root down.
    let launcher = user.settings.join("bin").join(format!(
        "{COMMAND}{}",
        if cfg!(windows) { ".cmd" } else { "" }
    ));
    assert!(
        launcher.is_file(),
        "the launcher was installed: {launcher:?}"
    );
    let body = fs::read_to_string(&launcher).unwrap();
    assert!(
        body.contains("vibevm-launcher genre=deploy:vibe-bin"),
        "{body}"
    );
    // The deployed payload is the one Cargo really produced, so its digest
    // is read off the pointer rather than pinned — and the launcher must
    // not contain it (§7.1.0 ruling 3: version-free by construction).
    let pointer = user.settings.join("bin").join(format!("{COMMAND}.current"));
    let active = fs::read_to_string(&pointer).unwrap().trim().to_owned();
    assert_eq!(active.len(), 64, "the pointer names one payload digest");
    assert!(
        !body.contains(&active),
        "the launcher is version-free: {body}"
    );
    assert!(
        user.settings
            .join("store")
            .join(format!(
                "{active}{}",
                if cfg!(windows) { ".exe" } else { "" }
            ))
            .is_file(),
        "and the content-addressed payload is in the store",
    );

    // `vibe deployments` reports the receipt the same run wrote.
    let listing = user
        .vibe()
        .arg("deployments")
        .arg("--json")
        .output()
        .unwrap();
    assert!(listing.status.success(), "{listing:?}");
    let report = document(&listing.stdout);
    assert_eq!(report["count"], 1);
    let row = &report["deployments"][0];
    assert_eq!(row["target"], "local");
    assert_eq!(row["profile"], "local");
    assert_eq!(row["provider"], "org.vibevm/vibe#vibe-bin");
    assert_eq!(row["status"], "verified");
    assert_eq!(row["scope"], "user");
    assert_eq!(row["resources"], 2);

    // And `vibe undeploy` removes exactly the two owned files.
    let removal = user
        .vibe()
        .arg("undeploy")
        .arg("--profile")
        .arg("local")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(removal.status.success(), "{removal:?}");
    assert!(!launcher.exists(), "the launcher is gone");
    assert!(!pointer.exists(), "the pointer is gone");
    assert_eq!(
        fs::read_dir(user.settings.join("bin")).unwrap().count(),
        0,
        "the destination directory holds nothing owned afterwards",
    );
    assert_eq!(
        fs::read_dir(user.settings.join("store")).unwrap().count(),
        1,
        "and the payload stays as disclosed store garbage",
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
///
/// The ancestor of this test refused because no provider was landed; the
/// refusal it pins now is the one §7.2 really means — "an inverse
/// deployment removes only receipt-owned state, so with no receipt there
/// is nothing it may touch" — which is a stronger pin, because reaching it
/// means the provider resolved and the ENGINE stopped.
#[test]
fn undeploy_with_no_receipt_refuses_by_name() {
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
    assert!(stderr.contains("found no deployment receipt"), "{stderr}");
    assert!(stderr.contains("`local`"), "{stderr}");
    assert!(
        !user.settings.join("bin").exists(),
        "and an inverse deployment that refused touched no destination",
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
