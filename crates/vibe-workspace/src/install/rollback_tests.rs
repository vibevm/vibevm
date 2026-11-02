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
