//! Transaction-level RED oracles: durable-stage authority, final CAS, and
//! pure visible verification.

use std::fs;
use std::path::Path;

use super::nofollow::Project;
use super::state::{read_receipt, write_receipt};
use super::tests::{begin_interrupted, lower, provider, seed};
use crate::pkgskill::{reconcile_project_skill_binding, recover_project_skill_bindings};

#[test]
fn missing_stage_directory_is_a_hard_refusal_with_intent_intact() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    begin_interrupted(project.path(), alpha);
    // Destroy the whole durable stage: recovery must refuse, not adopt.
    let nonce = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap()
        .applying
        .unwrap()
        .nonce;
    fs::remove_dir_all(
        project
            .path()
            .join(".vibe/package-skills/staged")
            .join(&nonce),
    )
    .unwrap();
    let error = recover_project_skill_bindings(project.path()).unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("opening no-follow directory"), "{error}");
    let receipt = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap();
    assert!(receipt.applying.is_some(), "intent stays for retry");
    assert!(!Path::new(&alpha.targets[0].path).join("SKILL.md").exists());
}

#[test]
fn recovery_converges_and_cleanup_removes_only_referenced_stage_files() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    begin_interrupted(project.path(), alpha);
    let nonce = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap()
        .applying
        .unwrap()
        .nonce;
    let files = project
        .path()
        .join(".vibe/package-skills/staged")
        .join(&nonce)
        .join("files");
    // An unexpected neighbour with a VALID content-addressed shape: its
    // filename is the exact SHA-256 hex of its own bytes. It is not
    // plan-required, so recovery never adopts it and cleanup never deletes it.
    let neighbour_bytes = b"unexpected-neighbour";
    let neighbour = files.join(
        super::state::digest(neighbour_bytes)
            .strip_prefix("sha256:")
            .unwrap(),
    );
    fs::write(&neighbour, neighbour_bytes).unwrap();

    recover_project_skill_bindings(project.path()).unwrap();
    let target = Path::new(&alpha.targets[0].path);
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "body-one"
    );
    // The correctly-shaped but unreferenced neighbour survives; the
    // plan-required digest file is gone and the neighbour keeps `files` (and
    // the nonce directory) alive.
    assert_eq!(
        fs::read_to_string(&neighbour).unwrap(),
        "unexpected-neighbour"
    );
    let digest_name = super::state::digest(b"body-one")
        .strip_prefix("sha256:")
        .unwrap()
        .to_string();
    assert_ne!(
        neighbour.file_name().unwrap().to_string_lossy(),
        digest_name
    );
    assert!(!files.join(&digest_name).exists());
    assert!(files.is_dir());
}

/// The final receipt replacement is a full CAS: capture the original plan
/// from the valid durable receipt, rewrite the receipt with the same
/// key+nonce but a changed after-plan, and finalise the ORIGINAL plan
/// directly — exactly the `receipt changed under transaction` family, with
/// the applying intent retained. Never the unrelated missing-stage error.
#[test]
fn final_cas_refuses_a_changed_plan_with_the_same_key_and_nonce() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    begin_interrupted(project.path(), &bindings[0]);
    let project_cap = Project::open(project.path()).unwrap();
    let _guard = project_cap.lock(super::nofollow::LOCK_FILE).unwrap();
    // (1) The original plan, captured from the valid durable receipt.
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    let applying = receipt.applying.clone().unwrap();
    let original = super::transaction::Plan {
        key: applying.key.clone(),
        nonce: applying.nonce.clone(),
        before: receipt.binding.clone(),
        after: applying.binding.clone(),
    };
    // (2) Same key and nonce, changed after-plan: an extra owned file row.
    let mut tampered = receipt.clone();
    let mut changed = applying.clone();
    changed.binding[0].target[0].file.push(
        vibe_wire::generated::package_skill_receipt::PackageSkillFile {
            path: "EXTRA.md".into(),
            sha256: format!("sha256:{}", "0".repeat(64)),
        },
    );
    tampered.applying = Some(changed);
    write_receipt(&project_cap, &tampered).unwrap();
    // (3) Finalise the original plan directly against the changed receipt.
    let error =
        super::transaction::finalize_receipt(&project_cap, project.path(), &original).unwrap_err();
    let error = format!("{error:#}");
    // (4) Exactly the CAS refusal family — never the missing-stage error.
    assert!(
        error.contains("receipt changed under transaction"),
        "{error}"
    );
    assert!(!error.contains("required staged bytes"), "{error}");
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    assert!(receipt.applying.is_some(), "the applying intent remains");
}

