//! §6.3.1's standalone-skill laws, sentence by sentence — every clause
//! that can refuse has a test here, driven against the real provider
//! verbs so each refusal reaches the caller exactly as an operator would
//! see it.
//!
//! The engine-driven half (plan/apply/verify/recover/remove through the
//! shipped executor, in isolated temp homes) lives in the lifecycle cell
//! next door; this cell owns the provider's own laws. The injected client
//! executables are all MISSING throughout — a skill destination is a
//! documented filesystem projection, so a suite that needed a client to
//! exist would be proving the wrong thing.

use specmark::verifies;
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

use super::SkillDeployProvider;
use super::client::SkillClient;
use super::support::*;
use crate::mechanism::DeployProvider;
use crate::mechanism::deploy::protocol::{ObservedResource, ResolvedDeployArtifact};
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::package::support::config;

/// The canonical demo entry document, matching the `demo` config name.
const DEMO_ENTRY: &str = "---\nname: demo\ndescription: A demonstration skill.\n---\n\nBody.\n";

/// The provider for one client.
fn provider(client: SkillClient) -> SkillDeployProvider {
    SkillDeployProvider::new(client)
}

/// One proven skill artifact over the demo entry.
fn demo_artifact(world: &World) -> ResolvedDeployArtifact {
    world.skill_artifact("demo.md", DEMO_ENTRY)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_three_clients_plan_one_exact_entry_each_under_their_own_root() {
    for (client, root) in [
        (SkillClient::Claude, ".claude/skills"),
        (SkillClient::Codex, ".agents/skills"),
        (SkillClient::OpenCode, ".config/opencode/skills"),
    ] {
        let world = World::new();
        let row = target(client, "skill-target", "demo.md", "demo");
        let artifact = demo_artifact(&world);
        let plan = plan_of(&provider(client), &world, &row, &artifact, None);
        assert_eq!(plan.resources.len(), 1, "{}", client.as_str());
        assert_eq!(
            plan.resources[0].resource,
            format!("home:{root}/demo/SKILL.md"),
            "the identity is exactly one forward-slashed home-relative member",
        );
        assert_eq!(plan.resources[0].desired_digest, artifact.digest);
        // §6.3.0.9: a normal provider's lock set IS its owned set.
        assert_eq!(plan.lock_resources, [plan.resources[0].resource.clone()]);
        // §6.3.1: creating an absent entry is reversible; an update is
        // not, and says so at plan time.
        assert!(plan.reversible, "a first deployment rolls back by removal");
        // A first deployment's destination is byte-absent, and plan
        // created none of it — not the skills root, not the entry.
        assert!(!world.at(root).exists(), "plan creates no skills root");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn fingerprints_and_config_digests_bind_client_and_name() {
    let world = World::new();
    let artifact = demo_artifact(&world);
    let mut digests = Vec::new();
    for client in SkillClient::ALL {
        let row = target(client, "skill-target", "demo.md", "demo");
        let plan = plan_of(&provider(client), &world, &row, &artifact, None);
        let fingerprint = provider(client)
            .fingerprint(&request(&world, &row, Some(&artifact), None, false), &plan)
            .expect("the fingerprint computes");
        digests.push((fingerprint.digest, plan.config_digest));
    }
    assert_ne!(
        digests[0], digests[1],
        "claude and codex are two destinations"
    );
    assert_ne!(
        digests[1], digests[2],
        "codex and opencode are two destinations"
    );
    assert_ne!(
        digests[0], digests[2],
        "claude and opencode are two destinations"
    );
    // The config digest is a pure function of client + name: the same
    // table digests identically, which is what makes §4.1's staleness
    // honest for an unchanged target.
    let row = target(SkillClient::Claude, "skill-target", "demo.md", "demo");
    let again = plan_of(
        &provider(SkillClient::Claude),
        &world,
        &row,
        &artifact,
        None,
    );
    assert_eq!(again.config_digest, digests[0].1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_recorded_plain_file_or_directory_artifact_refuses_by_kind_and_shape() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "notes.md", "demo");
    let plain = {
        let mut artifact = demo_artifact(&world);
        artifact.kind = ArtifactKind::File;
        artifact
    };
    let error = provider(client)
        .plan(&request(&world, &row, Some(&plain), None, false))
        .expect_err("a recorded plain `file` is not a skill by resemblance");
    match refusal(&error) {
        DeployProviderError::ArtifactKind {
            kind, supported, ..
        } => {
            assert_eq!(*kind, "file");
            assert_eq!(*supported, "skill");
        }
        other => panic!("expected the kind refusal, got {other}"),
    }

    let directory = {
        let mut artifact = demo_artifact(&world);
        artifact.shape = ArtifactShape::Directory;
        artifact
    };
    let error = provider(client)
        .plan(&request(&world, &row, Some(&directory), None, false))
        .expect_err("a directory is a different package kind");
    assert!(
        matches!(refusal(&error), DeployProviderError::SkillShape { .. }),
        "the shape refusal names the one-file law: {error}",
    );
    // And nothing reached a destination on the way out.
    assert!(!world.at(".claude/skills").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn unsafe_missing_or_unknown_config_members_refuse_naming_themselves() {
    let world = World::new();
    let client = SkillClient::Codex;
    let artifact = demo_artifact(&world);
    for (label, spelling) in [
        ("uppercase", "Demo"),
        ("underscore", "demo_skill"),
        ("multi-component", "a/b"),
        ("device name", "con"),
        ("empty", ""),
    ] {
        let row = target(client, "skill-target", "demo.md", spelling);
        let error = provider(client)
            .plan(&request(&world, &row, Some(&artifact), None, false))
            .expect_err(&format!(
                "`{label}` (`{spelling}`) is not a portable skill name"
            ));
        match refusal(&error) {
            DeployProviderError::Config { member, reason, .. } => {
                assert_eq!(member, "name");
                assert!(reason.contains("portable Agent Skills name"), "{reason}");
            }
            other => panic!("expected the config refusal for `{label}`, got {other}"),
        }
    }

    let mut row = target(client, "skill-target", "demo.md", "demo");
    row.config = Some(config("name = \"demo\"\nclient = \"claude\""));
    let error = provider(client)
        .plan(&request(&world, &row, Some(&artifact), None, false))
        .expect_err("no config string chooses the client");
    match refusal(&error) {
        DeployProviderError::Config { member, reason, .. } => {
            assert_eq!(member, "client");
            assert!(reason.contains("routing"), "{reason}");
        }
        other => panic!("expected the engine-owned refusal, got {other}"),
    }

    row.config = Some(config("rename = \"demo\""));
    let error = provider(client)
        .plan(&request(&world, &row, Some(&artifact), None, false))
        .expect_err("the config is exactly `name`");
    match refusal(&error) {
        DeployProviderError::Config { member, reason, .. } => {
            assert_eq!(member, "rename");
            assert!(reason.contains("exactly `name`"), "{reason}");
        }
        other => panic!("expected the unknown-member refusal, got {other}"),
    }

    row.config = None;
    let error = provider(client)
        .plan(&request(&world, &row, Some(&artifact), None, false))
        .expect_err("`name` is required with no default");
    assert!(matches!(
        refusal(&error),
        DeployProviderError::Config { .. }
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_frontmatter_name_that_disagrees_with_the_config_refuses() {
    let world = World::new();
    let client = SkillClient::OpenCode;
    let row = target(client, "skill-target", "demo.md", "demo");
    let other = world.skill_artifact(
        "other.md",
        "---\nname: other\ndescription: A different skill.\n---\n\nBody.\n",
    );
    let error = provider(client)
        .plan(&request(&world, &row, Some(&other), None, false))
        .expect_err("a skill has one identity, not two");
    match refusal(&error) {
        DeployProviderError::SkillName {
            declared, config, ..
        } => {
            assert_eq!(declared, "other");
            assert_eq!(config, "demo");
        }
        other => panic!("expected the identity refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn malformed_frontmatter_refuses_through_the_one_existing_parser() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "broken.md", "demo");
    let broken = world.skill_artifact("broken.md", "# demo\n\nNo frontmatter fence.\n");
    let error = provider(client)
        .plan(&request(&world, &row, Some(&broken), None, false))
        .expect_err("an Agent Skill has frontmatter");
    match refusal(&error) {
        DeployProviderError::SkillUnreadable { reason, .. } => {
            assert!(reason.contains("frontmatter"), "{reason}");
        }
        other => panic!("expected the parser's own refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_unowned_identical_occupant_refuses_at_plan() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = demo_artifact(&world);
    // The occupant holds EXACTLY the desired bytes and no receipt owns
    // it — identical-looking bytes are not ownership. Plan reports the
    // refusal apply would raise; the apply-side half of the law (a
    // post-plan occupant cannot be overwritten) is proven next door,
    // where a plan made against an ABSENT destination exists to be
    // trusted too much.
    write_home(&world, ".claude/skills/demo/SKILL.md", DEMO_ENTRY);
    let error = provider(client)
        .plan(&request(&world, &row, Some(&artifact), None, false))
        .expect_err("an absent receipt never authorises a foreign occupant");
    match refusal(&error) {
        DeployProviderError::OccupantUnowned {
            resource, observed, ..
        } => {
            assert_eq!(*resource, resource_of(client));
            assert_eq!(observed, &artifact.digest);
        }
        other => panic!("expected the unowned-occupant refusal, got {other}"),
    }
    assert_eq!(
        std::fs::read_to_string(world.at(".claude/skills/demo/SKILL.md")).unwrap(),
        DEMO_ENTRY,
        "the occupant is byte-identical to what it was",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn receipt_owned_digest_drift_refuses_and_is_not_overwritten() {
    let world = World::new();
    let client = SkillClient::Codex;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = demo_artifact(&world);
    let recorded = "1".repeat(64);
    let receipt = receipt_owning(0, &[(&resource_of(client), recorded.as_str())]);
    write_home(
        &world,
        ".agents/skills/demo/SKILL.md",
        "hand-edited after deployment\n",
    );
    let error = provider(client)
        .plan(&request(
            &world,
            &row,
            Some(&artifact),
            Some(&receipt),
            false,
        ))
        .expect_err("drifted bytes are never silently overwritten");
    match refusal(&error) {
        DeployProviderError::OccupantDrifted { resource, .. } => {
            assert_eq!(*resource, resource_of(client));
        }
        other => panic!("expected the drift refusal, got {other}"),
    }
    assert_eq!(
        std::fs::read_to_string(world.at(".agents/skills/demo/SKILL.md")).unwrap(),
        "hand-edited after deployment\n",
        "the drifted entry is untouched",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_receipt_owned_entry_at_its_recorded_digest_is_updateable() {
    let world = World::new();
    let client = SkillClient::OpenCode;
    let row = target(client, "skill-target", "demo.md", "demo");
    let prior_body = "---\nname: demo\ndescription: The prior generation.\n---\n\nOld body.\n";
    let prior = world.skill_artifact("prior.md", prior_body);
    let updated = demo_artifact(&world);
    write_home(&world, ".config/opencode/skills/demo/SKILL.md", prior_body);
    let receipt = receipt_owning(0, &[(&resource_of(client), prior.digest.as_str())]);
    let plan = plan_of(&provider(client), &world, &row, &updated, Some(&receipt));
    assert!(
        !plan.reversible,
        "an update holds no prior bytes to restore"
    );
    let report = apply_with_plan(
        &provider(client),
        &world,
        &row,
        &updated,
        Some(&receipt),
        &plan,
    )
    .expect("a receipt-owned entry at its recorded digest is updateable");
    assert!(report.evidence.contains("opencode"), "{}", report.evidence);
    assert_eq!(
        std::fs::read(world.resource_at(&resource_of(client))).unwrap(),
        DEMO_ENTRY.as_bytes(),
        "the exact proven bytes replaced the prior generation's",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn apply_rechecks_the_occupant_under_the_locks_after_plan() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = demo_artifact(&world);
    // Plan against an absent destination…
    let plan = plan_of(&provider(client), &world, &row, &artifact, None);
    // …then an occupant appears between plan and apply. Plan evidence is
    // not write authority: the recheck inside apply refuses.
    write_home(
        &world,
        ".claude/skills/demo/SKILL.md",
        "planted after the plan\n",
    );
    let error = apply_with_plan(&provider(client), &world, &row, &artifact, None, &plan)
        .expect_err("a post-plan occupant change cannot be overwritten");
    assert!(matches!(
        refusal(&error),
        DeployProviderError::OccupantUnowned { .. }
    ));
    assert_eq!(
        std::fs::read_to_string(world.at(".claude/skills/demo/SKILL.md")).unwrap(),
        "planted after the plan\n",
        "the planted occupant is untouched",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn remove_deletes_only_the_owned_entry_and_preserves_every_neighbour() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = demo_artifact(&world);
    apply(&provider(client), &world, &row, &artifact, None);
    // A foreign file BESIDE the entry, and a sibling skill under the same
    // root — neither recorded by the receipt.
    write_home(
        &world,
        ".claude/skills/demo/NOTES.txt",
        "foreign neighbour\n",
    );
    write_home(&world, ".claude/skills/other/SKILL.md", "sibling skill\n");
    let receipt = receipt_owning(0, &[(&resource_of(client), artifact.digest.as_str())]);

    let report = provider(client)
        .remove(
            &request(&world, &row, None, Some(&receipt), false),
            &[resource_of(client)],
            None,
        )
        .expect("the receipt-owned entry removes");
    assert_eq!(report.removed, [resource_of(client)]);
    assert!(
        !world.at(".claude/skills/demo/SKILL.md").exists(),
        "the receipt-owned entry is gone",
    );
    assert_eq!(
        std::fs::read_to_string(world.at(".claude/skills/demo/NOTES.txt")).unwrap(),
        "foreign neighbour\n",
        "a foreign file beside the entry is byte-identical",
    );
    assert_eq!(
        std::fs::read_to_string(world.at(".claude/skills/other/SKILL.md")).unwrap(),
        "sibling skill\n",
        "a sibling skill is byte-identical",
    );
    assert!(
        world.at(".claude/skills/demo").exists(),
        "the named skill directory is NOT pruned while a foreign file is inside it",
    );

    // Absence is idempotent: removing an already-absent entry succeeds.
    let report = provider(client)
        .remove(
            &request(&world, &row, None, Some(&receipt), false),
            &[resource_of(client)],
            None,
        )
        .expect("removing an absent entry succeeds");
    assert!(report.removed.is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn remove_prunes_only_proven_empty_directories_and_stops_at_the_boundary() {
    let world = World::new();
    let client = SkillClient::Codex;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = demo_artifact(&world);
    apply(&provider(client), &world, &row, &artifact, None);
    let receipt = receipt_owning(0, &[(&resource_of(client), artifact.digest.as_str())]);

    provider(client)
        .remove(
            &request(&world, &row, None, Some(&receipt), false),
            &[resource_of(client)],
            None,
        )
        .expect("the entry removes");
    assert!(
        !world.at(".agents/skills/demo").exists(),
        "the proven-empty skill directory pruned"
    );
    assert!(
        world.at(".agents/skills").exists(),
        "the skills root is the boundary and is never pruned",
    );
    assert!(
        world.home.path().join(".agents").exists(),
        "no ancestor above the boundary is touched",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn remove_refuses_an_entry_the_receipt_does_not_own() {
    let world = World::new();
    let client = SkillClient::OpenCode;
    let row = target(client, "skill-target", "demo.md", "demo");
    write_home(&world, ".config/opencode/skills/demo/SKILL.md", DEMO_ENTRY);
    // The receipt owns a DIFFERENT resource entirely.
    let stranger = "home:.config/opencode/skills/stranger/SKILL.md".to_owned();
    let receipt = receipt_owning(0, &[(stranger.as_str(), "0".repeat(64).as_str())]);
    let error = provider(client)
        .remove(
            &request(&world, &row, None, Some(&receipt), false),
            &[resource_of(client)],
            None,
        )
        .expect_err("an entry the receipt does not name is never the provider's to delete");
    match refusal(&error) {
        DeployProviderError::RemoveNotOwned { resource, .. } => {
            assert_eq!(*resource, resource_of(client));
        }
        other => panic!("expected the remove-ownership refusal, got {other}"),
    }
    assert_eq!(
        std::fs::read_to_string(world.at(".config/opencode/skills/demo/SKILL.md")).unwrap(),
        DEMO_ENTRY,
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn recover_treats_an_already_desired_entry_as_a_no_op_and_reconciles_anything_else() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = demo_artifact(&world);
    let plan = plan_of(&provider(client), &world, &row, &artifact, None);

    // The interrupted apply had already written the entry: recovery
    // checkpoints it and writes nothing.
    write_home(&world, ".claude/skills/demo/SKILL.md", DEMO_ENTRY);
    let desired = [ObservedResource {
        resource: resource_of(client),
        digest: Some(artifact.digest.clone()),
    }];
    let report = recover_with(
        &provider(client),
        &world,
        &row,
        &artifact,
        None,
        &plan,
        &desired,
    )
    .expect("an already-desired entry recovers as a no-op");
    assert!(
        report.evidence.contains("already desired"),
        "{}",
        report.evidence
    );

    // An interrupted apply that never reached the write: recovery
    // completes it.
    std::fs::remove_file(world.at(".claude/skills/demo/SKILL.md")).expect("the entry removes");
    let absent = [ObservedResource {
        resource: resource_of(client),
        digest: None,
    }];
    recover_with(
        &provider(client),
        &world,
        &row,
        &artifact,
        None,
        &plan,
        &absent,
    )
    .expect("an absent entry is reconciled");
    assert_eq!(
        std::fs::read(world.resource_at(&resource_of(client))).unwrap(),
        DEMO_ENTRY.as_bytes(),
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn verify_observes_the_named_entry_and_returns_absence_as_a_value() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "demo.md", "demo");
    let observed = provider(client)
        .verify(
            &request(&world, &row, None, None, false),
            &[resource_of(client)],
        )
        .expect("an absent entry is a value, not a fault");
    assert_eq!(observed[0].digest, None);

    write_home(&world, ".claude/skills/demo/SKILL.md", DEMO_ENTRY);
    let observed = provider(client)
        .verify(
            &request(&world, &row, None, None, false),
            &[resource_of(client)],
        )
        .expect("the entry observes");
    assert_eq!(
        observed[0].digest.as_deref(),
        Some(digest_at(&world.resource_at(&resource_of(client))).as_str()),
    );

    // A resource identity that is not home-rooted refuses rather than
    // resolving somewhere else.
    let error = provider(client)
        .verify(
            &request(&world, &row, None, None, false),
            &["bin/escape".to_owned()],
        )
        .expect_err("a resource without the home root refuses");
    assert!(matches!(
        refusal(&error),
        DeployProviderError::Observe { .. }
    ));
}
