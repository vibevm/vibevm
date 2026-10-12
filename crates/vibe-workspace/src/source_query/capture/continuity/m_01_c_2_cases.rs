//! Index shape and selected-source join cases. The install target separately
//! runs the canonical complete-tree replay; no local digest is its substitute.

use std::fs;

use vibe_core::PackageRef;
use vibe_safefs::Project;

use super::{CapturedSourceFile, SourceMembershipIndex, SourceQueryUnavailable};

#[test]
fn original_index_retains_exact_snapshot_members_and_rejects_wrong_pin() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("spec")).unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(root.path().join("spec/DOC.md"), "# Original\n").unwrap();
    let project = Project::open(root.path()).unwrap();
    let captured = ["spec/DOC.md", "vibe.toml"]
        .into_iter()
        .map(|relative| {
            CapturedSourceFile::new(
                relative.into(),
                project
                    .read_file_snapshot_bounded(relative, 1024)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let pin = PackageRef::parse("flow:org.demo/dep@=1.0.0").unwrap();
    // This catalogue is inert. The install replay owner alone mints proof.
    let index = SourceMembershipIndex::from_captured(&pin, captured.clone()).unwrap();
    assert_eq!(index.file_count(), 2);
    assert!(index.contains("spec/DOC.md"));
    assert!(!index.contains("spec/UNRECORDED.md"));
    let wrong = PackageRef::parse("flow:org.demo/other@=1.0.0").unwrap();
    assert!(matches!(
        SourceMembershipIndex::from_captured(&wrong, captured),
        Err(SourceQueryUnavailable::SourceProofUnavailable(
            "original package pin"
        ))
    ));
}
