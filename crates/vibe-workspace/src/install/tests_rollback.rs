//! B-206: a static lane that fails to compile leaves the project as it was.

use std::collections::BTreeMap;

use tempfile::TempDir;

use super::test_helpers::*;
use super::*;

fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
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

fn manifest(requires: &[&str]) -> String {
    let mut text =
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n[requires.packages]\n".to_string();
    for name in requires {
        text.push_str(&format!(
            "\"org.vibevm/{name}\" = {{ version = \"^1.0\", link = \"static\" }}\n"
        ));
    }
    text
}

fn simple(name: &str) -> (ResolvedDep, TempDir) {
    dep_with_boot(
        name,
        "1.0.0",
        &format!("[boot_snippet]\nsource = \"boot/{name}.md\"\n"),
        &format!("boot/{name}.md"),
        &format!("# {name}\n\n{name} rule.\n"),
    )
}

/// A `normal` package whose contract `#use`s a package nobody installed: its
/// slot places cleanly and its static lane cannot compile.
fn broken() -> (ResolvedDep, TempDir) {
    let (mut dep, dir) = dep_with_boot(
        "broken",
        "1.0.0",
        "[boot_snippet]\nsource = \"vibevm/vibespecs/boot/broken.md\"\n",
        "vibevm/vibespecs/boot/broken.md",
        "# Broken {#root}\n#use spec://org.vibevm/ghost/boot/base#root\nBROKEN\n",
    );
    write(
        dir.path(),
        "vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"broken\"\nkind = \"flow\"\n\
         version = \"1.0.0\"\nformat = \"normal\"\n\n\
         [boot_snippet]\nsource = \"vibevm/vibespecs/boot/broken.md\"\n",
    );
    dep.manifest = Manifest::read(dir.path().join("vibe.toml")).unwrap();
    (dep, dir)
}

fn apply(ws: &Workspace, resolution: &[ResolvedDep]) -> Result<InstallOutcome, WorkspaceError> {
    apply_resolution_with_spec_format_and_slot_lifecycle_traced(
        ws,
        resolution,
        // No verifier: a present slot is reconciled again, so the failed
        // apply below changes an existing slot as well as adding and pruning.
        SlotIntegrity::Verify,
        SpecFormat::Xml,
        None,
        SlotLifecycleMode::None,
        None,
    )
}

#[test]
fn a_failed_static_compile_leaves_slots_and_boot_byte_identical() {
    let ws_dir = TempDir::new().unwrap();
    write(ws_dir.path(), "vibe.toml", &manifest(&["kept", "pruned"]));
    write(ws_dir.path(), boot_rel("00-core.md"), "# core\n");
    let (kept, _kept_dir) = simple("kept");
    let (pruned, _pruned_dir) = simple("pruned");
    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply(&ws, &[kept.clone(), pruned]).unwrap();
    assert!(ws_dir.path().join(boot_rel("STATIC.xml")).is_file());

    write(ws_dir.path(), "vibe.toml", &manifest(&["kept", "broken"]));
    let before = snapshot(ws_dir.path());
    let ws = Workspace::load(ws_dir.path()).unwrap();
    let (broken, _broken_dir) = broken();
    let error = apply(&ws, &[kept, broken]).unwrap_err();

    let WorkspaceError::InlineCompile { lane, .. } = &error else {
        panic!("expected the static lane refusal, got {error}")
    };
    let message = error.to_string();
    assert!(lane.ends_with("STATIC.xml"), "{lane}");
    assert!(message.contains(&boot_rel("STATIC.xml")), "{message}");
    assert!(!message.contains("INLINE"), "{message}");
    assert_eq!(
        snapshot(ws_dir.path()),
        before,
        "the dependency tree and the boot lane are as they were before the install"
    );
}

#[test]
fn a_committed_apply_keeps_its_slots_and_a_rolled_back_one_restores_them() {
    let ws_dir = TempDir::new().unwrap();
    write(ws_dir.path(), "vibe.toml", &manifest(&["kept"]));
    let (kept, _kept_dir) = simple("kept");
    let ws = Workspace::load(ws_dir.path()).unwrap();
    let mut outcome = apply(&ws, std::slice::from_ref(&kept)).unwrap();
    let slot = vibedeps::slot_abs_path(&ws.root, &kept.group, &kept.name, &kept.version);
    assert!(slot.is_dir());
    outcome.roll_back(None).unwrap();
    assert!(
        !slot.exists(),
        "the caller's undo removes the slot it placed"
    );

    let mut outcome = apply(&ws, std::slice::from_ref(&kept)).unwrap();
    outcome.commit();
    outcome.roll_back(None).unwrap();
    assert!(
        slot.is_dir(),
        "an undo after the commit has nothing left to undo"
    );
}
