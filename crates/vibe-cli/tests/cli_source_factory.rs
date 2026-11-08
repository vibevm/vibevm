//! Public source factory tests. Owned numeric-loopback SSH, never operator keys.
#![cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
struct Fixture {
    process: Child,
    temporary: tempfile::TempDir,
    facts: serde_json::Value,
    config: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let script = include_str!(
            "../../vibe-registry/src/git_backend/shell/native_admission/ssh_fixture.py"
        )
        .replace("\\nmaterialization='in-place'", "");
        let mut process = Command::new("python3")
            .arg("-u")
            .arg("-c")
            .arg(script)
            .env("TMPDIR", temporary.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(process.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let facts: serde_json::Value =
            serde_json::from_str(&line).expect("authenticated fixture startup");
        let config = temporary.path().join("ssh.config");
        fs::write(&config, format!("Host ordinary-fixture\n HostName 127.0.0.1\n User {}\n Port {}\n IdentityFile {}\n IdentityAgent none\n IdentitiesOnly yes\n UserKnownHostsFile {}\n GlobalKnownHostsFile /dev/null\n StrictHostKeyChecking yes\n",facts["user"].as_str().unwrap(), facts["port"],facts["key"].as_str().unwrap(),facts["known_hosts"].as_str().unwrap())).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
        Self {
            process,
            temporary,
            facts,
            config,
        }
    }
    fn root(&self) -> &Path {
        self.temporary.path()
    }
    fn address(&self) -> String {
        self.facts["url"]
            .as_str()
            .unwrap()
            .replace("127.0.0.1", "ordinary-fixture")
    }
    fn project(&self, name: &str) -> PathBuf {
        let project = self.root().join(name);
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("vibe.toml"),format!("[project]\nname='consumer'\nversion='1.0.0'\n[source_policy]\nexclude_hosting=['host:127.0.0.2']\n[requires.packages]\n'org.fixture/demo'='1.0.0'\n[[registry]]\nname='fixture'\nurl='{}'\nindex_url='none'\n",self.address())).unwrap();
        project
    }
    fn command(&self, project: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .current_dir(project)
            .env("HOME", self.facts["home"].as_str().unwrap())
            .env("VIBE_SETTINGS", self.root().join("settings"))
            .env("VIBE_REGISTRY_CACHE", self.root().join("cache"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("CI", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.root().join("empty"))
            .env(
                "GIT_SSH_COMMAND",
                format!("/usr/bin/ssh -F {}", self.config.display()),
            )
            .env_remove("GIT_SSH")
            .env_remove("SSH_AUTH_SOCK");
        fs::write(self.root().join("empty"), "").unwrap();
        command
    }
    fn run(&self, project: &Path) {
        let output = self
            .command(project)
            .args(["--unattended", "install"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fn contacts(&self) -> usize {
        fs::read_to_string(Path::new(self.facts["root"].as_str().unwrap()).join("sshd-a.log"))
            .unwrap()
            .matches("Connection from ")
            .count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.process.stdin.take());
        let _ = self.process.wait();
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn constrained_new_registry_uses_ordinary_config_then_held_store_ignores_current_catalog() {
    let fixture = Fixture::new();
    let project = fixture.project("new-package");
    fixture.run(&project);
    let lock = fs::read(project.join("vibe.lock")).unwrap();
    let before = fixture.contacts();
    // Same source/ref/hash are held. A broken current declaration and tool are
    // irrelevant to the verified copied-store result, not alternate candidates.
    let manifest = fs::read_to_string(project.join("vibe.toml")).unwrap();
    fs::write(
        project.join("vibe.toml"),
        manifest.replace(
            &fixture.address(),
            "ssh://unused.invalid/no-current-retarget",
        ),
    )
    .unwrap();
    fs::create_dir_all(fixture.root().join("settings")).unwrap();
    fs::write(
        fixture.root().join("settings/registry.toml"),
        "this is malformed TOML",
    )
    .unwrap();
    let output = fixture
        .command(&project)
        .env("VIBE_GIT_BINARY", fixture.root().join("absent-git"))
        .args(["--unattended", "install"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(project.join("vibe.lock")).unwrap(), lock);
    assert_eq!(fixture.contacts(), before);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn excluded_effective_alias_refuses_before_missing_identity_and_contact() {
    let fixture = Fixture::new();
    let project = fixture.project("excluded");
    let manifest = fs::read_to_string(project.join("vibe.toml"))
        .unwrap()
        .replace("host:127.0.0.2", "host:127.0.0.1");
    fs::write(project.join("vibe.toml"), manifest).unwrap();
    fs::remove_file(fixture.facts["key"].as_str().unwrap()).unwrap();
    let before = fixture.contacts();
    let output = fixture
        .command(&project)
        .args(["--unattended", "install"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(
        diagnostic.contains("127.0.0.1") && diagnostic.contains("excluded"),
        "{diagnostic}"
    );
    assert_eq!(fixture.contacts(), before);
    assert!(!project.join("vibe.lock").exists());
}

/// Owner request (2026-11-08): a Git SSH registry in the machine
/// `registry.toml` works with the user's ordinary `~/.ssh/config` — no
/// `GIT_SSH_COMMAND`, no vibe-specific key settings — and once locked the
/// package keeps replaying its locked source after the registry is switched.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn machine_registry_ssh_url_uses_the_home_ssh_config_and_replays_after_a_switch() {
    let fixture = Fixture::new();
    let home = std::path::PathBuf::from(fixture.facts["home"].as_str().unwrap());
    let ssh = home.join(".ssh");
    fs::create_dir_all(&ssh).unwrap();
    fs::set_permissions(&ssh, fs::Permissions::from_mode(0o700)).unwrap();
    fs::copy(&fixture.config, ssh.join("config")).unwrap();
    fs::set_permissions(ssh.join("config"), fs::Permissions::from_mode(0o600)).unwrap();
    let settings = fixture.root().join("settings");
    fs::create_dir_all(&settings).unwrap();
    fs::write(
        settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='machine'\nurl='{}'\nindex_url='none'\n",
            fixture.address()
        ),
    )
    .unwrap();
    let project = fixture.root().join("machine-ssh");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        // An unrelated exclusion selects the controlled transport, whose SSH
        // capture reads `$HOME/.ssh/config` explicitly. Plain OpenSSH (the
        // policy-free path) resolves `~` from the passwd entry, not `$HOME`,
        // so an isolated test HOME could not stand in for it.
        "[project]\nname='consumer'\nversion='1.0.0'\n[source_policy]\nexclude_hosting=['host:127.0.0.2']\n[requires.packages]\n'org.fixture/demo'='1.0.0'\n",
    )
    .unwrap();
    let ordinary = |command: &mut Command| {
        command.env_remove("GIT_SSH_COMMAND");
    };
    let mut install = fixture.command(&project);
    ordinary(&mut install);
    let output = install.args(["--unattended", "install"]).output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let lock = fs::read(project.join("vibe.lock")).unwrap();
    assert!(
        String::from_utf8_lossy(&lock).contains("ordinary-fixture"),
        "the lock records the SSH registry source"
    );
    let contacts = fixture.contacts();
    assert!(contacts > 0, "the install reached the SSH registry");

    // Switch the machine registry to an unrelated HTTPS address: the locked
    // package replays its locked source from the verified store, with no new
    // contact and a byte-identical lock.
    fs::write(
        settings.join("registry.toml"),
        "[[registry]]\nname='machine'\nurl='https://unused.invalid/org'\nindex_url='none'\n",
    )
    .unwrap();
    let mut replay = fixture.command(&project);
    ordinary(&mut replay);
    let output = replay.args(["--unattended", "install"]).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(project.join("vibe.lock")).unwrap(), lock);
    assert_eq!(fixture.contacts(), contacts);
}
