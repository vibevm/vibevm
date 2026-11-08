//! Explicit builtin registration is distinct from observation and toggling.
use super::*;
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn identical_unreceipted_builtin_is_preserved_until_explicit_registration() {
    let root = tempfile::tempdir().unwrap();
    let agent = Agent::ClaudeCode;
    let scope = Scope::Project;
    let target = root.path().join(".claude/skills/vibevm");
    fs::create_dir_all(&target).unwrap();
    let desired = BTreeMap::from([("SKILL.md".into(), b"canonical builtin".to_vec())]);
    fs::write(target.join("SKILL.md"), &desired["SKILL.md"]).unwrap();
    let receipt = standalone_receipt_path(target.parent().unwrap(), "vibevm");
    assert_eq!(
        managed_skill_status(
            agent,
            scope,
            Some(root.path()),
            "org.vibevm.core/vibevm",
            "vibevm"
        )
        .unwrap()
        .observed,
        "foreign"
    );
    assert!(
        install_managed_skill_snapshot(
            agent,
            scope,
            Some(root.path()),
            Some("org.vibevm.core/vibevm"),
            "vibevm",
            &desired,
            false
        )
        .is_err()
    );
    let off = ActivationIntent {
        enabled: false,
        actor: None,
        provenance: "test:disable".into(),
    };
    assert!(
        change_managed_skill_activation(
            SkillActivationTarget {
                agent,
                scope,
                project: Some(root.path()),
                provider: "org.vibevm.core/vibevm",
                name: "vibevm"
            },
            &desired,
            &off,
            false
        )
        .is_err()
    );
    assert!(!receipt.exists());
    assert_eq!(
        fs::read(target.join("SKILL.md")).unwrap(),
        desired["SKILL.md"]
    );
    assert_eq!(
        register_builtin_skill_snapshot(agent, scope, Some(root.path()), &desired, true)
            .unwrap()
            .status,
        "would-adopt"
    );
    assert!(!receipt.exists());
    register_builtin_skill_snapshot(agent, scope, Some(root.path()), &desired, false).unwrap();
    let saved = read_standalone_receipt(&receipt, "vibevm")
        .unwrap()
        .unwrap();
    let intent = saved.activation.unwrap();
    assert!(intent.actor.is_none());
    assert_eq!(intent.provenance, "vibe mcp install:explicit-adoption");
    assert_eq!(
        change_managed_skill_activation(
            SkillActivationTarget {
                agent,
                scope,
                project: Some(root.path()),
                provider: "org.vibevm.core/vibevm",
                name: "vibevm"
            },
            &desired,
            &off,
            false
        )
        .unwrap()
        .observed,
        "disabled"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn known_standalone_owner_retains_off_intent_on_foreign_projection_conflict() {
    let root = tempfile::tempdir().unwrap();
    let desired = BTreeMap::from([("SKILL.md".into(), b"owned".to_vec())]);
    install_managed_skill_snapshot(
        Agent::ClaudeCode,
        Scope::Project,
        Some(root.path()),
        Some("org.example/pkg"),
        "demo",
        &desired,
        false,
    )
    .unwrap();
    let target = root.path().join(".claude/skills/demo/SKILL.md");
    let receipt = root
        .path()
        .join(".claude/skills/.demo.vibe-skill-receipt.toml");
    fs::write(&target, "foreign edit").unwrap();
    let before = fs::read(&receipt).unwrap();
    let off = ActivationIntent {
        enabled: false,
        actor: None,
        provenance: "test:disable".into(),
    };
    assert!(
        change_managed_skill_activation(
            SkillActivationTarget {
                agent: Agent::ClaudeCode,
                scope: Scope::Project,
                project: Some(root.path()),
                provider: "org.example/pkg",
                name: "demo"
            },
            &desired,
            &off,
            true
        )
        .is_err()
    );
    assert_eq!(fs::read(&receipt).unwrap(), before);
    let error = change_managed_skill_activation(
        SkillActivationTarget {
            agent: Agent::ClaudeCode,
            scope: Scope::Project,
            project: Some(root.path()),
            provider: "org.example/pkg",
            name: "demo",
        },
        &desired,
        &off,
        false,
    )
    .unwrap_err();
    assert!(error.to_string().contains("intent retained"));
    assert_eq!(fs::read_to_string(&target).unwrap(), "foreign edit");
    assert_eq!(
        read_standalone_receipt(&receipt, "demo")
            .unwrap()
            .unwrap()
            .activation,
        Some(off)
    );
    let status = managed_skill_status(
        Agent::ClaudeCode,
        Scope::Project,
        Some(root.path()),
        "org.example/pkg",
        "demo",
    )
    .unwrap();
    assert!(!status.desired_enabled);
    assert_eq!(status.observed, "changed");
}

/// A stale unreceipted builtin skill (written by a pre-receipt vibe) is a
/// typed ownership refusal without `force`, and is replaced and receipted
/// only under an explicit forced adoption.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn stale_unreceipted_builtin_is_replaced_only_by_forced_adoption() {
    use crate::activation::BuiltinAdoption;
    let root = tempfile::tempdir().unwrap();
    let (agent, scope) = (Agent::ClaudeCode, Scope::Project);
    let target = root.path().join(".claude/skills/vibevm");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "stale content").unwrap();
    let desired = BTreeMap::from([("SKILL.md".into(), b"canonical builtin".to_vec())]);
    let receipt = standalone_receipt_path(target.parent().unwrap(), "vibevm");

    let refusal = register_builtin_skill_snapshot(agent, scope, Some(root.path()), &desired, false)
        .unwrap_err();
    assert!(matches!(
        refusal.downcast_ref::<PackageSkillError>(),
        Some(PackageSkillError::Unowned { .. })
    ));
    assert!(
        refusal.to_string().contains("vibe mcp install --force"),
        "{refusal}"
    );
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "stale content"
    );
    assert!(!receipt.exists());

    let forced = BuiltinAdoption::forced("vibe mcp install");
    let preview =
        adopt_builtin_skill_snapshot(agent, scope, Some(root.path()), &desired, forced, true)
            .unwrap();
    assert_eq!(preview.status, "would-adopt");
    assert!(!receipt.exists());
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "stale content"
    );

    let applied =
        adopt_builtin_skill_snapshot(agent, scope, Some(root.path()), &desired, forced, false)
            .unwrap();
    assert_eq!(applied.status, "adopted");
    let note = applied.note.unwrap();
    assert!(
        note.contains(&digest_bytes(b"stale content")),
        "replaced digest reported: {note}"
    );
    assert_eq!(
        fs::read(target.join("SKILL.md")).unwrap(),
        desired["SKILL.md"]
    );
    let saved = read_standalone_receipt(&receipt, "vibevm")
        .unwrap()
        .unwrap();
    assert_eq!(
        saved.activation.unwrap().provenance,
        "vibe mcp install:forced-adoption"
    );
    assert_eq!(
        managed_skill_status(
            agent,
            scope,
            Some(root.path()),
            "org.vibevm.core/vibevm",
            "vibevm"
        )
        .unwrap()
        .observed,
        "enabled"
    );
}

/// Forced adoption never claims a file the builtin skill does not ship.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn forced_builtin_adoption_refuses_targets_with_foreign_files() {
    use crate::activation::BuiltinAdoption;
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(".claude/skills/vibevm");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "stale content").unwrap();
    fs::write(target.join("notes.md"), "operator notes").unwrap();
    let desired = BTreeMap::from([("SKILL.md".into(), b"canonical builtin".to_vec())]);
    let error = adopt_builtin_skill_snapshot(
        Agent::ClaudeCode,
        Scope::Project,
        Some(root.path()),
        &desired,
        BuiltinAdoption::forced("vibe mcp install"),
        false,
    )
    .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<PackageSkillError>(),
        Some(PackageSkillError::Unowned { .. })
    ));
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "stale content"
    );
    assert_eq!(
        fs::read_to_string(target.join("notes.md")).unwrap(),
        "operator notes"
    );
}
