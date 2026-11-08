//! Actual isolated SSH metadata access, without a real user profile.
use super::*;

#[test]
fn cold_ssh_metadata_preview_cleans_scratch_and_never_installs_the_checkout() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    struct SshFixture {
        child: std::process::Child,
        data: Value,
    }
    impl Drop for SshFixture {
        fn drop(&mut self) {
            self.child.stdin.take();
            let _ = self.child.wait();
            if let Some(root) = self.data["root"].as_str() {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
    let mut child = Command::new("python3")
        .args([
            "-u",
            "-c",
            include_str!(
                "../../../vibe-registry/src/git_backend/shell/native_admission/ssh_fixture.py"
            ),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("isolated Git/OpenSSH fixture prerequisites");
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let ssh = SshFixture {
        child,
        data: serde_json::from_str(&line).expect("SSH fixture started"),
    };
    let fixture = Fixture::new();
    let client = fixture.home.join(".ssh");
    fs::create_dir_all(&client).unwrap();
    // An owned empty regular file, not a special file: captured SSH input
    // admission refuses device/special trust inputs by design.
    let global_known_hosts = client.join("global_known_hosts");
    fs::write(&global_known_hosts, "").unwrap();
    fs::write(client.join("config"), format!(
        "Host *\n IdentityFile {}\n IdentitiesOnly yes\n IdentityAgent none\n UserKnownHostsFile {}\n GlobalKnownHostsFile {}\n StrictHostKeyChecking yes\n BatchMode yes\n UpdateHostKeys no\n",
        ssh.data["key"].as_str().unwrap(), ssh.data["known_hosts"].as_str().unwrap(),
        global_known_hosts.display(),
    )).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(client.join("config"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&global_known_hosts, fs::Permissions::from_mode(0o600)).unwrap();
    let before = fixture.snapshot();
    let output = fixture
        .user
        .vibe()
        .args([
            "install",
            "org.fixture/demo",
            "--plan-only",
            "--json",
            "--git",
        ])
        .arg(ssh.data["url"].as_str().unwrap())
        .args([
            "--tag",
            "v1.0.0",
            "--git-auth",
            "ssh",
            "--no-default-registry",
            "--path",
        ])
        .arg(&fixture.project)
        .env("HOME", &fixture.home)
        .env("USERPROFILE", &fixture.home)
        .env("CODEX_HOME", fixture.home.join(".codex"))
        .env("CLAUDE_CONFIG_DIR", fixture.home.join(".claude"))
        .env("XDG_CONFIG_HOME", fixture.home.join(".config"))
        .env("XDG_DATA_HOME", fixture.home.join(".local/share"))
        .env("XDG_CACHE_HOME", fixture.home.join(".cache"))
        .env("XDG_STATE_HOME", fixture.home.join(".local/state"))
        .env("XDG_RUNTIME_DIR", fixture.home.join(".runtime"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("GIT_SSH")
        .env_remove("GIT_SSH_COMMAND")
        .env_remove("SSH_AUTH_SOCK")
        .env("TMPDIR", fixture.root.path().join("temporary"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(before, fixture.snapshot());
    let docs = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter()
        .collect::<Result<Vec<Value>, _>>()
        .unwrap();
    let rows = plan(&docs)["packages"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["package"], "org.fixture/demo");
    assert_eq!(rows[0]["version"], "1.0.0");
    assert_eq!(
        rows[0]["source_evidence"]["source_url"],
        vibe_core::progress::transfer::safe_remote(ssh.data["url"].as_str().unwrap())
    );
    let contacts =
        fs::read_to_string(Path::new(ssh.data["root"].as_str().unwrap()).join("sshd-a.log"))
            .unwrap()
            .matches("Connection from ")
            .count();
    assert!(
        contacts > 1,
        "fixture readiness alone is not metadata acquisition"
    );
}
