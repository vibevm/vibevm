use super::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::tempdir;

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_git(cwd: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed in {}\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        cwd.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn init_bare(root: &Path) -> std::path::PathBuf {
    let bare = root.join("origin.git");
    run_git(root, &["init", "--bare", bare.to_str().unwrap()]);
    bare
}

fn write_package(source: &Path, protocol: &str) {
    fs::write(
        source.join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::create_dir_all(source.join("spec")).unwrap();
    fs::write(source.join("spec/PROTOCOL.md"), protocol).unwrap();
}

fn git_rev_parse(repo: &Path, reference: &str) -> String {
    String::from_utf8_lossy(&run_git(repo, &["rev-parse", reference]).stdout)
        .trim()
        .to_string()
}

#[test]
fn copy_dir_skips_dot_git_subtrees() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    fs::create_dir_all(src.path().join(".git/objects")).unwrap();
    fs::write(src.path().join(".git/HEAD"), "ref: refs/heads/main").unwrap();
    fs::write(src.path().join("README.md"), "hi").unwrap();
    fs::write(
        src.path().join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    copy_dir(src.path(), dst.path()).unwrap();

    assert!(dst.path().join("README.md").exists());
    assert!(dst.path().join("vibe.toml").exists());
    assert!(!dst.path().join(".git").exists());
    assert!(!dst.path().join(".git/HEAD").exists());
}

#[test]
fn push_release_against_local_bare_origin() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    // Build a bare origin we can push to.
    let outer = tempdir().unwrap();
    let bare = outer.path().join("origin.git");
    let init_status = Command::new("git")
        .args(["init", "--bare", bare.to_str().unwrap()])
        .env("LC_ALL", "C")
        .status()
        .unwrap();
    assert!(init_status.success());

    // Build a fake source dir with a manifest + spec file.
    let src = tempdir().unwrap();
    fs::write(
        src.path().join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::create_dir_all(src.path().join("spec")).unwrap();
    fs::write(src.path().join("spec/PROTOCOL.md"), "...").unwrap();

    let url = bare.to_string_lossy().into_owned();
    let v = semver::Version::parse("0.1.0").unwrap();
    push_release(src.path(), &url, "v0.1.0", "wal", &v).expect("push ok");

    // Inspect the bare repo: tag and main branch should both be there.
    let tags = Command::new("git")
        .args(["-C", bare.to_str().unwrap(), "tag", "--list"])
        .env("LC_ALL", "C")
        .output()
        .unwrap();
    let tag_list = String::from_utf8_lossy(&tags.stdout);
    assert!(
        tag_list.contains("v0.1.0"),
        "expected v0.1.0 in tags, got: {tag_list}"
    );

    let branches = Command::new("git")
        .args(["-C", bare.to_str().unwrap(), "branch", "--list"])
        .env("LC_ALL", "C")
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&branches.stdout).contains("main"),
        "expected main branch in bare origin"
    );
}

#[test]
fn republish_moves_same_version_tag_preserves_history_and_removes_stale_files() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    let outer = tempdir().unwrap();
    let bare = init_bare(outer.path());
    let src = tempdir().unwrap();
    write_package(src.path(), "first\n");
    fs::write(src.path().join("stale.txt"), "remove me\n").unwrap();

    let url = bare.to_string_lossy().into_owned();
    let version = semver::Version::parse("0.1.0").unwrap();
    push_release(src.path(), &url, "v0.1.0", "wal", &version).unwrap();
    let first_main = git_rev_parse(&bare, "refs/heads/main");
    run_git(&bare, &["tag", "v0.0.9", &first_main]);

    fs::remove_file(src.path().join("stale.txt")).unwrap();
    fs::write(src.path().join("spec/PROTOCOL.md"), "second\n").unwrap();
    push_release(src.path(), &url, "v0.1.0", "wal", &version).unwrap();

    let second_main = git_rev_parse(&bare, "refs/heads/main");
    assert_ne!(
        second_main, first_main,
        "changed payload needs a new commit"
    );
    assert_eq!(
        git_rev_parse(&bare, "refs/heads/main^"),
        first_main,
        "republish must append to main history"
    );
    assert_eq!(
        git_rev_parse(&bare, "refs/tags/v0.1.0^{}"),
        second_main,
        "same-version tag must select the replacement commit"
    );
    assert_eq!(
        git_rev_parse(&bare, "refs/tags/v0.0.9"),
        first_main,
        "unrelated older version tags must not move"
    );

    let tree = String::from_utf8_lossy(
        &run_git(&bare, &["ls-tree", "-r", "--name-only", "refs/heads/main"]).stdout,
    )
    .lines()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    assert_eq!(tree, vec!["spec/PROTOCOL.md", "vibe.toml"]);
    assert_eq!(
        String::from_utf8_lossy(
            &run_git(&bare, &["show", "refs/heads/main:spec/PROTOCOL.md"]).stdout
        ),
        "second\n"
    );
}

