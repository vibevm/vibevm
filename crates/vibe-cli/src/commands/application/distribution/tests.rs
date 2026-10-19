use super::*;
use tempfile::tempdir;
use vibe_core::manifest::ArchiveRunnerDecl;

fn identity() -> ApplicationIdentity {
    ApplicationIdentity {
        id: "demo".into(),
        package: super::super::model::PackageIdentity {
            group: "org.example".into(),
            name: "demo".into(),
            version: "1.0.0".into(),
        },
        installer_package: super::super::model::PackageIdentity {
            group: "org.example".into(),
            name: "demo".into(),
            version: "1.0.0".into(),
        },
        commands: vec!["demo".into()],
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn native_installer_file_requires_exact_sha256() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("installer.exe");
    fs::write(&source, b"native installer").unwrap();
    let destination = temp.path().join("verified.exe");
    let source = source.to_str().unwrap();
    assert!(
        verify_installer_file(
            source,
            &digest(b"different"),
            &destination,
            true,
            &Progress::default(),
        )
        .is_err()
    );
    assert!(!destination.exists());
    verify_installer_file(
        source,
        &digest(b"native installer"),
        &destination,
        true,
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(fs::read(destination).unwrap(), b"native installer");
}

#[test]
fn provider_local_archive_accepts_windows_runner_without_legacy_command_coverage() {
    let temp = tempdir().unwrap();
    let archive_path = temp.path().join("native.zip");
    let zip = File::create(&archive_path).unwrap();
    let mut zip = zip::ZipWriter::new(zip);
    zip.start_file("bin/demo.exe", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"native payload").unwrap();
    zip.finish().unwrap();
    let runner = ArchiveRunnerDecl {
        path: PathBuf::from("bin/demo.exe"),
        name: "demo.exe".into(),
    };
    let mut selected = identity();
    selected.commands = vec!["different".into()];
    let verified = verify_direct_archive(
        archive_path.to_str().unwrap(),
        DirectArchiveFormat::Zip,
        &digest(&fs::read(&archive_path).unwrap()),
        None,
        &[runner],
        &selected,
        &temp.path().join("staged"),
        true,
        &Progress::default(),
        true,
    )
    .unwrap();
    assert_eq!(verified.manifest.launchers[0].command, "demo");
    assert_eq!(verified.manifest.launchers[0].destination, "demo.exe");
}

#[cfg(not(windows))]
#[test]
fn direct_zip_and_tar_gz_publish_verified_payload() {
    let temp = tempdir().unwrap();
    let runner = ArchiveRunnerDecl {
        path: PathBuf::from("bin/demo"),
        name: "demo".into(),
    };
    let zip_path = temp.path().join("demo.zip");
    let zip = File::create(&zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(zip);
    zip.start_file("bin/demo", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"demo executable").unwrap();
    zip.finish().unwrap();

    let tar_path = temp.path().join("demo.tar.gz");
    let tar = File::create(&tar_path).unwrap();
    let encoder = flate2::write::GzEncoder::new(tar, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(b"demo executable".len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    tar.append_data(&mut header, "bin/demo", &b"demo executable"[..])
        .unwrap();
    tar.finish().unwrap();
    tar.into_inner().unwrap().finish().unwrap();

    for (format, archive, name) in [
        (DirectArchiveFormat::Zip, &zip_path, "zip"),
        (DirectArchiveFormat::TarGz, &tar_path, "tar"),
    ] {
        let staging = temp.path().join(name);
        let verified = verify_direct_archive(
            archive.to_str().unwrap(),
            format,
            &digest(&fs::read(archive).unwrap()),
            None,
            std::slice::from_ref(&runner),
            &identity(),
            &staging,
            true,
            &Progress::default(),
            false,
        )
        .unwrap();
        assert!(verified.direct_archive);
        assert_eq!(verified.manifest.launchers[0].destination, "demo");
        assert_eq!(
            fs::read(staging.join("bin/demo")).unwrap(),
            b"demo executable"
        );
    }
}

#[test]
fn archive_paths_reject_traversal_and_windows_unsafe_names() {
    for path in ["../outside", "/absolute", "C:/absolute", "aux", "a\\b"] {
        assert!(portable_path(path).is_err(), "{path}");
    }
}
