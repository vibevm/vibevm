//! Desired-off remains across fresh lowering and source updates; all roots are temporary.
use super::tests::{lower, provider, seed};
use super::*;
use crate::pkgskill::{reconcile_project_skill_binding, recover_project_skill_bindings};
use crate::{activation::ActivationIntent, agents::Agent};

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
    let mut before: vibe_wire::generated::package_skill::e2::receipt::PackageSkillReceipt =
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
    let after: vibe_wire::generated::package_skill::e2::receipt::PackageSkillReceipt =
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