#[test]
fn identical_republish_is_a_ref_and_history_noop() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    let outer = tempdir().unwrap();
    let bare = init_bare(outer.path());
    let src = tempdir().unwrap();
    write_package(src.path(), "same\n");
    let url = bare.to_string_lossy().into_owned();
    let version = semver::Version::parse("0.1.0").unwrap();

    push_release(src.path(), &url, "v0.1.0", "wal", &version).unwrap();
    let main_before = git_rev_parse(&bare, "refs/heads/main");
    let tag_before = git_rev_parse(&bare, "refs/tags/v0.1.0");
    push_release(src.path(), &url, "v0.1.0", "wal", &version).unwrap();

    assert_eq!(git_rev_parse(&bare, "refs/heads/main"), main_before);
    assert_eq!(git_rev_parse(&bare, "refs/tags/v0.1.0"), tag_before);
    let count = String::from_utf8_lossy(
        &run_git(&bare, &["rev-list", "--count", "refs/heads/main"]).stdout,
    )
    .trim()
    .parse::<usize>()
    .unwrap();
    assert_eq!(count, 1, "identical retry must not create another commit");
}

#[test]
fn concurrent_main_move_rejects_both_release_refs_atomically() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    let outer = tempdir().unwrap();
    let bare = init_bare(outer.path());
    let src = tempdir().unwrap();
    write_package(src.path(), "first\n");
    let url = bare.to_string_lossy().into_owned();
    let version = semver::Version::parse("0.1.0").unwrap();
    push_release(src.path(), &url, "v0.1.0", "wal", &version).unwrap();
    let original_tag = git_rev_parse(&bare, "refs/tags/v0.1.0");

    fs::write(src.path().join("spec/PROTOCOL.md"), "publisher change\n").unwrap();
    let competitor_parent = outer.path().to_path_buf();
    let competitor_root = outer.path().join("competitor");
    let bare_for_hook = bare.clone();
    let err = push_release_inner(src.path(), &url, "v0.1.0", "wal", &version, move |_| {
        run_git(
            &competitor_parent,
            &[
                "clone",
                "--branch=main",
                bare_for_hook.to_str().unwrap(),
                competitor_root.to_str().unwrap(),
            ],
        );
        run_git(
            &competitor_root,
            &["config", "user.email", "competitor@example.com"],
        );
        run_git(
            &competitor_root,
            &["config", "user.name", "Concurrent publisher"],
        );
        fs::write(competitor_root.join("competitor.txt"), "won\n").unwrap();
        run_git(&competitor_root, &["add", "-A"]);
        run_git(&competitor_root, &["commit", "-m", "concurrent publish"]);
        run_git(&competitor_root, &["push", "origin", "main"]);
        Ok(())
    })
    .expect_err("stale leases must reject the publish");

    assert!(
        matches!(&err, PublishError::ConcurrentUpdate { .. }),
        "expected concurrency classification, got: {err:?}"
    );
    assert_eq!(
        git_rev_parse(&bare, "refs/tags/v0.1.0"),
        original_tag,
        "atomic rejection must leave the version tag untouched"
    );
    assert_eq!(
        String::from_utf8_lossy(
            &run_git(&bare, &["show", "refs/heads/main:competitor.txt"]).stdout
        ),
        "won\n",
        "the competing main update must remain the only winning update"
    );
}

