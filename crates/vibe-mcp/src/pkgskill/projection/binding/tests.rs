use std::fs;
use std::path::Path;

use vibe_core::manifest::SkillDecl;
use vibe_core::{Group, PackageKind, PackageName};

use super::*;

fn provider(root: &Path, name: &str, skill: &str, path: &str) -> ProjectSkillProviderInput {
    ProjectSkillProviderInput {
        provider: DeclaredSkillProvider::Authored {
            group: Group::parse("org.example").unwrap(),
            name: PackageName::parse(name).unwrap(),
            version: "0.1.0".into(),
            kind: PackageKind::Tool,
            root: root.to_path_buf(),
        },
        declarations: vec![SkillDecl {
            name: skill.into(),
            path: path.into(),
            description: None,
            agents: vec!["claude".into()],
            include: Vec::new(),
        }],
    }
}

fn seed(root: &Path, relative: &str) {
    let source = root.join(relative);
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "body").unwrap();
}

#[test]
fn physical_collision_overlap_and_empty_selection_are_plan_errors() {
    let project = tempfile::tempdir().unwrap();
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    seed(one.path(), "skills/one");
    seed(two.path(), "skills/two");
    let error = lower_project_skill_bindings(
        project.path(),
        vec![
            provider(one.path(), "one", "same", "skills/one"),
            provider(two.path(), "two", "same", "skills/two"),
        ],
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("collide at physical target"), "{error}");

    seed(project.path(), ".claude/skills/overlap");
    let error = lower_project_skill_bindings(
        project.path(),
        vec![provider(
            project.path(),
            "overlap",
            "overlap",
            ".claude/skills/overlap",
        )],
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("overlaps target"), "{error}");

    let empty = tempfile::tempdir().unwrap();
    fs::create_dir_all(empty.path().join("skills/empty")).unwrap();
    let error = lower_project_skill_bindings(
        project.path(),
        vec![provider(empty.path(), "empty", "empty", "skills/empty")],
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("selects zero files"), "{error}");

    let zero = tempfile::tempdir().unwrap();
    seed(zero.path(), "skills/zero");
    let mut input = provider(zero.path(), "zero", "zero", "skills/zero");
    input.declarations[0].include = vec!["does-not-match/**".into()];
    let error = lower_project_skill_bindings(project.path(), vec![input])
        .unwrap_err()
        .to_string();
    assert!(error.contains("selects zero files"), "{error}");
}

/// Planning judges the complete selected source set through the shared fold
/// law before any staging or write: a source carrying both aliases of one
/// physical file refuses with zero target mutation and a readable, unchanged,
/// non-applying receipt.
#[test]
fn dual_alias_source_selection_refuses_with_zero_mutation() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    let body = package.path().join("skills/demo");
    fs::create_dir_all(&body).unwrap();
    fs::write(body.join("SKILL.md"), "body").unwrap();
    let binding = lower_project_skill_bindings(
        project.path(),
        vec![provider(package.path(), "one", "demo", "skills/demo")],
    )
    .unwrap()
    .remove(0);
    reconcile_project_skill_binding(project.path(), &binding).unwrap();
    let target = project.path().join(".claude/skills/demo");
    let receipt_path = project.path().join(".vibe/package-skills.toml");
    let committed = fs::read(&receipt_path).unwrap();

    // Both full-Unicode aliases in one source: distinct files here, one
    // physical file on a case-insensitive host.
    fs::write(body.join("Maße.md"), "eszett").unwrap();
    fs::write(body.join("MASSE.md"), "upper").unwrap();
    let error = lower_project_skill_bindings(
        project.path(),
        vec![provider(package.path(), "one", "demo", "skills/demo")],
    )
    .unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("one file on a case-insensitive host"),
        "{error}"
    );

    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "body");
    assert!(!target.join("Maße.md").exists());
    assert!(!target.join("MASSE.md").exists());
    assert_eq!(fs::read(&receipt_path).unwrap(), committed);
    let receipt: vibe_wire::generated::package_skill_receipt::PackageSkillReceipt =
        toml::from_str(&String::from_utf8(committed).unwrap()).unwrap();
    assert!(receipt.applying.is_none(), "no `applying` receipt exists");
}

/// The canonical identity is composition-aware, so a source carrying both
/// the NFC and the NFD spelling of one name refuses at planning — before any
/// stage, durable intent, or target write.
#[test]
fn nfc_and_nfd_source_spellings_refuse_before_stage_or_intent() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    let body = package.path().join("skills/demo");
    fs::create_dir_all(&body).unwrap();
    fs::write(body.join("SKILL.md"), "body").unwrap();
    let binding = lower_project_skill_bindings(
        project.path(),
        vec![provider(package.path(), "one", "demo", "skills/demo")],
    )
    .unwrap()
    .remove(0);
    reconcile_project_skill_binding(project.path(), &binding).unwrap();
    let target = project.path().join(".claude/skills/demo");
    let receipt_path = project.path().join(".vibe/package-skills.toml");
    let committed = fs::read(&receipt_path).unwrap();

    // Composed `é` (U+00E9) and decomposed `e` + U+0301: two files here,
    // one file on macOS.
    fs::write(body.join("caf\u{e9}.md"), "composed").unwrap();
    fs::write(body.join("cafe\u{301}.md"), "decomposed").unwrap();
    let error = lower_project_skill_bindings(
        project.path(),
        vec![provider(package.path(), "one", "demo", "skills/demo")],
    )
    .unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("one file on a case-insensitive host"),
        "{error}"
    );

    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "body");
    assert!(!target.join("caf\u{e9}.md").exists());
    assert!(!target.join("cafe\u{301}.md").exists());
    assert!(!project.path().join(".vibe/package-skills/staged").exists());
    assert_eq!(fs::read(&receipt_path).unwrap(), committed);
    let receipt: vibe_wire::generated::package_skill_receipt::PackageSkillReceipt =
        toml::from_str(&String::from_utf8(committed).unwrap()).unwrap();
    assert!(receipt.applying.is_none(), "no `applying` receipt exists");
}

