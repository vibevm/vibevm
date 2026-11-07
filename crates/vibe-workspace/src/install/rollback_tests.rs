use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::InstallRollback;
use crate::WorkspaceError;

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            if path.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                walk(root, &path, out);
            } else {
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn write(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn an_undo_puts_back_every_created_reconciled_removed_and_rewritten_path() {
    let ws = TempDir::new().unwrap();
    let deps = ws.path().join(vibe_core::layout::current_vibedeps_root());
    let kept = deps.join("flow-kept/1.0.0");
    let pruned = deps.join("flow-pruned/1.0.0");
    let ignore = ws.path().join(".gitignore");
    write(&kept.join("a.md"), "old a");
    write(&kept.join("sub/b.md"), "old b");
    write(&pruned.join("c.md"), "old c");
    write(&ignore, "target/\n");
    let before = tree(ws.path());

    let mut rollback = InstallRollback::new(ws.path());
    let created = deps.join("flow-new/1.0.0");
    rollback.before_reconcile(&created).unwrap();
    write(&created.join("n.md"), "new");
    rollback.before_reconcile(&kept).unwrap();
    // The slot stays where it is for the incremental diff.
    assert_eq!(fs::read_to_string(kept.join("sub/b.md")).unwrap(), "old b");
    // A materialisation write replaces a file through a rename, never in place.
    let staged = kept.join("a.md.tmp");
    fs::write(&staged, "new a").unwrap();
    fs::rename(&staged, kept.join("a.md")).unwrap();
    fs::remove_file(kept.join("sub/b.md")).unwrap();
    rollback.remove(&pruned).unwrap();
    let _ = fs::remove_dir(pruned.parent().unwrap());
    rollback.before_rewrite(&ignore).unwrap();
    fs::write(&ignore, "target/\nvibevm/vibedeps/x/\n").unwrap();

    let error = rollback.undo(
        WorkspaceError::IndexRender {
            reason: "stop".into(),
        },
        None,
        &[],
    );
    assert!(matches!(error, WorkspaceError::IndexRender { .. }));
    assert_eq!(tree(ws.path()), before, "the project is as it was");
}

#[test]
fn dropping_the_log_keeps_the_changes_and_removes_the_stash() {
    let ws = TempDir::new().unwrap();
    let deps = ws.path().join(vibe_core::layout::current_vibedeps_root());
    let pruned = deps.join("flow-pruned/1.0.0");
    write(&pruned.join("c.md"), "old c");
    let mut rollback = InstallRollback::new(ws.path());
    rollback.remove(&pruned).unwrap();
    drop(rollback);
    assert!(!pruned.exists());
    let leftovers: Vec<_> = fs::read_dir(deps.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| {
            name.to_string_lossy()
                .starts_with(".vibe-install-rollback-")
        })
        .collect();
    assert!(leftovers.is_empty(), "the stash is released: {leftovers:?}");
}

#[test]
#[cfg(unix)]
fn checked_native_two_slot_undo_restores_git_untracked_and_ignore_bytes() {
    let ws = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(&source.path().join(".git/config"), "new config");
    write(&source.path().join("payload"), "new payload");
    let a = ws.path().join("a");
    let b = ws.path().join("b");
    for slot in [&a, &b] {
        write(&slot.join(".git/config"), "old config");
        write(&slot.join(".git/refs/heads/local"), "old ref");
        write(&slot.join("untracked"), "old untracked");
    }
    write(&ws.path().join(".gitignore"), "original/\n");
    let before = tree(ws.path());
    let mut undo = InstallRollback::new(ws.path());
    undo.place_checked(&a, source.path(), || Ok(())).unwrap();
    undo.ignore_checked(ws.path(), "a").unwrap();
    undo.place_checked(&b, source.path(), || Ok(())).unwrap();
    undo.ignore_checked(ws.path(), "b").unwrap();
    let error = undo.undo(
        WorkspaceError::IndexRender {
            reason: "injected downstream failure".into(),
        },
        None,
        &[],
    );
    assert!(
        matches!(error, WorkspaceError::IndexRender { .. }),
        "{error}"
    );
    let mut after = tree(ws.path());
    // The ordinary lazy stash parent may remain empty after releasing its stash.
    after.remove("vibevm/");
    assert_eq!(after, before);
}

#[test]
#[cfg(unix)]
fn checked_native_undo_preserves_foreign_same_bytes_replacement_and_old_backup() {
    let ws = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let slot = ws.path().join("slot");
    write(&slot.join(".git/config"), "old config");
    write(&source.path().join("payload"), "new payload");
    let mut undo = InstallRollback::new(ws.path());
    undo.place_checked(&slot, source.path(), || Ok(())).unwrap();
    let moved = ws.path().join("original-published");
    fs::rename(&slot, &moved).unwrap();
    write(&slot.join("payload"), "new payload");
    let error = undo.undo(
        WorkspaceError::IndexRender {
            reason: "downstream failure".into(),
        },
        None,
        &[],
    );
    let message = error.to_string();
    assert!(message.contains("rollback conflict"), "{message}");
    assert_eq!(
        fs::read_to_string(slot.join("payload")).unwrap(),
        "new payload"
    );
    let stash = fs::read_dir(ws.path().join("vibevm"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".vibe-install-rollback-")
        })
        .unwrap();
    assert_eq!(
        fs::read_to_string(stash.join("0/.git/config")).unwrap(),
        "old config"
    );
}

#[test]
fn checked_ignore_noop_does_not_adopt_a_foreign_removed_entry() {
    let ws = TempDir::new().unwrap();
    let path = ws.path().join(".gitignore");
    write(&path, "slot/\n");
    let mut undo = InstallRollback::new(ws.path());
    undo.ignore_checked(ws.path(), "slot").unwrap();
    fs::write(&path, "foreign/\n").unwrap();
    assert!(undo.ignore_checked(ws.path(), "slot").is_err());
    let _ = undo.undo(
        WorkspaceError::IndexRender {
            reason: "stop".into(),
        },
        None,
        &[],
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "foreign/\n");
}

#[test]
#[cfg(unix)]
fn checked_copy_revalidates_before_moving_a_changed_target() {
    let ws = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let slot = ws.path().join("slot");
    write(&slot.join("old"), "original");
    write(&source.path().join("new"), "candidate");
    let expected = super::super::checked_placement::Tree::read(&slot).unwrap();
    let mut undo = InstallRollback::new(ws.path());
    let error = undo
        .place_checked(&slot, source.path(), || {
            let stash = fs::read_dir(ws.path().join("vibevm"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            assert!(stash.join("0.candidate/new").exists());
            write(&slot.join("old"), "concurrent change");
            if super::super::checked_placement::Tree::read(&slot).unwrap() != expected {
                Err("captured target changed".into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
    let error = undo.undo(error, None, &[]);
    assert!(error.to_string().contains("captured target changed"));
    assert_eq!(
        fs::read_to_string(slot.join("old")).unwrap(),
        "concurrent change"
    );
}
#[test]
#[cfg(unix)]
fn checked_rename_failure_after_stash_restores_old_directory_and_modes() {
    use std::os::unix::fs::PermissionsExt;
    let ws = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let slot = ws.path().join("slot");
    write(&slot.join(".git/config"), "old");
    write(&source.path().join("sub/new"), "new");
    fs::set_permissions(source.path().join("sub"), fs::Permissions::from_mode(0o750)).unwrap();
    let before = tree(&slot);
    let mut undo = InstallRollback::new(ws.path());
    let error = undo
        .place_checked(&slot, source.path(), || {
            let stash = fs::read_dir(ws.path().join("vibevm"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            assert_eq!(
                fs::metadata(stash.join("0.candidate/sub"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o750
            );
            fs::remove_dir_all(stash.join("0.candidate")).unwrap();
            Ok(())
        })
        .unwrap_err();
    assert!(
        !slot.exists(),
        "old directory was stashed before actual rename failed"
    );
    let error = undo.undo(error, None, &[]);
    assert!(
        !matches!(error, WorkspaceError::InstallRollback { .. }),
        "{error}"
    );
    assert_eq!(tree(&slot), before);
}
