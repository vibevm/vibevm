//! Desired-off remains across fresh lowering and source updates; all roots are temporary.
use super::tests::{lower, provider, seed};
use super::*;
use crate::pkgskill::{reconcile_project_skill_binding, recover_project_skill_bindings};
use crate::{activation::ActivationIntent, agents::Agent};

#[test]
fn default_all_shared_group_publishes_once_and_repeat_keeps_mtime() {
    let root = tempfile::tempdir().unwrap();
    let source = seed(root.path(), "default-all");
    let binding = lower(
        root.path(),
        vec![provider(source.path(), "package", "demo", &[])],
    )
    .remove(0);
    let reports = reconcile_project_skill_binding(root.path(), &binding).unwrap();
    assert!(reports.iter().any(|row| row.agent == "codex"));
    assert!(reports.iter().any(|row| row.agent == "pi"));
    let target = root.path().join(".agents/skills/demo/SKILL.md");
    let modified = std::fs::metadata(&target).unwrap().modified().unwrap();
    let receipt = root.path().join(".vibe/package-skills.toml");
    let before = std::fs::read(&receipt).unwrap();
    reconcile_project_skill_binding(root.path(), &binding).unwrap();
    assert_eq!(
        std::fs::metadata(&target).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(std::fs::read(receipt).unwrap(), before);
}

#[test]
fn legacy_singleton_receipt_is_admitted_and_published_as_current_group() {
    let root = tempfile::tempdir().unwrap();
    let source = seed(root.path(), "legacy");
    let binding = lower(
        root.path(),
        vec![provider(source.path(), "package", "demo", &["codex"])],
    )
    .remove(0);
    reconcile_project_skill_binding(root.path(), &binding).unwrap();
    let path = root.path().join(".vibe/package-skills.toml");
    let current: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut legacy = current.clone();
    legacy["schema"] = toml::Value::Integer(3);
    for row in legacy["binding"].as_array_mut().unwrap() {
        for target in row["target"].as_array_mut().unwrap() {
            let table = target.as_table_mut().unwrap();
            let aliases = table.remove("agents").unwrap();
            table.insert("agent".into(), aliases.as_array().unwrap()[0].clone());
        }
    }
    std::fs::write(&path, toml::to_string(&legacy).unwrap()).unwrap();
    reconcile_project_skill_binding(root.path(), &binding).unwrap();
    let reread: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(reread["schema"].as_integer(), Some(4));
    assert_eq!(
        reread["binding"][0]["target"][0]["agents"][0].as_str(),
        Some("codex")
    );
}

#[test]
fn shared_physical_group_keeps_aliases_and_refuses_conflicting_activation() {
    let root = tempfile::tempdir().unwrap();
    let source = seed(root.path(), "shared");
    let binding = lower(
        root.path(),
        vec![provider(source.path(), "package", "demo", &["codex", "pi"])],
    )
    .remove(0);
    reconcile_project_skill_binding(root.path(), &binding).unwrap();
    let receipt_path = root.path().join(".vibe/package-skills.toml");
    let before = std::fs::read(&receipt_path).unwrap();
    let parsed: vibe_wire::generated::package_skill::e3::receipt::PackageSkillReceipt =
        toml::from_str(std::str::from_utf8(&before).unwrap()).unwrap();
    assert_eq!(parsed.schema, 4);
    assert_eq!(parsed.binding[0].target.len(), 1);
    assert_eq!(parsed.binding[0].target[0].agents, ["codex", "pi"]);
    let off = ActivationIntent {
        enabled: false,
        actor: Some("operator".into()),
        provenance: "test:shared-off".into(),
    };
    assert!(
        project_activation(root.path(), &binding, Agent::Pi, &off, false)
            .unwrap_err()
            .to_string()
            .contains("whole shared group")
    );
    assert_eq!(std::fs::read(&receipt_path).unwrap(), before);
    project_group_activation(
        root.path(),
        &binding,
        &[Agent::Codex, Agent::Pi],
        &off,
        false,
    )
    .unwrap();
    assert!(!root.path().join(".agents/skills/demo/SKILL.md").exists());
    let on = ActivationIntent {
        enabled: true,
        actor: None,
        provenance: "test:shared-on".into(),
    };
    let disabled = std::fs::read(&receipt_path).unwrap();
    assert!(project_activation(root.path(), &binding, Agent::Pi, &on, false).is_err());
    assert_eq!(std::fs::read(&receipt_path).unwrap(), disabled);
    project_group_activation(
        root.path(),
        &binding,
        &[Agent::Codex, Agent::Pi],
        &on,
        false,
    )
    .unwrap();
    assert!(root.path().join(".agents/skills/demo/SKILL.md").exists());
    project_remove(root.path(), &binding, &[Agent::Pi], false).unwrap();
    assert!(root.path().join(".agents/skills/demo/SKILL.md").exists());
    assert_eq!(
        project_status(root.path(), &binding, Agent::Pi)
            .unwrap()
            .observed,
        "shared-unbound"
    );
    assert_eq!(
        project_status(root.path(), &binding, Agent::Codex)
            .unwrap()
            .observed,
        "enabled"
    );
    project_remove(root.path(), &binding, &[Agent::Codex], false).unwrap();
    assert!(!root.path().join(".agents/skills/demo/SKILL.md").exists());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn disabled_agent_survives_update_and_does_not_disable_its_sibling() {
    let root = tempfile::tempdir().unwrap();
    let source = seed(root.path(), "body");
    let inputs = || {
        vec![provider(
            source.path(),
            "package",
            "demo",
            &["claude", "codex"],
        )]
    };
    let binding = lower(root.path(), inputs()).remove(0);
    reconcile_project_skill_binding(root.path(), &binding).unwrap();
    let off = ActivationIntent {
        enabled: false,
        actor: Some("operator".into()),
        provenance: "test:disable".into(),
    };
    project_activation(root.path(), &binding, Agent::ClaudeCode, &off, false).unwrap();
    assert!(!root.path().join(".claude/skills/demo/SKILL.md").exists());
    assert!(root.path().join(".agents/skills/demo/SKILL.md").exists());
    std::fs::write(source.path().join("skills/body/SKILL.md"), "version two").unwrap();
    let updated = lower(root.path(), inputs()).remove(0);
    assert!(updated.targets.iter().all(|t| t.agent != Agent::ClaudeCode));
    reconcile_project_skill_binding(root.path(), &updated).unwrap();
    recover_project_skill_bindings(root.path()).unwrap();
    assert_eq!(
        project_status(root.path(), &updated, Agent::ClaudeCode)
            .unwrap()
            .observed,
        "disabled"
    );
    let on = ActivationIntent {
        enabled: true,
        actor: None,
        provenance: "test:enable".into(),
    };
    project_activation(root.path(), &updated, Agent::ClaudeCode, &on, false).unwrap();
    assert_eq!(
        std::fs::read_to_string(root.path().join(".claude/skills/demo/SKILL.md")).unwrap(),
        "version two"
    );
    assert!(source.path().join("skills/body/SKILL.md").exists());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn drift_retains_admitted_intent_and_preserves_foreign_contents() {
    let root = tempfile::tempdir().unwrap();
    let source = seed(root.path(), "body");
    let binding = lower(
        root.path(),
        vec![provider(source.path(), "package", "demo", &["claude"])],
    )
    .remove(0);
    reconcile_project_skill_binding(root.path(), &binding).unwrap();
    let receipt = root.path().join(".vibe/package-skills.toml");
    let mut before: vibe_wire::generated::package_skill::e3::receipt::PackageSkillReceipt =
        toml::from_str(&std::fs::read_to_string(&receipt).unwrap()).unwrap();
    let target = root.path().join(".claude/skills/demo/SKILL.md");
    std::fs::write(&target, "human change").unwrap();
    let off = ActivationIntent {
        enabled: false,
        actor: None,
        provenance: "test:disable".into(),
    };
    assert!(project_activation(root.path(), &binding, Agent::ClaudeCode, &off, false).is_err());
    assert_eq!(std::fs::read_to_string(target).unwrap(), "human change");
    before.binding[0]
        .activation
        .insert("claude".into(), off.clone());
    let after: vibe_wire::generated::package_skill::e3::receipt::PackageSkillReceipt =
        toml::from_str(&std::fs::read_to_string(receipt).unwrap()).unwrap();
    assert_eq!(after, before);
    let status = project_status(root.path(), &binding, Agent::ClaudeCode).unwrap();
    assert!(!status.desired_enabled);
    assert_eq!(status.observed, "changed");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn vanished_declaration_keeps_disabled_tombstone_for_reintroduction() {
    let root = tempfile::tempdir().unwrap();
    let source = seed(root.path(), "body");
    let inputs = || vec![provider(source.path(), "package", "demo", &["claude"])];
    let binding = lower(root.path(), inputs()).remove(0);
    reconcile_project_skill_binding(root.path(), &binding).unwrap();
    let off = ActivationIntent {
        enabled: false,
        actor: None,
        provenance: "test:disable".into(),
    };
    project_activation(root.path(), &binding, Agent::ClaudeCode, &off, false).unwrap();
    let absent = std::collections::BTreeSet::new();
    crate::pkgskill::reconcile_vanished_project_skill_bindings(root.path(), &absent).unwrap();
    assert!(
        crate::pkgskill::probe_vanished_project_skill_bindings(root.path(), &absent, &[]).unwrap()
    );
    let returned = lower(root.path(), inputs()).remove(0);
    assert!(returned.targets.is_empty());
    assert_eq!(returned.activation["claude"], off);
    reconcile_project_skill_binding(root.path(), &returned).unwrap();
    assert!(!root.path().join(".claude/skills/demo/SKILL.md").exists());
}
