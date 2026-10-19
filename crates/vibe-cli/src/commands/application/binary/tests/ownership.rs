use super::*;

#[test]
fn native_and_batch_variants_keep_exact_ownership_through_drift_and_uninstall() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let mut distribution = verified(temp.path(), 'a');
    for name in ["demo.exe", "demo.bat"] {
        let relative = format!("launchers/{name}");
        let bytes = format!("owned {name} variant").into_bytes();
        fs::write(distribution.payload_root.join(&relative), &bytes).unwrap();
        distribution.manifest.files.push(BundleFile {
            path: relative.clone(),
            sha256: digest(&bytes),
            size: bytes.len() as u64,
        });
        distribution.manifest.launchers.push(BundleLauncher {
            command: "demo".into(),
            path: relative,
            destination: name.into(),
        });
    }
    let publication = publish(&settings, &host, distribution, None, &[], false).unwrap();
    let management = publication.management();
    assert_eq!(publication.launchers().len(), 4);
    publication.commit();
    let executable = settings.join("opt/bin/demo.exe");
    let batch = settings.join("opt/bin/demo.bat");
    let owned = fs::read(&executable).unwrap();
    let outside = temp.path().join("outsider.exe");
    fs::write(&outside, &owned).unwrap();
    fs::write(&executable, b"foreign edit").unwrap();
    assert!(
        prepare_uninstall(&management, false)
            .unwrap_err()
            .to_string()
            .contains("drifted")
    );
    assert!(
        batch.is_file(),
        "drift refuses before any owned variant is removed"
    );
    fs::write(&executable, owned).unwrap();
    prepare_uninstall(&management, false)
        .unwrap()
        .1
        .cleanup()
        .unwrap();
    assert!(
        !host
            .join(format!("generations/{}", "a".repeat(64)))
            .exists()
    );
    for name in ["demo.cmd", "demo.ps1", "demo.exe", "demo.bat"] {
        assert!(!settings.join("opt/bin").join(name).exists());
    }
    assert!(
        outside.is_file(),
        "a same-hash outside file is never removed"
    );
}

#[test]
fn force_replaces_drifted_owned_runner_and_generation_with_rollback() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let initial = publish(
        &settings,
        &host,
        verified(temp.path(), 'a'),
        None,
        &[],
        false,
    )
    .unwrap();
    let management = initial.management();
    initial.commit();
    let runner = settings.join("opt/bin/demo.cmd");
    let generation_file = host.join(format!("generations/{}/launchers/demo.cmd", "a".repeat(64)));
    fs::write(&runner, b"local runner edit").unwrap();
    fs::write(&generation_file, b"local generation edit").unwrap();
    let staged = verified(temp.path(), 'a');
    assert!(publish(&settings, &host, staged, Some(&management), &[], false).is_err());
    let staged = verified(temp.path(), 'a');
    let replacement = publish(&settings, &host, staged, Some(&management), &[], true).unwrap();
    let replacement_root = read_management(&replacement.management())
        .unwrap()
        .payload_root;
    assert_eq!(fs::read(&runner).unwrap(), b"@echo cmd\r\n");
    assert_eq!(
        fs::read(replacement_root.join("launchers/demo.cmd")).unwrap(),
        b"@echo cmd\r\n"
    );
    assert_eq!(
        fs::read(&generation_file).unwrap(),
        b"local generation edit"
    );
    replacement.rollback().unwrap();
    assert_eq!(fs::read(&runner).unwrap(), b"local runner edit");
    assert_eq!(
        fs::read(&generation_file).unwrap(),
        b"local generation edit"
    );
    assert!(management.entry.is_file());
}

#[test]
fn force_uninstall_removes_only_drifted_receipt_owned_runners() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let initial = publish(
        &settings,
        &host,
        verified(temp.path(), 'b'),
        None,
        &[],
        false,
    )
    .unwrap();
    let management = initial.management();
    initial.commit();
    let runner = settings.join("opt/bin/demo.cmd");
    let unowned = settings.join("opt/bin/unowned.cmd");
    fs::write(&runner, b"local edit").unwrap();
    fs::write(&unowned, b"unowned").unwrap();
    assert!(prepare_uninstall(&management, false).is_err());
    prepare_uninstall(&management, true)
        .unwrap()
        .1
        .cleanup()
        .unwrap();
    assert!(!runner.exists());
    assert!(!settings.join("opt/bin/demo.ps1").exists());
    assert!(
        !host
            .join(format!("generations/{}", "b".repeat(64)))
            .exists()
    );
    assert_eq!(fs::read(unowned).unwrap(), b"unowned");
}

