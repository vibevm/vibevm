use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::package_git_payload::{PayloadLimits, stage_local_payload};

fn binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .find_map(|dir| dir.join(name).canonicalize().ok().filter(|p| p.is_file()))
}
fn git(binary: &Path, cwd: &Path, args: &[&str]) -> String {
    let out = Command::new(binary)
        .current_dir(cwd)
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("LC_ALL", "C")
        .env("HOME", cwd)
        .env("USERPROFILE", cwd)
        .env("PATH", binary.parent().unwrap())
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "local Git fixture {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn retained_store(binary: &Path, checkout: &Path, oid_len: usize) -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    if oid_len == 64 {
        git(
            binary,
            root.path(),
            &["init", "--bare", "-q", "--object-format=sha256"],
        );
    } else {
        git(binary, root.path(), &["init", "--bare", "-q"]);
    }
    let source = checkout.join(".git");
    git(
        binary,
        root.path(),
        &[
            "fetch",
            "--depth=1",
            "--no-tags",
            "--no-write-fetch-head",
            "--no-recurse-submodules",
            "--",
            source.to_str().unwrap(),
            "HEAD:refs/byteonly/selected",
        ],
    );
    let project = Project::open(root.path()).unwrap();
    (root, project)
}
fn limits() -> SubmoduleLimits {
    SubmoduleLimits {
        census: CensusLimits {
            max_index_bytes: 32 * 1024,
            max_probe_bytes: 4096,
            max_gitlinks: 8,
            max_path_bytes: 512,
        },
        max_gitdir_path_bytes: 4096,
        max_gitfile_bytes: 4096,
        max_git_output_bytes: 4096,
        max_config_bytes: 4096,
        max_pack_entries: 32,
        max_commit_bytes: 4096,
        max_tree_bytes: 16 * 1024,
        max_blob_bytes: 4096,
        max_total_raw_bytes: 32 * 1024,
        max_files: 16,
        max_directories: 16,
        max_children_per_tree: 16,
        max_depth: 5,
        max_path_bytes: 512,
    }
}
fn payload_limits() -> PayloadLimits {
    PayloadLimits {
        max_files: 16,
        max_directories: 16,
        max_children_per_directory: 16,
        max_depth: 6,
        max_path_bytes: 512,
        max_file_bytes: 4096,
        max_total_bytes: 32 * 1024,
        max_manifest_bytes: 32 * 1024,
    }
}
struct World {
    _root: tempfile::TempDir,
    package: Project,
    checkout: PathBuf,
    binary: PathBuf,
}
fn world(binary: &Path) -> World {
    let root = tempfile::tempdir().unwrap();
    git(binary, root.path(), &["init", "-q"]);
    fs::write(root.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let checkout = root.path().join("vendor/dep");
    fs::create_dir_all(&checkout).unwrap();
    git(binary, &checkout, &["init", "-q"]);
    git(
        binary,
        &checkout,
        &["config", "user.email", "fixture@local"],
    );
    git(binary, &checkout, &["config", "user.name", "fixture"]);
    fs::create_dir(checkout.join("docs")).unwrap();
    fs::write(checkout.join("docs/nested.txt"), b"nested\n").unwrap();
    fs::write(checkout.join("root.txt"), b"root\n").unwrap();
    fs::write(checkout.join(".gitmodules"), b"[submodule \"none\"]\n").unwrap();
    git(
        binary,
        &checkout,
        &["add", "docs/nested.txt", "root.txt", ".gitmodules"],
    );
    git(binary, &checkout, &["commit", "-qm", "submodule fixture"]);
    git(binary, root.path(), &["add", "vibe.toml"]);
    set_index_gitlink(binary, root.path(), &checkout);
    World {
        package: Project::open(root.path()).unwrap(),
        _root: root,
        checkout,
        binary: binary.into(),
    }
}
fn set_index_gitlink(binary: &Path, root: &Path, checkout: &Path) {
    let commit = git(binary, checkout, &["rev-parse", "HEAD"]);
    let row = format!("160000,{commit},vendor/dep");
    git(
        binary,
        root,
        &["update-index", "--add", "--cacheinfo", &row],
    );
}
fn candidate(world: &World) -> GitlinkCandidate {
    let GitIndexCensus::PopulatedCandidate { links, .. } =
        package_git_gitlink::census(&world.package, &world.binary, limits().census)
    else {
        panic!("indexed Gitlink absent")
    };
    links
        .into_iter()
        .find(|row| row.path == "vendor/dep")
        .unwrap()
}
fn approved(world: &World) -> Project {
    Project::open(&world.checkout.join(".git")).unwrap()
}
fn vault(world: &World) -> (tempfile::TempDir, Project, CompleteManifest) {
    let root = tempfile::tempdir().unwrap();
    let project = Project::open(root.path()).unwrap();
    let manifest = stage_local_payload(&world.package, &project, payload_limits()).unwrap();
    (root, project, manifest)
}

#[test]
fn nested_regular_files_match_raw_commit_bytes_and_omit_gitmodules_as_metadata() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    let candidate = candidate(&world);
    let (_root, vault, manifest) = vault(&world);
    let status = compare_one(
        &world.package,
        &vault,
        &manifest,
        &candidate,
        &approved(&world),
        &binary,
        limits(),
    );
    let SubmoduleInspection::ExactCandidate {
        files,
        directories,
        omitted_gitmodules,
        ..
    } = status
    else {
        panic!("raw comparison failed: {status:?}")
    };
    assert_eq!(files, 2);
    assert_eq!(directories, 1);
    assert_eq!(omitted_gitmodules.len(), 1);
    assert_eq!(omitted_gitmodules[0].path, ".gitmodules");
}

