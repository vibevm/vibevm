//! The three standalone-skill rows through the SHIPPED executor —
//! §6.3.1's acceptance shape: "All three builtin rows select one shared
//! provider and pass plan/apply/verify/recover/remove through the engine
//! in isolated temp homes."
//!
//! Everything here is real: the rows resolve through the one mechanism
//! registry (BuiltinDefault — provider id and logical name visibly
//! separate), the engine owns the transaction, and the destinations live
//! under INJECTED temp homes with three MISSING client executables, so no
//! run reaches an operator's home, a token, the network or a real client.
//!
//! The engine world, the crash-point provider and the entry documents
//! live in the suite's shared cell, beside this file — the update
//! recovery cell next door drives the same machinery.

use vibe_extension_registry::SelectionStep;

use super::client::SkillClient;
use super::support::{
    DEMO_ENTRY, EngineWorld, FailingAfterWrite, UPDATED_ENTRY, engine_state_of, selection, target,
    write_at,
};
use crate::mechanism::deploy::{
    Selected, apply_selection, execute_deploy_targets, list_deployments, undeploy_targets,
};

#[test]
fn all_three_clients_deploy_verify_list_and_undeploy_through_the_engine() {
    for (client, root) in [
        (SkillClient::Claude, ".claude/skills"),
        (SkillClient::Codex, ".agents/skills"),
        (SkillClient::OpenCode, ".config/opencode/skills"),
    ] {
        let world = EngineWorld::new();
        world.record_skill("demo.md", DEMO_ENTRY);
        let row = target(client, "skill-target", "demo.md", "demo");
        let selected = selection(&["skill-target"]);

        // ---- deploy, through the shipped executor and the one registry --
        let outcomes =
            execute_deploy_targets(&world.execution(std::slice::from_ref(&row), &selected))
                .unwrap_or_else(|error| panic!("{} deploys: {error}", client.as_str()));
        assert_eq!(outcomes.len(), 1);
        let outcome = &outcomes[0];
        assert_eq!(
            outcome.provider,
            client.pin(),
            "the row selects its own pin"
        );
        assert_eq!(outcome.via, "the shipped builtin default");
        assert_eq!(
            outcome.mechanism,
            format!("deploy:{}-skill", client.as_str())
        );
        assert_eq!(outcome.resources.len(), 1);
        assert_eq!(
            outcome.resources[0].resource,
            format!("home:{root}/demo/SKILL.md")
        );
        assert_eq!(
            std::fs::read(world.at(&format!("{root}/demo/SKILL.md"))).unwrap(),
            DEMO_ENTRY.as_bytes(),
            "the exact proven bytes are at the exact client path",
        );

        // ---- the listing reads receipt facts only ------------------------
        let rows = list_deployments(&world.state_home).expect("the state home lists");
        assert_eq!(rows.len(), 1, "{}", client.as_str());
        assert_eq!(rows[0].provider, client.pin());
        assert_eq!(rows[0].scope, "user");
        assert_eq!(rows[0].resources, 1);
        assert_eq!(rows[0].status.as_str(), "verified");

        // ---- undeploy, with foreign neighbours planted around the entry -
        write_at(
            &world,
            &format!("{root}/demo/NOTES.txt"),
            b"foreign neighbour\n",
        );
        write_at(
            &world,
            &format!("{root}/other/SKILL.md"),
            b"sibling skill\n",
        );
        let removals = undeploy_targets(&world.execution(std::slice::from_ref(&row), &selected))
            .unwrap_or_else(|error| panic!("{} undeploys: {error}", client.as_str()));
        assert_eq!(removals.len(), 1);
        assert_eq!(removals[0].removed, [format!("home:{root}/demo/SKILL.md")]);
        assert!(!world.at(&format!("{root}/demo/SKILL.md")).exists());
        assert_eq!(
            std::fs::read(world.at(&format!("{root}/demo/NOTES.txt"))).unwrap(),
            b"foreign neighbour\n",
        );
        assert_eq!(
            std::fs::read(world.at(&format!("{root}/other/SKILL.md"))).unwrap(),
            b"sibling skill\n",
        );
        assert!(
            world.at(root).exists(),
            "the skills root is the prune boundary and survives",
        );
    }
}

