use std::fs;
use std::path::Path;

use super::*;
use vibe_lifecycle::ReservationDecision;
use vibe_lifecycle::process::propose_run_id;
use vibe_wire::generated::publication_subject::{PublicationAsset, PublicationSmokeProbeStatus};

const SUBJECT: &str = include_str!(
    "../../../../../formats/corpora/publication_subject/e1/valid/complete-five-targets.json"
);

fn expected() -> PublicationSubjectExpectations<'static> {
    PublicationSubjectExpectations {
        version: "1.1.0",
        repository: "vibevm/vibevm",
        smoke_policy_id: "distribution-smoke/e1",
        smoke_policy_epoch: 1,
    }
}

fn payload(name: &str) -> Vec<u8> {
    format!("exact payload: {name}\n").into_bytes()
}

fn set_asset(asset: &mut PublicationAsset, bytes: &[u8]) {
    asset.size_bytes = bytes.len().to_string();
    asset.sha256 = digest(bytes);
}

struct World {
    workspace: tempfile::TempDir,
    output: tempfile::TempDir,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    run_id: String,
    subject: PublicationSubject,
    finals: [Vec<u8>; 4],
}

impl World {
    fn new() -> Self {
        let workspace = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let mut subject: PublicationSubject = serde_json::from_str(SUBJECT).unwrap();
        for target in &mut subject.targets {
            for asset in [
                &mut target.bundle,
                &mut target.bootstrap,
                &mut target.fragment,
            ] {
                let bytes = payload(&asset.name);
                set_asset(asset, &bytes);
                fs::write(output.path().join(&asset.name), bytes).unwrap();
            }
            target.components[0].size_bytes = target.bootstrap.size_bytes.clone();
            target.components[0].sha256 = target.bootstrap.sha256.clone();
        }
        for (probe, target) in subject.smoke.probes.iter_mut().zip(&subject.targets) {
            if matches!(
                probe.status,
                PublicationSmokeProbeStatus::Passed | PublicationSmokeProbeStatus::Failed
            ) {
                probe.tested_sha256 = Some(target.bundle.sha256.clone());
            }
        }
        let finals = [
            payload(&subject.aggregate.name),
            payload(&subject.installers[0].asset.name),
            payload(&subject.installers[1].asset.name),
            payload(&subject.installers[2].asset.name),
        ];
        set_asset(&mut subject.aggregate, &finals[0]);
        for (installer, bytes) in subject.installers.iter_mut().zip(&finals[1..]) {
            set_asset(&mut installer.asset, bytes);
        }
        expected_upload_catalogue(&subject, &expected()).unwrap();
        let lease = LifecycleLease::acquire(workspace.path()).unwrap();
        let scope = SubmissionScope {
            context: "vault-fixture".into(),
            target: "github.com/vibevm/vibevm".into(),
            principal: "fixture-maintainer".into(),
        };
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let run_id = propose_run_id(lease.root()).unwrap();
        assert!(matches!(
            SubmissionReservations::reserve(
                &lease,
                &namespace,
                &scope,
                "vault-key",
                b"fixture-request",
                b"fixture-plan",
                &run_id,
            )
            .unwrap(),
            ReservationDecision::Created { .. }
        ));
        Self {
            workspace,
            output,
            lease,
            namespace,
            scope,
            run_id,
            subject,
            finals,
        }
    }

    fn output(&self) -> Project {
        Project::open(self.output.path()).unwrap()
    }
    fn finals(&self) -> FinalPayloadBytes<'_> {
        FinalPayloadBytes {
            aggregate: &self.finals[0],
            bash: &self.finals[1],
            powershell: &self.finals[2],
            cmd: &self.finals[3],
        }
    }
    fn stage(&self) -> Result<VaultInspection, VaultError> {
        stage_payload_vault(
            &self.lease,
            &self.namespace,
            &self.scope,
            "vault-key",
            &self.run_id,
            &self.subject,
            &expected(),
            &self.output(),
            &self.finals(),
        )
    }
    fn inspect(&self) -> VaultInspection {
        inspect_payload_vault(
            &self.lease,
            &self.namespace,
            &self.scope,
            "vault-key",
            &self.run_id,
            &self.subject,
            &expected(),
        )
    }
    fn vault_path(&self) -> std::path::PathBuf {
        self.workspace.path().join(format!(
            ".vibe/publication-operations/{}.payloads",
            self.run_id,
        ))
    }
    fn pending(&self) {
        let row = SubmissionReservations::inspect_existing_key(
            &self.lease,
            &self.namespace,
            &self.scope,
            "vault-key",
        )
        .unwrap()
        .unwrap();
        assert_eq!(row.journal_admission, JournalAdmission::Pending);
    }
}

