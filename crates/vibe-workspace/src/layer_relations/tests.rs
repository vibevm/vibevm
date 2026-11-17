use super::*;
use std::fs;

fn layer(name: &str, tail: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("vibevm/vibespecs/common")).unwrap();
    fs::write(dir.path().join("vibe.toml"), format!("[project]\ngroup='org.demo'\nname='{name}'\nversion='1.0.0'\n[layers]\nschema=1\n{tail}")).unwrap();
    fs::write(
        dir.path().join("vibevm/vibespecs/common/STYLE.md"),
        "# Style {#width}\n\nWidth preference.\n",
    )
    .unwrap();
    let workspace = crate::Workspace::load(dir.path()).unwrap();
    crate::specmap_authority::publish(
        &workspace,
        dir.path(),
        &std::collections::BTreeMap::from([(".".into(), Vec::new())]),
    )
    .unwrap();
    dir
}
fn target(policy: &str) -> String {
    format!(
        "[[layers.targets]]\nid='width'\nsource='spec://org.demo/project/common/STYLE#width'\npolicy='{policy}'\n"
    )
}
fn relation(owner: &str, kind: &str) -> String {
    format!(
        "[[layers.relations]]\nid='width-{owner}'\nkind='{kind}'\ntarget_layer='project'\ntarget='width'\nsource='spec://org.demo/{owner}/common/STYLE#width'\n"
    )
}
fn input(layer: LayerName, root: &Path, enabled: bool) -> LayerInput<'_> {
    LayerInput {
        layer,
        root: Some(root),
        enabled,
        available: true,
    }
}
#[test]
fn explicit_replacements_supersede_only_with_lower_grant_and_disable_restores_lower() {
    let project = layer("project", &target("overridable"));
    let user = layer("user", &relation("user", "replace"));
    let override_layer = layer("override", &relation("override", "replace"));
    let active = resolve(&[
        input(LayerName::Project, project.path(), true),
        input(LayerName::User, user.path(), true),
        input(LayerName::Override, override_layer.path(), true),
    ]);
    assert!(active.denied_layers.is_empty(), "{:?}", active.diagnostics);
    assert_eq!(active.relations[0].state, "superseded");
    assert_eq!(active.relations[1].state, "active");
    assert!(
        active
            .notice(LayerName::Override)
            .contains("Earlier prefix bytes remain present")
    );
    let disabled = resolve(&[
        input(LayerName::Project, project.path(), true),
        input(LayerName::User, user.path(), true),
        input(LayerName::Override, override_layer.path(), false),
    ]);
    assert_eq!(disabled.relations[0].state, "active");
    assert_eq!(disabled.relations[1].state, "disabled");
}
#[test]
fn protected_replacement_refuses_but_supplement_does_not_claim_replacement() {
    let project = layer("project", &target("protected"));
    let user = layer("user", &relation("user", "replace"));
    let denied = resolve(&[
        input(LayerName::Project, project.path(), true),
        input(LayerName::User, user.path(), true),
    ]);
    assert!(denied.denied_layers.contains(&LayerName::User));
    assert!(denied.relations[0].reason.contains("protected"));
    let supplement = layer("user", &relation("user", "supplement"));
    let allowed = resolve(&[
        input(LayerName::Project, project.path(), true),
        input(LayerName::User, supplement.path(), true),
    ]);
    assert!(
        allowed.denied_layers.is_empty(),
        "{:?}",
        allowed.diagnostics
    );
    assert_eq!(allowed.relations[0].kind, LayerRelationKind::Supplement);
}
#[test]
fn ambiguous_anchor_and_competing_replacements_deny_whole_optional_layer() {
    let project = layer("project", &target("overridable"));
    let text =
        relation("user", "replace") + &relation("user", "replace").replace("width-user", "second");
    let user = layer("user", &text);
    let denied = resolve(&[
        input(LayerName::Project, project.path(), true),
        input(LayerName::User, user.path(), true),
    ]);
    assert!(denied.denied_layers.contains(&LayerName::User));
    assert!(denied.relations.iter().all(|r| r.state == "denied"));
    let user = layer("user", &relation("user", "replace"));
    fs::write(
        user.path().join("vibevm/vibespecs/common/STYLE.md"),
        "# A {#width}\n# B {#width}\n",
    )
    .unwrap();
    let denied = resolve(&[
        input(LayerName::Project, project.path(), true),
        input(LayerName::User, user.path(), true),
    ]);
    assert!(
        denied
            .diagnostics
            .iter()
            .any(|(_, d)| d.contains("ambiguous"))
    );
}

#[cfg(unix)]
#[test]
fn unsafe_manifest_is_not_followed_and_disabled_roots_are_not_read() {
    use std::os::unix::fs::symlink;
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("manifest"), "PRIVATE_MANIFEST_CANARY").unwrap();
    let root = tempfile::tempdir().unwrap();
    symlink(
        outside.path().join("manifest"),
        root.path().join("vibe.toml"),
    )
    .unwrap();
    let disabled = resolve(&[input(LayerName::User, root.path(), false)]);
    assert!(!disabled.diagnostics.is_empty());
    assert!(!format!("{:?}", disabled.diagnostics).contains("PRIVATE_MANIFEST_CANARY"));
    let enabled = resolve(&[input(LayerName::User, root.path(), true)]);
    assert!(enabled.denied_layers.contains(&LayerName::User));
    assert!(!format!("{:?}", enabled.diagnostics).contains("PRIVATE_MANIFEST_CANARY"));
}
#[test]
fn admitted_unindexed_relation_source_is_carried_in_notice() {
    let project = layer("project", &target("overridable"));
    let user = layer("user", &relation("user", "replace"));
    let model = resolve(&[
        input(LayerName::Project, project.path(), true),
        input(LayerName::User, user.path(), true),
    ]);
    assert!(model.notice(LayerName::User).contains("Width preference."));
}
