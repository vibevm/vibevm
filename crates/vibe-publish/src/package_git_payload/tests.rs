use std::fs;

use super::*;

fn limits() -> PayloadLimits {
    PayloadLimits {
        max_files: 12,
        max_directories: 12,
        max_children_per_directory: 12,
        max_depth: 5,
        max_path_bytes: 512,
        max_file_bytes: 4096,
        max_total_bytes: 16 * 1024,
        max_manifest_bytes: 16 * 1024,
    }
}
fn projects() -> (tempfile::TempDir, tempfile::TempDir, Project, Project) {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::write(source.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let input = Project::open(source.path()).unwrap();
    let output = Project::open(destination.path()).unwrap();
    (source, destination, input, output)
}

#[test]
fn nested_unicode_empty_directory_and_exact_copy_have_local_receipt_only() {
    let (source, destination, input, output) = projects();
    fs::create_dir(source.path().join("docs")).unwrap();
    fs::write(source.path().join("docs/Привет.md"), "hello\n").unwrap();
    fs::create_dir(source.path().join("empty")).unwrap();
    fs::create_dir(source.path().join(".git")).unwrap();
    fs::write(source.path().join(".git/config"), b"private git state").unwrap();
    fs::write(source.path().join(".gitmodules"), b"[submodule \"x\"]\n").unwrap();
    let complete = stage_local_payload(&input, &output, limits()).unwrap();
    assert_eq!(
        inspect_local_payload(&output, limits(), &complete.source_root_token, &complete.tree_sha256),
        LocalPayloadInspection::Complete(complete.clone())
    );
    assert_eq!(complete.format, FORMAT);
    assert_eq!(complete.provenance, ProvenanceStatus::NeedsGitlinkCheck);
    assert!(complete.skipped_gitmodules);
    assert_eq!(complete.directories, ["docs", "empty"]);
    assert_eq!(complete.files.len(), 2);
    assert_eq!(
        fs::read(destination.path().join("payload/docs/Привет.md")).unwrap(),
        b"hello\n"
    );
    assert!(destination.path().join("payload/empty").is_dir());
    assert!(!destination.path().join("payload/.git").exists());
    let bytes = fs::read(destination.path().join(COMPLETE)).unwrap();
    assert_eq!(
        serde_json::from_slice::<CompleteManifest>(&bytes).unwrap(),
        complete
    );
    assert!(
        matches!(
            stage_local_payload(&input, &output, limits()),
            Err(PayloadStageError::Partial {
                staged_files: 0,
                ..
            })
        ),
        "a completed run-scoped destination is read-only, never restaged"
    );
}

#[cfg(unix)]
#[test]
fn symlink_file_and_directory_refuse_before_any_destination_effect() {
    use std::os::unix::fs::symlink;
    for directory_link in [false, true] {
        let (source, destination, input, output) = projects();
        let foreign = tempfile::tempdir().unwrap();
        fs::write(foreign.path().join("foreign.txt"), b"secret").unwrap();
        if directory_link {
            symlink(foreign.path(), source.path().join("linked")).unwrap();
        } else {
            symlink(
                foreign.path().join("foreign.txt"),
                source.path().join("linked"),
            )
            .unwrap();
        }
        assert!(matches!(
            stage_local_payload(&input, &output, limits()),
            Err(PayloadStageError::Before(_))
        ));
        assert!(fs::read_dir(destination.path()).unwrap().next().is_none());
    }
}

#[test]
fn portable_casefold_collision_refuses_before_staging() {
    let (source, destination, input, output) = projects();
    fs::write(source.path().join("Readme"), b"a").unwrap();
    if fs::write(source.path().join("README"), b"b").is_err() {
        return;
    }
    let error = stage_local_payload(&input, &output, limits()).unwrap_err();
    assert!(
        matches!(&error, PayloadStageError::Before(reason) if reason.contains("case-fold")),
        "{error}"
    );
    assert!(fs::read_dir(destination.path()).unwrap().next().is_none());
}

#[test]
fn case_alias_of_skipped_git_metadata_refuses_before_staging() {
    let (source, destination, input, output) = projects();
    fs::write(source.path().join(".GITMODULES"), b"not portable").unwrap();
    let error = stage_local_payload(&input, &output, limits()).unwrap_err();
    assert!(
        matches!(&error, PayloadStageError::Before(reason)
        if reason.contains("Git metadata alias")),
        "{error}"
    );
    assert!(fs::read_dir(destination.path()).unwrap().next().is_none());
}

#[test]
fn source_change_after_copy_leaves_partial_without_complete_manifest() {
    let (source, destination, input, output) = projects();
    fs::write(source.path().join("a.txt"), b"old").unwrap();
    let mut changed = false;
    let result = stage_inner(
        &input,
        &output,
        limits(),
        |_, _| Ok(()),
        |_, row| {
            if !changed {
                fs::write(source.path().join(&row.path), b"changed during staging")?;
                changed = true;
            }
            Ok(())
        },
    );
    assert!(matches!(result, Err(PayloadStageError::Partial { .. })));
    assert!(!destination.path().join(COMPLETE).exists());
    assert!(matches!(
        stage_local_payload(&input, &output, limits()),
        Err(PayloadStageError::Partial {
            staged_files: 0,
            ..
        })
    ));
}

#[test]
fn interrupted_first_copy_is_partial_and_never_implicitly_retried() {
    let (_source, destination, input, output) = projects();
    let result = stage_inner(
        &input,
        &output,
        limits(),
        |_, _| Ok(()),
        |_, _| anyhow::bail!("injected stop"),
    );
    assert!(matches!(
        result,
        Err(PayloadStageError::Partial {
            staged_files: 1,
            ..
        })
    ));
    assert!(!destination.path().join(COMPLETE).exists());
    let source_token = input.root_correlation_token().unwrap();
    assert_eq!(
        inspect_local_payload(&output, limits(), &source_token, "sha256:unknown"),
        LocalPayloadInspection::Incomplete
    );
    assert!(matches!(
        stage_local_payload(&input, &output, limits()),
        Err(PayloadStageError::Partial {
            staged_files: 0,
            ..
        })
    ));
}

#[test]
fn cold_inspection_rehashes_payload_and_refuses_tampered_bytes() {
    let (_source, destination, input, output) = projects();
    let complete = stage_local_payload(&input, &output, limits()).unwrap();
    fs::write(destination.path().join("payload/vibe.toml"), b"tampered").unwrap();
    assert!(matches!(inspect_local_payload(&output, limits(), &complete.source_root_token,
        &complete.tree_sha256), LocalPayloadInspection::Uncertain(_)));
}

#[test]
fn explicit_limit_and_hardlink_refuse_before_staging() {
    let (source, destination, input, output) = projects();
    let mut small = limits();
    small.max_file_bytes = 1;
    assert!(matches!(
        stage_local_payload(&input, &output, small),
        Err(PayloadStageError::Before(_))
    ));
    assert!(fs::read_dir(destination.path()).unwrap().next().is_none());
    if fs::hard_link(source.path().join("vibe.toml"), source.path().join("alias")).is_ok() {
        assert!(matches!(
            stage_local_payload(&input, &output, limits()),
            Err(PayloadStageError::Before(_))
        ));
        assert!(fs::read_dir(destination.path()).unwrap().next().is_none());
    }
}

#[test]
fn changed_first_file_copy_is_definitely_before_publication() {
    let (source, destination, input, output) = projects();
    let result = stage_inner(
        &input,
        &output,
        limits(),
        |_, row| {
            fs::write(source.path().join(&row.path), b"changed before held copy")?;
            Ok(())
        },
        |_, _| Ok(()),
    );
    assert!(
        matches!(&result, Err(PayloadStageError::Before(reason))
        if reason.contains("created_directories")),
        "{result:?}"
    );
    assert!(fs::read_dir(destination.path()).unwrap().next().is_none());
}

#[test]
fn changed_later_file_copy_preserves_known_partial_count() {
    let (source, destination, input, output) = projects();
    fs::write(source.path().join("a.txt"), b"first").unwrap();
    let result = stage_inner(
        &input,
        &output,
        limits(),
        |_, _| Ok(()),
        |count, _| {
            if count == 1 {
                fs::write(source.path().join("vibe.toml"), b"changed second")?;
            }
            Ok(())
        },
    );
    assert!(
        matches!(&result, Err(PayloadStageError::Partial { staged_files: 1, reason })
        if reason.contains("created_directories")),
        "{result:?}"
    );
    assert!(!destination.path().join(COMPLETE).exists());
}

#[test]
fn occupied_completion_name_is_known_partial_and_never_overwritten() {
    let (_source, destination, input, output) = projects();
    let result = stage_inner(
        &input,
        &output,
        limits(),
        |_, _| Ok(()),
        |_, _| {
            fs::write(destination.path().join(COMPLETE), b"foreign occupant")?;
            Ok(())
        },
    );
    assert!(matches!(
        result,
        Err(PayloadStageError::Partial {
            staged_files: 1,
            ..
        })
    ));
    assert_eq!(
        fs::read(destination.path().join(COMPLETE)).unwrap(),
        b"foreign occupant"
    );
}

#[cfg(unix)]
#[test]
fn replaced_destination_path_after_copy_cannot_publish_complete() {
    let outer = tempfile::tempdir().unwrap();
    let source_path = outer.path().join("source");
    let destination_path = outer.path().join("vault");
    let retired_path = outer.path().join("retired-vault");
    fs::create_dir(&source_path).unwrap();
    fs::create_dir(&destination_path).unwrap();
    fs::write(source_path.join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let input = Project::open(&source_path).unwrap();
    let output = Project::open(&destination_path).unwrap();
    let result = stage_inner(
        &input,
        &output,
        limits(),
        |_, _| Ok(()),
        |_, _| {
            fs::rename(&destination_path, &retired_path)?;
            fs::create_dir(&destination_path)?;
            Ok(())
        },
    );
    assert!(matches!(
        result,
        Err(PayloadStageError::Partial {
            staged_files: 1,
            ..
        })
    ));
    assert!(!destination_path.join(COMPLETE).exists());
    assert!(!retired_path.join(COMPLETE).exists());
}

#[cfg(unix)]
#[test]
fn copied_same_bytes_at_replaced_source_path_still_refuses_completion() {
    let outer = tempfile::tempdir().unwrap();
    let source_path = outer.path().join("source");
    let destination_path = outer.path().join("vault");
    let retired_path = outer.path().join("retired-source");
    fs::create_dir(&source_path).unwrap();
    fs::create_dir(&destination_path).unwrap();
    let manifest = b"[package]\nname='wal'\n";
    fs::write(source_path.join("vibe.toml"), manifest).unwrap();
    let input = Project::open(&source_path).unwrap();
    let output = Project::open(&destination_path).unwrap();
    let result = stage_inner(
        &input,
        &output,
        limits(),
        |_, _| Ok(()),
        |_, _| {
            fs::rename(&source_path, &retired_path)?;
            fs::create_dir(&source_path)?;
            fs::write(source_path.join("vibe.toml"), manifest)?;
            Ok(())
        },
    );
    assert!(matches!(
        result,
        Err(PayloadStageError::Partial {
            staged_files: 1,
            ..
        })
    ));
    assert!(!destination_path.join(COMPLETE).exists());
}
