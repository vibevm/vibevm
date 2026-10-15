use super::adoption::CoordinatedSkills;
use vibe_safefs::Project;

#[test]
fn replaced_arena_refuses_stale_writer_before_skill_effects() {
    use crate::agents::{Agent, Scope};
    let dir = tempfile::tempdir().unwrap();
    let arena = dir.path().join("arena");
    let project = dir.path().join("project");
    let source = dir.path().join("source");
    for path in [&arena, &project, &source] {
        std::fs::create_dir(path).unwrap();
    }
    std::fs::write(
        source.join("SKILL.md"),
        b"---\nname: demo\ndescription: fixture\n---\nbody\n",
    )
    .unwrap();
    let stale = CoordinatedSkills::new(Project::open(&arena).unwrap()).unwrap();
    std::fs::rename(&arena, dir.path().join("old")).unwrap();
    std::fs::create_dir(&arena).unwrap();
    let fresh = CoordinatedSkills::new(Project::open(&arena).unwrap()).unwrap();
    let agent = Agent::parse_filter("claude").unwrap()[0];
    assert!(
        stale
            .install(
                agent,
                Scope::Project,
                Some(&project),
                "demo",
                &source,
                &[],
                false
            )
            .is_err()
    );
    assert!(stale.recover(&project).is_err());
    assert!(
        stale
            .reconcile_vanished(&project, &Default::default())
            .is_err()
    );
    assert_eq!(std::fs::read_dir(&project).unwrap().count(), 0);
    assert!(!dir.path().join("old/.vibe").exists());
    fresh
        .install(
            agent,
            Scope::Project,
            Some(&project),
            "demo",
            &source,
            &[],
            false,
        )
        .unwrap();
    let receipt = project.join(".claude/skills/.demo.vibe-skill-receipt.toml");
    let original = std::fs::read(&receipt).unwrap();
    assert!(
        stale
            .uninstall(agent, Scope::Project, Some(&project), "demo", false)
            .is_err()
    );
    assert_eq!(std::fs::read(&receipt).unwrap(), original);
    fresh
        .uninstall(agent, Scope::Project, Some(&project), "demo", false)
        .unwrap();
    assert!(!project.join(".claude/skills/demo/SKILL.md").exists());
}

#[test]
fn nested_destination_roots_share_explicit_arena_before_parents_exist() {
    let dir = tempfile::tempdir().unwrap();
    let arena = dir.path().join("arena");
    let destination = dir.path().join("destination");
    std::fs::create_dir(&arena).unwrap();
    std::fs::create_dir(&destination).unwrap();
    let first = CoordinatedSkills::new(Project::open(&arena).unwrap()).unwrap();
    let second = CoordinatedSkills::new(Project::open(&arena).unwrap()).unwrap();
    assert_eq!(first.identity(), second.identity());
    let held = Project::open(&arena)
        .unwrap()
        .lock("deploy-physical-coordination.lock")
        .unwrap();
    assert!(
        Project::open(&arena)
            .unwrap()
            .try_lock("deploy-physical-coordination.lock")
            .unwrap()
            .is_none()
    );
    assert!(!destination.join("nested/file").exists());
    std::fs::create_dir(destination.join("nested")).unwrap();
    assert!(
        Project::open(&arena)
            .unwrap()
            .try_lock("deploy-physical-coordination.lock")
            .unwrap()
            .is_none()
    );
    drop(held);
    assert!(
        Project::open(&arena)
            .unwrap()
            .try_lock("deploy-physical-coordination.lock")
            .unwrap()
            .is_some()
    );
}

#[test]
fn different_arena_is_not_claimed_as_same_authority() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let one = CoordinatedSkills::new(Project::open(a.path()).unwrap()).unwrap();
    let two = CoordinatedSkills::new(Project::open(b.path()).unwrap()).unwrap();
    assert_ne!(one.identity(), two.identity());
}

