//! §7.2's after-write crash window over an UPDATE — the general recovery
//! law's second half, engine-driven end to end.
//!
//! The first-deployment window next door strands desired bytes with no
//! receipt anywhere. An interrupted UPDATE is the harder shape: the
//! prior receipt still owns the entry at the PRIOR generation's digest,
//! the observed entry already holds the interrupted generation's desired
//! digest, and the strict drift law would refuse exactly the run that
//! exists to finish it. The plan seat's intent evidence — exact
//! resource, exact desired digest, `prior_generation` agreeing with the
//! injected receipt — is what tells that window from drift, and the
//! transaction's plan-hash law stays the only authority that may call
//! `recover`.
//!
//! The machinery (the engine world, the recorded artifacts, the
//! after-write crash-point provider) is the shared suite cell's, so the
//! two windows are proven over ONE implementation.

use vibe_extension_registry::SelectionStep;

use super::client::SkillClient;
use super::support::{
    DEMO_ENTRY, EngineWorld, FailingAfterWrite, UPDATED_ENTRY, engine_state_of, selection, target,
};
use crate::mechanism::deploy::{Selected, apply_selection, execute_deploy_targets};

#[test]
fn an_interrupted_update_after_its_write_is_recovered_by_the_next_normal_run() {
    let world = EngineWorld::new();
    let client = SkillClient::Codex;
    let first_digest = world.record_skill("demo.md", DEMO_ENTRY);
    let row = target(client, "skill-target", "demo.md", "demo");
    let selected = selection(&["skill-target"]);

    // ---- generation 0 deploys normally --------------------------------
    let outcomes = execute_deploy_targets(&world.execution(std::slice::from_ref(&row), &selected))
        .expect("generation 0 deploys");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].generation, 0);
    assert_eq!(outcomes[0].settlement, "none");

    // ---- generation 1 begins with CHANGED bytes, and crashes AFTER its
    // write: the real provider publishes the new bytes and checkpoints,
    // and only then does the injected crash point fire — before
    // finalisation, so the receipt never moves.
    let updated_digest = world.record_skill("updated.md", UPDATED_ENTRY);
    assert_ne!(first_digest, updated_digest, "the generations differ");
    let updated = target(client, "skill-target", "updated.md", "demo");
    let interrupted = Selected {
        target: &updated,
        provider: Box::new(FailingAfterWrite(client)),
        pin: client.pin().to_owned(),
        via: SelectionStep::BuiltinDefault,
        displaced: None,
    };
    let error = apply_selection(
        &world.execution(std::slice::from_ref(&updated), &selected),
        &[interrupted],
    )
    .expect_err("the interrupted update fails after its write");
    assert!(
        error.to_string().contains("sentinel-after-write"),
        "the wrapper's own failure is the reported one: {error}",
    );

    // ---- the after-write proof: the entry already holds generation 1's
    // desired bytes, the receipt still records generation 0 at
    // generation 0's digest, and the unretired journal says it was
    // opened over generation 0 — the exact interrupted-update shape.
    let entry = world.at(".agents/skills/demo/SKILL.md");
    assert_eq!(
        std::fs::read(&entry).unwrap(),
        UPDATED_ENTRY.as_bytes(),
        "the interrupted update published the new bytes before it died",
    );
    let (state, home) = engine_state_of(&world, &row);
    let receipt = state
        .read_receipt(&home)
        .expect("the state home reads")
        .expect("generation 0's receipt survived the crash");
    assert_eq!(receipt.generation, 0, "finalisation never ran");
    assert_eq!(receipt.resources[0].post_digest, first_digest);
    let intent = state
        .read_intent(&home)
        .expect("the state home reads")
        .expect("the durable intent is unretired after the crash");
    assert_eq!(intent.prior_generation, Some(0));
    assert_eq!(intent.target.generation, 1);
    assert_eq!(intent.resources[0].desired_digest, updated_digest);

    // ---- the next NORMAL run settles it: the plan admits its own
    // interrupted update through the injected intent (not as drift), the
    // three-digest law holds (observed == desired), and `recover` runs.
    let written_at = std::fs::metadata(&entry)
        .and_then(|metadata| metadata.modified())
        .expect("the stranded entry has a modification stamp");
    let outcomes =
        execute_deploy_targets(&world.execution(std::slice::from_ref(&updated), &selected))
            .expect("the next normal run recovers the interrupted update");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].settlement, "recovered",
        "rolled forward, not reapplied and not refused as drift",
    );
    assert_eq!(outcomes[0].generation, 1, "the journal's own generation");
    assert_eq!(outcomes[0].resources[0].post_digest, updated_digest);

    // The receipt finalised as generation 1, verified, carrying the
    // idempotent no-publish branch's own words — the deterministic proof
    // `recover` delegated to the desired branch and rewrote nothing.
    let receipt = state
        .read_receipt(&home)
        .expect("the state home reads")
        .expect("the recovered run finalised a receipt");
    assert_eq!(receipt.generation, 1);
    assert!(
        matches!(
            receipt.status,
            vibe_wire::generated::deploy_receipt::ReceiptStatus::Verified
        ),
        "the recovered generation finalised as verified",
    );
    let evidence = receipt.evidence.as_deref().unwrap_or_default();
    assert!(
        evidence.contains("recovered:") && evidence.contains("was already desired and stayed"),
        "the real recover took the idempotent desired branch: {evidence}",
    );

    // The bytes, digest and (supporting) modification stamp prove the
    // already-desired entry was not rewritten by its own recovery.
    assert_eq!(
        std::fs::read(&entry).unwrap(),
        UPDATED_ENTRY.as_bytes(),
        "the stranded bytes are the final bytes",
    );
    assert_eq!(
        crate::mechanism::contain::digest_file(&entry)
            .map(|(digest, _)| digest)
            .unwrap(),
        updated_digest,
    );
    let settled_at = std::fs::metadata(&entry)
        .and_then(|metadata| metadata.modified())
        .expect("the settled entry has a modification stamp");
    assert_eq!(
        written_at, settled_at,
        "a host with fine-grained stamps sees no rewrite; the receipt's \
         branch evidence above is the deterministic gate",
    );

    // And the journal retired: nothing is outstanding any more.
    assert!(
        state
            .read_intent(&home)
            .expect("the state home reads")
            .is_none(),
        "the intent retired with the settlement",
    );
}
