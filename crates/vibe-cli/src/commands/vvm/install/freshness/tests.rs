use super::*;
use std::cell::{Cell, RefCell};

use super::super::super::builder::{BuildOutput, Builder, ResolvedVersion};
use super::super::super::model::{Kind, Origin, Profile, VersionId};
use super::super::super::placer;
use super::super::super::store::{
    APP_PROXY_BINARY_NAME, BINARY_NAME, INDEX_BINARY_NAME, VersionStore,
};
use super::super::{InstallRequest, perform_install};
use crate::output;
use vibe_core::progress::Progress;

/// Opt-in regression against the owner's checkout; never uses the real store.
#[test]
#[ignore = "reads the current checkout and writes only a temporary isolated store"]
fn actual_checkout_can_seal_and_reuse_without_building_twice() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let commit = String::from_utf8(git(root, &["rev-parse", "HEAD"]).unwrap())
        .unwrap()
        .trim()
        .to_owned();
    let temp = tempfile::tempdir().unwrap();
    assert!(!temp.path().starts_with(root));
    let store = VersionStore::new(temp.path().join("opt"));
    let resolved = ResolvedVersion {
        id: VersionId::new(Kind::Branch, "main"),
        commit,
    };
    let builder = CountingBuilder::new();
    let req = request(&resolved);
    let first = install(&store, root, &req, &builder);
    assert!(
        placer::read_manifest(&first.home)
            .unwrap()
            .source_inputs_sha256
            .is_some()
    );
    let before = store.load_state().unwrap();
    let second = install(&store, root, &req, &builder);
    assert!(second.reused);
    assert_eq!(builder.calls.get(), 1);
    assert_eq!(first.home, second.home);
    assert_eq!(before, store.load_state().unwrap());
    println!(
        "actual-checkout: fingerprint sealed; repeated invocation made zero builder calls or installed-state changes"
    );
}

struct CountingBuilder {
    calls: Cell<usize>,
    toolchain: RefCell<String>,
    edit_during_build: Cell<bool>,
}

impl CountingBuilder {
    fn new() -> Self {
        Self {
            calls: Cell::new(0),
            toolchain: RefCell::new("rustc fake-1".into()),
            edit_during_build: Cell::new(false),
        }
    }
}

impl Builder for CountingBuilder {
    fn probe_toolchain(&self, _root: &Path) -> Option<String> {
        Some(self.toolchain.borrow().clone())
    }

    fn build(&self, root: &Path, target: &Path, profile: Profile) -> anyhow::Result<BuildOutput> {
        self.calls.set(self.calls.get() + 1);
        if self.edit_during_build.replace(false) {
            fs::write(root.join("input.rs"), "changed while building")?;
        }
        let target = target.join(profile.target_subdir());
        fs::create_dir_all(&target)?;
        let binary = target.join(BINARY_NAME);
        let index_binary = target.join(INDEX_BINARY_NAME);
        let app_proxy_binary = target.join(APP_PROXY_BINARY_NAME);
        for path in [&binary, &index_binary, &app_proxy_binary] {
            fs::write(path, b"fixed test artifact")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
            }
        }
        Ok(BuildOutput {
            binary,
            index_binary,
            app_proxy_binary,
            toolchain: self.toolchain.borrow().clone(),
        })
    }
}

fn git_fixture(root: &Path) -> String {
    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.name", "tester"]);
    run(&["config", "user.email", "tester@example.test"]);
    fs::write(root.join("input.rs"), b"aaaa").unwrap();
    fs::write(root.join(".gitignore"), b"target/\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-qm", "fixture"]);
    run(&["rev-parse", "HEAD"])
}

fn request(resolved: &ResolvedVersion) -> InstallRequest<'_> {
    InstallRequest {
        resolved,
        profile: Profile::Debug,
        force: false,
        now: "fixture",
        origin: Origin::External,
        source_path: Some("source fixture".into()),
        build_environment: &[],
    }
}

fn install(
    store: &VersionStore,
    source: &Path,
    req: &InstallRequest<'_>,
    builder: &CountingBuilder,
) -> super::super::InstallOutcome {
    let context =
        output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto);
    perform_install(&context, store, source, req, builder, &Progress::default()).unwrap()
}

