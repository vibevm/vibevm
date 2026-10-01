use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

fn git_binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .find_map(|dir| dir.join(name).canonicalize().ok().filter(|p| p.is_file()))
}

fn limits() -> CensusLimits {
    CensusLimits {
        max_index_bytes: 64 * 1024,
        max_probe_bytes: 4096,
        max_gitlinks: 8,
        max_path_bytes: 512,
    }
}

fn git(binary: &Path, cwd: &Path, args: &[&str]) -> String {
    let output = Command::new(binary)
        .current_dir(cwd)
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("HOME", cwd)
        .env("USERPROFILE", cwd)
        .env("LC_ALL", "C")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "local fixture git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

#[test]
fn non_repository_and_ordinary_repository_are_distinct() {
    let Some(binary) = git_binary() else {
        eprintln!("Git absent; local census fixture skipped");
        return;
    };
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let source = Project::open(root.path()).unwrap();
    assert_eq!(
        census(&source, &binary, limits()),
        GitIndexCensus::NoGitRepository
    );
    git(&binary, root.path(), &["init", "-q"]);
    git(&binary, root.path(), &["add", "vibe.toml"]);
    assert!(
        matches!(census(&source, &binary, limits()), GitIndexCensus::NoGitlinks { package_prefix, .. } if package_prefix.is_empty())
    );
}

#[cfg(unix)]
#[test]
fn failing_regular_executable_does_not_imply_no_git_repository() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let source = Project::open(root.path()).unwrap();
    assert!(matches!(
        census(&source, Path::new("/bin/false"), limits()),
        GitIndexCensus::Unavailable(_)
    ));
}

#[test]
fn monorepo_subtree_gitlink_populated_then_uninitialized() {
    let Some(binary) = git_binary() else {
        eprintln!("Git absent; local census fixture skipped");
        return;
    };
    let root = tempfile::tempdir().unwrap();
    git(&binary, root.path(), &["init", "-q"]);
    let package = root.path().join("packages/wal");
    let checkout = package.join("vendor/dep");
    fs::create_dir_all(&checkout).unwrap();
    fs::write(package.join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    git(&binary, &checkout, &["init", "-q"]);
    git(
        &binary,
        &checkout,
        &["config", "user.email", "fixture@local"],
    );
    git(&binary, &checkout, &["config", "user.name", "fixture"]);
    fs::write(checkout.join("lib.txt"), b"selected bytes\n").unwrap();
    git(&binary, &checkout, &["add", "lib.txt"]);
    git(&binary, &checkout, &["commit", "-qm", "fixture"]);
    let commit = git(&binary, &checkout, &["rev-parse", "HEAD"]);
    git(&binary, root.path(), &["add", "packages/wal/vibe.toml"]);
    let cacheinfo = format!("160000,{commit},packages/wal/vendor/dep");
    git(
        &binary,
        root.path(),
        &["update-index", "--add", "--cacheinfo", &cacheinfo],
    );
    let source = Project::open(&package).unwrap();
    let result = census(&source, &binary, limits());
    assert!(
        matches!(result, GitIndexCensus::PopulatedCandidate { package_prefix, links, .. }
        if package_prefix == "packages/wal"
        && links == [GitlinkCandidate { path: "vendor/dep".into(), indexed_oid: commit.clone(), checkout_head_oid: commit.clone() }])
    );
    fs::rename(
        checkout.join(".git"),
        checkout.join("detached-git-metadata"),
    )
    .unwrap();
    assert_eq!(
        census(&source, &binary, limits()),
        GitIndexCensus::Uninitialized {
            path: "vendor/dep".into(),
            indexed_oid: commit
        }
    );
}

#[test]
fn malformed_or_conflicted_index_row_never_means_no_links() {
    let bytes = b"160000 abcd 1\tvendor/dep\0";
    assert!(parse_index(bytes, "", limits()).is_err());
    let bytes = format!("160000 {} 1\tvendor/dep\0", "a".repeat(40));
    assert!(parse_index(bytes.as_bytes(), "", limits()).is_err());
    let bytes = format!(
        "160000 {} 1\tunrelated/conflict\0\
         160000 {} 0\tpackages/wal/vendor/dep\0",
        "a".repeat(40),
        "b".repeat(40)
    );
    assert_eq!(
        parse_index(bytes.as_bytes(), "packages/wal", limits()).unwrap(),
        [("vendor/dep".into(), "b".repeat(40))]
    );
}