/// Visible tamper between execute and finalisation refuses without receipt
/// promotion (the pure verifier, not a healing pass).
#[test]
fn visible_tamper_between_execute_and_finalize_refuses_without_promotion() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    begin_interrupted(project.path(), alpha);
    // Execute the stored plan exactly as recovery would…
    let project_cap = Project::open(project.path()).unwrap();
    let _guard = project_cap.lock(super::nofollow::LOCK_FILE).unwrap();
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    let applying = receipt.applying.clone().unwrap();
    let plan = super::transaction::Plan {
        key: applying.key.clone(),
        nonce: applying.nonce.clone(),
        before: receipt.binding.clone(),
        after: applying.binding.clone(),
    };
    let stage =
        super::stage::Stage::existing(&project_cap, &plan.nonce, &plan.required_stage_digests())
            .unwrap();
    super::transaction::execute_plan(&project_cap, project.path(), Some(&stage), &plan).unwrap();
    // …then tamper the visible target before finalisation.
    fs::write(
        Path::new(&alpha.targets[0].path).join("SKILL.md"),
        "swapped-after-execute",
    )
    .unwrap();
    let error =
        super::transaction::finalize_receipt(&project_cap, project.path(), &plan).unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("instead of") || error.contains("verifying published"),
        "{error}"
    );
    // The receipt was not promoted: the intent stays durable for retry.
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    assert!(receipt.applying.is_some());
}

/// A whole-binding removal whose owned file is recreated before finalisation
/// refuses the pure verification — the root-added whole-binding removal
/// branch of `verify_visible`, not the retained-target branch.
#[test]
fn removed_target_recreated_before_finalize_refuses_without_promotion() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    // Commit the binding so the receipt owns one target with one file.
    reconcile_project_skill_binding(project.path(), alpha).unwrap();
    let project_cap = Project::open(project.path()).unwrap();
    let _guard = project_cap.lock(super::nofollow::LOCK_FILE).unwrap();
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    // A pure removal plan: before keeps the committed row, after drops it.
    let plan = super::transaction::Plan {
        key: alpha.identity(),
        nonce: super::state::fresh_nonce(),
        before: receipt.binding.clone(),
        after: Vec::new(),
    };
    let mut intent = vibe_wire::generated::package_skill_receipt::PackageSkillReceipt {
        applying: None,
        binding: plan.before.clone(),
        schema: 2,
    };
    intent.applying = Some(plan.applying());
    write_receipt(&project_cap, &intent).unwrap();
    super::transaction::execute_plan(&project_cap, project.path(), None, &plan).unwrap();
    let target = Path::new(&alpha.targets[0].path);
    assert!(!target.join("SKILL.md").exists(), "removal executed");
    // Recreate the exact owned bytes before finalisation.
    fs::create_dir_all(target).unwrap();
    fs::write(target.join("SKILL.md"), "body-one").unwrap();
    let error =
        super::transaction::finalize_receipt(&project_cap, project.path(), &plan).unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("removed owned file") && error.contains("still present"),
        "{error}"
    );
    // The receipt was not promoted: the removal intent stays durable.
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    assert!(receipt.applying.is_some());
}

#[test]
fn split_relative_rejects_empty_and_unsafe_segments() {
    for relative in [
        "a//b",
        "a/",
        "/a",
        "",
        "a/./b",
        "a/../b",
        "a\\b",
        "CON",
        "a/CON",
        "a/b.",
        // Extension-bearing device aliases delegate to the shared core table.
        "a/CON.txt",
        "a/NUL.md",
        "a/COM1.json",
        "a/CONIN$",
        "a/COM¹",
    ] {
        assert!(
            super::nofollow::split_relative(relative).is_err(),
            "{relative}"
        );
    }
    let (parents, file) = super::nofollow::split_relative("a/b/c.md").unwrap();
    assert_eq!(parents, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(file, "c.md");
}

#[test]
fn exclusive_stage_directory_creation_refuses_reuse() {
    let project = tempfile::tempdir().unwrap();
    let capability = Project::open(project.path()).unwrap();
    let root = capability.dir(&[".vibe"], true).unwrap();
    root.create_child_exclusive("nonce-x").unwrap();
    let error = root.create_child_exclusive("nonce-x").unwrap_err();
    assert!(
        error.to_string().contains("exclusively creating"),
        "{error}"
    );
}

#[test]
fn hardlinked_lock_file_refuses() {
    let project = tempfile::tempdir().unwrap();
    let capability = Project::open(project.path()).unwrap();
    drop(capability.lock(super::nofollow::LOCK_FILE).unwrap());
    // A second name for the lock file makes it a shared hard link: taking
    // the lock through either name must refuse.
    let lock = project.path().join(".vibe/package-skills.lock");
    let alias = project.path().join(".vibe/package-skills.lock.alias");
    fs::hard_link(&lock, &alias).unwrap();
    let error = capability.lock(super::nofollow::LOCK_FILE).unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("hard link"), "{error}");
}

