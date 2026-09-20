use super::*;
use crate::commands::application::distribution::{
    BundleFile, BundleLauncher, BundleManagement, BundleManifest, VerifiedDistribution,
};
use crate::commands::application::model::{ApplicationIdentity, PackageIdentity};
use tempfile::tempdir;

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn verified(root: &Path, asset: char) -> VerifiedDistribution {
    let payload = root.join(format!("staged-{asset}"));
    fs::create_dir_all(payload.join("management")).unwrap();
    fs::create_dir_all(payload.join("launchers")).unwrap();
    let rows = [
        ("management/launch.cmd", b"@echo management\r\n".as_slice()),
        ("launchers/demo.cmd", b"@echo cmd\r\n".as_slice()),
        ("launchers/demo.ps1", b"Write-Output ps1\r\n".as_slice()),
    ];
    for &(path, bytes) in &rows {
        fs::write(payload.join(path), bytes).unwrap();
    }
    let files = rows
        .iter()
        .map(|(path, bytes)| BundleFile {
            path: (*path).into(),
            sha256: digest(bytes),
            size: bytes.len() as u64,
        })
        .collect();
    VerifiedDistribution {
        manifest: BundleManifest {
            protocol: "vibe-application-distribution/1".into(),
            application: ApplicationIdentity {
                id: "demo".into(),
                package: PackageIdentity {
                    group: "org.example".into(),
                    name: "demo".into(),
                    version: "1.0.0".into(),
                },
                installer_package: PackageIdentity {
                    group: "org.example".into(),
                    name: "installer".into(),
                    version: "1.0.0".into(),
                },
                commands: vec!["demo".into()],
            },
            os: "windows".into(),
            arch: "x86_64".into(),
            libc: None,
            source_commit: "a".repeat(40),
            source_tree: format!("sha256-tree/1:{}", "b".repeat(64)),
            management: BundleManagement {
                runtime: "builtin".into(),
                entry: "management/launch.cmd".into(),
            },
            launchers: vec![
                BundleLauncher {
                    command: "demo".into(),
                    path: "launchers/demo.cmd".into(),
                    destination: "demo.cmd".into(),
                },
                BundleLauncher {
                    command: "demo".into(),
                    path: "launchers/demo.ps1".into(),
                    destination: "demo.ps1".into(),
                },
            ],
            files,
        },
        payload_root: payload,
        asset_sha256: asset.to_string().repeat(64),
    }
}

fn verified_posix(root: &Path) -> VerifiedDistribution {
    let payload = root.join("staged-posix");
    fs::create_dir_all(payload.join("management")).unwrap();
    fs::create_dir_all(payload.join("launchers")).unwrap();
    let rows = [
        ("management/launch.sh", b"#!/bin/sh\nexit 0\n".as_slice()),
        ("launchers/demo", b"#!/bin/sh\nexit 0\n".as_slice()),
    ];
    for &(path, bytes) in &rows {
        fs::write(payload.join(path), bytes).unwrap();
    }
    VerifiedDistribution {
        manifest: BundleManifest {
            protocol: "vibe-application-distribution/1".into(),
            application: ApplicationIdentity {
                id: "demo".into(),
                package: PackageIdentity {
                    group: "org.example".into(),
                    name: "demo".into(),
                    version: "1.0.0".into(),
                },
                installer_package: PackageIdentity {
                    group: "org.example".into(),
                    name: "installer".into(),
                    version: "1.0.0".into(),
                },
                commands: vec!["demo".into()],
            },
            os: "linux".into(),
            arch: "x86_64".into(),
            libc: Some("gnu".into()),
            source_commit: "a".repeat(40),
            source_tree: format!("sha256-tree/1:{}", "b".repeat(64)),
            management: BundleManagement {
                runtime: "builtin".into(),
                entry: "management/launch.sh".into(),
            },
            launchers: vec![BundleLauncher {
                command: "demo".into(),
                path: "launchers/demo".into(),
                destination: "demo".into(),
            }],
            files: rows
                .iter()
                .map(|(path, bytes)| BundleFile {
                    path: (*path).into(),
                    sha256: digest(bytes),
                    size: bytes.len() as u64,
                })
                .collect(),
        },
        payload_root: payload,
        asset_sha256: "f".repeat(64),
    }
}

#[test]
fn paired_extensions_publish_independently_and_rollback_together() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let publication = publish(&settings, &host, verified(temp.path(), 'c'), None, &[]).unwrap();
    assert!(settings.join("opt/bin/demo.cmd").is_file());
    assert!(settings.join("opt/bin/demo.ps1").is_file());
    publication.rollback().unwrap();
    assert!(!settings.join("opt/bin/demo.cmd").exists());
    assert!(!settings.join("opt/bin/demo.ps1").exists());
}

