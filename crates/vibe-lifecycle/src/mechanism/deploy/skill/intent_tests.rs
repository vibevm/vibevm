//! §7.2's crash windows at the plan seat — the frozen corrections' own
//! laws, focused on the occupant judgement's ONE extra case.
//!
//! A crash AFTER atomic publication leaves this deployment's desired
//! bytes at the destination and an unretired durable intent beside them,
//! while the receipt — wherever one exists — does not describe those
//! bytes. The next ordinary run must be able to PLAN (so it reaches the
//! settlement that completes the interrupted generation) without that
//! admission ever becoming ownership:
//!
//! 1. a MATCHING intent — names the exact resource, its recorded desired
//!    digest equals the independently observed digest, and its
//!    `prior_generation` agrees with the injected receipt state — admits
//!    the plan as interrupted/recovery occupancy, in BOTH shapes: the
//!    stranded first deployment (no receipt) and the stranded update
//!    (receipt-owned prior generation, observed at the new desired
//!    digest that would otherwise read as drift);
//! 2. a STALE or NONMATCHING intent — another plan's digest, another
//!    resource's name, an occupant the journal does not describe, or a
//!    `prior_generation` that disagrees with the injected receipt —
//!    grants nothing, and the strict refusal stands: `OccupantUnowned`
//!    with no receipt, `OccupantDrifted` over one;
//! 3. the ordinary NO-INTENT identical occupant still refuses (the law
//!    next door already pins it at plan; here it is pinned at the
//!    apply-time recheck, where no intent is ever injected);
//! 4. the admission is never WRITE authority: `apply` with the same
//!    intent available still refuses the unowned occupant under the
//!    locks, because settlement reachability is the transaction's to
//!    decide, by plan hash, not the plan's;
//! 5. per-plan reversibility stays honest: a stranded first deployment
//!    settles reversibly, a stranded update over a prior receipt does
//!    not.

use specmark::verifies;
use vibe_core::manifest::DeployTarget;
use vibe_wire::generated::deploy_intent::DeployIntent;

use super::SkillDeployProvider;
use super::client::SkillClient;
use super::support::*;
use crate::mechanism::DeployProvider;
use crate::mechanism::error::DeployProviderError;

/// The canonical demo entry document, matching the `demo` config name.
const DEMO_ENTRY: &str = "---\nname: demo\ndescription: A demonstration skill.\n---\n\nBody.\n";

/// The next generation's entry document — same skill, changed bytes.
const UPDATED_ENTRY: &str =
    "---\nname: demo\ndescription: A demonstration skill, updated.\n---\n\nNew body.\n";

