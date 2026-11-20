use super::host_path::observe;
#[test]
fn held_host_path_rewalk_refuses_disappearance_and_ancestor_replacement() {
    for world in ["missing", "ancestor", "symlink", "kind"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(root.join("parent")).unwrap();
        std::fs::write(root.join("parent/target"), b"contents never needed").unwrap();
        let proof = observe(&root, "parent/target").unwrap();
        match world {
            "missing" => std::fs::remove_file(root.join("parent/target")).unwrap(),
            "kind" => {
                std::fs::remove_file(root.join("parent/target")).unwrap();
                std::fs::create_dir(root.join("parent/target")).unwrap();
            }
            "ancestor" => {
                std::fs::rename(root.join("parent"), root.join("retained")).unwrap();
                std::fs::create_dir(root.join("parent")).unwrap();
                std::fs::write(root.join("parent/target"), b"same spelling").unwrap();
            }
            "symlink" => {
                std::fs::rename(root.join("parent"), root.join("retained")).unwrap();
                std::os::unix::fs::symlink(root.join("retained"), root.join("parent")).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(proof.recheck().is_err(), "{world}");
    }
}