#[test]
fn stages_nineteen_exact_files_then_complete_manifest_without_advancing_c1() {
    let world = World::new();
    let catalogue = expected_upload_catalogue(&world.subject, &expected()).unwrap();
    assert_eq!(
        world.inspect(),
        VaultInspection::Incomplete {
            run_id: world.run_id.clone(),
            verified_files: 0
        }
    );
    assert_eq!(
        world.stage().unwrap(),
        VaultInspection::Complete {
            run_id: world.run_id.clone(),
            catalogue_digest: catalogue.digest.clone(),
        }
    );
    assert_eq!(
        world.inspect(),
        VaultInspection::Complete {
            run_id: world.run_id.clone(),
            catalogue_digest: catalogue.digest,
        }
    );
    assert_eq!(fs::read_dir(world.vault_path()).unwrap().count(), 20);
    world.pending();
    let manifest = world.vault_path().join(COMPLETE);
    let mut noncanonical = fs::read(&manifest).unwrap();
    noncanonical.push(b' ');
    fs::write(manifest, noncanonical).unwrap();
    assert!(matches!(world.inspect(), VaultInspection::Uncertain { .. }));
}

#[test]
fn missing_or_changed_output_refuses_before_completion_and_c1_stays_pending() {
    let world = World::new();
    let missing = &world.subject.targets[0].bundle.name;
    fs::remove_file(world.output.path().join(missing)).unwrap();
    assert!(matches!(world.stage(), Err(VaultError::Refused(_))));
    assert!(!world.vault_path().exists());
    world.pending();

    let changed = World::new();
    let name = &changed.subject.targets[0].fragment.name;
    fs::write(changed.output.path().join(name), b"changed").unwrap();
    assert!(matches!(changed.stage(), Err(VaultError::Refused(_))));
    assert!(!changed.vault_path().exists());
    changed.pending();
}

#[test]
fn interrupted_or_occupied_vault_never_retries_or_replaces_residue() {
    let world = World::new();
    let result = stage_with_interrupt(
        &world.lease,
        &world.namespace,
        &world.scope,
        "vault-key",
        &world.run_id,
        &world.subject,
        &expected(),
        &world.output(),
        &world.finals(),
        Some(3),
        |_, _| {},
    );
    assert!(matches!(result, Err(VaultError::RecoveryRequired(_))));
    assert_eq!(
        world.inspect(),
        VaultInspection::Incomplete {
            run_id: world.run_id.clone(),
            verified_files: 3
        }
    );
    assert!(!world.vault_path().join(COMPLETE).exists());
    assert!(matches!(
        world.stage(),
        Err(VaultError::RecoveryRequired(_))
    ));
    world.pending();

    let occupied = World::new();
    fs::create_dir_all(occupied.vault_path()).unwrap();
    let name = &occupied.subject.targets[0].bundle.name;
    fs::write(occupied.vault_path().join(name), b"foreign occupant").unwrap();
    assert!(matches!(
        occupied.stage(),
        Err(VaultError::RecoveryRequired(_))
    ));
    assert_eq!(
        fs::read(occupied.vault_path().join(name)).unwrap(),
        b"foreign occupant"
    );
    assert!(!occupied.vault_path().join(COMPLETE).exists());
    occupied.pending();
}

#[test]
fn early_staged_byte_drift_cannot_be_hidden_by_completion_readback() {
    let world = World::new();
    let early = world.subject.targets[0].bundle.name.clone();
    let result = stage_with_interrupt(
        &world.lease,
        &world.namespace,
        &world.scope,
        "vault-key",
        &world.run_id,
        &world.subject,
        &expected(),
        &world.output(),
        &world.finals(),
        None,
        |index, vault| {
            if index == 3 {
                fs::write(vault.root_path().join(&early), b"late drift").unwrap();
            }
        },
    );
    assert!(matches!(result, Err(VaultError::RecoveryRequired(_))));
    assert!(world.vault_path().join(COMPLETE).exists());
    assert!(matches!(world.inspect(), VaultInspection::Uncertain { .. }));
    world.pending();
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

#[test]
fn copied_same_path_root_cannot_inspect_as_complete() {
    let world = World::new();
    world.stage().unwrap();
    let World {
        workspace,
        output: _,
        lease,
        namespace,
        scope,
        run_id,
        subject,
        finals: _,
    } = world;
    let root = workspace.path().to_path_buf();
    let original_token = lease
        .publication_journal_project()
        .root_correlation_token()
        .unwrap();
    drop(lease);
    let moved = root.with_extension("original");
    fs::rename(&root, &moved).unwrap();
    copy_tree(&moved, &root);
    let lease = LifecycleLease::acquire(&root).unwrap();
    assert_ne!(
        lease
            .publication_journal_project()
            .root_correlation_token()
            .unwrap(),
        original_token
    );
    assert!(matches!(
        inspect_payload_vault(
            &lease,
            &namespace,
            &scope,
            "vault-key",
            &run_id,
            &subject,
            &expected(),
        ),
        VaultInspection::Uncertain { .. }
    ));
}