#[test]
fn missing_source_is_an_honest_planned_state() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    let bindings = lower_project_skill_bindings(
        project.path(),
        vec![provider(
            package.path(),
            "missing",
            "missing",
            "skills/missing",
        )],
    )
    .unwrap();
    assert_eq!(bindings[0].source_snapshot, "missing");
    assert!(bindings[0].selected_files.is_none());
}

#[test]
fn source_and_target_link_ancestors_are_refused() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    let outside_source = tempfile::tempdir().unwrap();
    seed(outside_source.path(), "demo");
    if make_dir_link(&package.path().join("skills"), outside_source.path()) {
        let error = lower_project_skill_bindings(
            project.path(),
            vec![provider(package.path(), "linked", "linked", "skills/demo")],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("unsafe source"), "{error}");
    }

    let normal = tempfile::tempdir().unwrap();
    seed(normal.path(), "skills/demo");
    let outside_target = tempfile::tempdir().unwrap();
    if make_dir_link(&project.path().join(".claude"), outside_target.path()) {
        let error = lower_project_skill_bindings(
            project.path(),
            vec![provider(normal.path(), "target", "target", "skills/demo")],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("unsafe `claude` target"), "{error}");
    }
}

#[test]
fn apply_rechecks_source_and_target_ancestors_after_safe_planning() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    seed(package.path(), "skills/demo");
    let binding = lower_project_skill_bindings(
        project.path(),
        vec![provider(package.path(), "apply", "apply", "skills/demo")],
    )
    .unwrap()
    .remove(0);
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("CANARY"), "outside").unwrap();
    if make_dir_link(&project.path().join(".claude"), outside.path()) {
        let error = reconcile_project_skill_binding(project.path(), &binding).unwrap_err();
        let error = format!("{error:#}");
        assert!(
            error.contains("no-follow") || error.contains("symlink") || error.contains("reparse"),
            "{error}"
        );
        assert_eq!(
            fs::read_to_string(outside.path().join("CANARY")).unwrap(),
            "outside"
        );
    }

    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    seed(package.path(), "skills/demo");
    let binding = lower_project_skill_bindings(
        project.path(),
        vec![provider(package.path(), "source", "source", "skills/demo")],
    )
    .unwrap()
    .remove(0);
    fs::remove_dir_all(package.path().join("skills")).unwrap();
    let outside = tempfile::tempdir().unwrap();
    seed(outside.path(), "demo");
    if make_dir_link(&package.path().join("skills"), outside.path()) {
        let error = reconcile_project_skill_binding(project.path(), &binding).unwrap_err();
        let error = format!("{error:#}");
        assert!(
            error.contains("symlink") || error.contains("reparse"),
            "{error}"
        );
        assert!(!project.path().join(".claude").exists());
    }
}

#[test]
fn vanished_provider_reconciliation_removes_owned_files_and_preserves_neighbors() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    seed(package.path(), "skills/demo");
    let binding = lower_project_skill_bindings(
        project.path(),
        vec![provider(package.path(), "provider", "demo", "skills/demo")],
    )
    .unwrap()
    .remove(0);
    reconcile_project_skill_binding(project.path(), &binding).unwrap();
    let target = project.path().join(".claude/skills/demo");
    fs::write(target.join("NEIGHBOR.md"), "foreign").unwrap();

    let reports = reconcile_vanished_project_skill_bindings(
        project.path(),
        &std::collections::BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(reports.len(), 1);
    assert!(!target.join("SKILL.md").exists());
    assert_eq!(
        fs::read_to_string(target.join("NEIGHBOR.md")).unwrap(),
        "foreign"
    );
}

#[cfg(unix)]
fn make_dir_link(link: &Path, target: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn make_dir_link(link: &Path, target: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn explicit_agent_list_must_name_supported_agents() {
    let project = tempfile::tempdir().unwrap();
    for (agents, needle) in [
        (vec!["wat"], "unknown agent `wat`"),
        (vec!["cursor"], "no project-scope skill loader"),
        (vec!["wat", "cursor"], "unknown agent `wat`"),
    ] {
        let package = tempfile::tempdir().unwrap();
        fs::create_dir_all(package.path().join("skills/body")).unwrap();
        fs::write(package.path().join("skills/body/SKILL.md"), "body").unwrap();
        let mut input = provider(package.path(), "agents", "alpha", "skills/body");
        input.declarations[0].agents = agents.iter().map(|agent| (*agent).into()).collect();
        let error = lower_project_skill_bindings(project.path(), vec![input]).unwrap_err();
        let error = format!("{error:#}");
        assert!(error.contains(needle), "{needle} not in {error}");
    }
}