#[test]
fn unchanged_source_skips_build_placement_and_inventory_rewrite() {
    let source = tempfile::tempdir().unwrap();
    let resolved = ResolvedVersion {
        id: VersionId::new(Kind::Branch, "main"),
        commit: git_fixture(source.path()),
    };
    let holder = tempfile::tempdir().unwrap();
    let store = VersionStore::new(holder.path());
    let builder = CountingBuilder::new();
    let req = request(&resolved);
    let first = install(&store, source.path(), &req, &builder);
    assert!(!first.reused);
    let state = fs::read(store.state_path()).unwrap();
    let manifest = fs::read(first.home.join(".vvm-manifest.toml")).unwrap();
    let second = install(&store, source.path(), &req, &builder);
    assert!(second.reused);
    assert_eq!(builder.calls.get(), 1);
    assert_eq!(first.home, second.home);
    assert_eq!(fs::read(store.state_path()).unwrap(), state);
    assert_eq!(
        fs::read(first.home.join(".vvm-manifest.toml")).unwrap(),
        manifest
    );
    fs::create_dir(source.path().join("target")).unwrap();
    fs::write(source.path().join("target/generated"), "ignored output").unwrap();
    assert!(install(&store, source.path(), &req, &builder).reused);
    assert_eq!(builder.calls.get(), 1);
}

#[test]
fn opaque_nested_clone_and_worktree_allow_parent_build_reuse() {
    for linked_worktree in [false, true] {
        let source = tempfile::tempdir().unwrap();
        let resolved = ResolvedVersion {
            id: VersionId::new(Kind::Branch, "main"),
            commit: git_fixture(source.path()),
        };
        let outside = tempfile::tempdir().unwrap();
        git_fixture(outside.path());
        let nested = source.path().join("cache/agent-plugins-spec");
        fs::create_dir_all(nested.parent().unwrap()).unwrap();
        let mut command = Command::new("git");
        if linked_worktree {
            command
                .current_dir(outside.path())
                .args(["worktree", "add", "--detach"])
                .arg(&nested);
        } else {
            command
                .args(["clone", "--quiet"])
                .arg(outside.path())
                .arg(&nested);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            git(
                source.path(),
                &["ls-files", "--others", "--exclude-standard", "-z"]
            )
            .unwrap(),
            b"cache/agent-plugins-spec/\0"
        );

        let holder = tempfile::tempdir().unwrap();
        let store = VersionStore::new(holder.path());
        let builder = CountingBuilder::new();
        let req = request(&resolved);
        let first = install(&store, source.path(), &req, &builder);
        assert!(!first.reused);
        assert!(
            placer::read_manifest(&first.home)
                .unwrap()
                .source_inputs_sha256
                .is_some()
        );
        let state = fs::read(store.state_path()).unwrap();
        let manifest = fs::read(first.home.join(".vvm-manifest.toml")).unwrap();
        for edit in [false, true] {
            if edit {
                fs::write(nested.join("input.rs"), "external tracked edit").unwrap();
                fs::write(nested.join("new.rs"), "external untracked input").unwrap();
            }
            let repeated = install(&store, source.path(), &req, &builder);
            assert!(repeated.reused);
            assert_eq!(repeated.home, first.home);
            assert_eq!(builder.calls.get(), 1);
            assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 1);
            assert_eq!(fs::read(store.state_path()).unwrap(), state);
            assert_eq!(
                fs::read(first.home.join(".vvm-manifest.toml")).unwrap(),
                manifest
            );
        }
        fs::write(source.path().join("input.rs"), "parent tracked edit").unwrap();
        assert!(!install(&store, source.path(), &req, &builder).reused);
        fs::write(
            source.path().join("cache/ordinary.rs"),
            "parent untracked input",
        )
        .unwrap();
        assert!(!install(&store, source.path(), &req, &builder).reused);
        assert_eq!(builder.calls.get(), 3);
    }
}

#[test]
fn unknown_directory_and_invalid_git_marker_are_not_external_worktrees() {
    let source = tempfile::tempdir().unwrap();
    git_fixture(source.path());
    let relative = Path::new("unknown");
    fs::create_dir(source.path().join(relative)).unwrap();
    assert!(external_worktree(source.path(), relative).is_none());
    fs::write(source.path().join(relative).join(".git"), "invalid gitfile").unwrap();
    assert!(external_worktree(source.path(), relative).is_none());
}

#[test]
fn redirected_nested_directory_and_git_marker_disable_freshness() {
    let source = tempfile::tempdir().unwrap();
    let commit = git_fixture(source.path());
    let outside = tempfile::tempdir().unwrap();
    git_fixture(outside.path());
    let link = source.path().join("nested-link");
    if !freshness_dir_redirect(outside.path(), &link) {
        return;
    }
    assert!(external_worktree(source.path(), Path::new("nested-link")).is_none());
    assert!(SourceSnapshot::capture(source.path(), &commit, &[]).is_none());

    // Exercise a .git directory redirect independently of the first link.
    let other_source = tempfile::tempdir().unwrap();
    let other_commit = git_fixture(other_source.path());
    let nested = other_source.path().join("nested");
    fs::create_dir(&nested).unwrap();
    if !freshness_dir_redirect(&outside.path().join(".git"), &nested.join(".git")) {
        return;
    }
    assert!(external_worktree(other_source.path(), Path::new("nested")).is_none());
    assert!(SourceSnapshot::capture(other_source.path(), &other_commit, &[]).is_none());
}

