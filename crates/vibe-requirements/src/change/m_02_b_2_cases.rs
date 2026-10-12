use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
};
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/valid/representative.toml"
));
const WORK: &str = "schema=1\n[[items]]\nid='work-a'\ntouches=['src/a.rs']\n[[items]]\nid='work-b'\npredecessors=['work-a']\nneeds=[{kind='verification',target='tests/a.rs'}]\n[[items]]\nid='work-c'\ntouches=['src/c.rs']\n";
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n",
    )
    .unwrap();
    let dir = root.path().join("docs/custom-specs/changes").join(ID);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("change.toml"),
        format!(
            "{TEXT}\n[[documents]]\nrole='work'\npath='work.toml'\nnormative=false\ninput=true\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.join("intent.md"),
        "# Intent\n@fact:INTENT Shared intent.\n",
    )
    .unwrap();
    fs::write(
        dir.join("acceptance.md"),
        "# Acceptance\n@fact:ACCEPT Local criterion.\n",
    )
    .unwrap();
    fs::write(dir.join("work.toml"), WORK).unwrap();
    (root, dir)
}
fn saved(root: &Path) -> SavedChange {
    read_saved_change(root, ".", Some(Path::new("docs/custom-specs")), ID).unwrap()
}
fn inventory(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut paths: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        paths.sort();
        for p in paths {
            if p.is_dir() {
                walk(root, &p, out)
            } else {
                out.push((p.strip_prefix(root).unwrap().into(), fs::read(p).unwrap()));
            }
        }
    }
    let mut out = vec![];
    walk(root, root, &mut out);
    out
}
#[test]
fn actual_saved_lookup_separates_normative_work_and_closure() {
    let (root, dir) = fixture();
    let before = inventory(root.path());
    let captured = saved(root.path());
    let a = captured.identity_analysis().unwrap();
    assert_eq!(a.graph.items().len(), 3);
    assert_eq!(inventory(root.path()), before);
    let manifest = fs::read_to_string(dir.join("change.toml")).unwrap();
    fs::write(
        dir.join("change.toml"),
        manifest
            .replace("first-title", "cosmetic")
            .replace("Initial user title", "Renamed display"),
    )
    .unwrap();
    fs::write(
        dir.join("closure.md"),
        "Closed snapshot refers to original intent.\n",
    )
    .unwrap();
    fs::write(
        dir.join("run.log"),
        "done owner current are observations only\n",
    )
    .unwrap();
    let cosmetic = saved(root.path()).identity_analysis().unwrap();
    assert_eq!(a.intent, cosmetic.intent);
    assert_eq!(a.work, cosmetic.work);
    fs::write(
        dir.join("intent.md"),
        "# Intent\n@fact:INTENT Changed normative promise.\n",
    )
    .unwrap();
    let edited = saved(root.path()).identity_analysis().unwrap();
    assert_ne!(a.intent, edited.intent);
    assert_eq!(a.work, edited.work);
    assert_eq!(captured.identity_analysis().unwrap().intent, a.intent);
    let state = inventory(root.path());
    let _ = saved(root.path()).identity_analysis().unwrap();
    assert_eq!(state, inventory(root.path()));
    println!(
        "M02_B2_IDENTITIES {}",
        serde_json::json!({"intent_before":a.intent.digest(),"intent_after":edited.intent.digest(),"work_before":a.work.digest(),"work_after":edited.work.digest(),"retained_snapshot_unchanged":true,"query_files_unchanged":state.len()})
    );
}
#[test]
fn selected_work_excludes_unrelated_item_edits() {
    let (root, dir) = fixture();
    let first = saved(root.path());
    let a = first.identity_analysis().unwrap();
    let selected = a
        .graph
        .contribution_identity(&first.manifest, &["work-b"])
        .unwrap();
    fs::write(
        dir.join("work.toml"),
        WORK.replace("src/c.rs", "src/other.rs"),
    )
    .unwrap();
    let next = saved(root.path());
    let b = next.identity_analysis().unwrap();
    assert_eq!(a.intent, b.intent);
    assert_ne!(a.work, b.work);
    assert_eq!(
        selected,
        b.graph
            .contribution_identity(&next.manifest, &["work-b"])
            .unwrap()
    );
    fs::write(
        dir.join("work.toml"),
        WORK.replace("src/a.rs", "src/ancestor.rs"),
    )
    .unwrap();
    let changed = saved(root.path());
    let c = changed.identity_analysis().unwrap();
    assert_ne!(
        selected,
        c.graph
            .contribution_identity(&changed.manifest, &["work-b"])
            .unwrap()
    );
    println!(
        "M02_B2_WORK {}",
        serde_json::json!({"whole_before":a.work.digest(),"whole_after":b.work.digest(),"selected_before":selected.digest(),"selected_after":b.graph.contribution_identity(&next.manifest,&["work-b"]).unwrap().digest()})
    );
}
#[test]
fn missing_normative_observation_refuses() {
    let (root, dir) = fixture();
    let mut captured = saved(root.path());
    captured
        .input_observations
        .retain(|(p, _)| !p.ends_with("intent.md"));
    assert!(
        matches!(captured.identity_analysis(),Err(SavedChangeError::MissingDocument(path))if path=="intent.md")
    );
    fs::write(
        dir.join("work.toml"),
        WORK.replace("id='work-c'", "id='work-c'\ndone=true"),
    )
    .unwrap();
    let before = inventory(root.path());
    assert!(
        matches!(saved(root.path()).identity_analysis(),Err(SavedChangeError::Declaration(e))if e.kind==vibe_core::manifest::change::ChangeErrorKind::Work&&e.field=="work")
    );
    assert_eq!(before, inventory(root.path()));
}
