//! Archive extraction: plain entries, refusals and symbolic links.

use super::*;
use std::io::Write;
use zip::write::SimpleFileOptions;

fn zip_with(entries: &[(&str, &[u8])]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("payload.zip");
    let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
    for (name, bytes) in entries {
        zip.start_file(*name, SimpleFileOptions::default().unix_permissions(0o755))
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
    (dir, path)
}

#[test]
fn zip_payload_root_is_selected() {
    let (dir, archive) = zip_with(&[("app/bin/tool", b"tool"), ("app/README", b"r")]);
    let destination = dir.path().join("out");
    extract(
        &archive,
        ArchiveFormat::Zip,
        Some(Path::new("app")),
        &destination,
    )
    .unwrap();
    assert_eq!(fs::read(destination.join("bin/tool")).unwrap(), b"tool");
    assert!(!dir.path().join("out.unpack").exists());
}

#[test]
fn unsafe_entries_are_refused() {
    for entries in [
        vec![("../escape", b"x".as_slice())],
        vec![("a/CON", b"x".as_slice())],
        vec![
            ("Bin/tool", b"x".as_slice()),
            ("bin/other", b"y".as_slice()),
        ],
        vec![("c:/x", b"x".as_slice())],
    ] {
        let (dir, archive) = zip_with(&entries);
        let destination = dir.path().join("out");
        assert!(
            extract(&archive, ArchiveFormat::Zip, None, &destination).is_err(),
            "{entries:?}"
        );
        assert!(!destination.exists());
    }
}

#[test]
fn tar_gz_links_are_refused_and_files_extract() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("payload.tar.gz");
    let encoder =
        flate2::write::GzEncoder::new(File::create(&path).unwrap(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o755);
    header.set_cksum();
    tar.append_data(&mut header, "bin/tool", b"tool".as_slice())
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    let destination = dir.path().join("out");
    extract(&path, ArchiveFormat::TarGz, None, &destination).unwrap();
    assert_eq!(fs::read(destination.join("bin/tool")).unwrap(), b"tool");

    let linked = dir.path().join("link.tar.gz");
    let encoder =
        flate2::write::GzEncoder::new(File::create(&linked).unwrap(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_cksum();
    tar.append_link(&mut header, "bin/evil", "/etc/passwd")
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    assert!(
        extract(
            &linked,
            ArchiveFormat::TarGz,
            None,
            &dir.path().join("out2")
        )
        .is_err()
    );

    // A PAX global header, as GitHub's tarballs carry, is metadata and
    // never refused as a special entry.
    let pax = dir.path().join("pax.tar.gz");
    let encoder =
        flate2::write::GzEncoder::new(File::create(&pax).unwrap(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);
    let mut header = tar::Header::new_ustar();
    header.set_entry_type(tar::EntryType::XGlobalHeader);
    header.set_size(20);
    header.set_cksum();
    tar.append_data(
        &mut header,
        "pax_global_header",
        b"52 comment=abcdef12\n".as_slice(),
    )
    .unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "repo-1.0/README", b"read".as_slice())
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    let out3 = dir.path().join("out3");
    extract(&pax, ArchiveFormat::TarGz, None, &out3).unwrap();
    assert_eq!(fs::read(out3.join("repo-1.0/README")).unwrap(), b"read");
}

/// One tar.gz entry: a file with contents, or a symbolic link.
enum Entry<'a> {
    File(&'a str, &'a [u8]),
    Link(&'a str, &'a str),
}

fn tar_with(dir: &Path, entries: &[Entry<'_>]) -> PathBuf {
    let path = dir.join(format!("payload-{}.tar.gz", entries.len()));
    let encoder =
        flate2::write::GzEncoder::new(File::create(&path).unwrap(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);
    for entry in entries {
        let mut header = tar::Header::new_gnu();
        match entry {
            Entry::File(name, bytes) => {
                header.set_size(bytes.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                tar.append_data(&mut header, name, *bytes).unwrap();
            }
            Entry::Link(name, target) => {
                header.set_entry_type(tar::EntryType::Symlink);
                header.set_size(0);
                header.set_cksum();
                tar.append_link(&mut header, name, target).unwrap();
            }
        }
    }
    tar.into_inner().unwrap().finish().unwrap();
    path
}

/// The Node.js shape: `bin/npm -> ../lib/node_modules/npm/bin/npm-cli.js`.
fn node_like(dir: &Path) -> PathBuf {
    tar_with(
        dir,
        &[
            Entry::File("node-v1/bin/node", b"node"),
            Entry::File("node-v1/lib/node_modules/npm/bin/npm-cli.js", b"npm"),
            Entry::Link("node-v1/bin/npm", "../lib/node_modules/npm/bin/npm-cli.js"),
            Entry::Link("node-v1/bin/npm-again", "./npm"),
            Entry::Link("node-v1/share", "lib/node_modules"),
            // Outside the payload root: discarded with it, never checked.
            Entry::Link("elsewhere", "/etc/passwd"),
        ],
    )
}

#[test]
#[cfg(unix)]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
fn relative_links_inside_the_payload_are_kept_as_links() {
    let dir = tempfile::tempdir().unwrap();
    let archive = node_like(dir.path());
    let out = dir.path().join("out");
    let extracted = extract_with(
        &archive,
        ArchiveFormat::TarGz,
        Some(Path::new("node-v1")),
        &out,
        LinkMode::Symlink,
    )
    .unwrap();
    assert!(extracted.notes.is_empty(), "{:?}", extracted.notes);
    assert_eq!(
        fs::read_link(out.join("bin/npm")).unwrap(),
        Path::new("../lib/node_modules/npm/bin/npm-cli.js")
    );
    assert_eq!(fs::read(out.join("bin/npm")).unwrap(), b"npm");
    assert_eq!(fs::read(out.join("bin/npm-again")).unwrap(), b"npm");
    assert_eq!(
        fs::read(out.join("share/npm/bin/npm-cli.js")).unwrap(),
        b"npm"
    );
    assert!(!dir.path().join("out.unpack").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
fn the_copy_mode_copies_linked_files_and_notes_directory_links() {
    let dir = tempfile::tempdir().unwrap();
    let archive = node_like(dir.path());
    let out = dir.path().join("out");
    let extracted = extract_with(
        &archive,
        ArchiveFormat::TarGz,
        Some(Path::new("node-v1")),
        &out,
        LinkMode::Copy,
    )
    .unwrap();
    for copy in ["bin/npm", "bin/npm-again"] {
        let metadata = fs::symlink_metadata(out.join(copy)).unwrap();
        assert!(metadata.is_file(), "{copy} is a plain copy");
        assert_eq!(fs::read(out.join(copy)).unwrap(), b"npm");
    }
    assert!(fs::symlink_metadata(out.join("share")).is_err());
    assert_eq!(extracted.notes.len(), 1, "{:?}", extracted.notes);
    assert!(
        extracted.notes[0].contains("`node-v1/share`"),
        "{:?}",
        extracted.notes
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
fn links_that_could_leave_or_write_through_the_payload_are_refused() {
    let cases: &[(&str, &[Entry<'_>])] = &[
        (
            "absolute",
            &[Entry::File("p/f", b"f"), Entry::Link("p/a", "/etc/passwd")],
        ),
        (
            "escape",
            &[Entry::File("p/f", b"f"), Entry::Link("p/a", "../../f")],
        ),
        // Inside the archive, but above the declared payload root.
        (
            "above the payload root",
            &[
                Entry::File("p/f", b"f"),
                Entry::File("other/f", b"o"),
                Entry::Link("p/a", "../other/f"),
            ],
        ),
        (
            "through a link",
            &[
                Entry::File("p/sub/f", b"f"),
                Entry::Link("p/l", "sub"),
                Entry::Link("p/m", "l/f"),
            ],
        ),
        (
            "a link popped by `..`",
            &[
                Entry::File("p/deep/er/f", b"f"),
                Entry::Link("p/l", "deep/er"),
                Entry::Link("p/m", "l/../f"),
            ],
        ),
        (
            "inside a link",
            &[
                Entry::File("p/sub/f", b"f"),
                Entry::Link("p/l", "sub"),
                Entry::Link("p/l/x", "../sub/f"),
            ],
        ),
        (
            "cycle",
            &[
                Entry::File("p/f", b"f"),
                Entry::Link("p/a", "b"),
                Entry::Link("p/b", "a"),
            ],
        ),
        (
            "dangling",
            &[Entry::File("p/f", b"f"), Entry::Link("p/a", "missing")],
        ),
        (
            "contains itself",
            &[Entry::File("p/sub/f", b"f"), Entry::Link("p/sub/up", "..")],
        ),
        (
            "collides with a directory",
            &[Entry::File("p/a/f", b"f"), Entry::Link("p/a", "a/f")],
        ),
        (
            "backslash",
            &[Entry::File("p/f", b"f"), Entry::Link("p/a", "..\\f")],
        ),
    ];
    for mode in [LinkMode::Symlink, LinkMode::Copy] {
        for (label, entries) in cases {
            let dir = tempfile::tempdir().unwrap();
            let archive = tar_with(dir.path(), entries);
            let out = dir.path().join("out");
            let error = extract_with(
                &archive,
                ArchiveFormat::TarGz,
                Some(Path::new("p")),
                &out,
                mode,
            )
            .expect_err(label);
            assert!(
                matches!(error, InstallerError::Archive { .. })
                    && error.to_string().contains("link `p/"),
                "{label}: {error}"
            );
            assert!(!out.exists(), "{label}");
            assert!(!dir.path().join("out.unpack").exists(), "{label}");
        }
    }
}

#[test]
#[cfg(unix)]
fn a_zip_symlink_entry_is_admitted_like_a_tar_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("links.zip");
    let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
    zip.start_file("app/lib/tool.js", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"tool").unwrap();
    zip.add_symlink(
        "app/bin/tool",
        "../lib/tool.js",
        SimpleFileOptions::default(),
    )
    .unwrap();
    zip.add_symlink(
        "app/bin/evil",
        "../../../etc/passwd",
        SimpleFileOptions::default(),
    )
    .unwrap();
    zip.finish().unwrap();
    let out = dir.path().join("out");
    let error = extract_with(
        &path,
        ArchiveFormat::Zip,
        Some(Path::new("app")),
        &out,
        LinkMode::Symlink,
    )
    .unwrap_err();
    assert!(error.to_string().contains("app/bin/evil"), "{error}");

    let path = dir.path().join("good.zip");
    let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
    zip.start_file("app/lib/tool.js", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"tool").unwrap();
    zip.add_symlink(
        "app/bin/tool",
        "../lib/tool.js",
        SimpleFileOptions::default(),
    )
    .unwrap();
    zip.finish().unwrap();
    extract_with(
        &path,
        ArchiveFormat::Zip,
        Some(Path::new("app")),
        &out,
        LinkMode::Symlink,
    )
    .unwrap();
    assert_eq!(
        fs::read_link(out.join("bin/tool")).unwrap(),
        Path::new("../lib/tool.js")
    );
    assert_eq!(fs::read(out.join("bin/tool")).unwrap(), b"tool");
}