#[test]
fn concurrent_tag_move_rejects_main_and_tag_atomically() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    let outer = tempdir().unwrap();
    let bare = init_bare(outer.path());
    let src = tempdir().unwrap();
    write_package(src.path(), "first\n");
    let url = bare.to_string_lossy().into_owned();
    let version = semver::Version::parse("0.1.0").unwrap();
    push_release(src.path(), &url, "v0.1.0", "wal", &version).unwrap();
    let original_main = git_rev_parse(&bare, "refs/heads/main");

    fs::write(src.path().join("spec/PROTOCOL.md"), "publisher change\n").unwrap();
    let bare_for_hook = bare.clone();
    let err = push_release_inner(src.path(), &url, "v0.1.0", "wal", &version, move |_| {
        // Replace the annotated tag with a lightweight tag at the
        // same commit. Its ref value still changed, so the exact tag
        // lease must reject this publisher's otherwise valid update.
        run_git(&bare_for_hook, &["tag", "-f", "v0.1.0", "refs/heads/main"]);
        Ok(())
    })
    .expect_err("stale tag lease must reject the publish");

    assert!(
        matches!(&err, PublishError::ConcurrentUpdate { .. }),
        "expected concurrency classification, got: {err:?}"
    );
    assert_eq!(
        git_rev_parse(&bare, "refs/heads/main"),
        original_main,
        "atomic rejection must leave main untouched"
    );
    assert_eq!(
        git_rev_parse(&bare, "refs/tags/v0.1.0"),
        original_main,
        "the concurrent tag value must remain the winner"
    );
}

