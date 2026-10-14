//! Isolated deploy skill characterization: no executable is available.
use super::skill::{SkillDeployProvider, client::SkillClient, support::*};
use super::{execute_deploy_targets, undeploy_targets};
use crate::mechanism::DeployProvider;
use specmark::verifies;
use vibe_wire::generated::artifact_record::ArtifactShape;

const FIRST: &str = include_str!("../../../../../fixtures/next/adapters/skill-v1.md");
const SECOND: &str = include_str!("../../../../../fixtures/next/adapters/skill-v2.md");

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-SURFACES")]
fn lifecycle_skill_file_install_update_remove_preserves_bytes_and_receipt_owner() {
    for (client, relative) in [
        (SkillClient::Claude, ".claude/skills/demo/SKILL.md"),
        (SkillClient::Codex, ".agents/skills/demo/SKILL.md"),
        (
            SkillClient::OpenCode,
            ".config/opencode/skills/demo/SKILL.md",
        ),
    ] {
        let world = EngineWorld::new();
        assert!(
            world
                .clients
                .all()
                .iter()
                .all(|c| c.resolved_path().is_none())
        );
        world.record_skill("fixture", FIRST);
        let targets = [target(client, "skill-target", "fixture", "demo")];
        let selection = selection(&["skill-target"]);
        let execution = world.execution(&targets, &selection);
        let first = execute_deploy_targets(&execution).unwrap();
        assert_eq!(first[0].resources[0].resource, format!("home:{relative}"));
        assert_eq!(std::fs::read(world.at(relative)).unwrap(), FIRST.as_bytes());
        let (state, home) = engine_state_of(&world, &targets[0]);
        let receipt = state.read_receipt(&home).unwrap().unwrap();
        assert_eq!(receipt.target, "skill-target");
        assert_eq!(receipt.provider.key, first[0].provider);
        assert_eq!(receipt.resources[0].resource, format!("home:{relative}"));
        assert!(receipt.reversible);
        drop(state);
        let neighbor = world.at(relative).parent().unwrap().join("foreign.txt");
        std::fs::write(&neighbor, b"keep\n").unwrap();
        world.record_skill("fixture", SECOND);
        let second = execute_deploy_targets(&execution).unwrap();
        assert!(!second[0].reversible);
        assert_eq!(
            std::fs::read(world.at(relative)).unwrap(),
            SECOND.as_bytes()
        );
        undeploy_targets(&execution).unwrap();
        assert!(!world.at(relative).exists());
        assert_eq!(std::fs::read(neighbor).unwrap(), b"keep\n");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-OWNERSHIP")]
fn lifecycle_refuses_equal_byte_standalone_occupants_and_directory_artifacts() {
    for client in SkillClient::ALL {
        let world = World::new();
        let row = target(client, "skill-target", "fixture", "demo");
        let mut artifact = world.skill_artifact("fixture", FIRST);
        let provider = SkillDeployProvider::new(client);
        artifact.shape = ArtifactShape::Directory;
        assert!(
            provider
                .plan(&request(&world, &row, Some(&artifact), None, false))
                .unwrap_err()
                .to_string()
                .contains("file")
        );
        artifact.shape = ArtifactShape::File;
        // Seed through a different receipt family, using a project root chosen
        // to equal this fixture home. OpenCode's project path differs, so its
        // equal-byte foreign occupant is explicitly seeded instead.
        let agent = match client {
            SkillClient::Claude => vibe_agent_projection::agents::Agent::ClaudeCode,
            SkillClient::Codex => vibe_agent_projection::agents::Agent::Codex,
            SkillClient::OpenCode => vibe_agent_projection::agents::Agent::OpenCode,
        };
        if client == SkillClient::OpenCode {
            write_home(&world, ".config/opencode/skills/demo/SKILL.md", FIRST);
        } else {
            vibe_agent_projection::pkgskill::install_package_skill(
                agent,
                vibe_agent_projection::agents::Scope::Project,
                Some(world.home.path()),
                "demo",
                &artifact.absolute,
                false,
            )
            .unwrap();
        }
        let error = provider
            .plan(&request(&world, &row, Some(&artifact), None, false))
            .unwrap_err();
        assert!(matches!(
            refusal(&error),
            crate::mechanism::error::DeployProviderError::OccupantUnowned { .. }
        ));
        assert_eq!(
            std::fs::read(world.resource_at(&resource_of(client))).unwrap(),
            FIRST.as_bytes()
        );
        let resource = "home:foreign/sentinel".to_owned();
        write_home(&world, "foreign/sentinel", "keep\n");
        let receipt = receipt_owning(0, &[(&resource, &"0".repeat(64))]);
        assert!(
            provider
                .remove(
                    &request(&world, &row, None, Some(&receipt), false),
                    &[resource],
                    None
                )
                .is_err()
        );
        assert_eq!(
            std::fs::read(world.at("foreign/sentinel")).unwrap(),
            b"keep\n"
        );
    }
}
