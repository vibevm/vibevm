//! Source requests survive actual workspace placeholder finalization.
use super::*;
use std::fs;
use tempfile::TempDir;
#[cfg(test)]
fn write(dir: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn configuration_intent_survives_version_finalization_without_merging_nodes() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(
        root,
        "vibe.toml",
        "[project]\nname='root'\nversion='0.1.0'\n[workspace]\nmembers=['a','b']\n[workspace.versions]\nshared='^1'\n",
    );
    for (node, members, optional) in [("a", "", true), ("b", ",members=[]", false)] {
        write(
            root,
            format!("{node}/vibe.toml"),
            &format!(
                "[project]\nname='{node}'\nversion='0.1.0'\n[requires.packages.'org.example/shared']\nversion.var='shared'\nlink='dynamic'\noptional={optional}\n[requires.packages.'org.example/shared'.configuration]\npolicy='require-selection'\nselections.mode={{intent='exact'{members}}}\n"
            ),
        );
    }
    let original_a = Manifest::read(root.join("a/vibe.toml")).unwrap();
    let original_b = Manifest::read(root.join("b/vibe.toml")).unwrap();
    let ws = Workspace::load(root).unwrap();
    for (name, original) in [("a", &original_a), ("b", &original_b)] {
        let current = &ws.member_by_rel_path(name).unwrap().manifest;
        assert!(current.requires.var_packages.is_empty());
        assert_eq!(current.requires.optional, original.requires.optional);
        assert_eq!(
            current.requires.optional.get("org.example/shared"),
            Some(&(name == "a"))
        );
        assert_eq!(
            current.requires.packages[0].to_string(),
            "org.example/shared@^1"
        );
        assert_eq!(
            current.requires.configurations,
            original.requires.configurations
        );
        let path = root.join(format!("{name}-finalized.toml"));
        current.write(&path).unwrap();
        assert_eq!(
            Manifest::read(path).unwrap().requires.configurations,
            original.requires.configurations
        );
    }
    assert_ne!(
        original_a.requires.configurations,
        original_b.requires.configurations
    );
    assert_eq!(
        Manifest::read(root.join("a/vibe.toml")).unwrap(),
        original_a
    );
    assert_eq!(
        Manifest::read(root.join("b/vibe.toml")).unwrap(),
        original_b
    );
}
