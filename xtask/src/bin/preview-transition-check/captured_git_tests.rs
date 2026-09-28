//! Actual captured-query policy with owned repositories and isolated child envs.

use super::{capture, fs};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CAPTURE-GIT-READ-BOUNDARY");

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn startup(command: &mut Command) {
    command.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}

struct Scratch {
    root: PathBuf,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "captured-git-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }

    // Fixture setup/control only. This is never a production captured-query path.
    fn setup(&self, repo: &Path, args: &[&str]) -> Output {
        let mut command = Command::new("git");
        startup(&mut command);
        let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
        command.envs([
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_CONFIG_GLOBAL", null),
            ("GIT_CONFIG_SYSTEM", null),
            ("LC_ALL", "C"),
            ("LANG", "C"),
            ("GIT_TERMINAL_PROMPT", "0"),
        ]);
        command.arg("-C").arg(repo).args(args).output().unwrap()
    }

    fn git(&self, repo: &Path, args: &[&str]) -> String {
        let output = self.setup(repo, args);
        assert!(
            output.status.success(),
            "fixture Git {:?}: {:?}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }

    fn repo(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let repo = self.root.join(name);
        fs::create_dir(&repo).unwrap();
        self.git(&repo, &["init", "--quiet", "--object-format=sha1"]);
        self.git(&repo, &["config", "user.name", "Fixture"]);
        self.git(&repo, &["config", "user.email", "fixture@example.invalid"]);
        self.git(&repo, &["config", "commit.gpgsign", "false"]);
        fs::write(repo.join("payload.txt"), bytes).unwrap();
        self.git(&repo, &["add", "payload.txt"]);
        self.git(&repo, &["commit", "--quiet", "-m", "fixture"]);
        repo
    }
}

fn read(repo: &Path, args: &[&str]) -> Output {
    capture::git_output(repo, args)
        .unwrap_or_else(|error| panic!("captured runner: {}", error.code))
}

fn child(test: &str, kind: &str, selected: &Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    startup(&mut command);
    command.args(["--exact", test, "--nocapture"]);
    command
        .env("CAPTURED_GIT_CHILD", kind)
        .env("CAPTURED_GIT_SELECTED", selected);
    command
}

fn selected() -> PathBuf {
    PathBuf::from(std::env::var_os("CAPTURED_GIT_SELECTED").unwrap())
}

#[test]
fn polluted_environment_cannot_select_another_repository_or_config() {
    if std::env::var("CAPTURED_GIT_CHILD").as_deref() == Ok("polluted") {
        let repo = selected();
        let output = read(&repo, &["rev-parse", "--show-toplevel"]);
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            repo.to_str().unwrap()
        );
        let output = read(&repo, &["rev-parse", "HEAD"]);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            std::env::var("CAPTURED_GIT_COMMIT").unwrap()
        );
        let output = read(&repo, &["show", "HEAD:payload.txt"]);
        assert!(output.status.success());
        assert_eq!(output.stdout, b"selected\n");
        assert!(
            !read(&repo, &["config", "--get", "test.poison"])
                .status
                .success()
        );
        for key in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_COMMON_DIR",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_NAMESPACE",
            "GIT_EXEC_PATH",
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_ASKPASS",
            "GIT_SSH_COMMAND",
            "HTTPS_PROXY",
            "CAPTURED_GIT_POISON",
        ] {
            let flag = format!("--config-env=test.probe={key}");
            assert!(
                !read(&repo, &[&flag, "config", "--get", "test.probe"])
                    .status
                    .success(),
                "inherited {key}"
            );
        }
        // Git re-exports code-owned -c settings internally. Its presence is
        // legitimate; inherited synthetic config and effective policy decide.
        let output = read(&repo, &["config", "--get", "protocol.allow"]);
        assert_eq!(output.stdout, b"never\n");
        let output = read(
            &repo,
            &[
                "--config-env=test.probe=GIT_CONFIG_PARAMETERS",
                "config",
                "--get",
                "test.probe",
            ],
        );
        assert!(output.status.success());
        let parameters = String::from_utf8(output.stdout).unwrap();
        assert!(parameters.contains("protocol.allow"));
        assert!(!parameters.contains("test.poison"));
        let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
        for (key, expected) in [
            ("LC_ALL", "C"),
            ("LANG", "C"),
            ("GIT_NO_LAZY_FETCH", "1"),
            ("GIT_NO_REPLACE_OBJECTS", "1"),
            ("GIT_OPTIONAL_LOCKS", "0"),
            ("GIT_TERMINAL_PROMPT", "0"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_CONFIG_GLOBAL", null),
            ("GIT_CONFIG_SYSTEM", null),
            ("GIT_GRAFT_FILE", null),
        ] {
            let flag = format!("--config-env=test.probe={key}");
            let output = read(&repo, &[&flag, "config", "--get", "test.probe"]);
            assert!(output.status.success());
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
        }
        return;
    }
    let scratch = Scratch::new();
    let repo = scratch.repo("selected", b"selected\n");
    let other = scratch.repo("other", b"other\n");
    let poison = scratch.root.join("poison.config");
    fs::write(&poison, "[test]\npoison = inherited\n").unwrap();
    let mut command = child(
        "captured_git_tests::polluted_environment_cannot_select_another_repository_or_config",
        "polluted",
        &repo,
    );
    command.env(
        "CAPTURED_GIT_COMMIT",
        scratch.git(&repo, &["rev-parse", "HEAD"]),
    );
    for (key, value) in [
        ("GIT_DIR", other.join(".git")),
        ("GIT_WORK_TREE", other.clone()),
        ("GIT_COMMON_DIR", other.join(".git")),
        ("GIT_INDEX_FILE", other.join(".git/index")),
        ("GIT_OBJECT_DIRECTORY", other.join(".git/objects")),
        (
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            other.join(".git/objects"),
        ),
        ("GIT_CONFIG_GLOBAL", poison.clone()),
        ("GIT_CONFIG_SYSTEM", poison.clone()),
        ("GIT_CONFIG", poison),
    ] {
        command.env(key, value);
    }
    for (key, value) in [
        ("GIT_NAMESPACE", "wrong-namespace"),
        ("GIT_EXEC_PATH", "missing-exec-dir"),
        ("GIT_CONFIG_COUNT", "1"),
        ("GIT_CONFIG_KEY_0", "test.poison"),
        ("GIT_CONFIG_VALUE_0", "injected"),
        ("GIT_CONFIG_PARAMETERS", "'test.poison=injected'"),
        ("GIT_ASKPASS", "missing-askpass"),
        ("GIT_SSH_COMMAND", "missing-ssh"),
        ("HTTPS_PROXY", "http://127.0.0.1:9"),
        ("CAPTURED_GIT_POISON", "synthetic"),
        ("GIT_NO_LAZY_FETCH", "0"),
        ("GIT_NO_REPLACE_OBJECTS", "0"),
        ("GIT_OPTIONAL_LOCKS", "1"),
        ("GIT_TERMINAL_PROMPT", "1"),
        ("LC_ALL", "invalid-fixture-locale"),
        ("LANG", "invalid-fixture-locale"),
    ] {
        command.env(key, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "policy child stdout: {:?}; stderr: {:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn replacement_and_graft_history_cannot_change_captured_tree_or_ancestry() {
    let scratch = Scratch::new();
    let repo = scratch.repo("source", b"root\n");
    let root = scratch.git(&repo, &["rev-parse", "HEAD"]);
    let root_tree = scratch.git(&repo, &["rev-parse", "HEAD^{tree}"]);
    fs::write(repo.join("payload.txt"), b"child\n").unwrap();
    scratch.git(&repo, &["add", "payload.txt"]);
    scratch.git(&repo, &["commit", "--quiet", "-m", "child"]);
    let commit = scratch.git(&repo, &["rev-parse", "HEAD"]);
    let tree = scratch.git(&repo, &["rev-parse", "HEAD^{tree}"]);
    let orphan = scratch.git(&repo, &["commit-tree", &root_tree, "-m", "replacement"]);
    scratch.git(&repo, &["replace", &commit, &orphan]);
    assert_eq!(
        scratch.git(&repo, &["rev-parse", &format!("{commit}^{{tree}}")]),
        root_tree
    );
    assert!(
        !scratch
            .setup(&repo, &["merge-base", "--is-ancestor", &root, &commit])
            .status
            .success()
    );
    let output = read(&repo, &["rev-parse", &format!("{commit}^{{tree}}")]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), tree);
    assert!(
        read(&repo, &["merge-base", "--is-ancestor", &root, &commit])
            .status
            .success()
    );
    scratch.git(&repo, &["replace", "-d", &commit]);
    fs::write(repo.join(".git/info/grafts"), format!("{commit}\n")).unwrap();
    assert!(
        !scratch
            .setup(&repo, &["merge-base", "--is-ancestor", &root, &commit])
            .status
            .success()
    );
    assert!(
        read(&repo, &["merge-base", "--is-ancestor", &root, &commit])
            .status
            .success()
    );
}

#[test]
fn bracket_filenames_are_literal_pathspecs_and_stdout_remains_nul_delimited() {
    let scratch = Scratch::new();
    let repo = scratch.repo("source", b"root\n");
    let before = scratch.git(&repo, &["rev-parse", "HEAD"]);
    for path in ["page[1].xml", "page1.xml"] {
        fs::write(repo.join(path), b"page\n").unwrap();
        scratch.git(&repo, &["add", "--", path]);
    }
    scratch.git(&repo, &["commit", "--quiet", "-m", "pages"]);
    let commit = scratch.git(&repo, &["rev-parse", "HEAD"]);
    let control = scratch.setup(
        &repo,
        &[
            "diff-tree",
            "-r",
            "-z",
            "--name-only",
            "--no-commit-id",
            &before,
            &commit,
            "--",
            "page[1].xml",
        ],
    );
    assert_eq!(control.stdout, b"page1.xml\0page[1].xml\0");
    let output = read(
        &repo,
        &[
            "diff-tree",
            "-r",
            "-z",
            "--name-only",
            "--no-commit-id",
            &before,
            &commit,
            "--",
            "page[1].xml",
        ],
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"page[1].xml\0");
}

#[cfg(unix)]
#[test]
fn missing_promisor_blob_does_not_invoke_local_fetch_helper() {
    if std::env::var("CAPTURED_GIT_CHILD").as_deref() == Ok("promisor") {
        let repo = selected();
        let blob = std::env::var("CAPTURED_GIT_BLOB").unwrap();
        let marker = PathBuf::from(std::env::var_os("CAPTURED_GIT_MARKER").unwrap());
        let output = read(&repo, &["cat-file", "blob", &blob]);
        assert!(!output.status.success());
        assert!(
            !marker.exists(),
            "GIT_NO_LAZY_FETCH failed: local helper was invoked"
        );
        let error = capture::git(&repo, &["cat-file", "blob", &blob])
            .err()
            .unwrap();
        assert_eq!(error.code, "git-object-missing");
        assert!(!marker.exists(), "wrapper invoked local fetch helper");
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new();
    let repo = scratch.repo("partial", b"promised blob\n");
    let blob = scratch.git(&repo, &["rev-parse", "HEAD:payload.txt"]);
    scratch.git(&repo, &["config", "core.repositoryformatversion", "1"]);
    scratch.git(&repo, &["config", "extensions.partialclone", "origin"]);
    scratch.git(&repo, &["config", "remote.origin.url", "probe::fixture"]);
    scratch.git(&repo, &["config", "remote.origin.promisor", "true"]);
    scratch.git(
        &repo,
        &["config", "remote.origin.partialclonefilter", "blob:none"],
    );
    scratch.git(&repo, &["config", "protocol.probe.allow", "always"]);
    fs::remove_file(repo.join(".git/objects").join(&blob[..2]).join(&blob[2..])).unwrap();
    let bin = scratch.root.join("bin");
    fs::create_dir(&bin).unwrap();
    let marker = scratch.root.join("fetch-spy-called");
    let script = bin.join("git-remote-probe");
    let quoted = format!("'{}'", marker.to_string_lossy().replace('\'', "'\\''"));
    fs::write(
        &script,
        format!("#!/bin/sh\nprintf 'called\\n' >> {quoted}\nexit 1\n"),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let mut path = OsString::from(&bin);
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap());
    let mut command = child(
        "captured_git_tests::missing_promisor_blob_does_not_invoke_local_fetch_helper",
        "promisor",
        &repo,
    );
    command
        .env("PATH", &path)
        .env("CAPTURED_GIT_BLOB", &blob)
        .env("CAPTURED_GIT_MARKER", &marker);
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "no-lazy child: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!marker.exists());
    // Specific local allow deliberately beats protocol.allow=never: the control
    // must invoke the spy, making the no-lazy environment the decisive fence.
    let mut control = Command::new("git");
    startup(&mut control);
    control
        .env("PATH", path)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0");
    let output = control
        .args(["-c", "protocol.allow=never", "-C"])
        .arg(&repo)
        .args(["cat-file", "blob", &blob])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        marker.exists(),
        "promisor control did not invoke the local helper; no-fetch proof is inconclusive"
    );
}