#[test]
fn worktree_gitfile_points_to_pinned_external_gitdir() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    let modules = world.package.root_path().join(".git/modules");
    fs::create_dir_all(&modules).unwrap();
    fs::rename(world.checkout.join(".git"), modules.join("dep")).unwrap();
    fs::write(
        world.checkout.join(".git"),
        b"gitdir: ../../.git/modules/dep\n",
    )
    .unwrap();
    let selected = candidate(&world);
    let (_root, vault, manifest) = vault(&world);
    let held_gitdir = Project::open(&modules.join("dep")).unwrap();
    let status = compare_one(
        &world.package,
        &vault,
        &manifest,
        &selected,
        &held_gitdir,
        &binary,
        limits(),
    );
    assert!(
        matches!(status, SubmoduleInspection::ExactCandidate { files: 2, .. }),
        "{status:?}"
    );
    let foreign_root = tempfile::tempdir().unwrap();
    let foreign = Project::open(foreign_root.path()).unwrap();
    assert!(matches!(
        compare_one(
            &world.package,
            &vault,
            &manifest,
            &selected,
            &foreign,
            &binary,
            limits()
        ),
        SubmoduleInspection::Unavailable(_)
    ));
}

#[test]
fn transformed_checkout_bytes_cannot_claim_raw_commit_provenance() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    fs::write(world.checkout.join("root.txt"), b"root\r\n").unwrap();
    let selected = candidate(&world);
    let (_root, vault, manifest) = vault(&world);
    assert!(matches!(
        compare_one(
            &world.package,
            &vault,
            &manifest,
            &selected,
            &approved(&world),
            &binary,
            limits()
        ),
        SubmoduleInspection::Unavailable(_)
    ));
}

#[test]
fn checkout_drift_after_vault_capture_does_not_claim_historical_bytes_as_current() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    let selected = candidate(&world);
    let (_root, vault, manifest) = vault(&world);
    fs::write(world.checkout.join("root.txt"), b"changed after vault\n").unwrap();
    assert!(matches!(
        compare_one(
            &world.package,
            &vault,
            &manifest,
            &selected,
            &approved(&world),
            &binary,
            limits()
        ),
        SubmoduleInspection::Unavailable(_)
    ));
}

#[test]
fn untracked_extra_empty_directory_is_not_in_committed_tree() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    fs::create_dir(world.checkout.join("scratch")).unwrap();
    let selected = candidate(&world);
    let (_root, vault, manifest) = vault(&world);
    assert!(matches!(
        compare_one(
            &world.package,
            &vault,
            &manifest,
            &selected,
            &approved(&world),
            &binary,
            limits()
        ),
        SubmoduleInspection::Unavailable(_)
    ));
}

#[cfg(unix)]
#[test]
fn committed_symlink_refuses_raw_flattened_comparison() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    std::os::unix::fs::symlink("root.txt", world.checkout.join("link.txt")).unwrap();
    git(&binary, &world.checkout, &["add", "link.txt"]);
    git(&binary, &world.checkout, &["commit", "-qm", "symlink"]);
    set_index_gitlink(&binary, world.package.root_path(), &world.checkout);
    fs::remove_file(world.checkout.join("link.txt")).unwrap(); // vault can be captured, commit still carries mode 120000
    let selected = candidate(&world);
    let (_root, vault, manifest) = vault(&world);
    assert!(matches!(
        compare_one(
            &world.package,
            &vault,
            &manifest,
            &selected,
            &approved(&world),
            &binary,
            limits()
        ),
        SubmoduleInspection::Unavailable(_)
    ));
}

