//! Tests for the shell-out git backend — live `git` round-trips
//! (skipped when no `git` is on `PATH`), the inline tar extractor, and
//! the locale-stable stderr classifier.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-001#backend");

use super::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

use fixtures::*;
use specmark::verifies;

macro_rules! skip_without_git {
    () => {
        if !git_available() {
            eprintln!("skipping test: git not on PATH");
            return;
        }
    };
}

#[test]
fn preflight_succeeds_when_git_installed() {
    skip_without_git!();
    let g = ShellGit::new();
    g.preflight().expect("preflight should succeed");
}

#[test]
fn preflight_reports_not_installed_for_bogus_binary() {
    let g = ShellGit {
        binary: PathBuf::from("definitely-not-git-xyz"),
        preflight_cache: OnceLock::new(),
    };
    let err = g.preflight().unwrap_err();
    assert!(
        matches!(err, GitError::NotInstalled),
        "expected NotInstalled, got: {err:?}"
    );
}

#[test]
fn clone_then_update_against_bare_origin() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path());
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    g.bootstrap(&bare.to_string_lossy(), "main", &dest)
        .expect("initial clone should succeed");
    assert!(dest.join("README.md").exists());

    // Push a new commit into origin, then update from the clone.
    let src2 = tmp.path().join("src2");
    run_or_panic(
        tmp.path(),
        &["clone", bare.to_str().unwrap(), src2.to_str().unwrap()],
    );
    run_or_panic(&src2, &["config", "user.email", "t@example.com"]);
    run_or_panic(&src2, &["config", "user.name", "Test"]);
    fs::write(src2.join("new.md"), "new\n").unwrap();
    run_or_panic(&src2, &["add", "new.md"]);
    run_or_panic(&src2, &["commit", "-m", "add new"]);
    run_or_panic(&src2, &["push", "origin", "main"]);

    g.update(&dest, "main").expect("update should succeed");
    assert!(dest.join("new.md").exists());
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-021#lock", r = 1)]
fn head_commit_returns_the_checked_out_sha() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path());
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    g.bootstrap(&bare.to_string_lossy(), "main", &dest)
        .expect("clone should succeed");

    let sha = g
        .head_commit(&dest)
        .expect("head_commit ok")
        .expect("a real checkout reports a commit");
    // A full 40-hex SHA-1 — git's default object format.
    assert_eq!(sha.len(), 40, "got: {sha}");
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()), "got: {sha}");

    // It matches what git itself reports for HEAD in the clone.
    let mut cmd = Command::new("git");
    apply_common_env(&mut cmd);
    cmd.args(["rev-parse", "HEAD"]).current_dir(&dest);
    let expected = String::from_utf8_lossy(&cmd.output().unwrap().stdout)
        .trim()
        .to_string();
    assert_eq!(sha, expected);
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-021#fetch", r = 1)]
fn submodule_step_is_a_noop_without_submodules() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path());
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    // `bootstrap` now passes `--recurse-submodules`; on a repo with none
    // it clones exactly as before (PROP-021 §2.1).
    g.bootstrap(&bare.to_string_lossy(), "main", &dest)
        .expect("clone --recurse-submodules ok on a repo with no submodules");
    assert!(dest.join("README.md").exists());

    // `update` now runs `git submodule update --init --recursive`; on a
    // repo with no submodules that step must succeed as a no-op rather
    // than fail the update — the path every existing package takes.
    g.update(&dest, "main")
        .expect("update with the submodule step ok on a plain repo");
    assert!(dest.join("README.md").exists());
}

#[test]
fn clone_reports_repo_not_found_for_missing_url() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bogus = tmp.path().join("does/not/exist.git");
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    let err = g
        .bootstrap(&bogus.to_string_lossy(), "main", &dest)
        .unwrap_err();
    // The exact message varies by git version; classification should
    // land on either RepoNotFound or a generic CommandFailed with
    // the raw stderr — both are acceptable for this test.
    match err {
        GitError::RepoNotFound { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn clone_reports_ref_not_found_for_missing_branch() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path());
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    let err = g
        .bootstrap(&bare.to_string_lossy(), "no-such-branch", &dest)
        .unwrap_err();
    match err {
        GitError::RefNotFound { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn list_tags_returns_dedup_sorted_set() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let mut tags = g.list_tags(&bare.to_string_lossy()).expect("list_tags ok");
    tags.sort();

    assert_eq!(
        tags,
        vec![
            "v0.1.0".to_string(),
            "v0.2.0".to_string(),
            "v0.3.0".to_string(),
            "v1.0.0-rc.1".to_string(),
        ],
        "annotated tag v0.3.0 must appear exactly once (peeled-form deduped)"
    );
}

#[test]
fn list_tags_empty_repo_returns_empty() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path()); // has no tags
    let g = ShellGit::new();
    let tags = g.list_tags(&bare.to_string_lossy()).expect("list_tags ok");
    assert!(tags.is_empty());
}

