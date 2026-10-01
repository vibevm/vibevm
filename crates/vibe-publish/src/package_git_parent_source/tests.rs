use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::package_git_plan::{
    CommitMetadata, GitPerson, PackageCoordinate, StagedCommit, StagedObjects, TagMetadata,
};

fn binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|dir| {
        dir.join(name)
            .canonicalize()
            .ok()
            .filter(|path| path.is_file())
    })
}
fn limits() -> SourceLimits {
    SourceLimits {
        max_path_bytes: 4096,
        max_probe_bytes: 4096,
        max_refs_bytes: 8192,
    }
}
fn git(binary: &Path, cwd: &Path, args: &[&str], input: &[u8]) -> String {
    let mut child = Command::new(binary)
        .current_dir(cwd)
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .env("HOME", cwd)
        .env("USERPROFILE", cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "local Git fixture {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn bare(binary: &Path) -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("origin.git");
    fs::create_dir(&path).unwrap();
    git(binary, &path, &["init", "--bare", "-q"], b"");
    let source = Project::open(&path).unwrap();
    (root, source)
}
fn seed_main(binary: &Path, source: &Project, seconds: i64) -> String {
    let path = source.root_path().to_str().unwrap();
    let tree = git(
        binary,
        source.root_path(),
        &["--git-dir", path, "mktree"],
        b"",
    );
    let body = format!(
        "tree {tree}\nauthor Fixture <fixture@example.test> {seconds} +0000\ncommitter Fixture <fixture@example.test> {seconds} +0000\n\nfixture\n"
    );
    let oid = git(
        binary,
        source.root_path(),
        &[
            "--git-dir",
            path,
            "hash-object",
            "-t",
            "commit",
            "-w",
            "--stdin",
        ],
        body.as_bytes(),
    );
    git(
        binary,
        source.root_path(),
        &["--git-dir", path, "update-ref", "refs/heads/main", &oid],
        b"",
    );
    oid
}
fn seed_annotated_tag(binary: &Path, source: &Project, commit: &str) -> String {
    let path = source.root_path().to_str().unwrap();
    let body = format!(
        "object {commit}\ntype commit\ntag v1.1.0\ntagger Fixture <fixture@example.test> 1700000000 +0000\n\nfixture tag\n"
    );
    let oid = git(
        binary,
        source.root_path(),
        &[
            "--git-dir",
            path,
            "hash-object",
            "-t",
            "tag",
            "-w",
            "--stdin",
        ],
        body.as_bytes(),
    );
    git(
        binary,
        source.root_path(),
        &["--git-dir", path, "update-ref", "refs/tags/v1.1.0", &oid],
        b"",
    );
    oid
}
fn plan(source: &Project, refs: RefSnapshot) -> PackageGitPlan {
    let identity = local_bare_identity_digest(source, limits()).unwrap();
    let parent = match &refs.main {
        RawRef::Absent => None,
        RawRef::Present { oid } => Some(oid.clone()),
    };
    let person = GitPerson {
        name: "Oleg Chirukhin".into(),
        email: "oleg@example.test".into(),
        timestamp_seconds: 1_700_000_000,
        timezone_offset_minutes: 180,
    };
    let mut plan = PackageGitPlan {
        epoch: 2,
        package: PackageCoordinate {
            kind: "flow".into(),
            group: "org.vibevm".into(),
            name: "wal".into(),
            version: "1.1.0".into(),
            mutable: true,
        },
        version_tag_ref: "refs/tags/v1.1.0".into(),
        target: RepositoryTarget::DirectGitExisting {
            remote_identity_sha256: identity.clone(),
        },
        payload_sha256: format!("sha256:{}", "a".repeat(64)),
        flattened_submodules_sha256: format!("sha256:{}", "b".repeat(64)),
        oid_hex_len: 40,
        before_repo: RepositoryObservation::Present(RepositoryIdentity::Direct {
            remote_identity_sha256: identity,
        }),
        before_refs: refs,
        staged: StagedObjects {
            git_tree_oid: "c".repeat(40),
            commit: StagedCommit::New {
                oid: "d".repeat(40),
                parent_oid: parent,
            },
            annotated_tag_oid: Some("e".repeat(40)),
            tag_target_commit_oid: "d".repeat(40),
            commit_metadata: Some(CommitMetadata {
                author: person.clone(),
                committer: person.clone(),
                message: "Release wal@1.1.0\n".into(),
            }),
            tag_metadata: Some(TagMetadata {
                tagger: person,
                message: "wal@1.1.0\n".into(),
            }),
        },
    };
    plan.derive_staged_object_ids().unwrap();
    plan
}

#[test]
fn empty_bare_is_exact_absent_ref_baseline() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local source fixture skipped");
        return;
    };
    let (_root, source) = bare(&binary);
    let plan = plan(
        &source,
        RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
    );
    assert_eq!(
        observe_local_bare(&source, &binary, &plan, limits()),
        ParentSourceObservation::Exact {
            refs: plan.before_refs.clone(),
            repository_root_token: source.root_correlation_token().unwrap(),
            local_bare_identity_sha256: local_bare_identity_digest(&source, limits()).unwrap()
        }
    );
}