/// A plan-required staged file whose bytes no longer hash to its filename is
/// a hard refusal; the intent stays intact for a manual repair + retry.
#[test]
fn corrupt_required_staged_file_refuses_with_intent_intact() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    begin_interrupted(project.path(), alpha);
    let nonce = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap()
        .applying
        .unwrap()
        .nonce;
    let files = project
        .path()
        .join(".vibe/package-skills/staged")
        .join(&nonce)
        .join("files");
    let digest_name = super::state::digest(b"body-one")
        .strip_prefix("sha256:")
        .unwrap()
        .to_string();
    fs::write(files.join(&digest_name), "corrupted-bytes").unwrap();
    let error = recover_project_skill_bindings(project.path()).unwrap_err();
    let error = format!("{error:#}");
    // Pinned exactly, not by fragments. A literal wrapped across source lines
    // without a continuation bakes its indentation into the message, and every
    // `contains("hashes to")` assertion in this file would still have passed
    // while an operator read a sentence with eighteen spaces in the middle.
    assert!(
        error.contains(&format!(
            "required staged file `{digest_name}` hashes to `{}` instead of \
             `sha256:{digest_name}`; refusing to trust the durable stage",
            super::state::digest(b"corrupted-bytes"),
        )),
        "{error}"
    );
    assert!(
        !error.contains("  "),
        "no run of spaces may survive: {error}"
    );
    let receipt = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap();
    assert!(receipt.applying.is_some(), "intent stays for retry");
    assert!(!Path::new(&alpha.targets[0].path).join("SKILL.md").exists());
}

/// Changed-row scoping: a recovery that removes one binding while retaining
/// an unrelated one never executes, verifies, or blocks on the unchanged
/// row — a tampered retained target stays byte-identical through recovery
/// (no removal-recovery wedge, no stage required), and the retained binding's
/// own later transaction heals it.
#[test]
fn changed_row_scoping_recovery_leaves_unrelated_tampered_targets_alone() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let two = seed(project.path(), "two");
    let bindings = lower(
        project.path(),
        vec![
            provider(one.path(), "one", "alpha", &["claude"]),
            provider(two.path(), "two", "beta", &["claude"]),
        ],
    );
    let (alpha, beta) = (bindings[0].clone(), bindings[1].clone());
    reconcile_project_skill_binding(project.path(), &alpha).unwrap();
    reconcile_project_skill_binding(project.path(), &beta).unwrap();
    // Tamper the retained binding's target after commit.
    let beta_target = Path::new(&beta.targets[0].path);
    fs::write(beta_target.join("SKILL.md"), "tampered-beta").unwrap();

    // Publish a removal-only intent for alpha that retains beta's row.
    {
        let project_cap = Project::open(project.path()).unwrap();
        let _guard = project_cap.lock(super::nofollow::LOCK_FILE).unwrap();
        let receipt = read_receipt(&project_cap).unwrap().unwrap();
        let beta_row = receipt
            .binding
            .iter()
            .find(|row| row.key == beta.identity())
            .cloned()
            .unwrap();
        let plan = super::transaction::Plan {
            key: alpha.identity(),
            nonce: super::state::fresh_nonce(),
            before: receipt.binding.clone(),
            after: vec![beta_row],
        };
        assert!(
            plan.required_stage_digests().is_empty(),
            "a removal-only plan retaining another binding requires no stage"
        );
        let mut intent = vibe_wire::generated::package_skill_receipt::PackageSkillReceipt {
            applying: None,
            binding: plan.before.clone(),
            schema: 2,
        };
        intent.applying = Some(plan.applying());
        write_receipt(&project_cap, &intent).unwrap();
    }

    // Recovery converges without a stage and without touching tampered beta.
    let recovered = recover_project_skill_bindings(project.path()).unwrap();
    assert!(
        recovered
            .iter()
            .any(|report| report.status == "removed" && report.path.is_some()),
        "{recovered:?}"
    );
    assert!(!Path::new(&alpha.targets[0].path).join("SKILL.md").exists());
    assert_eq!(
        fs::read_to_string(beta_target.join("SKILL.md")).unwrap(),
        "tampered-beta",
        "unchanged rows are never executed or verified by an unrelated transaction"
    );
    let receipt = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap();
    assert!(receipt.applying.is_none());
    assert_eq!(receipt.binding.len(), 1, "only the retained row remains");

    // The retained binding's own later transaction heals the tampered bytes.
    reconcile_project_skill_binding(project.path(), &beta).unwrap();
    assert_eq!(
        fs::read_to_string(beta_target.join("SKILL.md")).unwrap(),
        "body-two"
    );
}

/// Receipt target ownership requires the exact canonical spelling on every
/// host: a case-fold-equivalent but differently spelled target path is
/// refused by the strict reader before it can authorize the canonical
/// target. Purely string-level — no filesystem aliases are created.
#[test]
fn case_fold_alias_target_spelling_refuses_strict_read() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let receipt_path = project.path().join(".vibe/package-skills.toml");
    let text = fs::read_to_string(&receipt_path).unwrap();
    let canonical =
        format!("\"{}/.claude/skills/alpha\"", project.path().display()).replace('\\', "/");
    let alias = canonical.replace("/.claude/", "/.CLAUDE/");
    assert!(text.contains(&canonical), "canonical target row present");
    fs::write(&receipt_path, text.replace(&canonical, &alias)).unwrap();
    let error = read_receipt(&Project::open(project.path()).unwrap()).unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("is not the canonical"), "{error}");
}
