//! Real config/receipt writers under deterministic temp-root failure cuts.
use super::*;

#[cfg(test)]
fn replace_same_bytes(path: &Path) {
    let bytes = fs::read(path).unwrap();
    let replacement = path.with_extension("foreign-copy");
    fs::write(&replacement, bytes).unwrap();
    fs::set_permissions(&replacement, fs::metadata(path).unwrap().permissions()).unwrap();
    fs::rename(&replacement, path).unwrap();
}

#[cfg(test)]
fn observed(path: &Path) -> vibe_safefs::StableFileSnapshot {
    vibe_safefs::Project::open(path.parent().unwrap())
        .unwrap()
        .read_file_snapshot_bounded(path.file_name().unwrap().to_str().unwrap(), 1 << 20)
        .unwrap()
        .unwrap()
}

#[cfg(test)]
fn update_first(batch: &mut PublicationBatch, config: &Path) -> Result<()> {
    let mut server = remote();
    server.version = "2.0.0".into();
    server.decl.url = Some("https://example.test/new".into());
    let payload = server_payload(Agent::ClaudeCode, config.parent().unwrap(), &server, true)?;
    register::register_requested_in(
        Agent::ClaudeCode,
        Scope::User,
        config,
        &server,
        &payload,
        None,
        Mutation {
            dry_run: false,
            batch,
        },
    )?;
    Ok(())
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn matching_owned_publication_rolls_back_both_files_and_mode() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let server = remote();
    let payload = server_payload(Agent::ClaudeCode, root.path(), &server, true).unwrap();
    register_one(
        Agent::ClaudeCode,
        Scope::User,
        &config,
        &server,
        &payload,
        false,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&config, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let before = observed(&config);
    let receipt_before = fs::read(receipt_path(&config).unwrap()).unwrap();
    let mut batch = PublicationBatch::default();
    update_first(&mut batch, &config).unwrap();
    batch.rollback().unwrap();
    let after = observed(&config);
    assert_eq!(after.bytes, before.bytes);
    assert_eq!(after.unix_mode, before.unix_mode);
    assert_eq!(
        fs::read(receipt_path(&config).unwrap()).unwrap(),
        receipt_before
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn same_byte_foreign_replacement_blocks_whole_config_receipt_inverse() {
    for replace_receipt in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("first.json");
        let receipt = receipt_path(&config).unwrap();
        let server = remote();
        let payload = server_payload(Agent::ClaudeCode, root.path(), &server, true).unwrap();
        register_one(
            Agent::ClaudeCode,
            Scope::User,
            &config,
            &server,
            &payload,
            false,
        )
        .unwrap();
        let mut batch = PublicationBatch::default();
        update_first(&mut batch, &config).unwrap();
        let owned_config = observed(&config);
        let owned_receipt = observed(&receipt);
        let foreign = if replace_receipt { &receipt } else { &config };
        replace_same_bytes(foreign);
        let planted = observed(foreign);
        assert_ne!(
            planted.identity,
            if replace_receipt {
                owned_receipt.identity
            } else {
                owned_config.identity
            }
        );
        let error = batch.failed(anyhow::anyhow!("later row failed"), "batch registration");
        assert!(error.to_string().contains("recovery required"));
        assert_eq!(observed(foreign), planted);
        assert_eq!(fs::read(&config).unwrap(), owned_config.bytes);
        assert_eq!(fs::read(&receipt).unwrap(), owned_receipt.bytes);
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn possibly_published_error_retains_original_stage_identity() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let server = remote();
    let payload = server_payload(Agent::ClaudeCode, root.path(), &server, true).unwrap();
    register_one(
        Agent::ClaudeCode,
        Scope::User,
        &config,
        &server,
        &payload,
        false,
    )
    .unwrap();
    let target = config.clone();
    let mut batch = PublicationBatch::default();
    batch.on_before_write(move |path| {
        if path == target {
            vibe_safefs::arm_before_publish_verify(Some(Box::new(|parent, name| {
                replace_same_bytes(&parent.join(name))
            })));
        }
        Ok(())
    });
    let error = update_first(&mut batch, &config).unwrap_err();
    vibe_safefs::arm_before_publish_verify(None);
    let foreign = observed(&config);
    let receipt = fs::read(receipt_path(&config).unwrap()).unwrap();
    let error = batch.failed(error, "batch registration");
    assert!(error.to_string().contains("recovery required"));
    assert_eq!(observed(&config), foreign);
    assert_eq!(fs::read(receipt_path(&config).unwrap()).unwrap(), receipt);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn foreign_absence_and_mode_are_not_permission_to_restore() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let mut batch = PublicationBatch::default();
    update_first(&mut batch, &config).unwrap();
    fs::remove_file(&config).unwrap();
    assert!(batch.rollback().is_err());
    assert!(!config.exists());
    assert!(receipt_path(&config).unwrap().exists());
}

#[cfg(unix)]
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn foreign_mode_change_blocks_undo_even_when_bytes_and_identity_match() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let mut batch = PublicationBatch::default();
    update_first(&mut batch, &config).unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o644)).unwrap();
    let foreign = observed(&config);
    assert!(batch.rollback().is_err());
    assert_eq!(observed(&config), foreign);
}
