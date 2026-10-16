use super::*;
use vibe_safefs::Project;

fn capture(rows: &[(&str, &str)]) -> Vec<CapturedSourceFile> {
    let root = tempfile::tempdir().unwrap();
    for (path, text) in rows {
        let target = root.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, text).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    rows.iter()
        .map(|(path, _)| {
            CapturedSourceFile::new(
                (*path).into(),
                project
                    .read_file_snapshot_bounded(path, 4096)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap()
        })
        .collect()
}

const DECL: &str = "[subskill]\npath='stack/rust'\ndelivery='lazy-push'\ndescription='Rust guidance'\n[content]\nfiles_written=['guide.md']\n";

#[test]
fn selection_preserves_original_bytes_and_independent_skill() {
    let files = capture(&[
        ("subskills/stack/rust/vibe-subskill.toml", DECL),
        ("subskills/stack/rust/guide.md", "# Captured Rust\n"),
        ("skills/unrelated/SKILL.md", "Independent skill\n"),
        ("unselected.txt", "Keep this too\n"),
    ]);
    let before: Vec<_> = files.iter().map(|f| f.snapshot().bytes.clone()).collect();
    assert!(
        select_captured_subskills(&files, &BTreeSet::new())
            .unwrap()
            .is_empty()
    );
    let active = select_captured_subskills(&files, &["stack/rust".into()].into()).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(
        active[0].declaration.subskill.delivery,
        vibe_core::manifest::DeliveryMode::LazyPush
    );
    assert_eq!(
        active[0].content["subskills/stack/rust/guide.md"],
        b"# Captured Rust\n"
    );
    assert!(!active[0].definition_digest.is_empty());
    assert_eq!(
        before,
        files
            .iter()
            .map(|f| f.snapshot().bytes.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn missing_declaration_content_and_escaping_paths_refuse() {
    let selected = ["stack/rust".into()].into();
    assert!(
        select_captured_subskills(&[], &selected)
            .unwrap_err()
            .contains("no captured declaration")
    );
    let files = capture(&[("subskills/stack/rust/vibe-subskill.toml", DECL)]);
    assert!(
        select_captured_subskills(&files, &selected)
            .unwrap_err()
            .contains("missing captured content")
    );
    for invalid in [
        DECL.replace("guide.md", "../outside.md"),
        DECL.replace("stack/rust", "other"),
        DECL.replace("Rust guidance", " "),
    ] {
        let files = capture(&[("subskills/stack/rust/vibe-subskill.toml", &invalid)]);
        assert!(select_captured_subskills(&files, &selected).is_err());
    }
}
