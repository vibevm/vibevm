//! Exact inverse containment for the standalone-skill provider.
//!
//! A receipt is ownership evidence only inside the provider's declared
//! perimeter. Even if a caller and a receipt agree on a foreign resource, this
//! provider removes exactly its configured `<skills>/<name>/SKILL.md` and
//! nothing else.

use super::SkillDeployProvider;
use super::client::SkillClient;
use super::support::{World, receipt_owning, request, target, write_home};
use crate::mechanism::DeployProvider;
use crate::mechanism::error::DeployProviderError;

#[test]
fn a_receipt_owned_foreign_request_is_not_this_provider_entry_to_remove() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "demo.md", "demo");
    let foreign = "home:.claude/skills/foreign/SKILL.md";
    let foreign_bytes = "foreign skill\n";
    write_home(&world, ".claude/skills/foreign/SKILL.md", foreign_bytes);
    let digest = super::support::digest_at(&world.at(".claude/skills/foreign/SKILL.md"));
    let configured = "home:.claude/skills/demo/SKILL.md";
    let configured_digest = "0".repeat(64);
    let receipt = receipt_owning(
        0,
        &[
            (configured, configured_digest.as_str()),
            (foreign, digest.as_str()),
        ],
    );

    let error = SkillDeployProvider::new(client)
        .remove(
            &request(&world, &row, None, Some(&receipt), false),
            &[foreign.to_owned()],
            None,
        )
        .expect_err("receipt ownership never broadens this provider's configured entry");
    assert!(matches!(
        error,
        crate::mechanism::MechanismError::Deploy(DeployProviderError::RemoveNotOwned { .. })
    ));
    assert_eq!(
        std::fs::read_to_string(world.at(".claude/skills/foreign/SKILL.md")).unwrap(),
        foreign_bytes,
        "the foreign receipt-listed file remains byte-exact",
    );
}
