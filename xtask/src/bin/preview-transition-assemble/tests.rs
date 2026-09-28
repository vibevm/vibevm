use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const TRANSITION: &str =
    include_str!("../../../../formats/corpora/preview-transition/e1/valid/captured.json");
const RECORD: &str =
    include_str!("../../../../formats/corpora/release-impact/e1/valid/internal-refactor.json");

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "preview-assembler-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> (Transition, ReleaseImpact) {
    let transition: Transition = serde_json::from_str(TRANSITION).unwrap();
    let mut record: ReleaseImpact = serde_json::from_str(RECORD).unwrap();
    record.before_capture_id = transition.dp1.capture_id.clone();
    (transition, record)
}

#[test]
fn declared_relations_do_not_turn_into_net_delivery() {
    let (mut transition, mut first) = fixture();
    first.change_id = "change-a".into();
    let mut second = first.clone();
    second.change_id = "change-b".into();
    second.relations.push(
        vibe_wire::generated::preview::e1::release_impact::Relation {
            change_id: first.change_id.clone(),
            kind: vibe_wire::generated::preview::e1::release_impact::RelationKind::Reverts,
        },
    );
    transition.change_records = vec![first.change_id.clone(), second.change_id.clone()];
    let drafts = draft::render(&transition, &[first, second]).unwrap();
    assert!(drafts[0].contains("reverts"));
    assert!(drafts[0].contains("Net effect requires independent"));
    assert!(drafts[0].contains("record after-states remain declared"));
}

#[test]
fn second_staged_publication_failure_rolls_back_and_retry_is_stable() {
    let scratch = Scratch::new();
    let (transition, record) = fixture();
    let drafts = draft::render(&transition, &[record]).unwrap();
    let dir = scratch.0.join("drafts");
    vibe_safefs::fail_before_publish(Some("WORKLIST-DRAFT.md"));
    let result = output::write(&dir, &drafts);
    vibe_safefs::fail_before_publish(None);
    assert!(result.is_err());
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    output::write(&dir, &drafts).unwrap();
    assert_eq!(
        fs::read_to_string(dir.join("MIGRATION-DRAFT.md")).unwrap(),
        drafts[0]
    );
    assert_eq!(
        fs::read_to_string(dir.join("WORKLIST-DRAFT.md")).unwrap(),
        drafts[1]
    );
}

#[test]
fn first_staged_publication_failure_leaves_no_files() {
    let scratch = Scratch::new();
    vibe_safefs::fail_before_publish(Some("MIGRATION-DRAFT.md"));
    let result = output::write(&scratch.0, &["first".into(), "second".into()]);
    vibe_safefs::fail_before_publish(None);
    assert!(result.is_err());
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 0);
}

#[test]
fn second_exclusive_open_failure_preserves_concurrent_editorial_file() {
    let scratch = Scratch::new();
    let second = scratch.0.join("WORKLIST-DRAFT.md");
    let result = output::write_with(&scratch.0, &["first".into(), "second".into()], |event| {
        if matches!(event, output::Boundary::AfterPublish(0)) {
            fs::write(&second, "editorial")?;
        }
        Ok(())
    });
    assert!(result.is_err());
    assert!(!scratch.0.join("MIGRATION-DRAFT.md").exists());
    assert_eq!(fs::read_to_string(second).unwrap(), "editorial");
}