/// The plan-time digest of one client's `demo` entry, observed
/// independently through the shared digest helper.
fn observed_digest(world: &World, client: SkillClient) -> String {
    digest_at(&world.at(&format!("{}/demo/SKILL.md", client.skills_relative())))
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_matching_durable_intent_admits_the_interrupted_occupant_at_plan() {
    let world = World::new();
    let client = SkillClient::Claude;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = world.skill_artifact("demo.md", DEMO_ENTRY);
    // The crash-after-write shape: the entry holds the DESIRED bytes, no
    // receipt exists, and the unretired journal describes exactly that.
    write_home(&world, ".claude/skills/demo/SKILL.md", DEMO_ENTRY);
    assert_eq!(observed_digest(&world, client), artifact.digest);
    let intent = intent_desiring(&[(&resource_of(client), artifact.digest.as_str())]);

    let plan = super::SkillDeployProvider::new(client)
        .plan(&request_with_intent(
            &world,
            &row,
            Some(&artifact),
            None,
            false,
            Some(&intent),
        ))
        .expect("the interrupted occupant is reachable settlement, not a refusal");
    assert_eq!(plan.resources.len(), 1);
    assert_eq!(plan.resources[0].desired_digest, artifact.digest);
    assert!(
        plan.reversible,
        "the interrupted generation settles as its own first generation, whose inverse is removal",
    );
    // And the plan created nothing while admitting it.
    assert_eq!(
        std::fs::read_to_string(world.at(".claude/skills/demo/SKILL.md")).unwrap(),
        DEMO_ENTRY,
        "the interrupted entry is untouched by its own admission",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_stale_or_nonmatching_intent_grants_no_admission() {
    let world = World::new();
    let client = SkillClient::Codex;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = world.skill_artifact("demo.md", DEMO_ENTRY);
    write_home(&world, ".agents/skills/demo/SKILL.md", DEMO_ENTRY);

    // A journal from a DIFFERENT plan: it names the resource, but at the
    // digest THAT plan wanted — not the digest observed now.
    let stale = intent_desiring(&[(&resource_of(client), &"9".repeat(64))]);
    for label in ["stale digest", "another resource", "empty journal"] {
        let intent: DeployIntent = match label {
            "stale digest" => stale.clone(),
            "another resource" => intent_desiring(&[(
                "home:.agents/skills/stranger/SKILL.md",
                artifact.digest.as_str(),
            )]),
            _ => intent_desiring(&[]),
        };
        let error = SkillDeployProvider::new(client)
            .plan(&request_with_intent(
                &world,
                &row,
                Some(&artifact),
                None,
                false,
                Some(&intent),
            ))
            .expect_err(&format!("`{label}` grants no admission"));
        assert!(
            matches!(refusal(&error), DeployProviderError::OccupantUnowned { .. }),
            "`{label}` is the strict no-receipt refusal: {error}",
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_matching_admission_is_never_write_authority() {
    let world = World::new();
    let client = SkillClient::OpenCode;
    let row = target(client, "skill-target", "demo.md", "demo");
    let artifact = world.skill_artifact("demo.md", DEMO_ENTRY);
    write_home(&world, ".config/opencode/skills/demo/SKILL.md", DEMO_ENTRY);
    let intent = intent_desiring(&[(&resource_of(client), artifact.digest.as_str())]);

    // The plan admits the interrupted occupant…
    let plan = SkillDeployProvider::new(client)
        .plan(&request_with_intent(
            &world,
            &row,
            Some(&artifact),
            None,
            false,
            Some(&intent),
        ))
        .expect("the plan seat admits the interrupted occupant");

    // …and the APPLY-time recheck — receipt-only, intent deliberately
    // absent from the request the engine builds there — still refuses the
    // unowned occupant. Settlement reachability is the transaction's, by
    // plan hash; a plan that passed is not a licence to write.
    let error = apply_with_plan(
        &SkillDeployProvider::new(client),
        &world,
        &row,
        &artifact,
        None,
        &plan,
    )
    .expect_err("plan evidence alone is never write authority");
    assert!(
        matches!(refusal(&error), DeployProviderError::OccupantUnowned { .. }),
        "the apply recheck is receipt-only: {error}",
    );
    assert_eq!(
        std::fs::read_to_string(world.at(".config/opencode/skills/demo/SKILL.md")).unwrap(),
        DEMO_ENTRY,
        "the occupant is untouched",
    );
}

/// The interrupted-UPDATE shape at the plan seat: a generation-0 receipt
/// owns the entry at ITS digest, while the entry on disk already holds
/// the next generation's desired bytes — the exact state a crash after
/// publishing an update leaves. One helper builds it, because the three
/// laws below are the same world judged three ways.
fn interrupted_update_world(client: SkillClient) -> (World, DeployTarget, String, String) {
    let world = World::new();
    let row = target(client, "skill-target", "updated.md", "demo");
    let first = world.skill_artifact("demo.md", DEMO_ENTRY);
    let updated = world.skill_artifact("updated.md", UPDATED_ENTRY);
    write_home(
        &world,
        &format!("{}/demo/SKILL.md", client.skills_relative()),
        UPDATED_ENTRY,
    );
    (world, row, first.digest, updated.digest)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_matching_intent_over_a_prior_receipt_admits_the_interrupted_update_as_irreversible() {
    let client = SkillClient::Claude;
    let (world, row, first_digest, updated_digest) = interrupted_update_world(client);
    let updated = world.skill_artifact("updated.md", UPDATED_ENTRY);
    // The journal the crashed update left: opened for generation 1 over
    // generation 0's receipt, desiring exactly the bytes now observed.
    let intent = intent_opened_over(
        1,
        Some(0),
        &[(&resource_of(client), updated_digest.as_str())],
    );
    let prior = receipt_owning(0, &[(&resource_of(client), first_digest.as_str())]);

    let plan = SkillDeployProvider::new(client)
        .plan(&request_with_intent(
            &world,
            &row,
            Some(&updated),
            Some(&prior),
            false,
            Some(&intent),
        ))
        .expect("the interrupted update is reachable settlement, not drift");
    assert_eq!(plan.resources[0].desired_digest, updated_digest);
    assert!(
        !plan.reversible,
        "the interrupted update settles over a receipt whose prior bytes are already gone: \
         removing the entry cannot restore them, and the plan says so BEFORE apply",
    );
    assert_eq!(
        std::fs::read_to_string(world.at(&format!("{}/demo/SKILL.md", client.skills_relative())))
            .unwrap(),
        UPDATED_ENTRY,
        "the stranded bytes are untouched by their own admission",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_intent_whose_prior_generation_disagrees_with_the_receipt_does_not_mask_drift() {
    let client = SkillClient::Codex;
    let (world, row, first_digest, updated_digest) = interrupted_update_world(client);
    let updated = world.skill_artifact("updated.md", UPDATED_ENTRY);
    let prior = receipt_owning(0, &[(&resource_of(client), first_digest.as_str())]);

    // The journal names the exact resource at the exact observed digest —
    // everything drift would want to hide behind — but its ownership
    // history disagrees with the injected receipt: it claims a FIRST
    // deployment, or a prior generation the receipt never recorded.
    for (label, prior_generation) in [
        ("claims no receipt", None),
        ("claims generation 2", Some(2)),
    ] {
        let intent = intent_opened_over(
            1,
            prior_generation,
            &[(&resource_of(client), updated_digest.as_str())],
        );
        let error = SkillDeployProvider::new(client)
            .plan(&request_with_intent(
                &world,
                &row,
                Some(&updated),
                Some(&prior),
                false,
                Some(&intent),
            ))
            .expect_err(&format!(
                "a journal that disagrees with the receipt state `{label}` masks no drift"
            ));
        assert!(
            matches!(refusal(&error), DeployProviderError::OccupantDrifted { .. }),
            "`{label}` leaves the ordinary drift refusal standing: {error}",
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_intent_claiming_a_prior_generation_no_receipt_witnesses_grants_nothing() {
    let client = SkillClient::OpenCode;
    let (world, row, _, updated_digest) = interrupted_update_world(client);
    let updated = world.skill_artifact("updated.md", UPDATED_ENTRY);

    // The mirror disagreement: no receipt is injected, yet the journal
    // claims it was opened over generation 0 — evidence describing an
    // ownership history this request does not hold.
    let intent = intent_opened_over(
        1,
        Some(0),
        &[(&resource_of(client), updated_digest.as_str())],
    );
    let error = SkillDeployProvider::new(client)
        .plan(&request_with_intent(
            &world,
            &row,
            Some(&updated),
            None,
            false,
            Some(&intent),
        ))
        .expect_err("a journal from another ownership history grants no admission");
    assert!(
        matches!(refusal(&error), DeployProviderError::OccupantUnowned { .. }),
        "the strict no-receipt refusal stands: {error}",
    );
}