#[test]
fn prepared_uninstall_is_side_effect_free_until_cleanup() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let publication = publish(
        &settings,
        &host,
        verified(temp.path(), 'e'),
        None,
        &[],
        false,
    )
    .unwrap();
    let management = publication.management();
    publication.commit();
    let runner = settings.join("opt/bin/demo.cmd");
    let payload = host.join(format!("generations/{}/launchers/demo.cmd", "e".repeat(64)));
    fs::write(&runner, b"locally changed runner").unwrap();
    let (_, prepared) = prepare_uninstall(&management, true).unwrap();
    assert_eq!(fs::read(&runner).unwrap(), b"locally changed runner");
    assert_eq!(fs::read(&payload).unwrap(), b"@echo cmd\r\n");
    assert!(management.entry.is_file());
    prepared.cleanup().unwrap();
    assert!(!runner.exists());
    assert!(!payload.exists());
}

#[test]
fn force_never_overwrites_unowned_runner_destination() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let initial = publish(
        &settings,
        &host,
        verified(temp.path(), 'c'),
        None,
        &[],
        false,
    )
    .unwrap();
    let management = initial.management();
    initial.commit();
    let unowned = settings.join("opt/bin/demo.bat");
    fs::write(&unowned, b"outside owner").unwrap();
    let mut staged = verified(temp.path(), 'd');
    let relative = "launchers/demo.bat";
    let bytes = b"@echo new variant\r\n";
    fs::write(staged.payload_root.join(relative), bytes).unwrap();
    staged.manifest.files.push(BundleFile {
        path: relative.into(),
        sha256: digest(bytes),
        size: bytes.len() as u64,
    });
    staged.manifest.launchers.push(BundleLauncher {
        command: "demo".into(),
        path: relative.into(),
        destination: "demo.bat".into(),
    });
    assert!(publish(&settings, &host, staged, Some(&management), &[], true).is_err());
    assert_eq!(fs::read(unowned).unwrap(), b"outside owner");
    assert!(management.entry.is_file());
    assert!(
        !host
            .join(format!("generations/{}", "d".repeat(64)))
            .exists()
    );
}

#[test]
fn versioned_provider_keeps_runners_local_and_validates_nested_receipt() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo/org.example/package/1.0.0");
    fs::create_dir_all(settings.join("opt/bin")).unwrap();
    let slot_alias = settings.join("opt/bin/demo.cmd");
    fs::write(&slot_alias, b"slot-owned alias").unwrap();
    let publication = publish(
        &settings,
        &host,
        verified(temp.path(), 'a'),
        None,
        &[],
        false,
    )
    .unwrap();
    let management = publication.management();
    let runners = publication.provider_runners();
    assert_eq!(runners.len(), 2);
    assert_eq!(runners[0].name, "demo.cmd");
    assert_eq!(runners[0].logical_name, "demo");
    assert_eq!(
        runners[0].relative_path,
        PathBuf::from("launchers/demo.cmd")
    );
    assert!(publication.launchers().is_empty());
    publication.commit();
    assert_eq!(fs::read(&slot_alias).unwrap(), b"slot-owned alias");
    assert_eq!(
        payload_root(&management).unwrap(),
        host.join(format!("generations/{}", "a".repeat(64)))
    );
    let (_, removal) = prepare_uninstall(&management, false).unwrap();
    removal.cleanup().unwrap();
    assert_eq!(fs::read(slot_alias).unwrap(), b"slot-owned alias");
}

#[test]
fn nested_direct_archive_keeps_windows_runner_inside_payload() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo/org.example/package/1.0.0");
    let mut distribution = verified(temp.path(), 'f');
    distribution.direct_archive = true;
    let publication = publish(&settings, &host, distribution, None, &[], false).unwrap();
    assert!(publication.launchers().is_empty());
    assert_eq!(publication.provider_runners()[0].logical_name, "demo");
    publication.commit();
    assert!(!settings.join("opt/bin/demo.cmd").exists());
}

#[test]
fn versioned_provider_update_accepts_prior_receipt_from_sibling_version() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let slot = settings.join("opt/apps/demo/org.example/package");
    let prior_host = slot.join("1.0.0");
    let next_host = slot.join("1.1.0");
    let prior = publish(
        &settings,
        &prior_host,
        verified(temp.path(), 'b'),
        None,
        &[],
        false,
    )
    .unwrap();
    let management = prior.management();
    prior.commit();
    let next = publish(
        &settings,
        &next_host,
        verified(temp.path(), 'c'),
        Some(&management),
        &[],
        false,
    )
    .unwrap();
    assert!(next.launchers().is_empty());
    assert_eq!(next.provider_runners().len(), 2);
    next.commit();
    assert!(payload_root(&management).unwrap().is_dir());
}
