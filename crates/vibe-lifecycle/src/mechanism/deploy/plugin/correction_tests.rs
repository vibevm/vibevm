use std::path::Path;

use specmark::verifies;

use super::fixture::{World, selection, target, write};
use super::*;
use crate::mechanism::deploy::{execute_deploy_targets, undeploy_targets};

const CLAUDE_FILES: &[(&str, &str)] = &[
    (
        ".claude-plugin/plugin.json",
        "{\"name\":\"demo-plugin\",\"version\":\"1.4.2\"}\n",
    ),
    (
        "skills/demo/SKILL.md",
        "---\nname: demo\ndescription: Demo.\n---\n\nBody.\n",
    ),
];

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn nonportable_projected_skill_name_refuses_before_any_destination_write() {
    let world = World::new();
    let files = [(
        "skills/WithCaps/SKILL.md",
        "---\nname: WithCaps\ndescription: Unsafe.\n---\n",
    )];
    world.record_projection("opencode-unsafe-skill", &files);
    let targets = [target(
        PluginClient::OpenCode,
        "unsafe-skill-target",
        "opencode-unsafe-skill",
        "portable-plugin",
    )];
    let selected = selection("unsafe-skill-target");
    let home_before = world.home_census();
    let error = execute_deploy_targets(&world.execution(&targets, &selected))
        .unwrap_err()
        .to_string();
    assert!(error.contains("portable lowercase-kebab"), "{error}");
    assert_eq!(world.home_census(), home_before);
    assert!(!world.state_home.exists());
    assert!(!world.settings.path().join("client-marketplaces").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn opencode_inverse_rejects_nonportable_receipt_skill_and_preserves_bytes() {
    let world = World::new();
    let targets = [target(
        PluginClient::OpenCode,
        "unsafe-remove-target",
        "unused",
        "portable-plugin",
    )];
    let relative = ".config/opencode/skills/WithCaps/SKILL.md";
    write(world.home.path(), relative, "foreign exact bytes\n");
    let before = std::fs::read(world.at(relative)).unwrap();
    let resource = format!("home:{relative}");
    let receipt = crate::mechanism::deploy::skill::support::receipt_owning(
        0,
        &[(resource.as_str(), &"0".repeat(64))],
    );
    let request = DeployTargetRequest {
        target: &targets[0],
        profile: "local",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: Some(&receipt),
        recovery_intent: None,
        artifact: None,
        staging: None,
    };
    let result = ClientPluginProvider::new(PluginClient::OpenCode).remove(
        &request,
        std::slice::from_ref(&resource),
        None,
    );
    let after = std::fs::read(world.at(relative));
    assert!(result.is_err(), "nonportable receipt resource was accepted");
    assert_eq!(after.unwrap(), before);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn undeploy_refuses_a_receipt_witness_that_is_not_active_user_scope() {
    let world = World::new();
    let digest = world.record_projection("claude-inactive", CLAUDE_FILES);
    let targets = [target(
        PluginClient::Claude,
        "inactive-target",
        "claude-inactive",
        "demo-plugin",
    )];
    let selected = selection("inactive-target");
    let execution = world.execution(&targets, &selected);
    execute_deploy_targets(&execution).expect("valid active user receipt is created first");
    let support = world
        .settings
        .path()
        .join("client-marketplaces/claude/inactive-target")
        .join(digest);
    let support_before = crate::mechanism::contain::tree_digest(&support)
        .unwrap()
        .digest;
    world.set_claude_witness(true, false);

    let error = undeploy_targets(&execution).unwrap_err().to_string();
    assert!(error.contains("not enabled in user scope"), "{error}");
    assert_eq!(
        crate::mechanism::contain::tree_digest(&support)
            .unwrap()
            .digest,
        support_before
    );
    assert!(
        world
            .trace()
            .iter()
            .all(|line| !line.contains("plugin\tuninstall\t")),
        "refused witness must not invoke remove"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn marketplace_parent_link_refuses_without_writing_through_it() {
    let world = World::new();
    world.record_projection("claude-linked-parent", CLAUDE_FILES);
    let outside = tempfile::tempdir().unwrap();
    let link = world.settings.path().join("client-marketplaces");
    if !link_to(outside.path(), &link) {
        return;
    }
    let targets = [target(
        PluginClient::Claude,
        "linked-parent-target",
        "claude-linked-parent",
        "demo-plugin",
    )];
    let selected = selection("linked-parent-target");
    let error = execute_deploy_targets(&world.execution(&targets, &selected))
        .unwrap_err()
        .to_string();
    assert!(error.contains("no-follow directory"), "{error}");
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
}

fn link_to(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}