#[test]
fn main_and_annotated_tag_are_observed_with_raw_and_peeled_oids() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local source fixture skipped");
        return;
    };
    let (_root, source) = bare(&binary);
    let main = seed_main(&binary, &source, 1_700_000_000);
    let tag = seed_annotated_tag(&binary, &source, &main);
    let refs = RefSnapshot {
        main: RawRef::Present { oid: main.clone() },
        version_tag: TagRef::Present {
            raw_oid: tag,
            peeled_commit_oid: main,
        },
    };
    let plan = plan(&source, refs.clone());
    assert!(
        matches!(observe_local_bare(&source, &binary, &plan, limits()), ParentSourceObservation::Exact { refs: observed, .. } if observed == refs)
    );
}

#[test]
fn changed_refs_between_epochs_and_stable_third_state_are_drift() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local source fixture skipped");
        return;
    };
    let (_root, source) = bare(&binary);
    let first = seed_main(&binary, &source, 1_700_000_000);
    let baseline = RefSnapshot {
        main: RawRef::Present { oid: first },
        version_tag: TagRef::Absent,
    };
    let plan = plan(&source, baseline);
    assert!(matches!(
        observe_inner(&source, &binary, &plan, limits(), || {
            seed_main(&binary, &source, 1_700_000_001);
        }),
        ParentSourceObservation::Drift(_)
    ));
    assert!(matches!(
        observe_local_bare(&source, &binary, &plan, limits()),
        ParentSourceObservation::Drift(_)
    ));
}

#[test]
fn malformed_duplicate_and_foreign_oid_rows_refuse() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local source fixture skipped");
        return;
    };
    let (_root, source) = bare(&binary);
    let plan = plan(
        &source,
        RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
    );
    let oid = "a".repeat(40);
    assert!(
        parse_refs(
            format!("{oid}\trefs/heads/main\n{oid}\trefs/heads/main\n").as_bytes(),
            &plan
        )
        .is_err()
    );
    assert!(parse_refs(format!("{oid}\trefs/heads/foreign\n").as_bytes(), &plan).is_err());
    assert!(
        parse_refs(
            format!("{}\trefs/heads/main\n", "b".repeat(64)).as_bytes(),
            &plan
        )
        .is_err()
    );
    let mut foreign = plan.clone();
    foreign.target = RepositoryTarget::DirectGitExisting {
        remote_identity_sha256: format!("sha256:{}", "f".repeat(64)),
    };
    foreign.before_repo = RepositoryObservation::Present(RepositoryIdentity::Direct {
        remote_identity_sha256: format!("sha256:{}", "f".repeat(64)),
    });
    assert!(matches!(
        observe_local_bare(&source, &binary, &foreign, limits()),
        ParentSourceObservation::Unavailable(_)
    ));
}

#[test]
fn replaced_same_path_local_bare_root_is_drift() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local source fixture skipped");
        return;
    };
    let (root, source) = bare(&binary);
    let plan = plan(
        &source,
        RefSnapshot {
            main: RawRef::Absent,
            version_tag: TagRef::Absent,
        },
    );
    let path = source.root_path().to_path_buf();
    let retired = root.path().join("retired.git");
    let result = observe_inner(&source, &binary, &plan, limits(), || {
        fs::rename(&path, &retired).unwrap();
        fs::create_dir(&path).unwrap();
        git(&binary, &path, &["init", "--bare", "-q"], b"");
    });
    assert!(matches!(result, ParentSourceObservation::Drift(_)));
}

#[test]
fn main_ref_to_tree_is_not_an_existing_commit_parent() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local source fixture skipped");
        return;
    };
    let (_root, source) = bare(&binary);
    let path = source.root_path().to_str().unwrap();
    let tree = git(
        &binary,
        source.root_path(),
        &["--git-dir", path, "mktree"],
        b"",
    );
    // Git's update-ref guards branch types, so write a malformed local bare
    // ref directly to exercise the read-side refusal.
    fs::write(
        source.root_path().join("refs/heads/main"),
        format!("{tree}\n"),
    )
    .unwrap();
    let plan = plan(
        &source,
        RefSnapshot {
            main: RawRef::Present { oid: tree },
            version_tag: TagRef::Absent,
        },
    );
    assert!(matches!(
        observe_local_bare(&source, &binary, &plan, limits()),
        ParentSourceObservation::Unavailable(_)
    ));
}