#[cfg(unix)]
#[test]
fn special_source_is_not_silently_excluded() {
    let source = tempfile::tempdir().unwrap();
    let commit = git_fixture(source.path());
    let relative = Path::new("input.rs");
    fs::remove_file(source.path().join(relative)).unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(source.path().join(relative)).unwrap();
    assert!(external_worktree(source.path(), relative).is_none());
    assert!(SourceSnapshot::capture(source.path(), &commit, &[]).is_none());
}

#[cfg(unix)]
fn freshness_dir_redirect(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).unwrap();
    true
}

#[cfg(windows)]
fn freshness_dir_redirect(target: &Path, link: &Path) -> bool {
    use std::os::windows::process::CommandExt;
    Command::new("cmd")
        .args(["/d", "/c"])
        .raw_arg(format!(
            "mklink /J \"{}\" \"{}\"",
            link.display(),
            target.display()
        ))
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn same_head_edits_revert_deletion_and_untracked_inputs_build_again() {
    let source = tempfile::tempdir().unwrap();
    let resolved = ResolvedVersion {
        id: VersionId::new(Kind::Branch, "main"),
        commit: git_fixture(source.path()),
    };
    let holder = tempfile::tempdir().unwrap();
    let store = VersionStore::new(holder.path());
    let builder = CountingBuilder::new();
    let req = request(&resolved);
    install(&store, source.path(), &req, &builder);
    for bytes in [b"bbbb", b"aaaa"] {
        fs::write(source.path().join("input.rs"), bytes).unwrap();
        assert!(!install(&store, source.path(), &req, &builder).reused);
    }
    fs::remove_file(source.path().join("input.rs")).unwrap();
    assert!(!install(&store, source.path(), &req, &builder).reused);
    for bytes in [b"xxxx", b"yyyy"] {
        fs::write(source.path().join("new input.rs"), bytes).unwrap();
        assert!(!install(&store, source.path(), &req, &builder).reused);
    }
    fs::rename(
        source.path().join("new input.rs"),
        source.path().join("renamed.rs"),
    )
    .unwrap();
    assert!(!install(&store, source.path(), &req, &builder).reused);
    assert_eq!(builder.calls.get(), 7);
}

#[test]
fn staging_unchanged_worktree_bytes_does_not_build_again() {
    let source = tempfile::tempdir().unwrap();
    let resolved = ResolvedVersion {
        id: VersionId::new(Kind::Branch, "main"),
        commit: git_fixture(source.path()),
    };
    let holder = tempfile::tempdir().unwrap();
    let store = VersionStore::new(holder.path());
    let builder = CountingBuilder::new();
    let req = request(&resolved);
    fs::write(source.path().join("input.rs"), b"bbbb").unwrap();
    fs::write(source.path().join("new input.rs"), b"untracked").unwrap();
    let first = install(&store, source.path(), &req, &builder);
    let output = Command::new("git")
        .current_dir(source.path())
        .args(["add", "."])
        .output()
        .unwrap();
    assert!(output.status.success());
    let second = install(&store, source.path(), &req, &builder);
    assert!(second.reused);
    assert_eq!(first.home, second.home);
    assert_eq!(builder.calls.get(), 1);
}

#[test]
fn force_corruption_profile_toolchain_and_source_provenance_cannot_reuse() {
    let source = tempfile::tempdir().unwrap();
    let resolved = ResolvedVersion {
        id: VersionId::new(Kind::Branch, "main"),
        commit: git_fixture(source.path()),
    };
    let holder = tempfile::tempdir().unwrap();
    let store = VersionStore::new(holder.path());
    let builder = CountingBuilder::new();
    let mut req = request(&resolved);
    let first = install(&store, source.path(), &req, &builder);
    fs::write(first.home.join("bin").join(BINARY_NAME), b"corrupt").unwrap();
    assert!(!install(&store, source.path(), &req, &builder).reused);
    req.force = true;
    let forced = install(&store, source.path(), &req, &builder);
    assert!(!forced.reused);
    req.force = false;
    req.profile = Profile::Release;
    assert!(!install(&store, source.path(), &req, &builder).reused);
    *builder.toolchain.borrow_mut() = "rustc fake-2".into();
    assert!(!install(&store, source.path(), &req, &builder).reused);
    req.source_path = Some("different source".into());
    assert!(!install(&store, source.path(), &req, &builder).reused);
    req.origin = Origin::Managed;
    assert!(!install(&store, source.path(), &req, &builder).reused);
    assert_eq!(builder.calls.get(), 7);
    assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 7);
}