#[test]
fn automatic_hold_blocks_writer_and_restore_preserves_legacy_reader() {
    use super::adoption::SkillDonor;
    use super::tests::{lower, provider, seed};
    use sha2::{Digest, Sha256};
    use vibe_wire::generated::deploy_generic_state::*;
    let project = tempfile::tempdir().unwrap();
    let source = seed(project.path(), "demo");
    let binding = lower(
        project.path(),
        vec![provider(source.path(), "pkg", "demo", &["claude"])],
    )[0]
    .clone();
    crate::pkgskill::reconcile_project_skill_binding(project.path(), &binding).unwrap();
    let donor =
        SkillDonor::automatic(Project::open(project.path()).unwrap(), &binding.identity()).unwrap();
    let original = donor.receipt_bytes().to_vec();
    let mut record: DeployGenericState = serde_json::from_str(include_str!(
        "../../../../../formats/corpora/deploy-generic-state/e1/valid/committed.json"
    ))
    .unwrap();
    let mut b = record.committed.take().unwrap();
    b.operation = BindingOperation::Adoption;
    b.phase = BindingPhase::Prepared;
    b.receipt_digest = None;
    for r in &mut b.resources {
        r.progress = ResourceProgress::NotStarted;
    }
    b.resources = donor
        .resources()
        .iter()
        .map(|(path, digest)| Resource {
            scope: ResourceScope::Project,
            destination: path.clone(),
            key: Vec::new(),
            operation: ResourceOperation::File,
            desired: Value::Present(Box::new(ValuePresent {
                blob: Blob {
                    digest: digest.clone(),
                    bytes: std::fs::read(project.path().join(path)).unwrap().len() as u32,
                    mode: 0o600,
                },
            })),
            before: Before::Unavailable(Box::new(BeforeUnavailable {
                reason: BeforeUnavailableReason::LegacyNoBeforeState,
            })),
            progress: ResourceProgress::NotStarted,
        })
        .collect();
    let digest = format!("{:x}", Sha256::digest(&original));
    b.transfer = Some(Transfer {
        family: TransferFamily::PackageSkill,
        donor_id: donor.id().into(),
        donor_root_identity: donor.root_identity().into(),
        receipt_digest: digest.clone(),
        hold_id: "a".repeat(64),
        phase: TransferPhase::Held,
        receipt_blob: Blob {
            digest,
            bytes: original.len() as u32,
            mode: 0o600,
        },
        resources: b.resources.clone(),
    });
    b.semantics_digest = vibe_wire::behaviour::deploy_generic_state::semantics_digest(&b).unwrap();
    record.pending = Some(b);
    let _lease = donor.lock().unwrap();
    donor.hold(&record).unwrap();
    donor.retire(&"a".repeat(64)).unwrap();
    drop(_lease);
    assert!(crate::pkgskill::reconcile_project_skill_binding(project.path(), &binding).is_err());
    let resumed = SkillDonor::resume(Project::open(project.path()).unwrap()).unwrap();
    assert!(resumed.retire(&"b".repeat(64)).is_err());
    resumed.restore(&"a".repeat(64)).unwrap();
    assert_eq!(
        std::fs::read(project.path().join(".vibe/package-skills.toml")).unwrap(),
        original
    );
    crate::pkgskill::reconcile_project_skill_binding(project.path(), &binding).unwrap();
}

#[test]
fn changed_donor_receipt_and_foreign_equal_bytes_refuse() {
    use super::adoption::SkillDonor;
    use super::tests::{lower, provider, seed};
    let project = tempfile::tempdir().unwrap();
    let source = seed(project.path(), "demo");
    let binding = lower(
        project.path(),
        vec![provider(source.path(), "pkg", "demo", &["claude"])],
    )[0]
    .clone();
    assert!(
        SkillDonor::automatic(Project::open(project.path()).unwrap(), &binding.identity()).is_err()
    );
    crate::pkgskill::reconcile_project_skill_binding(project.path(), &binding).unwrap();
    let donor =
        SkillDonor::automatic(Project::open(project.path()).unwrap(), &binding.identity()).unwrap();
    std::fs::write(
        project.path().join(".vibe/package-skills.toml"),
        b"schema = 99\n",
    )
    .unwrap();
    assert!(donor.validate().is_err());
    assert!(
        SkillDonor::automatic(Project::open(project.path()).unwrap(), &binding.identity()).is_err()
    );
}

#[test]
fn current_standalone_writer_waits_for_shared_arena() {
    use crate::agents::{Agent, Scope};
    use std::sync::mpsc;
    let project = tempfile::tempdir().unwrap();
    let arena = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(
        source.path().join("SKILL.md"),
        b"---\nname: demo\ndescription: fixture\n---\nbody\n",
    )
    .unwrap();
    let lock = Project::open(arena.path())
        .unwrap()
        .lock("deploy-physical-coordination.lock")
        .unwrap();
    let target = project.path().to_path_buf();
    let input = source.path().to_path_buf();
    let capability = Project::open(arena.path()).unwrap();
    let (ready, started) = mpsc::channel();
    let (done, completed) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let writer = CoordinatedSkills::new(capability).unwrap();
        ready.send(()).unwrap();
        let result = writer.install(
            Agent::parse_filter("claude").unwrap()[0],
            Scope::Project,
            Some(&target),
            "demo",
            &input,
            &[],
            false,
        );
        done.send(result.is_ok()).unwrap();
    });
    started.recv().unwrap();
    assert!(
        completed
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err()
    );
    assert!(!project.path().join(".claude/skills/demo/SKILL.md").exists());
    drop(lock);
    assert!(
        completed
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
    );
    worker.join().unwrap();
}
