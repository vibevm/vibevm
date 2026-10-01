use std::io::Read;

use super::*;
use vibe_safefs::StreamOutcome;

#[test]
fn complete_ready_vault_opens_only_exact_catalogue_asset_stream() {
    let world = World::new();
    assert!(matches!(
        world.stage().unwrap(),
        VaultInspection::Complete { .. }
    ));
    SubmissionReservations::mark_journal_ready(
        &world.lease,
        &world.namespace,
        &world.scope,
        "vault-key",
        &world.run_id,
    )
    .unwrap();
    let selected = &world.subject.targets[0].bundle;
    let mut stream = open_vault_asset_stream(
        &world.lease,
        &world.namespace,
        &world.scope,
        "vault-key",
        &world.run_id,
        &world.subject,
        &expected(),
        0,
        &selected.name,
    )
    .unwrap();
    assert_eq!(stream.name, selected.name);
    assert_eq!(stream.media_type, selected.media_type);
    assert_eq!(stream.size_bytes.to_string(), selected.size_bytes);
    assert_eq!(stream.sha256, selected.sha256);
    assert_eq!(stream.completion.outcome(), StreamOutcome::Incomplete);
    let mut bytes = Vec::new(); // The fixture payload is tiny; production uses a streaming body.
    stream.reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, payload(&selected.name));
    assert_eq!(
        stream.completion.outcome(),
        StreamOutcome::Complete {
            bytes: bytes.len() as u64,
            sha256: selected.sha256.strip_prefix("sha256:").unwrap().into(),
        }
    );
    for (index, name) in [(0, "foreign"), (19, selected.name.as_str())] {
        assert!(matches!(
            open_vault_asset_stream(
                &world.lease,
                &world.namespace,
                &world.scope,
                "vault-key",
                &world.run_id,
                &world.subject,
                &expected(),
                index,
                name,
            ),
            Err(VaultError::Refused(_))
        ));
    }
    let foreign = SubmissionScope {
        principal: "foreign".into(),
        ..world.scope.clone()
    };
    assert!(matches!(
        open_vault_asset_stream(
            &world.lease,
            &world.namespace,
            &foreign,
            "vault-key",
            &world.run_id,
            &world.subject,
            &expected(),
            0,
            &selected.name,
        ),
        Err(VaultError::Refused(_))
    ));
}

#[test]
fn pending_or_changed_vault_never_opens_stream() {
    let world = World::new();
    world.stage().unwrap();
    let name = &world.subject.targets[0].bundle.name;
    assert!(matches!(
        open_vault_asset_stream(
            &world.lease,
            &world.namespace,
            &world.scope,
            "vault-key",
            &world.run_id,
            &world.subject,
            &expected(),
            0,
            name,
        ),
        Err(VaultError::Refused(_))
    ));
    SubmissionReservations::mark_journal_ready(
        &world.lease,
        &world.namespace,
        &world.scope,
        "vault-key",
        &world.run_id,
    )
    .unwrap();
    let file = world.vault_path().join(name);
    let original = fs::read(&file).unwrap();
    fs::write(&file, b"changed").unwrap();
    assert!(matches!(
        open_vault_asset_stream(
            &world.lease,
            &world.namespace,
            &world.scope,
            "vault-key",
            &world.run_id,
            &world.subject,
            &expected(),
            0,
            name,
        ),
        Err(VaultError::RecoveryRequired(_))
    ));
    fs::write(&file, original).unwrap();
    fs::remove_file(world.vault_path().join(COMPLETE)).unwrap();
    assert!(matches!(
        open_vault_asset_stream(
            &world.lease,
            &world.namespace,
            &world.scope,
            "vault-key",
            &world.run_id,
            &world.subject,
            &expected(),
            0,
            name,
        ),
        Err(VaultError::RecoveryRequired(_))
    ));
}

#[test]
fn copied_same_path_root_cannot_open_stream() {
    let world = World::new();
    world.stage().unwrap();
    SubmissionReservations::mark_journal_ready(
        &world.lease,
        &world.namespace,
        &world.scope,
        "vault-key",
        &world.run_id,
    )
    .unwrap();
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
    drop(lease);
    let root = workspace.path().to_path_buf();
    let moved = root.with_extension("stream-original");
    fs::rename(&root, &moved).unwrap();
    copy_tree(&moved, &root);
    let lease = LifecycleLease::acquire(&root).unwrap();
    let name = &subject.targets[0].bundle.name;
    assert!(matches!(
        open_vault_asset_stream(
            &lease,
            &namespace,
            &scope,
            "vault-key",
            &run_id,
            &subject,
            &expected(),
            0,
            name,
        ),
        Err(VaultError::RecoveryRequired(_))
    ));
    drop(lease);
    fs::remove_dir_all(&root).unwrap();
    fs::rename(moved, root).unwrap();
}