#[test]
fn posix_bundle_publishes_extensionless_command_and_shell_management_launcher() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let publication = publish(&settings, &host, verified_posix(temp.path()), None, &[]).unwrap();
    publication.commit();
    assert!(settings.join("opt/bin/demo").is_file());
    let stable = host.join("management/launch.sh");
    assert!(stable.is_file());
    let body = String::from_utf8(fs::read(stable).unwrap()).unwrap();
    assert!(body.starts_with("#!/bin/sh\nexec '"), "{body}");
}

#[test]
fn source_owned_different_launcher_bytes_transition_to_binary_ownership() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    fs::create_dir_all(settings.join("opt/bin")).unwrap();
    let cmd = settings.join("opt/bin/demo.cmd");
    let ps1 = settings.join("opt/bin/demo.ps1");
    let legacy = settings.join("opt/bin/demo-legacy.cmd");
    fs::write(&cmd, b"@echo source\r\n").unwrap();
    fs::write(&ps1, b"Write-Output source\r\n").unwrap();
    fs::write(&legacy, b"@echo legacy\r\n").unwrap();
    let prior = vec![
        ApplicationLauncherOwnership {
            destination: cmd.clone(),
            sha256: hash_file(&cmd).unwrap(),
        },
        ApplicationLauncherOwnership {
            destination: ps1.clone(),
            sha256: hash_file(&ps1).unwrap(),
        },
        ApplicationLauncherOwnership {
            destination: legacy.clone(),
            sha256: hash_file(&legacy).unwrap(),
        },
    ];
    let publication = publish(&settings, &host, verified(temp.path(), 'd'), None, &prior).unwrap();
    publication.commit();
    assert!(host.join("management/binary.json").is_file());
    assert_eq!(fs::read(cmd).unwrap(), b"@echo cmd\r\n");
    assert_eq!(fs::read(ps1).unwrap(), b"Write-Output ps1\r\n");
    assert!(!legacy.exists());
}

#[test]
fn tampered_receipt_cannot_remove_an_outside_same_hash_file() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let publication = publish(&settings, &host, verified(temp.path(), 'e'), None, &[]).unwrap();
    let management = publication.management();
    publication.commit();
    let outside = temp.path().join("outside.cmd");
    fs::write(&outside, b"@echo cmd\r\n").unwrap();
    let mut receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(&management.entry).unwrap()).unwrap();
    receipt["launchers"][0]["destination"] =
        serde_json::Value::String(outside.display().to_string());
    fs::write(&management.entry, serde_json::to_vec(&receipt).unwrap()).unwrap();
    let error = uninstall(&management).unwrap_err().to_string();
    assert!(error.contains("invalid launcher destination"), "{error}");
    assert!(outside.is_file());
}

#[test]
fn failed_index_commit_after_source_transition_restores_binary_bytes() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let publication = publish(&settings, &host, verified(temp.path(), 'f'), None, &[]).unwrap();
    let management = publication.management();
    publication.commit();
    let original_cmd = fs::read(settings.join("opt/bin/demo.cmd")).unwrap();
    let original_receipt = fs::read(&management.entry).unwrap();
    let suspended = suspend(&host, &management).unwrap();

    let cmd = settings.join("opt/bin/demo.cmd");
    let ps1 = settings.join("opt/bin/demo.ps1");
    fs::create_dir_all(host.join("management")).unwrap();
    fs::write(&cmd, b"@echo source\r\n").unwrap();
    fs::write(&ps1, b"Write-Output source\r\n").unwrap();
    fs::write(host.join("management/binary.json"), b"source replacement").unwrap();
    fs::write(host.join("management/launch.cmd"), b"source replacement").unwrap();
    let source = vec![
        ApplicationLauncherOwnership {
            destination: cmd.clone(),
            sha256: hash_file(&cmd).unwrap(),
        },
        ApplicationLauncherOwnership {
            destination: ps1.clone(),
            sha256: hash_file(&ps1).unwrap(),
        },
    ];
    suspended.rollback_after_source(&source).unwrap();
    assert_eq!(fs::read(cmd).unwrap(), original_cmd);
    assert_eq!(fs::read(management.entry).unwrap(), original_receipt);
}

#[test]
fn successful_source_transition_gets_a_clean_host_and_retires_binary_backup() {
    let temp = tempdir().unwrap();
    let settings = temp.path().join("settings");
    let host = settings.join("opt/apps/demo");
    let publication = publish(&settings, &host, verified(temp.path(), 'g'), None, &[]).unwrap();
    let management = publication.management();
    publication.commit();

    let suspended = suspend(&host, &management).unwrap();
    assert!(
        !host.exists(),
        "the source updater receives a clean host path"
    );
    fs::create_dir_all(&host).unwrap();
    fs::write(host.join("source-owner.json"), b"owned by source").unwrap();
    suspended.commit().unwrap();

    assert_eq!(
        fs::read(host.join("source-owner.json")).unwrap(),
        b"owned by source"
    );
    assert!(!settings.join("opt/bin/demo.cmd").exists());
    assert!(
        fs::read_dir(settings.join("opt/apps"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("suspended"))
    );
}
