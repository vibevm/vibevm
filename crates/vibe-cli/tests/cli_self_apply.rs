//! `vibe self apply` (PROP-019 `#site-profile`) through the real CLI in an
//! isolated settings home, install root and HOME: dry-run / diff / apply /
//! idempotent re-apply / show / remove, a refused profile writing nothing,
//! the site layer's precedence (user beats site, site beats the built-in
//! defaults), the consented PATH step, and a closed-network machine set up
//! from one profile with zero non-loopback connection attempts.

#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;
use serde_json::Value;
use vibe_wire::generated::self_apply_report::{
    ApplyAction, PathPlan, RegistryScope, SelfApplyReport, SkipReason,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One isolated machine: settings home, install root, HOME.
struct Machine {
    user: UserScratch,
    base: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Machine {
    fn new() -> Self {
        Machine {
            user: UserScratch::new(),
            base: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        }
    }

    fn cmd(&self) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        command
            .env("VIBEVM_INSTALL_ROOT", self.base.path())
            .env("HOME", self.home.path())
            .env("SHELL", "/bin/bash")
            .env("PATH", "/usr/bin:/bin")
            .env_remove("VIBEVM_HOME")
            .env_remove("VIBEVM_SHELL_HOME")
            .env_remove("VIBE_UNATTENDED")
            .env_remove("VIBEVM_RELEASE_ORIGIN")
            .env_remove("VIBEVM_USER_CONFIG")
            .env_remove("VIBE_OFFLINE")
            // `pac = "system"` is resolved on every host now
            // (PROP-068 `#proxy-system`), and its lowest source is the
            // environment: an exported `https_proxy` of the developer
            // who runs the suite would otherwise answer, and so would a
            // fixture left over from another test.
            .env_remove("VIBEVM_SYSTEM_PROXY_FIXTURE")
            .env_remove("VIBEVM_SYSTEM_PROXY_OS");
        for name in [
            "https_proxy",
            "HTTPS_PROXY",
            "http_proxy",
            "HTTP_PROXY",
            "all_proxy",
            "ALL_PROXY",
            "no_proxy",
            "NO_PROXY",
        ] {
            command.env_remove(name);
        }
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.cmd().args(args).output().unwrap()
    }

    /// `run`, with the proxy-configuration seam of PROP-068
    /// `#proxy-system` pointed at a fixture directory: `<dir>/gsettings`,
    /// `<dir>/kioslaverc`, `<dir>/scutil-proxy`, `<dir>/dns-domains` and
    /// `<dir>/wpad` stand in for this machine's own desktop, and
    /// `kind` names the resolution order to run.
    fn run_as_host(&self, args: &[&str], kind: &str, fixture: &Path) -> Output {
        self.cmd()
            .env("VIBEVM_SYSTEM_PROXY_OS", kind)
            .env("VIBEVM_SYSTEM_PROXY_FIXTURE", fixture)
            .args(args)
            .output()
            .unwrap()
    }

    fn apply(&self, profile: &Path, extra: &[&str]) -> Value {
        let mut command = self.cmd();
        command
            .args(["--json", "--unattended", "self", "apply"])
            .arg(profile)
            .args(extra);
        json(&command.output().unwrap())
    }

    /// A tiny active instance, so `apply` does not import the (large)
    /// test binary itself.
    fn import_active(&self) {
        let payload = self.base.path().join("ready-vibe");
        fs::write(&payload, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&payload, fs::Permissions::from_mode(0o755)).unwrap();
        let mut command = self.cmd();
        command
            .args(["--json", "--unattended", "self", "import"])
            .arg(&payload)
            .args(["--tag", "1.2.3", "--use"]);
        ok(&command.output().unwrap());
    }

    fn settings(&self, name: &str) -> PathBuf {
        self.user.settings.join(name)
    }

    fn shim_dir(&self) -> PathBuf {
        self.base.path().join("opt").join("bin")
    }

    fn bashrc(&self) -> Option<String> {
        fs::read_to_string(self.home.path().join(".bashrc")).ok()
    }
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Every registered `--json` document these tests read goes through its
/// generated reader (`common::wire`) before anything is asserted about it.
fn json(output: &Output) -> Value {
    common::wire::checked(&ok(output))
}

/// A site profile for directories under `root`.
fn corp_profile(root: &Path, path_fix: Option<&str>) -> String {
    let path = path_fix
        .map(|fix| format!("\n[path]\nfix = \"{fix}\"\n"))
        .unwrap_or_default();
    format!(
        r#"# corp profile — written by the administrator, kept verbatim
schema = 1
name = "corp"

[[registry]]
name = "corp"
type = "LocalDir"
url = "file://{root}/registry"

[source_policy]
exclude_hosting = ["github", "gitverse"]

[artifacts]
vibe = "{root}/releases"
packages = "{root}/payloads"

[self]
source = "localdir"
{path}"#,
        root = root.display()
    )
}

fn write_profile(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("corp.toml");
    fs::write(&path, body).unwrap();
    path
}

#[test]
fn dry_run_diff_apply_reapply_show_and_remove() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    for dir in ["registry", "releases", "payloads"] {
        fs::create_dir_all(site.path().join(dir)).unwrap();
    }
    // The user's own files, which no step may change.
    let user_config = "# mine\n[init]\nlast_author = 'Me <me@example.com>'\n";
    let user_registry =
        "# my registries\n[[registry]]\nname = 'mine'\nurl = 'file:///home/me/reg'\n";
    fs::write(machine.settings("config.toml"), user_config).unwrap();
    fs::write(machine.settings("registry.toml"), user_registry).unwrap();
    let body = corp_profile(site.path(), Some("never"));
    let profile = write_profile(site.path(), &body);
    let stored = machine.settings("site-profile.toml");
    let receipt = machine.settings("site-profile.receipt.toml");

    // --dry-run reports, writes nothing.
    let planned = machine.apply(&profile, &["--dry-run"]);
    assert_eq!(planned["protocol"], "vibe-self-apply/1");
    assert_eq!(planned["action"], "dry-run");
    assert_eq!(planned["changed"], true);
    assert_eq!(planned["files"][0]["change"], "create");
    assert_eq!(planned["path"]["planned"], "would_skip");
    assert!(!stored.exists() && !receipt.exists());

    // --diff prints each file's lines, writes nothing.
    let mut diff = machine.cmd();
    diff.args(["--unattended", "self", "apply"])
        .arg(&profile)
        .arg("--diff");
    let text = ok(&diff.output().unwrap());
    assert!(
        text.contains("+# corp profile — written by the administrator"),
        "{text}"
    );
    assert!(text.contains("+schema = 1"), "{text}");
    assert!(!stored.exists());

    // Apply: stored verbatim (comments and all), receipt beside it.
    let applied = machine.apply(&profile, &[]);
    assert_eq!(applied["action"], "apply");
    assert_eq!(applied["changed"], true);
    assert_eq!(fs::read_to_string(&stored).unwrap(), body);
    let receipt_text = fs::read_to_string(&receipt).unwrap();
    assert!(
        receipt_text.contains(&profile.display().to_string()),
        "{receipt_text}"
    );
    let effective = &applied["effective"];
    let registries = effective["registries"].as_array().unwrap();
    assert_eq!(registries[0]["name"], "mine");
    assert_eq!(registries[0]["scope"], "user");
    assert_eq!(registries[1]["name"], "corp");
    assert_eq!(registries[1]["type"], "localdir");
    assert_eq!(registries[1]["scope"], "site");
    assert_eq!(effective["release"]["source"], "localdir");
    assert!(
        effective["release"]["from"]
            .as_str()
            .unwrap()
            .contains("site-profile.toml"),
        "{effective}"
    );
    assert_eq!(
        effective["exclude_hosting"],
        serde_json::json!(["github", "gitverse"])
    );

    // Where to put things: this version's and the next one's release
    // directory with this platform's file names, and the payload layout.
    let releases = applied["where"]["releases"].as_array().unwrap();
    assert_eq!(releases.len(), 2);
    assert_eq!(releases[0]["version"], VERSION);
    let release_dir = site.path().join("releases").join(format!("v{VERSION}"));
    assert_eq!(
        releases[0]["example_dir"],
        release_dir.display().to_string()
    );
    let files: Vec<&str> = releases[0]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file.as_str().unwrap())
        .collect();
    assert!(files.contains(&"DISTRIBUTIONS.json"), "{files:?}");
    assert!(
        files
            .iter()
            .any(|file| file.starts_with(&format!("vibevm-{VERSION}-")) && file.ends_with(".zip")),
        "{files:?}"
    );
    let mut next = semver::Version::parse(VERSION).unwrap();
    next.patch += 1;
    next.pre = semver::Prerelease::EMPTY;
    assert_eq!(releases[1]["version"], next.to_string());
    let packages = &applied["where"]["packages"];
    assert_eq!(
        packages["dir"],
        site.path().join("payloads").display().to_string()
    );
    assert!(
        packages["example"][0]
            .as_str()
            .unwrap()
            .starts_with(&format!(
                "{}/org.example/tool/1.2.0/tool-1.2.0-",
                site.path().join("payloads").display()
            )),
        "{packages}"
    );
    assert_eq!(applied["path"]["outcome"], "skipped");
    assert_eq!(applied["path"]["skip_reason"], "profile_never");
    assert!(
        applied["next_steps"][0]
            .as_str()
            .unwrap()
            .contains("vibe update --sources-only")
    );

    // Re-applying the same bytes changes nothing, receipt included.
    let receipt_before = fs::read(&receipt).unwrap();
    let again = machine.apply(&profile, &[]);
    assert_eq!(again["changed"], false);
    assert!(
        again["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|file| file["change"] == "unchanged")
    );
    assert_eq!(fs::read(&receipt).unwrap(), receipt_before);

    // The human report says where things go.
    let mut human = machine.cmd();
    human.args(["--unattended", "self", "apply"]).arg(&profile);
    let text = ok(&human.output().unwrap());
    assert!(
        text.contains(&format!("{}/", release_dir.display())),
        "{text}"
    );
    assert!(text.contains("put package payloads into"), "{text}");

    // --show reads the installation back.
    let shown = json(&machine.run(&["--json", "self", "apply", "--show"]));
    assert_eq!(shown["action"], "show");
    assert_eq!(shown["profile"]["name"], "corp");
    assert_eq!(shown["effective"]["release"]["source"], "localdir");

    // An edited profile replaces the stored one.
    let edited = body.replace("name = \"corp\"", "name = \"corp-2\"");
    fs::write(&profile, &edited).unwrap();
    let updated = machine.apply(&profile, &["--diff"]);
    assert_eq!(updated["files"][0]["change"], "update");
    assert!(
        updated["files"][0]["diff"]
            .as_str()
            .unwrap()
            .contains("+name = \"corp-2\"")
    );
    assert_eq!(
        fs::read_to_string(&stored).unwrap(),
        body,
        "--diff writes nothing"
    );
    machine.apply(&profile, &[]);
    assert_eq!(fs::read_to_string(&stored).unwrap(), edited);

    // --remove deletes exactly what apply owns; the user's files survive.
    let removed = json(&machine.run(&["--json", "self", "apply", "--remove"]));
    assert_eq!(removed["action"], "remove");
    assert_eq!(removed["changed"], true);
    assert_eq!(removed["files"].as_array().unwrap().len(), 2);
    assert!(!stored.exists() && !receipt.exists());
    assert_eq!(
        fs::read_to_string(machine.settings("config.toml")).unwrap(),
        user_config
    );
    assert_eq!(
        fs::read_to_string(machine.settings("registry.toml")).unwrap(),
        user_registry
    );
    let nothing = json(&machine.run(&["--json", "self", "apply", "--remove"]));
    assert_eq!(nothing["changed"], false);
}

#[test]
fn an_invalid_profile_is_refused_naming_file_key_and_fix_and_writes_nothing() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    let body = corp_profile(site.path(), None).replace(
        &format!("vibe = \"{}/releases\"", site.path().display()),
        "vibe = \"relative/releases\"",
    );
    let profile = write_profile(site.path(), &body);
    let mut command = machine.cmd();
    command
        .args(["--unattended", "self", "apply"])
        .arg(&profile)
        .arg("--fix-path");
    let output = command.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&profile.display().to_string()), "{stderr}");
    assert!(stderr.contains("artifacts.vibe"), "{stderr}");
    // The refusal renderer lays the fix out as its own labelled line.
    assert!(
        stderr.contains("fix") && stderr.contains("give an absolute directory"),
        "{stderr}"
    );
    assert!(
        stderr.contains("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile"),
        "{stderr}"
    );
    assert!(!machine.settings("site-profile.toml").exists());
    assert!(!machine.settings("site-profile.receipt.toml").exists());
    assert_eq!(machine.bashrc(), None, "PATH untouched on refusal");
}