#[test]
fn an_engine_update_refuses_a_hand_edited_destination_and_leaves_it_intact() {
    let world = EngineWorld::new();
    world.record_skill("demo.md", DEMO_ENTRY);
    let row = target(SkillClient::Claude, "skill-target", "demo.md", "demo");
    let selected = selection(&["skill-target"]);
    execute_deploy_targets(&world.execution(std::slice::from_ref(&row), &selected))
        .expect("the first generation deploys");

    // A hand edit after deployment, then an update to new bytes: the
    // receipt-owned drift law refuses and the hand edit survives.
    std::fs::write(world.at(".claude/skills/demo/SKILL.md"), b"hand-edited\n")
        .expect("the hand edit writes");
    world.record_skill("updated.md", UPDATED_ENTRY);
    let updated = target(SkillClient::Claude, "skill-target", "updated.md", "demo");
    let error = execute_deploy_targets(&world.execution(std::slice::from_ref(&updated), &selected))
        .expect_err("drifted bytes are never silently overwritten");
    assert!(
        error.to_string().contains("now holds"),
        "the provider's drift refusal reached the caller: {error}",
    );
    assert_eq!(
        std::fs::read(world.at(".claude/skills/demo/SKILL.md")).unwrap(),
        b"hand-edited\n",
    );
}

#[test]
fn an_unowned_identical_occupant_refuses_through_the_engine_before_any_write() {
    let world = EngineWorld::new();
    world.record_skill("demo.md", DEMO_ENTRY);
    let row = target(SkillClient::Codex, "skill-target", "demo.md", "demo");
    // The occupant holds EXACTLY the desired bytes, owned by nobody.
    std::fs::create_dir_all(world.at(".agents/skills/demo"))
        .expect("the fixture directory creates");
    std::fs::write(world.at(".agents/skills/demo/SKILL.md"), DEMO_ENTRY)
        .expect("the fixture occupant writes");
    let selected = selection(&["skill-target"]);
    let error = execute_deploy_targets(&world.execution(std::slice::from_ref(&row), &selected))
        .expect_err("an absent receipt never authorises a foreign occupant");
    assert!(
        error.to_string().contains("does not own"),
        "the unowned-occupant refusal reached the caller: {error}",
    );
    assert_eq!(
        std::fs::read(world.at(".agents/skills/demo/SKILL.md")).unwrap(),
        DEMO_ENTRY.as_bytes(),
        "the occupant is byte-identical",
    );
    assert!(
        !world.state_home.exists(),
        "the pre-apply epoch created no state home refusing",
    );
}

