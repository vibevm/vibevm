use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
};
use vibe_core::manifest::change::{CHANGE_UPGRADE, ChangeMigrationSelection};
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/valid/representative.toml"
));
fn fixture(text: &str) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n",
    )
    .unwrap();
    let directory = root.path().join("docs/custom-specs/changes").join(ID);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("change.toml"), text).unwrap();
    fs::write(directory.join("intent.md"), "# Intent\n").unwrap();
    fs::write(directory.join("acceptance.md"), "# Acceptance\n").unwrap();
    (root, directory)
}
fn bytes(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, p: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<_> = fs::read_dir(p)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(root, &path, out)
            } else {
                out.push((
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut out = vec![];
    walk(root, root, &mut out);
    out
}
#[test]
fn configured_nondefault_root_is_used_by_actual_lookup() {
    let (root, dir) = fixture(TEXT);
    let before = bytes(root.path());
    let saved =
        read_saved_change(root.path(), ".", Some(Path::new("docs/custom-specs")), ID).unwrap();
    assert_eq!(saved.input_observations.len(), 2);
    assert_eq!(saved.manifest.id().as_str(), ID);
    assert!(saved.manifest.directory().starts_with("docs/custom-specs"));
    assert!(matches!(
        read_saved_change(root.path(), ".", None, ID),
        Err(SavedChangeError::Unavailable("change manifest"))
    ));
    fs::write(
        dir.join("change.toml"),
        TEXT.replace("first-title", "new-slug")
            .replace("Initial user title", "Renamed title"),
    )
    .unwrap();
    let renamed =
        read_saved_change(root.path(), ".", Some(Path::new("docs/custom-specs")), ID).unwrap();
    assert_eq!(saved.manifest.id(), renamed.manifest.id());
    assert_eq!(saved.manifest.directory(), renamed.manifest.directory());
    fs::write(dir.join("change.toml"), TEXT).unwrap();
    assert_eq!(bytes(root.path()), before);
}
#[test]
fn actual_lookup_refuses_missing_role_document_without_writes() {
    let (root, dir) = fixture(TEXT);
    fs::remove_file(dir.join("intent.md")).unwrap();
    let before = bytes(root.path());
    assert!(
        matches!(read_saved_change(root.path(),".",Some(Path::new("docs/custom-specs")),ID),Err(SavedChangeError::MissingDocument(path)) if path=="intent.md")
    );
    assert_eq!(bytes(root.path()), before);
}
#[test]
fn actual_saved_migration_preserves_bytes_without_publication() {
    let pilot = TEXT.replacen("schema = 1", "schema = 0", 1);
    let (root, dir) = fixture(&pilot);
    let before = bytes(root.path());
    let (plan, snapshot) = plan_saved_change_migration(
        root.path(),
        ".",
        Some(Path::new("docs/custom-specs")),
        ID,
        ChangeMigrationSelection {
            source_epoch: 0,
            target_epoch: 1,
            recipe: CHANGE_UPGRADE,
        },
    )
    .unwrap();
    assert_eq!(snapshot.bytes, pilot.as_bytes());
    assert_eq!(plan.after, TEXT);
    assert_eq!(fs::read(dir.join("change.toml")).unwrap(), pilot.as_bytes());
    assert_eq!(bytes(root.path()), before);
    println!(
        "M02_B1_MIGRATION {}",
        serde_json::json!({"before":plan.before,"after":plan.after,"unchanged_files":before})
    );
}