/// A profile written for Windows desktops validates on a Linux box: a UNC
/// release drop is absolute (not "relative") and is accepted with a
/// warning that only a Windows host can read it (PROP-019
/// `#site-profile`). `[proxy] pac = "system"` carries no such warning any
/// more — it is resolved on every host (PROP-068 `#proxy-system`).
#[test]
fn a_windows_profile_validates_here_with_windows_only_warnings() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    let body = corp_profile(site.path(), None).replace(
        &format!("vibe = \"{}/releases\"", site.path().display()),
        "vibe = '\\\\fileserver\\vibe\\releases'",
    ) + "\n[proxy]\nmode = \"px\"\npac = \"system\"\n";
    let profile = write_profile(site.path(), &body);
    let output = machine
        .cmd()
        .args(["--unattended", "self", "apply", "--dry-run"])
        .arg(&profile)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{stderr}");
    assert!(!stderr.contains("relative"), "{stderr}");
    assert!(
        stderr.contains("artifacts.vibe") && stderr.contains("only a Windows host reads it"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("works only on Windows"),
        "`pac = \"system\"` is resolved on this host too: {stderr}"
    );
    assert!(!machine.settings("site-profile.toml").exists());
}

#[test]
fn the_user_layer_beats_the_site_and_the_site_beats_the_defaults() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let rows = |shown: &Value| -> Vec<(String, String)> {
        shown["effective"]["registries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["name"].as_str().unwrap().to_owned(),
                    row["scope"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let show = |defaults: bool| {
        let mut command = machine.cmd();
        if defaults {
            command.env_remove("VIBE_NO_DEFAULT_REGISTRY");
        }
        command.args(["--json", "self", "apply", "--show"]);
        json(&command.output().unwrap())
    };

    // No site, no user file: the built-in defaults.
    let before = show(true);
    assert!(
        rows(&before).iter().all(|(_, scope)| scope == "default") && !rows(&before).is_empty(),
        "{before}"
    );

    let profile = write_profile(
        site.path(),
        &format!(
            "schema = 1\n\
             [[registry]]\nname = 'corp'\nurl = 'file:///site/corp'\n\
             [[registry]]\nname = 'shared'\nurl = 'file:///site/shared'\n\
             [[remap]]\nprefix = 'https://downloads.example.invalid/'\npath = '{}/payloads'\n\
             [artifacts]\nvibe = '{}/releases'\n",
            site.path().display(),
            site.path().display()
        ),
    );
    machine.apply(&profile, &["--no-fix-path"]);

    // Site beats the defaults: its registries replace them.
    assert_eq!(
        rows(&show(true)),
        [
            ("corp".to_owned(), "site".to_owned()),
            ("shared".to_owned(), "site".to_owned())
        ]
    );
    // `vibe init` (a registry-needing verb) does not seed the defaults
    // into the user file above the site's registries.
    let project = tempfile::tempdir().unwrap();
    let mut init = machine.cmd();
    init.env_remove("VIBE_NO_DEFAULT_REGISTRY")
        .args(["--unattended", "init", "--path"])
        .arg(project.path());
    ok(&init.output().unwrap());
    assert!(!machine.settings("registry.toml").exists());

    // User beats site: a user row hides the site's row of the same name.
    fs::write(
        machine.settings("registry.toml"),
        "[[registry]]\nname = 'corp'\nurl = 'file:///mine/corp'\n\
         [[registry]]\nname = 'own'\nurl = 'file:///mine/own'\n",
    )
    .unwrap();
    let shown = show(false);
    assert_eq!(
        rows(&shown),
        [
            ("corp".to_owned(), "user".to_owned()),
            ("own".to_owned(), "user".to_owned()),
            ("shared".to_owned(), "site".to_owned())
        ]
    );
    assert_eq!(
        shown["effective"]["registries"][0]["url"],
        "file:///mine/corp"
    );

    // The release source: the site's drop, until the user chooses their own.
    let source = json(&machine.run(&["--json", "self", "source", "show"]));
    assert_eq!(source["source"], "localdir");
    assert!(
        source["from"]
            .as_str()
            .unwrap()
            .contains("site-profile.toml"),
        "{source}"
    );
    ok(&machine.run(&["--json", "self", "source", "set", "github"]));
    let source = json(&machine.run(&["--json", "self", "source", "show"]));
    assert_eq!(source["source"], "github");
    let config = fs::read_to_string(machine.settings("config.toml")).unwrap();
    assert!(
        !config.contains("releases"),
        "no site value leaks into config.toml: {config}"
    );
    // The reset reports what is in effect afterwards: the site's drop,
    // named by the site profile — not the built-in GitHub default.
    let reset = json(&machine.run(&["--json", "self", "source", "reset"]));
    assert_eq!(reset["source"], "localdir", "{reset}");
    assert!(
        reset["from"]
            .as_str()
            .unwrap()
            .contains("site-profile.toml"),
        "{reset}"
    );
    let reset_text = ok(&machine.run(&["self", "source", "reset"]));
    assert!(
        reset_text.contains("release source: localdir") && !reset_text.contains("built-in"),
        "{reset_text}"
    );
    let source = json(&machine.run(&["--json", "self", "source", "show"]));
    assert_eq!(source["source"], "localdir");

    // The site's remap rows are visible where every row is.
    let explained = ok(&machine.run(&[
        "remap",
        "explain",
        "https://downloads.example.invalid/tool.tgz",
    ]));
    assert!(explained.contains("site profile"), "{explained}");
}

#[test]
fn the_path_step_asks_for_consent_and_honours_flag_and_profile() {
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let plain = write_profile(site.path(), &corp_profile(site.path(), None));

    // Unattended without a flag: nothing written, the hint names the flag.
    let machine = Machine::new();
    machine.import_active();
    let skipped = machine.apply(&plain, &[]);
    assert_eq!(skipped["path"]["outcome"], "skipped");
    assert_eq!(skipped["path"]["skip_reason"], "not_interactive");
    assert!(
        skipped["path"]["fix_hint"]
            .as_str()
            .unwrap()
            .contains("--fix-path")
    );
    assert_eq!(machine.bashrc(), None);

    // `--fix-path`: applied through the rc-file persister, then idempotent.
    let applied = machine.apply(&plain, &["--fix-path"]);
    assert_eq!(applied["path"]["outcome"], "applied");
    assert_eq!(applied["path"]["consent"], "flag");
    let rc = machine.bashrc().unwrap();
    assert!(
        rc.contains(&machine.shim_dir().display().to_string()),
        "{rc}"
    );
    let again = machine.apply(&plain, &["--fix-path"]);
    assert_eq!(again["path"]["outcome"], "already_on_path");
    assert_eq!(machine.bashrc().unwrap(), rc);
    let receipt = fs::read_to_string(machine.settings("site-profile.receipt.toml")).unwrap();
    assert!(receipt.contains("already_on_path"), "{receipt}");

    // `[path] fix = "always"` applies unattended; `--no-fix-path` beats it.
    let always = site.path().join("always");
    fs::create_dir_all(&always).unwrap();
    let always = write_profile(&always, &corp_profile(site.path(), Some("always")));
    let declined = Machine::new();
    declined.import_active();
    let refused = declined.apply(&always, &["--no-fix-path"]);
    assert_eq!(refused["path"]["skip_reason"], "flag_declined");
    assert_eq!(declined.bashrc(), None);
    let by_profile = declined.apply(&always, &[]);
    assert_eq!(by_profile["path"]["outcome"], "applied");
    assert_eq!(by_profile["path"]["consent"], "profile");
}

/// Every `connect(2)` to an IP address that is not loopback, from an
/// `strace -f -e trace=connect` log.
fn non_loopback_connects(log: &str) -> Vec<String> {
    log.lines()
        .filter(|line| line.contains("connect(") && line.contains("AF_INET"))
        .filter(|line| {
            let address = line
                .split("inet_addr(\"")
                .nth(1)
                .or_else(|| line.split("inet_pton(AF_INET6, \"").nth(1))
                .and_then(|rest| rest.split('"').next())
                .unwrap_or("");
            !(address.starts_with("127.") || address == "::1" || address.starts_with("::ffff:127."))
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn non_loopback_connects_are_recognised() {
    let log = "1 connect(3, {sa_family=AF_INET, sin_port=htons(53), sin_addr=inet_addr(\"127.0.0.53\")}, 16) = 0\n\
               2 connect(4, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr(\"140.82.121.4\")}, 16) = -1\n\
               3 connect(5, {sa_family=AF_INET6, sin6_port=htons(443), sin6_flowinfo=htonl(0), inet_pton(AF_INET6, \"::1\", &sin6_addr)}, 28) = 0\n\
               4 connect(6, {sa_family=AF_UNIX, sun_path=\"/run/x\"}, 110) = 0\n";
    let found = non_loopback_connects(log);
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("140.82.121.4"));
}

fn tool(name: &str) -> bool {
    std::process::Command::new(name)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// A closed corporate network, end to end: a fresh machine with a `vibe`
/// downloaded by hand, one site profile with a localdir registry, a
/// release drop, a package drop and `exclude_hosting = ["github",
/// "gitverse"]`. `vibe self apply` imports the binary and configures the
/// installation; `vibe install` resolves from the site's registry and
/// `vibe self releases` lists the drop — with no network namespace to
/// reach (bwrap) and no `connect(2)` to anything but loopback (strace).
#[test]
fn a_closed_network_machine_is_configured_by_one_profile() {
    if !tool("strace") {
        eprintln!("skipping: strace is not available to observe connections");
        return;
    }
    let isolate = tool("bwrap")
        && std::process::Command::new("bwrap")
            .args(["--dev-bind", "/", "/", "--unshare-net", "true"])
            .status()
            .is_ok_and(|status| status.success());
    let machine = Machine::new();
    let site = tempfile::tempdir().unwrap();
    let registry = site.path().join("registry");
    common::copy_tree(
        &common::fixture_registry().join("org.vibevm/integration-beta"),
        &registry.join("org.vibevm/integration-beta"),
    );
    for version in ["1.0.11", "1.0.13"] {
        let dir = site.path().join("releases").join(format!("v{version}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("DISTRIBUTIONS.json"), b"{}").unwrap();
    }
    fs::create_dir_all(site.path().join("payloads")).unwrap();
    let profile = write_profile(site.path(), &corp_profile(site.path(), None));
    let logs = tempfile::tempdir().unwrap();

    let observed = |label: &str, args: &[&str], cwd: Option<&Path>| -> Output {
        let log = logs.path().join(format!("{label}.strace"));
        let mut command = if isolate {
            let mut command = std::process::Command::new("bwrap");
            command.args(["--dev-bind", "/", "/", "--unshare-net", "strace"]);
            command
        } else {
            std::process::Command::new("strace")
        };
        command
            .args(["-f", "-qq", "-e", "trace=connect", "-o"])
            .arg(&log)
            .arg(env!("CARGO_BIN_EXE_vibe"))
            .args(args)
            .env(
                vibe_core::settings::SETTINGS_DIR_ENV,
                &machine.user.settings,
            )
            .env("VIBE_REGISTRY_CACHE", &machine.user.cache)
            .env("VIBEVM_SEARCH_CACHE_DIR", &machine.user.search_cache)
            .env("VIBEVM_INSTALL_ROOT", machine.base.path())
            .env("HOME", machine.home.path())
            .env("SHELL", "/bin/bash")
            .env("PATH", "/usr/bin:/bin")
            .env_remove("VIBE_NO_DEFAULT_REGISTRY")
            .env_remove("VIBEVM_HOME")
            .env_remove("VIBEVM_RELEASE_ORIGIN")
            .env_remove("VIBEVM_USER_CONFIG")
            .env_remove("VIBE_OFFLINE")
            .env_remove("HTTPS_PROXY")
            .env_remove("HTTP_PROXY")
            .env_remove("ALL_PROXY");
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let output = command.output().unwrap();
        let trace = fs::read_to_string(&log).unwrap_or_default();
        let outside = non_loopback_connects(&trace);
        assert!(
            outside.is_empty(),
            "`vibe {}` tried to reach the network: {outside:#?}",
            args.join(" ")
        );
        output
    };

    // A downloaded binary configures the machine and imports itself.
    let profile_arg = profile.display().to_string();
    let applied = json(&observed(
        "apply",
        &[
            "--json",
            "--unattended",
            "self",
            "apply",
            &profile_arg,
            "--no-fix-path",
        ],
        None,
    ));
    assert_eq!(applied["changed"], true);
    assert_eq!(applied["imported"]["planned"], false, "{applied}");
    assert!(machine.shim_dir().join("vibe").exists());

    let project = tempfile::tempdir().unwrap();
    let project_arg = project.path().display().to_string();
    ok(&observed(
        "init",
        &["--unattended", "init", "--path", &project_arg],
        None,
    ));
    ok(&observed(
        "install",
        &[
            "--unattended",
            "install",
            "org.vibevm/integration-beta",
            "--path",
            &project_arg,
            "--assume-yes",
        ],
        None,
    ));
    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].registry.as_deref(), Some("corp"));

    let listed = json(&observed("releases", &["--json", "self", "releases"], None));
    let versions: Vec<&str> = listed["releases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["version"].as_str().unwrap())
        .collect();
    assert_eq!(versions, ["1.0.13", "1.0.11"]);

    let where_to_put = json(&observed("where", &["--json", "artifacts", "where"], None));
    assert_eq!(where_to_put["release"]["source"], "localdir");
    assert_eq!(
        where_to_put["packages"],
        site.path().join("payloads").display().to_string()
    );
}

/// `vibe-self-apply/1` is the document its registered schema describes: the
/// closed vocabularies carry the values the command computed, and the PATH
/// member reads as a planned step on `--show`.
#[test]
fn the_apply_report_reads_as_its_registered_wire_type() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let profile = write_profile(site.path(), &corp_profile(site.path(), Some("never")));

    let mut command = machine.cmd();
    command
        .args(["--json", "--unattended", "self", "apply"])
        .arg(&profile)
        .args(["--no-fix-path"]);
    let applied: SelfApplyReport = common::wire::typed(&ok(&command.output().unwrap()));
    assert_eq!(applied.protocol, "vibe-self-apply/1");
    assert_eq!(applied.command, "self:apply");
    assert_eq!(applied.action, ApplyAction::Apply);
    assert!(applied.changed);
    let path = applied.path.expect("apply reaches the PATH step");
    assert_eq!(path.skip_reason, Some(SkipReason::FlagDeclined));
    assert!(
        path.planned.is_none() && path.durable_environment.is_some(),
        "a performed step carries the observation, never a plan"
    );

    let shown: SelfApplyReport = common::wire::typed(&ok(&machine.run(&[
        "--json",
        "--unattended",
        "self",
        "apply",
        "--show",
    ])));
    assert_eq!(shown.action, ApplyAction::Show);
    let path = shown
        .path
        .expect("--show reports the PATH step it would take");
    assert_eq!(path.planned, Some(PathPlan::WouldSkip));
    assert!(
        path.outcome.is_none() && path.durable_environment.is_none(),
        "a planned step reaches no persister"
    );
    let effective = shown.effective.expect("--show reports the installation");
    assert!(
        effective
            .registries
            .iter()
            .all(|row| row.scope != RegistryScope::Default),
        "the site profile replaces the built-in defaults"
    );
}

/// A tiny active instance in one install root, so `apply` does not import
/// the (large) test binary itself.
fn import_active_in(machine: &Machine, root: &Path) {
    let payload = root.join("ready-vibe");
    fs::create_dir_all(root).unwrap();
    fs::write(&payload, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&payload, fs::Permissions::from_mode(0o755)).unwrap();
    let mut command = machine.cmd();
    command
        .env("VIBEVM_INSTALL_ROOT", root)
        .args(["--json", "--unattended", "self", "import"])
        .arg(&payload)
        .args(["--tag", "1.2.3", "--use"]);
    ok(&command.output().unwrap());
}

/// A network that only allows executables under one directory declares it
/// once, in the site profile: the store, the shims the PATH fix points at
/// and the durable `$VIBEVM_INSTALL_ROOT` all follow
/// (`PROP-019#install-root`).
#[test]
fn a_site_profile_chooses_the_install_root_and_makes_it_durable() {
    let machine = Machine::new();
    // The one allowed location, standing in for `C:\Work`.
    let allowed = tempfile::tempdir().unwrap();
    import_active_in(&machine, allowed.path());
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let profile = write_profile(
        site.path(),
        &format!(
            "schema = 1\nname = \"corp\"\n\n[self]\ninstall_root = \"{}\"\n",
            allowed.path().display()
        ),
    );

    let mut command = machine.cmd();
    command
        .env_remove("VIBEVM_INSTALL_ROOT")
        .args(["--json", "--unattended", "self", "apply"])
        .arg(&profile)
        .args(["--fix-path"]);
    let applied = json(&command.output().unwrap());

    let root = &applied["effective"]["install_root"];
    assert_eq!(root["path"], allowed.path().display().to_string());
    assert!(
        root["from"]
            .as_str()
            .unwrap()
            .starts_with("[self] install_root in"),
        "{root}"
    );
    let shims = allowed.path().join("opt").join("bin");
    assert_eq!(root["shim_dir"], shims.display().to_string());
    assert_eq!(
        root["data_dir"],
        allowed
            .path()
            .join("opt")
            .join("vibevm")
            .display()
            .to_string()
    );
    // The PATH step points at that root's shims, not at this process's.
    assert_eq!(applied["path"]["shim_dir"], shims.display().to_string());
    assert_eq!(applied["path"]["outcome"], "applied");
    assert!(
        shims.join("vibe").is_file(),
        "the shims live under the root"
    );

    // Both durable writes went through the one persister, under one consent.
    let rc = machine.bashrc().unwrap();
    assert!(rc.contains(&shims.display().to_string()), "{rc}");
    // The persister quotes what it writes, so the assertion names the
    // variable and its value rather than one spelling of the line.
    let assignment = rc
        .lines()
        .find(|line| line.contains("VIBEVM_INSTALL_ROOT"))
        .unwrap_or_else(|| panic!("{rc}"));
    assert!(
        assignment.starts_with("export VIBEVM_INSTALL_ROOT=")
            && assignment.contains(&allowed.path().display().to_string()),
        "{rc}"
    );

    // Re-applying writes nothing new.
    let mut command = machine.cmd();
    command
        .env_remove("VIBEVM_INSTALL_ROOT")
        .args(["--json", "--unattended", "self", "apply"])
        .arg(&profile)
        .args(["--fix-path"]);
    let again = json(&command.output().unwrap());
    assert!(!again["changed"].as_bool().unwrap());
    assert_eq!(machine.bashrc().unwrap(), rc);
}

/// The install root is a ladder, highest first: the inherited environment
/// variable, the user's own `[self] install_root`, the site profile's,
/// then `~/.vibe` (`PROP-019#install-root`).
#[test]
fn the_environment_beats_the_user_file_and_the_user_file_beats_the_site() {
    let machine = Machine::new();
    machine.import_active();
    let allowed = tempfile::tempdir().unwrap();
    let mine = tempfile::tempdir().unwrap();
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let profile = write_profile(
        site.path(),
        &format!(
            "schema = 1\nname = \"corp\"\n\n[self]\ninstall_root = \"{}\"\n",
            allowed.path().display()
        ),
    );

    // `Machine::cmd` sets `VIBEVM_INSTALL_ROOT`, which outranks every file.
    let applied = machine.apply(&profile, &["--no-fix-path"]);
    assert_eq!(
        applied["effective"]["install_root"]["path"],
        machine.base.path().display().to_string()
    );
    assert_eq!(
        applied["effective"]["install_root"]["from"],
        "$VIBEVM_INSTALL_ROOT"
    );
    // Nothing was made durable: the environment already names it.
    assert_eq!(machine.bashrc(), None);

    // Without it, the applied site decides.
    let shown = json(
        &machine
            .cmd()
            .env_remove("VIBEVM_INSTALL_ROOT")
            .args(["--json", "--unattended", "self", "apply", "--show"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        shown["effective"]["install_root"]["path"],
        allowed.path().display().to_string()
    );

    // The user's own file beats the site, and keeps doing so after the user
    // also picks a release source of their own.
    let config = machine.settings("config.toml");
    fs::write(
        &config,
        format!(
            "[self]\ninstall_root = \"{}\"\nsource = \"github\"\n",
            mine.path().display()
        ),
    )
    .unwrap();
    let shown = json(
        &machine
            .cmd()
            .env_remove("VIBEVM_INSTALL_ROOT")
            .args(["--json", "--unattended", "self", "apply", "--show"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        shown["effective"]["install_root"]["path"],
        mine.path().display().to_string()
    );
    assert!(
        shown["effective"]["install_root"]["from"]
            .as_str()
            .unwrap()
            .contains("config.toml"),
        "{shown}"
    );
}

/// A site profile with `pac = "system"`, stored once and read back by
/// every case below.
fn system_pac_machine() -> Machine {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    for dir in ["registry", "releases", "payloads"] {
        fs::create_dir_all(site.path().join(dir)).unwrap();
    }
    let body = corp_profile(site.path(), None)
        + "\n[proxy]\nmode = \"px\"\npac = \"windows\"\nno_proxy = \".corp.example\"\n";
    let profile = write_profile(site.path(), &body);
    machine.apply(&profile, &[]);
    machine
}

/// `pac = "system"` is resolved by vibe, so `--show` says what it
/// resolved to and which source answered — and when nothing on the host
/// configures a proxy, that the Linux desktops and the environment were
/// all looked at and found empty (PROP-068 `#proxy-system`). The
/// `[proxy]` table's own keys still read back exactly as written.
#[test]
fn a_system_pac_is_reported_as_resolved_or_as_unresolvable_here() {
    let machine = system_pac_machine();
    // An empty fixture directory IS a host with nothing configured: no
    // `gsettings`, no `kioslaverc`, and `cmd()` removed the proxy
    // variables.
    let bare = tempfile::tempdir().unwrap();

    let shown =
        json(&machine.run_as_host(&["--json", "self", "apply", "--show"], "unix", bare.path()));
    let proxy = &shown["effective"]["proxy"];
    // The alias is accepted and read as the one kind `system` names.
    assert_eq!(proxy["pac"], "windows", "{proxy}");
    assert_eq!(proxy["pac_kind"], "system", "{proxy}");
    assert_eq!(proxy["pac_origin"], "site profile", "{proxy}");
    let resolved = proxy["pac_resolved"].as_str().expect("resolved or why not");
    for part in [
        "GNOME settings",
        "KDE settings",
        "environment https_proxy",
        "fix:",
    ] {
        assert!(resolved.contains(part), "{part} missing from: {resolved}");
    }
    // The default revocation setting is reported as such and prints nothing.
    assert_eq!(proxy["revocation"], "check", "{proxy}");
    assert_eq!(proxy["revocation_origin"], "default", "{proxy}");
    let text = String::from_utf8_lossy(
        &machine
            .run_as_host(&["self", "apply", "--show"], "unix", bare.path())
            .stdout,
    )
    .into_owned();
    assert!(
        text.contains("proxy pac: windows (system, from site profile)"),
        "{text}"
    );
    assert!(text.contains(resolved), "{text}");
    assert!(!text.contains("proxy revocation:"), "{text}");
}

/// A Linux host whose GNOME desktop is set to "automatic proxy
/// configuration": `pac = "system"` resolves to the PAC URL GNOME names,
/// which is the `--pac=<url>` px is started with, and every report says
/// the setting it came from (PROP-068 `#proxy-system`).
#[test]
fn a_gnome_auto_configuration_resolves_to_the_pac_url_it_names() {
    let machine = system_pac_machine();
    let fixture = tempfile::tempdir().unwrap();
    fs::write(
        fixture.path().join("gsettings"),
        "org.gnome.system.proxy autoconfig-url 'https://pac.example.corp/proxy.pac'\n\
         org.gnome.system.proxy ignore-hosts ['localhost', '*.corp.example']\n\
         org.gnome.system.proxy mode 'auto'\n",
    )
    .unwrap();

    let shown = json(&machine.run_as_host(
        &["--json", "self", "apply", "--show"],
        "unix",
        fixture.path(),
    ));
    assert_eq!(
        shown["effective"]["proxy"]["pac_resolved"],
        "system \u{2192} https://pac.example.corp/proxy.pac · bypass localhost,*.corp.example \
         (from GNOME settings)",
        "{shown}"
    );
    let text = String::from_utf8_lossy(
        &machine
            .run_as_host(&["self", "apply", "--show"], "unix", fixture.path())
            .stdout,
    )
    .into_owned();
    assert!(
        text.contains("system \u{2192} https://pac.example.corp/proxy.pac"),
        "{text}"
    );
    assert!(text.contains("from GNOME settings"), "{text}");
}

/// The same host, with the macOS order: the network settings of the
/// current network service name a fixed proxy, and that is what px is
/// given as `--proxy` (PROP-068 `#proxy-system`).
#[test]
fn the_macos_network_settings_resolve_to_the_proxy_they_name() {
    let machine = system_pac_machine();
    let fixture = tempfile::tempdir().unwrap();
    fs::write(
        fixture.path().join("scutil-proxy"),
        "<dictionary> {\n  ExceptionsList : <array> {\n    0 : *.corp.example\n  }\n\
         \x20 HTTPSEnable : 1\n  HTTPSPort : 8080\n  HTTPSProxy : proxy.example.corp\n}\n",
    )
    .unwrap();

    let shown = json(&machine.run_as_host(
        &["--json", "self", "apply", "--show"],
        "macos",
        fixture.path(),
    ));
    assert_eq!(
        shown["effective"]["proxy"]["pac_resolved"],
        "system \u{2192} proxy proxy.example.corp:8080 · bypass *.corp.example \
         (from macOS network settings)",
        "{shown}"
    );
}

/// The lowest source: an exported `https_proxy` with no desktop
/// configuration at all, reported as the variable it came from
/// (PROP-068 `#proxy-system`).
#[test]
fn an_exported_https_proxy_is_the_last_source_and_is_named() {
    let machine = system_pac_machine();
    let bare = tempfile::tempdir().unwrap();
    let shown = json(
        &machine
            .cmd()
            .env("VIBEVM_SYSTEM_PROXY_OS", "unix")
            .env("VIBEVM_SYSTEM_PROXY_FIXTURE", bare.path())
            .env("https_proxy", "http://proxy.example.corp:8080")
            .env("no_proxy", ".corp.example")
            .args(["--json", "self", "apply", "--show"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        shown["effective"]["proxy"]["pac_resolved"],
        "system \u{2192} proxy proxy.example.corp:8080 · bypass .corp.example \
         (from environment https_proxy)",
        "{shown}"
    );
}

/// `[proxy] revocation` is a `[proxy]` key like any other: the site sets
/// it, the user's file outranks the site and the environment outranks
/// both, and `--show` names the layer (PROP-068 `#proxy-revocation`).
#[test]
fn the_revocation_setting_is_reported_with_the_layer_that_chose_it() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    for dir in ["registry", "releases", "payloads"] {
        fs::create_dir_all(site.path().join(dir)).unwrap();
    }
    let body = corp_profile(site.path(), None)
        + "\n[proxy]\nhttps = \"http://proxy.example.corp:3128\"\nrevocation = \"off\"\n";
    let profile = write_profile(site.path(), &body);
    machine.apply(&profile, &[]);

    let shown = json(&machine.run(&["--json", "self", "apply", "--show"]));
    assert_eq!(shown["effective"]["proxy"]["revocation"], "off");
    assert_eq!(
        shown["effective"]["proxy"]["revocation_origin"],
        "site profile"
    );
    let text = ok(&machine.run(&["self", "apply", "--show"]));
    assert!(
        text.contains("proxy revocation: off (from site profile)"),
        "{text}"
    );

    // The user's own file outranks the site.
    fs::write(
        machine.settings("config.toml"),
        "[proxy]\nrevocation = \"best-effort\"\n",
    )
    .unwrap();
    let shown = json(&machine.run(&["--json", "self", "apply", "--show"]));
    assert_eq!(shown["effective"]["proxy"]["revocation"], "best-effort");
    assert_eq!(
        shown["effective"]["proxy"]["revocation_origin"],
        "user config"
    );

    // And the environment outranks both; an unusable value is refused by
    // name and ignored, leaving the files in charge.
    let shown = json(
        &machine
            .cmd()
            .env("VIBEVM_PROXY_REVOCATION", "off")
            .args(["--json", "self", "apply", "--show"])
            .output()
            .unwrap(),
    );
    assert_eq!(shown["effective"]["proxy"]["revocation"], "off");
    assert_eq!(
        shown["effective"]["proxy"]["revocation_origin"],
        "VIBEVM_PROXY_REVOCATION"
    );
    let output = machine
        .cmd()
        .env("VIBEVM_PROXY_REVOCATION", "maybe")
        .args(["--json", "self", "apply", "--show"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        stderr.contains("VIBEVM_PROXY_REVOCATION=`maybe` is not a revocation setting"),
        "{stderr}"
    );
    let shown = common::wire::checked(&ok(&output));
    assert_eq!(shown["effective"]["proxy"]["revocation"], "best-effort");
}

/// The pinned px of this build, its store directory under `root` and the
/// companion asset this platform needs — `None` when the pin has no px
/// here, the one case the step has nothing to do.
fn px_pin(root: &Path) -> Option<(PathBuf, PathBuf, String)> {
    let pin = vibe_net::px_store::PxPin::embedded();
    let asset = vibe_net::px_store::running_target().and_then(|target| pin.asset(target))?;
    let store = root
        .join("opt")
        .join("vibevm")
        .join("tools")
        .join("px")
        .join(&pin.version);
    Some((
        store.clone(),
        store.join(&asset.executable),
        pin.release_asset_name(asset),
    ))
}

/// A profile that selects px (`PROP-019#companion-tools`, B-251): the
/// release zip carries no px, so apply names the gap and the repair on
/// every run until the companion is in the store — and once it is, says
/// so and changes nothing. The px the step reports is always the store of
/// the install root this profile chooses.
#[test]
fn a_profile_selecting_px_reports_the_missing_companion_until_it_is_there() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    for dir in ["registry", "releases", "payloads"] {
        fs::create_dir_all(site.path().join(dir)).unwrap();
    }
    let body = corp_profile(site.path(), Some("never"))
        + "\n[proxy]\nmode = \"px\"\nupstream = \"proxy.example.corp:8080\"\n";
    let profile = write_profile(site.path(), &body);
    let Some((store, program, asset)) = px_pin(machine.base.path()) else {
        eprintln!("skipping: the pin has no px for this platform");
        return;
    };

    // A dry run says what it would find, and writes nothing.
    let planned = machine.apply(&profile, &["--dry-run"]);
    assert_eq!(planned["px"]["needed"], true, "{planned}");
    assert_eq!(planned["px"]["planned"], true, "{planned}");
    assert_eq!(planned["px"]["outcome"], "missing", "{planned}");
    assert_eq!(planned["px"]["store"], store.display().to_string());
    assert_eq!(planned["px"]["asset"], asset);
    assert!(!store.exists(), "a dry run writes nothing");

    // Apply: the profile is applied (exit 0) and the gap is a warning
    // naming the asset, the store and the repair.
    let applied = machine.apply(&profile, &[]);
    assert_eq!(applied["px"]["outcome"], "missing", "{applied}");
    assert_eq!(applied["px"]["planned"], false);
    assert_eq!(applied["px"]["program"], Value::Null);
    let fix = applied["px"]["fix"].as_str().expect("the repair");
    assert!(fix.contains(&asset), "{fix}");
    assert!(fix.contains("vibe self px install --from"), "{fix}");
    assert!(fix.contains(&store.display().to_string()), "{fix}");
    let warnings: Vec<&str> = applied["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|warning| warning.as_str().unwrap())
        .collect();
    assert!(
        warnings.contains(&fix),
        "the warning is the repair: {warnings:?}"
    );

    // The human summary says it where a person looks, not only as a grey
    // skipped line.
    let mut human = machine.cmd();
    human.args(["--unattended", "self", "apply"]).arg(&profile);
    let output = human.output().unwrap();
    let text = ok(&output);
    assert!(text.contains("px: NOT INSTALLED"), "{text}");
    assert!(text.contains("proxy mode: px"), "{text}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("vibe self px install --from"), "{stderr}");

    // The operator puts px in the store (here by hand; `vibe self px
    // install` and the apply step itself verify it against the pin).
    fs::create_dir_all(&store).unwrap();
    fs::write(&program, "fake px").unwrap();
    let ready = machine.apply(&profile, &[]);
    assert_eq!(ready["px"]["outcome"], "already_installed", "{ready}");
    assert_eq!(ready["px"]["program"], program.display().to_string());
    assert_eq!(ready["px"]["fix"], Value::Null);
    assert!(
        ready["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|warning| !warning.as_str().unwrap().contains("px is not installed")),
        "{ready}"
    );
    // And the run after that is the same run: idempotent.
    let again = machine.apply(&profile, &[]);
    assert_eq!(again["px"], ready["px"]);
    let text = ok(&machine
        .cmd()
        .args(["--unattended", "self", "apply"])
        .arg(&profile)
        .output()
        .unwrap());
    assert!(text.contains("px: already installed"), "{text}");
    assert!(!text.contains("NOT INSTALLED"), "{text}");
}

/// The px step reads the store of the install root the profile declares,
/// not the one this process happens to run under (`PROP-019#install-root`).
#[test]
fn the_px_step_follows_the_profiles_install_root() {
    let machine = Machine::new();
    let allowed = tempfile::tempdir().unwrap();
    import_active_in(&machine, allowed.path());
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let profile = write_profile(
        site.path(),
        &format!(
            "schema = 1\nname = \"corp\"\n\n[self]\ninstall_root = \"{}\"\n\
             \n[proxy]\nmode = \"px\"\nupstream = \"proxy.example.corp:8080\"\n",
            allowed.path().display()
        ),
    );
    let Some((store, _, _)) = px_pin(allowed.path()) else {
        eprintln!("skipping: the pin has no px for this platform");
        return;
    };

    let applied = json(
        &machine
            .cmd()
            .env_remove("VIBEVM_INSTALL_ROOT")
            .args(["--json", "--unattended", "self", "apply"])
            .arg(&profile)
            .args(["--no-fix-path"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        applied["px"]["store"],
        store.display().to_string(),
        "{applied}"
    );
    assert_eq!(applied["px"]["needed"], true);
}

/// `[proxy] px_path` names the operator's own px, so the companion store
/// is not vibe's business and the step says nothing about it.
#[test]
fn a_profile_naming_its_own_px_asks_nothing_of_the_companion_store() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let own = site.path().join("px");
    fs::write(&own, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&own, fs::Permissions::from_mode(0o755)).unwrap();
    let body = corp_profile(site.path(), Some("never"))
        + &format!(
            "\n[proxy]\nmode = \"px\"\nupstream = \"proxy.example.corp:8080\"\npx_path = \"{}\"\n",
            own.display()
        );
    let profile = write_profile(site.path(), &body);

    let applied = machine.apply(&profile, &[]);
    assert_eq!(applied["px"]["outcome"], "not_needed", "{applied}");
    assert_eq!(applied["px"]["needed"], false);
    assert_eq!(applied["px"]["fix"], Value::Null);
    let text = ok(&machine
        .cmd()
        .args(["--unattended", "self", "apply"])
        .arg(&profile)
        .output()
        .unwrap());
    assert!(!text.contains("px:"), "{text}");
}

/// `vibe self px status` with a px mode and no px is not a success: it
/// reports, then refuses, the way `vibe self doctor` does with an
/// unresolved problem (`PROP-019#companion-tools`, B-251).
#[test]
fn px_status_refuses_when_a_px_route_has_no_px() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let body = corp_profile(site.path(), Some("never"))
        + "\n[proxy]\nmode = \"px\"\nupstream = \"proxy.example.corp:8080\"\n";
    machine.apply(&write_profile(site.path(), &body), &[]);
    let Some((store, program, asset)) = px_pin(machine.base.path()) else {
        eprintln!("skipping: the pin has no px for this platform");
        return;
    };
    // An empty PATH, so a px this machine happens to carry cannot answer.
    let empty = tempfile::tempdir().unwrap();
    let status = || {
        machine
            .cmd()
            .env("PATH", empty.path())
            .args(["--json", "self", "px", "status"])
            .output()
            .unwrap()
    };

    let output = status();
    assert!(!output.status.success(), "a px route with no px is not ok");
    let doc: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(doc["ok"], false, "{doc}");
    assert_eq!(doc["needed"], true, "{doc}");
    assert_eq!(doc["in_store"], false, "{doc}");
    assert_eq!(doc["program"], Value::Null, "{doc}");
    assert_eq!(doc["asset"], asset, "{doc}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("vibe self px install --from"), "{stderr}");

    // With px in the store, the same verb is a plain success.
    fs::create_dir_all(&store).unwrap();
    fs::write(&program, "fake px").unwrap();
    let output = status();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(doc["ok"], true, "{doc}");
    assert_eq!(doc["in_store"], true, "{doc}");
    assert_eq!(doc["program"], program.display().to_string(), "{doc}");

    // Without a px route at all, a store without px is merely a fact.
    fs::remove_file(&program).unwrap();
    let output = machine
        .cmd()
        .env("PATH", empty.path())
        .env("VIBEVM_PROXY_MODE", "none")
        .args(["--json", "self", "px", "status"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(doc["ok"], true, "{doc}");
    assert_eq!(doc["needed"], false, "{doc}");
    assert_eq!(doc["in_store"], false, "{doc}");
}

/// `[self] install_root = "~"` — the one path a profile cannot write out
/// in full, because it is a different directory on every machine that
/// reads it (`PROP-019#install-root`). It expands against the same home
/// `~/.vibe` comes from, so one profile puts vibe in each developer's
/// `~/opt` with shims at `~/opt/bin`; `~user` and a relative root are
/// refused by name.
#[test]
fn an_install_root_of_tilde_lands_in_this_users_home() {
    let machine = Machine::new();
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();
    let profile = write_profile(
        site.path(),
        "schema = 1\nname = \"corp\"\n\n[self]\ninstall_root = \"~\"\n",
    );
    let home = machine.home.path();
    let apply = |extra: &[&str]| {
        let mut command = machine.cmd();
        command
            .env_remove("VIBEVM_INSTALL_ROOT")
            .args(["--json", "--unattended", "self", "apply"])
            .arg(&profile)
            .args(extra);
        command.output().unwrap()
    };

    // A dry run already names the expanded directory and its shims.
    let planned = json(&apply(&["--dry-run"]));
    let root = &planned["effective"]["install_root"];
    assert_eq!(root["path"], home.display().to_string(), "{planned}");
    assert_eq!(
        root["shim_dir"],
        home.join("opt").join("bin").display().to_string()
    );
    assert_eq!(
        root["data_dir"],
        home.join("opt").join("vibevm").display().to_string()
    );
    assert!(
        root["from"]
            .as_str()
            .unwrap()
            .starts_with("[self] install_root in"),
        "{root}"
    );
    assert_eq!(
        planned["path"]["shim_dir"],
        home.join("opt").join("bin").display().to_string()
    );

    // And an apply puts the shims there.
    import_active_in(&machine, home);
    let applied = json(&apply(&["--fix-path"]));
    assert_eq!(
        applied["effective"]["install_root"]["path"],
        home.display().to_string()
    );
    assert!(
        home.join("opt").join("bin").join("vibe").is_file(),
        "the shims live under the expanded home"
    );
    let shown = ok(&machine
        .cmd()
        .env_remove("VIBEVM_INSTALL_ROOT")
        .args(["--unattended", "self", "apply", "--show"])
        .output()
        .unwrap());
    assert!(
        shown.contains(&format!("install root: {}", home.display())),
        "{shown}"
    );
    assert!(!shown.contains("install root: ~"), "{shown}");
}

/// Another account's home and a relative directory are both refused, in a
/// profile and in the user's own `config.toml`
/// (`PROP-019#install-root`).
#[test]
fn an_install_root_vibe_cannot_resolve_is_refused_by_name() {
    let machine = Machine::new();
    let site = tempfile::tempdir().unwrap();
    fs::create_dir_all(site.path().join("releases")).unwrap();

    for (written, expected) in [("~bob/opt", "another account"), ("opt", "relative")] {
        let dir = site.path().join(written.replace(['/', '~'], "_"));
        fs::create_dir_all(&dir).unwrap();
        let profile = write_profile(
            &dir,
            &format!("schema = 1\nname = \"corp\"\n\n[self]\ninstall_root = \"{written}\"\n"),
        );
        let output = machine
            .cmd()
            .args(["--unattended", "self", "apply"])
            .arg(&profile)
            .output()
            .unwrap();
        assert!(!output.status.success(), "`{written}` must be refused");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("self.install_root"), "{stderr}");
        assert!(stderr.contains(written), "{stderr}");
        assert!(!machine.settings("site-profile.toml").exists());
        let _ = expected;
    }

    // The user's own file is refused where the root is resolved, naming
    // the written value and the fix.
    fs::write(
        machine.settings("config.toml"),
        "[self]\ninstall_root = \"~bob/opt\"\n",
    )
    .unwrap();
    let output = machine
        .cmd()
        .env_remove("VIBEVM_INSTALL_ROOT")
        .args(["--unattended", "self", "apply", "--show"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("~bob/opt"), "{stderr}");
    assert!(stderr.contains("another account's home"), "{stderr}");
}

/// `vibe self px install --from <file>` installs whatever px the operator
/// names, and `vibe self px status` then says exactly which bytes are
/// there — the path, what the program reports, where vibe read it and the
/// SHA-256 it measured. Nothing is checked against a digest (owner
/// decision 2026-11-09, `PROP-019#companion-tools`).
#[test]
fn px_install_from_a_local_file_takes_any_px_and_status_names_the_bytes() {
    let machine = Machine::new();
    machine.import_active();
    let files = tempfile::tempdir().unwrap();
    // A px of the operator's own: a program that answers `--version`.
    let own = files.path().join("px-of-my-own");
    fs::write(&own, "#!/bin/sh\necho 'px my-own 9.9.9'\n").unwrap();
    fs::set_permissions(&own, fs::Permissions::from_mode(0o755)).unwrap();
    let sha256 = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(fs::read(&own).unwrap()))
    };

    let installed = json_plain(
        &machine
            .cmd()
            .args(["--json", "self", "px", "install", "--from"])
            .arg(&own)
            .output()
            .unwrap(),
    );
    assert_eq!(installed["ok"], true, "{installed}");
    assert_eq!(installed["sha256"], sha256, "{installed}");
    assert_eq!(installed["source"], own.display().to_string());
    assert_eq!(installed["px_version"], "px my-own 9.9.9", "{installed}");
    let program = PathBuf::from(installed["installed"].as_str().unwrap());
    assert!(program.is_file(), "{}", program.display());
    assert!(
        program.starts_with(machine.base.path()),
        "the store under this install root: {}",
        program.display()
    );

    // Status reports those exact bytes, with no verdict about them.
    let output = machine
        .cmd()
        .args(["self", "px", "status"])
        .output()
        .unwrap();
    let text = ok(&output);
    assert!(text.contains(&program.display().to_string()), "{text}");
    assert!(text.contains(&format!("sha256: {sha256}")), "{text}");
    assert!(text.contains("reports: px my-own 9.9.9"), "{text}");
    assert!(!text.to_lowercase().contains("verif"), "{text}");
    let doc = json_plain(
        &machine
            .cmd()
            .args(["--json", "self", "px", "status"])
            .output()
            .unwrap(),
    );
    assert_eq!(doc["in_store"], true, "{doc}");
    assert_eq!(doc["sha256"], sha256, "{doc}");
    assert_eq!(doc["ok"], true, "{doc}");

    // A second install of the same file changes nothing; `--force` replaces.
    let again = json_plain(
        &machine
            .cmd()
            .args(["--json", "self", "px", "install", "--from"])
            .arg(&own)
            .output()
            .unwrap(),
    );
    assert_eq!(again["installed"], program.display().to_string());
}

/// The artifact drops may start at the reader's home too
/// (`PROP-066#drops`): a profile that says `packages = "~/opt/drop/packages"`
/// resolves to this user's own drop — it is not the relative path such a
/// value used to be read as and ignored.
#[test]
fn artifact_drops_may_start_at_the_readers_own_home() {
    let machine = Machine::new();
    machine.import_active();
    let site = tempfile::tempdir().unwrap();
    let home = machine.home.path();
    for dir in ["opt/drop/packages", "opt/drop/releases"] {
        fs::create_dir_all(home.join(dir)).unwrap();
    }
    let profile = write_profile(
        site.path(),
        "schema = 1\nname = \"corp\"\n\n[artifacts]\nvibe = \"~/opt/drop/releases\"\n\
         packages = \"~/opt/drop/packages\"\n\n[self]\nsource = \"localdir\"\n",
    );

    let applied = machine.apply(&profile, &["--no-fix-path"]);
    assert_eq!(
        applied["effective"]["packages_drop"],
        home.join("opt/drop/packages").display().to_string(),
        "{applied}"
    );
    assert_eq!(
        applied["effective"]["release"]["location"],
        home.join("opt/drop/releases").display().to_string(),
        "{applied}"
    );
    assert!(
        applied["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|warning| !warning.as_str().unwrap().contains("is relative")),
        "{applied}"
    );

    // The stored profile keeps the `~` the administrator wrote: the next
    // machine to apply it has a different home.
    let stored = fs::read_to_string(machine.settings("site-profile.toml")).unwrap();
    assert!(
        stored.contains("packages = \"~/opt/drop/packages\""),
        "{stored}"
    );

    // And every reader of the drops resolves it the same way.
    let where_to_put = json(&machine.run(&["--json", "artifacts", "where"]));
    assert_eq!(
        where_to_put["packages"],
        home.join("opt/drop/packages").display().to_string(),
        "{where_to_put}"
    );
    let shown = ok(&machine.run(&["self", "apply", "--show"]));
    assert!(
        shown.contains(&home.join("opt/drop/packages").display().to_string()),
        "{shown}"
    );
    assert!(!shown.contains("~/opt/drop"), "{shown}");
}

/// A `--json` document that is not a registered wire type.
fn json_plain(output: &Output) -> Value {
    serde_json::from_str(&ok(output)).unwrap()
}