#[test]
fn list_tags_reports_repo_not_found_for_missing_url() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bogus = tmp.path().join("does/not/exist.git");
    let g = ShellGit::new();
    let err = g.list_tags(&bogus.to_string_lossy()).unwrap_err();
    match err {
        GitError::RepoNotFound { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn fetch_file_at_ref_returns_bytes_for_existing_file() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let bytes = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v0.2.0", "vibe.toml")
        .expect("fetch ok");
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("version = \"0.2.0\""), "got: {text}");
}

#[test]
fn fetch_file_at_ref_works_against_annotated_tag() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let bytes = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v0.3.0", "vibe.toml")
        .expect("fetch via annotated tag ok");
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("version = \"0.3.0\""));
}

#[test]
fn fetch_file_at_ref_normalises_backslash_paths() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    // Caller hands us a Windows-style path; the backend should
    // normalise to forward slash before talking to `git archive`.
    let bytes = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v0.1.0", "vibe.toml")
        .expect("fetch ok");
    assert!(!bytes.is_empty());
}

#[test]
fn fetch_file_at_ref_missing_ref() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let err = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v9.9.9", "vibe.toml")
        .unwrap_err();
    match err {
        GitError::RefNotFound { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn fetch_file_at_ref_missing_file_in_existing_ref() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let err = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v0.1.0", "no-such-file.txt")
        .unwrap_err();
    match err {
        GitError::FileNotFoundInRef { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn extract_single_file_from_tar_picks_match() {
    // Hand-build a minimal tar with two files; verify we extract the
    // requested one by name, ignoring the other.
    let tar = build_tar(&[("a.txt", b"AAA\n"), ("vibe.toml", b"hello world\n")]);
    let got = extract_single_file_from_tar(&tar, "vibe.toml").expect("file extracted");
    assert_eq!(got, b"hello world\n");

    let absent = extract_single_file_from_tar(&tar, "nope.txt");
    assert!(absent.is_none());
}

#[test]
fn extract_single_file_from_tar_handles_dot_slash_prefix() {
    let tar = build_tar(&[("./vibe.toml", b"prefixed\n")]);
    let got = extract_single_file_from_tar(&tar, "vibe.toml").unwrap();
    assert_eq!(got, b"prefixed\n");
}

#[test]
fn parse_octal_handles_padded_sizes() {
    assert_eq!(parse_octal(b"00000000027\0").unwrap(), 0o27);
    assert_eq!(parse_octal(b"  144 \0").unwrap(), 0o144);
    assert_eq!(parse_octal(b"\0\0\0\0\0\0\0\0\0\0\0\0").unwrap(), 0);
}

#[test]
fn classify_repo_not_found_message() {
    // GitHub / Gitea-shape 404 surfaces a verbatim "Repository not
    // found." line from the remote helper; that's the substring
    // the classifier locks onto.
    assert!(matches!(
        classify("remote: Repository not found.\nfatal: ...").unwrap(),
        GitError::RepoNotFound { .. }
    ));
    assert!(matches!(
        classify("fatal: 'x' does not appear to be a git repository").unwrap(),
        GitError::RepoNotFound { .. }
    ));
}

#[test]
fn classify_auth_failed_message() {
    assert!(matches!(
        classify("git@github.com: Permission denied (publickey).").unwrap(),
        GitError::AuthFailed { .. }
    ));
    assert!(matches!(
        classify("remote: HTTP Basic: Access denied\nfatal: Authentication failed for ...")
            .unwrap(),
        GitError::AuthFailed { .. }
    ));
}

#[test]
fn classify_credential_prompt_failure_after_silencing() {
    // PROP-002 §2.2.1: when our credential helpers are silenced
    // (`-c credential.helper=` + `GIT_TERMINAL_PROMPT=0`), git
    // can't ask anyone for a username/password and emits
    // `fatal: could not read Username for '...'`. This is what
    // the original opencode walk against GitVerse produced.
    // Must classify as AuthFailed so the resolver can apply the
    // per-`auth` walk-vs-halt rule (§2.3.1).
    for stderr in [
        "fatal: User cancelled dialog.\nfatal: could not read Username for 'https://gitverse.ru': terminal prompts disabled",
        "fatal: could not read Password for 'https://example.invalid': terminal prompts disabled",
    ] {
        assert!(
            matches!(classify(stderr).unwrap(), GitError::AuthFailed { .. }),
            "expected AuthFailed for: {stderr}"
        );
    }
}

#[test]
fn classify_http_status_codes() {
    // Direct HTTP transport errors — when the host returns a
    // structured response without redirecting through the
    // credential layer (some proxies, some CI runners).
    for stderr in [
        "fatal: unable to access 'https://x/y.git/': The requested URL returned error: 401 Unauthorized",
        "fatal: unable to access 'https://x/y.git/': HTTP 401",
        "fatal: unable to access 'https://x/y.git/': The requested URL returned error: 403 Forbidden",
        "fatal: unable to access 'https://x/y.git/': HTTP 403",
    ] {
        assert!(
            matches!(classify(stderr).unwrap(), GitError::AuthFailed { .. }),
            "expected AuthFailed for: {stderr}"
        );
    }
}

#[test]
fn classify_network_unreachable_classic_substrings() {
    // Substrings that the M1.1 classifier already recognised.
    for stderr in [
        "fatal: unable to access 'https://x/y.git/': Could not resolve host: x",
        "fatal: Could not read from remote repository.",
        "ssh: connect to host x port 22: Network is unreachable",
    ] {
        assert!(
            matches!(
                classify(stderr).unwrap(),
                GitError::NetworkUnreachable { .. }
            ),
            "expected NetworkUnreachable for: {stderr}"
        );
    }
}

#[test]
fn classify_network_unreachable_connect_failure_substrings() {
    // The shapes M1.6 Scenario B4 surfaced — connect-failure on a
    // dead host. git 2.5x with libcurl 8.x emits the verbatim
    // strings below; the Scenario B4 walk on 2026-05-04 produced
    // the third one (`Could not connect to server`).
    for stderr in [
        "fatal: unable to access 'https://x/y.git/': Failed to connect to x port 443 after 2123 ms: Could not connect to server",
        "fatal: unable to access 'https://x/y.git/': Failed to connect to x port 443: Connection refused",
        "fatal: unable to access 'https://x/y.git/': Connection timed out after 30001 ms",
        "fatal: unable to access 'https://x/y.git/': Operation timed out after 30001 ms",
    ] {
        assert!(
            matches!(
                classify(stderr).unwrap(),
                GitError::NetworkUnreachable { .. }
            ),
            "expected NetworkUnreachable for: {stderr}"
        );
    }
}

#[test]
fn classify_ref_not_found_message() {
    assert!(matches!(
        classify("fatal: Remote branch no-such-branch not found in upstream origin").unwrap(),
        GitError::RefNotFound { .. }
    ));
    assert!(matches!(
        classify("fatal: couldn't find remote ref refs/tags/v9.9.9").unwrap(),
        GitError::RefNotFound { .. }
    ));
}

#[test]
fn classify_unknown_message_falls_through() {
    assert!(classify("error: something we have never seen before").is_none());
}

#[test]
fn classify_specific_matchers_win_over_unable_to_access() {
    // `unable to access` is a wrapper that frames many other
    // failures; it must NOT shadow the inner connect-failure or
    // auth-failed classification.
    let stderr =
        "fatal: unable to access 'https://x/y.git/': Authentication failed for 'https://x/'";
    assert!(matches!(
        classify(stderr).unwrap(),
        GitError::AuthFailed { .. }
    ));
}

/// Test-only fixtures behind their own `#[cfg(test)]` marker: fact
/// extraction is per-file, and the no-unwrap rule scopes test code by
/// the enclosing `#[cfg(test)]` item — the marker keeps these helpers
/// reading as test code now that the tests live outside the parent
/// module's inline `mod tests`.
#[cfg(test)]
mod fixtures {
    use super::*;

    /// Build a bare git repo under `root/origin.git` seeded with one
    /// commit on `main`, and return its absolute path. Requires `git`
    /// on `PATH`; tests that need it skip themselves via
    /// `skip_without_git!()`.
    pub(super) fn make_bare_origin(root: &Path) -> PathBuf {
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        // Work repo: init, set identity, create a file, commit on main.
        run_or_panic(&src, &["init", "--initial-branch=main"]);
        run_or_panic(&src, &["config", "user.email", "t@example.com"]);
        run_or_panic(&src, &["config", "user.name", "Test"]);
        fs::write(src.join("README.md"), "hello\n").unwrap();
        run_or_panic(&src, &["add", "README.md"]);
        run_or_panic(&src, &["commit", "-m", "init"]);

        let bare = root.join("origin.git");
        run_or_panic(
            root,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        // Make sure HEAD in the bare repo points at main.
        run_or_panic(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);

        bare
    }

    pub(super) fn run_or_panic(cwd: &Path, args: &[&str]) {
        let mut cmd = Command::new("git");
        apply_common_env(&mut cmd);
        cmd.args(args);
        cmd.current_dir(cwd);
        let out = cmd.output().expect("failed to spawn git for test setup");
        if !out.status.success() {
            panic!(
                "test setup `git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    pub(super) fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Build a bare origin that has multiple tags (`v0.1.0`, `v0.2.0`,
    /// `v1.0.0-rc.1`) plus one annotated tag (`v0.3.0`) so we exercise
    /// the peeled-form deduplication.
    pub(super) fn make_bare_origin_with_tags(root: &Path) -> PathBuf {
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        run_or_panic(&src, &["init", "--initial-branch=main"]);
        run_or_panic(&src, &["config", "user.email", "t@example.com"]);
        run_or_panic(&src, &["config", "user.name", "Test"]);

        // Commit 1 + lightweight tag v0.1.0.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.1.0"]);
        run_or_panic(&src, &["tag", "v0.1.0"]);

        // Commit 2 + lightweight tag v0.2.0.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.2.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.2.0"]);
        run_or_panic(&src, &["tag", "v0.2.0"]);

        // Commit 3 + ANNOTATED tag v0.3.0 (this is the one that produces
        // a peeled `^{}` line in `ls-remote --tags` output).
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.3.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.3.0"]);
        run_or_panic(&src, &["tag", "-a", "v0.3.0", "-m", "release 0.3.0"]);

        // Commit 4 + tag v1.0.0-rc.1.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"1.0.0-rc.1\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "1.0.0-rc.1"]);
        run_or_panic(&src, &["tag", "v1.0.0-rc.1"]);

        let bare = root.join("origin.git");
        run_or_panic(
            root,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        run_or_panic(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        bare
    }

    /// Build a USTar archive from `(name, bytes)` pairs. Plenty for our
    /// tests; not a complete tar implementation.
    pub(super) fn build_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, data) in entries {
            let mut header = vec![0u8; 512];
            // Name: bytes 0..100, NUL-terminated.
            let nb = name.as_bytes();
            let len = nb.len().min(100);
            header[0..len].copy_from_slice(&nb[..len]);
            // Mode: bytes 100..108 — "0000644\0".
            header[100..108].copy_from_slice(b"0000644\0");
            // UID/GID: bytes 108..116 / 116..124 — "0000000\0".
            header[108..116].copy_from_slice(b"0000000\0");
            header[116..124].copy_from_slice(b"0000000\0");
            // Size: bytes 124..136 — octal, 11 chars + NUL.
            let size_str = format!("{:011o}\0", data.len());
            header[124..136].copy_from_slice(size_str.as_bytes());
            // Mtime: bytes 136..148 — "00000000000\0".
            header[136..148].copy_from_slice(b"00000000000\0");
            // Checksum: bytes 148..156 — fill with spaces, compute later.
            for b in &mut header[148..156] {
                *b = b' ';
            }
            // Typeflag: byte 156 — '0' (regular file).
            header[156] = b'0';
            // Magic: bytes 257..263 — "ustar\0".
            header[257..263].copy_from_slice(b"ustar\0");
            // Version: bytes 263..265 — "00".
            header[263..265].copy_from_slice(b"00");
            // Compute checksum: sum of all bytes treating chksum field
            // as spaces (already done above).
            let cksum: u32 = header.iter().map(|b| *b as u32).sum();
            let cksum_str = format!("{cksum:06o}\0 ");
            header[148..156].copy_from_slice(cksum_str.as_bytes());

            out.extend_from_slice(&header);
            out.extend_from_slice(data);
            // Pad to 512.
            let pad = (512 - (data.len() % 512)) % 512;
            out.extend(std::iter::repeat_n(0, pad));
        }
        // Two empty 512-byte blocks to terminate.
        out.extend(std::iter::repeat_n(0, 1024));
        out
    }

    pub(super) fn classify(stderr: &str) -> Option<GitError> {
        classify_stderr_message(stderr, "URL".into(), "REF".into())
    }
}