#[test]
fn nested_gitlink_and_uninitialized_checkout_refuse() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    let row = format!("160000,{},nested/child", "a".repeat(40));
    git(
        &binary,
        &world.checkout,
        &["update-index", "--add", "--cacheinfo", &row],
    );
    git(
        &binary,
        &world.checkout,
        &["commit", "-qm", "nested gitlink"],
    );
    set_index_gitlink(&binary, world.package.root_path(), &world.checkout);
    let selected = candidate(&world);
    let (_root, vault, manifest) = vault(&world);
    let held_gitdir = approved(&world);
    assert!(matches!(
        compare_one(
            &world.package,
            &vault,
            &manifest,
            &selected,
            &held_gitdir,
            &binary,
            limits()
        ),
        SubmoduleInspection::Unavailable(_)
    ));
    fs::rename(
        world.checkout.join(".git"),
        world.checkout.join("detached-git-metadata"),
    )
    .unwrap();
    assert!(matches!(
        compare_one(
            &world.package,
            &vault,
            &manifest,
            &selected,
            &held_gitdir,
            &binary,
            limits()
        ),
        SubmoduleInspection::Unavailable(_)
    ));
}

#[test]
fn another_populated_gitlink_appearing_during_read_is_drift() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    let other = world.package.root_path().join("vendor/other");
    fs::create_dir_all(&other).unwrap();
    git(&binary, &other, &["init", "-q"]);
    git(&binary, &other, &["config", "user.email", "fixture@local"]);
    git(&binary, &other, &["config", "user.name", "fixture"]);
    fs::write(other.join("other.txt"), b"another checkout\n").unwrap();
    git(&binary, &other, &["add", "other.txt"]);
    git(&binary, &other, &["commit", "-qm", "other"]);
    let other_oid = git(&binary, &other, &["rev-parse", "HEAD"]);
    let selected = candidate(&world);
    let (_root, vault, manifest) = vault(&world);
    let approved = approved(&world);
    let result = compare_inner(
        &world.package,
        &vault,
        &manifest,
        &selected,
        &approved,
        &binary,
        limits(),
        || {
            let row = format!("160000,{other_oid},vendor/other");
            git(
                &binary,
                world.package.root_path(),
                &["update-index", "--add", "--cacheinfo", &row],
            );
        },
    );
    assert!(
        matches!(result, SubmoduleInspection::Drift(_)),
        "{result:?}"
    );
}

#[test]
fn sha256_parent_and_submodule_object_formats_are_compared_raw() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    if !crate::package_git_tree::git::LocalGit::probe_sha256(&binary).unwrap() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    git(
        &binary,
        root.path(),
        &["init", "-q", "--object-format=sha256"],
    );
    fs::write(root.path().join("vibe.toml"), b"[package]\nname='wal'\n").unwrap();
    let checkout = root.path().join("vendor/dep");
    fs::create_dir_all(&checkout).unwrap();
    git(
        &binary,
        &checkout,
        &["init", "-q", "--object-format=sha256"],
    );
    git(
        &binary,
        &checkout,
        &["config", "user.email", "fixture@local"],
    );
    git(&binary, &checkout, &["config", "user.name", "fixture"]);
    fs::write(checkout.join("data.txt"), b"sha256 selected bytes\n").unwrap();
    git(&binary, &checkout, &["add", "data.txt"]);
    git(&binary, &checkout, &["commit", "-qm", "sha256 submodule"]);
    git(&binary, root.path(), &["add", "vibe.toml"]);
    set_index_gitlink(&binary, root.path(), &checkout);
    let world = World {
        package: Project::open(root.path()).unwrap(),
        _root: root,
        checkout,
        binary: binary.clone(),
    };
    let selected = candidate(&world);
    assert_eq!(selected.indexed_oid.len(), 64);
    let (_root, vault, manifest) = vault(&world);
    assert!(matches!(
        compare_one(
            &world.package,
            &vault,
            &manifest,
            &selected,
            &approved(&world),
            &binary,
            limits()
        ),
        SubmoduleInspection::ExactCandidate { files: 1, .. }
    ));
    let (_store_root, store) = retained_store(&binary, &world.checkout, 64);
    assert!(matches!(
        compare_retained_byte_store(
            &vault,
            &manifest,
            &store,
            &selected.path,
            &selected.indexed_oid,
            &binary,
            limits()
        ),
        RawInspection::Exact(RawComparison { files: 1, .. })
    ));
}

#[path = "tests_retained.rs"]
mod tests_retained;