#[test]
fn error_after_both_publications_rolls_back_both_proved_files() {
    let scratch = Scratch::new();
    let result = output::write_with(&scratch.0, &["first".into(), "second".into()], |event| {
        if matches!(event, output::Boundary::AfterPublish(1)) {
            return Err("injected post-pair failure".into());
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 0);
}

#[test]
fn rollback_refuses_replaced_file_and_reports_retained_residue() {
    let scratch = Scratch::new();
    let first = scratch.0.join("MIGRATION-DRAFT.md");
    let result = output::write_with(&scratch.0, &["first".into(), "second".into()], |event| {
        match event {
            output::Boundary::BeforePublish(1) => return Err("injected second failure".into()),
            output::Boundary::BeforeRollback(0) => {
                fs::rename(&first, scratch.0.join("original-held"))?;
                fs::write(&first, "editorial replacement")?;
            }
            _ => {}
        }
        Ok(())
    });
    assert!(result.unwrap_err().to_string().contains("residue retained"));
    assert_eq!(fs::read_to_string(&first).unwrap(), "editorial replacement");
    assert!(!scratch.0.join("WORKLIST-DRAFT.md").exists());
}

#[test]
fn possibly_published_second_file_is_retained_without_unproved_cleanup() {
    let scratch = Scratch::new();
    vibe_safefs::fail_before_stage_cleanup(Some("WORKLIST-DRAFT.md"));
    let result = output::write(&scratch.0, &["first".into(), "second".into()]);
    vibe_safefs::fail_before_stage_cleanup(None);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("unproved publication residue")
    );
    assert!(!scratch.0.join("MIGRATION-DRAFT.md").exists());
    assert_eq!(
        fs::read_to_string(scratch.0.join("WORKLIST-DRAFT.md")).unwrap(),
        "second"
    );
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
    assert!(output::write(&scratch.0, &["first".into(), "second".into()]).is_err());
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn namespace_swap_cannot_redirect_publication_to_symlink_target() {
    let scratch = Scratch::new();
    let dir = scratch.0.join("drafts");
    fs::create_dir(&dir).unwrap();
    let moved = scratch.0.join("pinned-original");
    let outside = scratch.0.join("outside");
    fs::create_dir(&outside).unwrap();
    output::write_with(&dir, &["first".into(), "second".into()], |event| {
        if matches!(event, output::Boundary::Pinned) {
            fs::rename(&dir, &moved)?;
            std::os::unix::fs::symlink(&outside, &dir)?;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    assert_eq!(
        fs::read_to_string(moved.join("MIGRATION-DRAFT.md")).unwrap(),
        "first"
    );
    assert_eq!(
        fs::read_to_string(moved.join("WORKLIST-DRAFT.md")).unwrap(),
        "second"
    );
}

#[test]
fn existing_output_preflight_preserves_both_editorial_files() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("WORKLIST-DRAFT.md"), "edited worklist").unwrap();
    assert!(output::write(&scratch.0, &["first".into(), "second".into()]).is_err());
    assert!(!scratch.0.join("MIGRATION-DRAFT.md").exists());
    assert_eq!(
        fs::read_to_string(scratch.0.join("WORKLIST-DRAFT.md")).unwrap(),
        "edited worklist"
    );
}

#[test]
fn effect_collection_order_has_stable_draft_bytes() {
    use vibe_wire::generated::preview::e1::release_impact::{Effect, EffectKind};
    let (transition, mut record) = fixture();
    let effect = Effect {
        after: "new".into(),
        audience: "user".into(),
        before: "old".into(),
        capability_id: "cap".into(),
        contract_anchors: vec!["spec://b".into(), "spec://a".into()],
        interface_id: "b".into(),
        kind: EffectKind::Changed,
        platform: "Linux".into(),
    };
    let mut other = effect.clone();
    other.interface_id = "a".into();
    record.effects = vec![effect, other];
    let first = draft::render(&transition, &[record.clone()]).unwrap();
    record.effects.reverse();
    for effect in &mut record.effects {
        effect.contract_anchors.reverse();
    }
    assert_eq!(first, draft::render(&transition, &[record]).unwrap());
}

#[test]
fn catalogue_headers_retain_supplied_identity_without_claiming_proof() {
    let (mut transition, mut record) = fixture();
    let mut intermediate = transition.dp1.clone();
    intermediate.capture_id = "intermediate-before".into();
    intermediate.source = "evidence/intermediate.json".into();
    record.before_capture_id = intermediate.capture_id.clone();
    record.after_capture_id = transition.dp1.capture_id.clone();
    transition.captures = Some(vec![intermediate.clone()]);
    let drafts = draft::render(&transition, &[record]).unwrap();
    for draft in drafts {
        assert!(draft.contains("Additional supplied capture (unverified by assembly)"));
        assert!(draft.contains(&intermediate.capture_file_sha256));
        assert!(draft.contains(&intermediate.commit_sha1));
        assert!(draft.contains(&intermediate.git_tree_sha1));
        assert!(draft.contains("evidence\\/intermediate\\.json"));
        assert!(!draft.contains("UNRESOLVED before capture"));
        assert!(!draft.contains("UNRESOLVED after capture"));
    }
}

#[test]
fn unknown_pending_record_subjects_are_explicit_unresolved_and_escaped() {
    let (transition, mut record) = fixture();
    record.before_capture_id = "unknown\n# before".into();
    record.after_capture_id = "unknown-after".into();
    for draft in draft::render(&transition, &[record]).unwrap() {
        assert!(draft.contains("UNRESOLVED before capture: unknown\\\\u{a}\\# before"));
        assert!(draft.contains("UNRESOLVED after capture: unknown\\-after"));
        assert!(!draft.contains("\n# before"));
    }
}