#[test]
fn legacy_manifest_builds_once_and_source_changed_during_build_is_unsealed() {
    let source = tempfile::tempdir().unwrap();
    let resolved = ResolvedVersion {
        id: VersionId::new(Kind::Branch, "main"),
        commit: git_fixture(source.path()),
    };
    let holder = tempfile::tempdir().unwrap();
    let store = VersionStore::new(holder.path());
    let builder = CountingBuilder::new();
    let req = request(&resolved);
    let first = install(&store, source.path(), &req, &builder);
    let mut manifest = placer::read_manifest(&first.home).unwrap();
    manifest.source_inputs_sha256 = None;
    fs::write(
        first.home.join(".vvm-manifest.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    assert!(!install(&store, source.path(), &req, &builder).reused);
    assert!(install(&store, source.path(), &req, &builder).reused);
    assert_eq!(builder.calls.get(), 2);
    fs::write(source.path().join("input.rs"), "before build").unwrap();
    builder.edit_during_build.set(true);
    let changed = install(&store, source.path(), &req, &builder);
    assert!(
        placer::read_manifest(&changed.home)
            .unwrap()
            .source_inputs_sha256
            .is_none()
    );
    assert!(!install(&store, source.path(), &req, &builder).reused);
    assert!(install(&store, source.path(), &req, &builder).reused);
    assert_eq!(builder.calls.get(), 4);
}

#[test]
fn nongit_and_unresolved_source_snapshot_are_unprovable() {
    let source = tempfile::tempdir().unwrap();
    assert!(SourceSnapshot::capture(source.path(), "unknown", &[]).is_none());
    let commit = git_fixture(source.path());
    assert!(SourceSnapshot::capture(source.path(), &commit, &[]).is_some());
    assert!(SourceSnapshot::capture(source.path(), "different", &[]).is_none());
}

#[test]
fn gitlinks_disable_freshness_instead_of_omitting_dependency_inputs() {
    let source = tempfile::tempdir().unwrap();
    let commit = git_fixture(source.path());
    let output = Command::new("git")
        .current_dir(source.path())
        .args([
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{commit},vendor"),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(SourceSnapshot::capture(source.path(), &commit, &[]).is_none());
}

#[test]
fn injected_build_environment_changes_snapshot_without_ambient_reads() {
    let source = tempfile::tempdir().unwrap();
    let commit = git_fixture(source.path());
    let absent = vec![("RUSTFLAGS".into(), None)];
    let empty = vec![("RUSTFLAGS".into(), Some(std::ffi::OsString::new()))];
    let changed = vec![(
        "RUSTFLAGS".into(),
        Some(std::ffi::OsString::from("-C opt-level=2")),
    )];
    let initial = SourceSnapshot::capture(source.path(), &commit, &absent).unwrap();
    assert!(SourceSnapshot::capture(source.path(), &commit, &absent).unwrap() == initial);
    assert!(SourceSnapshot::capture(source.path(), &commit, &empty).unwrap() != initial);
    assert!(SourceSnapshot::capture(source.path(), &commit, &changed).unwrap() != initial);
    assert!(
        SourceSnapshot::capture(source.path(), &commit, &empty).unwrap()
            != SourceSnapshot::capture(source.path(), &commit, &changed).unwrap()
    );
}

#[cfg(unix)]
#[test]
fn raw_non_utf8_paths_are_hashed_and_symlinks_are_not_followed() {
    use std::os::unix::ffi::OsStringExt;
    let source = tempfile::tempdir().unwrap();
    let commit = git_fixture(source.path());
    let path = source
        .path()
        .join(OsString::from_vec(b"raw-\xff\n.rs".to_vec()));
    fs::write(&path, "one").unwrap();
    let before = SourceSnapshot::capture(source.path(), &commit, &[]).unwrap();
    fs::write(&path, "two").unwrap();
    assert!(SourceSnapshot::capture(source.path(), &commit, &[]).unwrap() != before);
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("external"), "outside").unwrap();
    std::os::unix::fs::symlink(outside.path().join("external"), source.path().join("link"))
        .unwrap();
    assert!(SourceSnapshot::capture(source.path(), &commit, &[]).is_none());
}
