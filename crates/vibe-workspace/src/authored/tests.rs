//! Raw variable identity and map-order equivalence are independent invariants.
#![cfg(test)]
use super::*;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("member")).unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[project]\nname='root'\nversion='1.0.0'\n[workspace]\nmembers=['member']\n[workspace.versions]\ncore='^1'\nalias='^1'\n").unwrap();
    std::fs::write(root.path().join("member/vibe.toml"), "[project]\nname='member'\nversion='1.0.0'\n[requires.packages]\n'org.example/pkg'={version.var='core'}\n").unwrap();
    root
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT-LAW")]
fn discovery_retains_raw_variable_identity_beside_effective_constraint() {
    let root = fixture();
    let workspace = Workspace::load(root.path()).unwrap();
    let authored = workspace.authored_node("member").unwrap();
    assert_eq!(authored.requires.var_packages.len(), 1);
    assert!(authored.requires.packages.is_empty());
    assert!(
        workspace.members[0]
            .manifest
            .requires
            .var_packages
            .is_empty()
    );
    assert_eq!(
        workspace.members[0].manifest.requires.packages[0]
            .version
            .to_string(),
        "^1"
    );
    crate::install::validate_source_admission(
        &workspace,
        &[],
        vibe_core::manifest::SpecFormat::Mixed,
    )
    .unwrap();
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT-LAW")]
fn same_effective_constraint_cannot_hide_a_changed_authoring_variable() {
    let root = fixture();
    let before = Workspace::load(root.path()).unwrap();
    let path = root.path().join("member/vibe.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("var='core'", "var='alias'");
    std::fs::write(&path, &text).unwrap();
    let after = Workspace::load(root.path()).unwrap();
    assert_eq!(before.members[0].manifest, after.members[0].manifest);
    assert!(
        crate::install::validate_source_admission(
            &before,
            &[],
            vibe_core::manifest::SpecFormat::Mixed
        )
        .is_err()
    );
    assert!(
        before
            .validate_authored_transition(&after, &before.root, before.authored_node(".").unwrap())
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), text);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT-LAW")]
fn known_reverse_order_requires_delta_matches_published_toml_map() {
    let root = fixture();
    let before = Workspace::load(root.path()).unwrap();
    let mut selected = before.authored_node(".").unwrap().clone();
    selected.requires.packages = vec![
        "org.example/z@^1".parse().unwrap(),
        "org.example/a@^1".parse().unwrap(),
    ];
    let after = Workspace::discover_with_selected_manifest(&before.root, &selected).unwrap();
    before
        .validate_authored_transition(&after, &before.root, &selected)
        .unwrap();
    selected.write(root.path().join("vibe.toml")).unwrap();
    assert_eq!(after.root_manifest.requires.packages[0].name.as_str(), "z");
    assert_eq!(
        Manifest::read(root.path().join("vibe.toml"))
            .unwrap()
            .requires
            .packages[0]
            .name
            .as_str(),
        "a"
    );
    crate::install::validate_source_admission(&after, &[], vibe_core::manifest::SpecFormat::Mixed)
        .unwrap();
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT-LAW")]
fn override_cannot_adopt_changed_or_added_nonselected_authored_nodes() {
    for added in [false, true] {
        let root = fixture();
        let before = Workspace::load(root.path()).unwrap();
        if added {
            let nested = root.path().join("member");
            std::fs::write(
                nested.join("vibe.toml"),
                "[project]\nname='member'\nversion='1.0.0'\n[workspace]\nmembers=['new']\n",
            )
            .unwrap();
            std::fs::create_dir(nested.join("new")).unwrap();
            std::fs::write(
                nested.join("new/vibe.toml"),
                "[project]\nname='new'\nversion='1.0.0'\n",
            )
            .unwrap();
        } else {
            let path = root.path().join("member/vibe.toml");
            let text = std::fs::read_to_string(&path)
                .unwrap()
                .replace("version='1.0.0'", "version='2.0.0'");
            std::fs::write(path, text).unwrap();
        }
        let selected = before.authored_node(".").unwrap();
        let after = Workspace::discover_with_selected_manifest(&before.root, selected).unwrap();
        assert!(
            before
                .validate_authored_transition(&after, &before.root, selected)
                .is_err()
        );
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT-LAW")]
fn mutating_effective_fields_does_not_rebase_the_retained_authored_guard() {
    let root = fixture();
    let mut workspace = Workspace::load(root.path()).unwrap();
    workspace.members[0].manifest.project.as_mut().unwrap().name = "different".into();
    assert!(workspace.validate_authored_projection().is_err());
    let original = workspace.authored_node(".").unwrap().clone();
    assert!(
        workspace
            .advance_authored_selected(&workspace.root.clone(), &original, &original)
            .is_err()
    );
    assert_eq!(workspace.authored_node(".").unwrap(), &original);
}