#[test]
fn release_publish_never_persists_target_url_in_git_config() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    let outer = tempdir().unwrap();
    let bare = init_bare(outer.path());
    let src = tempdir().unwrap();
    write_package(src.path(), "payload\n");
    let url = bare.to_string_lossy().into_owned();
    let inspected_url = url.clone();
    let version = semver::Version::parse("0.1.0").unwrap();

    push_release_inner(
        src.path(),
        &url,
        "v0.1.0",
        "wal",
        &version,
        move |staging| {
            let config = fs::read_to_string(staging.join(".git/config")).unwrap();
            assert!(!config.contains(&inspected_url));
            assert!(!config.contains("[remote \"origin\"]"));
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn redact_credentials_hides_user_info() {
    let url = "https://x-access-token:abcd1234@github.com/vibespecs/flow-wal.git";
    let scrubbed = redact_credentials(url);
    assert_eq!(
        scrubbed, "https://***@github.com/vibespecs/flow-wal.git",
        "credentials must be replaced with `***`"
    );
    assert!(!scrubbed.contains("abcd1234"));
}

#[test]
fn redact_credentials_passthrough_when_no_credentials() {
    let url = "https://github.com/vibespecs/flow-wal.git";
    assert_eq!(redact_credentials(url), url);
}

#[test]
fn redact_credentials_handles_ssh_no_scheme() {
    // SSH shorthand `git@host:path` does not match the userinfo
    // pattern (no `://`); pass-through is correct here because
    // this form has no embedded password to hide.
    let url = "git@github.com:vibespecs/flow-wal.git";
    assert_eq!(redact_credentials(url), url);
}

#[test]
fn redact_credentials_handles_ssh_scheme() {
    let url = "ssh://git@github.com/vibespecs/flow-wal.git";
    // `ssh://git@github.com/...` has user `git` but no password —
    // the helper still scrubs it to be safe (consistent with the
    // PROP-000 §20 "never any credential-like token in output"
    // posture). Operators that genuinely needed to see "git" can
    // read the registry URL from `vibe.toml`.
    assert_eq!(
        redact_credentials(url),
        "ssh://***@github.com/vibespecs/flow-wal.git"
    );
}

#[test]
fn redact_credentials_within_message() {
    let msg =
        "git remote add origin https://x-access-token:secret@github.com/foo/bar.git failed: oops";
    let scrubbed = redact_credentials(msg);
    assert!(!scrubbed.contains("secret"));
    assert!(scrubbed.contains("https://***@github.com/foo/bar.git"));
    assert!(scrubbed.contains("failed: oops"));
}

#[test]
fn redact_credentials_preserves_unicode_around_url() {
    let msg = "публикация https://user:secret@example.org/пакет завершена";
    assert_eq!(
        redact_credentials(msg),
        "публикация https://***@example.org/пакет завершена"
    );
}

#[test]
fn concurrency_classifier_does_not_hide_host_policy_or_atomic_capability_errors() {
    assert!(is_concurrent_ref_rejection(
        "! [rejected] main -> main (stale info)"
    ));
    assert!(!is_concurrent_ref_rejection(
        "fatal: the receiving end does not support --atomic push"
    ));
    assert!(!is_concurrent_ref_rejection(
        "remote: protected tag update failed; atomic push failed"
    ));
    assert!(!is_concurrent_ref_rejection(
        "remote rejected: cannot lock ref 'refs/tags/v1.0.0'"
    ));
}

#[test]
fn commit_and_push_lands_local_edit_on_bare_origin() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    // Build a bare origin, seed it via push_initial so it has a
    // `main` HEAD, then clone, edit a file, and exercise
    // commit_and_push. Mirrors the real workflow of
    // `vibe registry redirect-update`.
    let outer = tempdir().unwrap();
    let bare = outer.path().join("origin.git");
    let init_status = Command::new("git")
        .args(["init", "--bare", bare.to_str().unwrap()])
        .env("LC_ALL", "C")
        .status()
        .unwrap();
    assert!(init_status.success());

    let seed = tempdir().unwrap();
    fs::write(
        seed.path().join("vibe-redirect.toml"),
        "[redirect]\ntarget_url = \"https://example.invalid/v1\"\n",
    )
    .unwrap();
    let url = bare.to_string_lossy().into_owned();
    push_initial(seed.path(), &url, "stub: initial").expect("seed ok");

    let work = shallow_clone(&url).expect("clone ok");
    fs::write(
        work.path().join("vibe-redirect.toml"),
        "[redirect]\ntarget_url = \"https://example.invalid/v2\"\n",
    )
    .unwrap();

    commit_and_push(work.path(), &url, "stub: retarget to v2").expect("commit_and_push ok");

    // The bare origin must now carry two commits on main.
    let log = Command::new("git")
        .args(["-C", bare.to_str().unwrap(), "log", "--oneline", "main"])
        .env("LC_ALL", "C")
        .output()
        .unwrap();
    let log_out = String::from_utf8_lossy(&log.stdout);
    let lines: Vec<&str> = log_out.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "expected exactly two commits on main, got: {log_out}"
    );
    assert!(
        lines[0].contains("retarget"),
        "newest commit subject lost: {log_out}"
    );
}

#[test]
fn commit_and_push_refuses_when_working_tree_clean() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    let outer = tempdir().unwrap();
    let bare = outer.path().join("origin.git");
    let init_status = Command::new("git")
        .args(["init", "--bare", bare.to_str().unwrap()])
        .env("LC_ALL", "C")
        .status()
        .unwrap();
    assert!(init_status.success());

    let seed = tempdir().unwrap();
    fs::write(seed.path().join("file.txt"), "hi").unwrap();
    let url = bare.to_string_lossy().into_owned();
    push_initial(seed.path(), &url, "initial").expect("seed ok");

    let work = shallow_clone(&url).expect("clone ok");
    // No edits — working tree is clean against HEAD.
    let err = commit_and_push(work.path(), &url, "should fail").unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("nothing to commit"),
        "expected nothing-to-commit error, got: {msg}"
    );
}

#[test]
fn redact_credentials_multiple_urls_in_message() {
    let msg = "trying https://user:pw1@host.example/a then https://user:pw2@other.example/b done";
    let scrubbed = redact_credentials(msg);
    assert!(!scrubbed.contains("pw1"));
    assert!(!scrubbed.contains("pw2"));
    assert!(scrubbed.contains("https://***@host.example/a"));
    assert!(scrubbed.contains("https://***@other.example/b"));
}