#[test]
fn an_interrupted_apply_after_its_write_is_recovered_by_the_next_normal_run() {
    let world = EngineWorld::new();
    world.record_skill("demo.md", DEMO_ENTRY);
    let row = target(SkillClient::OpenCode, "skill-target", "demo.md", "demo");
    let selected = selection(&["skill-target"]);

    // The interrupted run: the provider completes its REAL work — the
    // entry is published and checkpointed — and then dies, AFTER the
    // atomic publication and BEFORE finalisation. §7.2's crash window at
    // its hardest: desired bytes at the destination, an unretired durable
    // intent beside them, and no receipt anywhere. The next ordinary run
    // must reach `recover`, checkpoint without rewriting those bytes,
    // finalise the interrupted generation and retire the intent.
    let interrupted = Selected {
        target: &row,
        provider: Box::new(FailingAfterWrite(SkillClient::OpenCode)),
        pin: crate::mechanism::BUILTIN_OPENCODE_SKILL_PIN.to_owned(),
        via: SelectionStep::BuiltinDefault,
        displaced: None,
    };
    let error = apply_selection(
        &world.execution(std::slice::from_ref(&row), &selected),
        &[interrupted],
    )
    .expect_err("the interrupted run fails after its write");
    assert!(
        error.to_string().contains("sentinel-after-write"),
        "the wrapper's own failure is the reported one: {error}",
    );

    // The after-write proof: the entry EXISTS at the exact desired bytes
    // (the write the crash stranded), no receipt was finalised, and the
    // durable intent is still unretired beside it.
    let entry = world.at(".config/opencode/skills/demo/SKILL.md");
    assert_eq!(
        std::fs::read(&entry).unwrap(),
        DEMO_ENTRY.as_bytes(),
        "the interrupted apply published the entry before it died",
    );
    let desired = crate::mechanism::contain::digest_file(&entry)
        .map(|(digest, _)| digest)
        .expect("the stranded entry digests");
    let rows = list_deployments(&world.state_home).expect("the state home lists");
    assert!(
        rows.is_empty(),
        "no receipt was finalised for the interrupted run",
    );
    let (state, home) = engine_state_of(&world, &row);
    let intent = state
        .read_intent(&home)
        .expect("the state home reads")
        .expect("the durable intent is unretired after the crash");
    assert_eq!(intent.resources.len(), 1);
    assert_eq!(intent.resources[0].desired_digest, desired);

    // The next NORMAL run — ordinary dispatch, ordinary executor —
    // settles the journal: the plan admits its own interrupted occupant
    // through the injected intent, the transaction's three-digest law
    // holds (observed == desired), and `recover` runs.
    let written_at = std::fs::metadata(&entry)
        .and_then(|metadata| metadata.modified())
        .expect("the stranded entry has a modification stamp");
    let outcomes = execute_deploy_targets(&world.execution(std::slice::from_ref(&row), &selected))
        .expect("the next normal run recovers the interrupted deployment");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].settlement, "recovered",
        "rolled forward, not reapplied",
    );
    assert_eq!(outcomes[0].generation, 0, "the journal's own generation");
    assert_eq!(outcomes[0].resources[0].post_digest, desired);

    // The receipt finalised as verified, and its evidence carries the
    // real provider's own recover branch: "was already desired and
    // stayed" is produced ONLY by the idempotent no-publish branch, so
    // the finalised record proves `recover` delegated there and wrote
    // nothing — a deterministic, host-independent no-rewrite proof
    // (file-identity/mtime are supporting evidence, not the gate).
    let receipt = state
        .read_receipt(&home)
        .expect("the state home reads")
        .expect("the recovered run finalised a receipt");
    assert_eq!(receipt.generation, 0);
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
    // already-desired entry was not rewritten: a staged-rename publish
    // would have replaced both the bytes' identity and the stamp.
    assert_eq!(
        std::fs::read(&entry).unwrap(),
        DEMO_ENTRY.as_bytes(),
        "the stranded bytes are the final bytes",
    );
    assert_eq!(
        crate::mechanism::contain::digest_file(&entry)
            .map(|(digest, _)| digest)
            .unwrap(),
        desired,
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

#[test]
fn a_stale_intent_retires_and_the_ordinary_apply_still_refuses_the_unowned_occupant() {
    let world = EngineWorld::new();
    world.record_skill("demo.md", DEMO_ENTRY);
    let row = target(SkillClient::Claude, "skill-target", "demo.md", "demo");
    let selected = selection(&["skill-target"]);

    // The same after-write crash, stranded at generation 0's desired
    // bytes under Claude's root.
    let interrupted = Selected {
        target: &row,
        provider: Box::new(FailingAfterWrite(SkillClient::Claude)),
        pin: crate::mechanism::BUILTIN_CLAUDE_SKILL_PIN.to_owned(),
        via: SelectionStep::BuiltinDefault,
        displaced: None,
    };
    apply_selection(
        &world.execution(std::slice::from_ref(&row), &selected),
        &[interrupted],
    )
    .expect_err("the interrupted run fails after its write");
    assert!(world.at(".claude/skills/demo/SKILL.md").exists());

    // The next run wants DIFFERENT bytes: a new artifact makes a new plan
    // hash, so the unretired journal is now stale. Its three-digest law
    // still holds (the entry sits at the journal's own desired digest),
    // so settlement retires it — and then this run's own ordinary apply
    // meets an occupant no receipt owns, and the frozen law refuses.
    world.record_skill("updated.md", UPDATED_ENTRY);
    let updated = target(SkillClient::Claude, "skill-target", "updated.md", "demo");
    let error = execute_deploy_targets(&world.execution(std::slice::from_ref(&updated), &selected))
        .expect_err("a stale intent is not write authority");
    assert!(
        error.to_string().contains("does not own"),
        "the refusal is the apply-time unowned-occupant one: {error}",
    );
    assert_eq!(
        std::fs::read(world.at(".claude/skills/demo/SKILL.md")).unwrap(),
        DEMO_ENTRY.as_bytes(),
        "the interrupted generation's bytes survive the refused update",
    );

    // The stale journal retired; what remains unretired is the refused
    // run's OWN journal (desiring the updated digest) — proof the
    // settlement took the stale-retire branch and never rolled forward.
    let (state, home) = engine_state_of(&world, &row);
    let remaining = state
        .read_intent(&home)
        .expect("the state home reads")
        .expect("the refused run left its own journal");
    let updated_digest = crate::mechanism::contain::digest_file(
        &world
            .project
            .path()
            .join("target/vibe-package/updated.md/SKILL.md"),
    )
    .map(|(digest, _)| digest)
    .expect("the updated artifact digests");
    assert_eq!(
        remaining.resources[0].desired_digest, updated_digest,
        "the stale journal retired; the refused plan's own journal is what is left",
    );
}
