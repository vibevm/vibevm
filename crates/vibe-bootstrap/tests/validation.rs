use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use vibe_bootstrap::{kit::parse_checksums, source_extract::extract_source_archive, verify_bundle};
use vibe_wire::behaviour::release_manifest::{
    BundleDistributionManifest, DistributionComponent, DistributionComponentName,
    DistributionSourceArchive,
};
use zip::{ZipWriter, write::SimpleFileOptions};
fn digest(b: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(b))
}
fn zip_entries(rows: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = ZipWriter::new(Cursor::new(Vec::new()));
    for (n, b) in rows {
        z.start_file(*n, SimpleFileOptions::default()).unwrap();
        z.write_all(b).unwrap();
    }
    z.finish().unwrap().into_inner()
}
fn bundle() -> Vec<u8> {
    let source = zip_entries(&[
        ("Cargo.toml", b"[workspace]\n"),
        ("crates/vibe-cli/src/main.rs", b"fn main() {}"),
    ]);
    let components = [
        ("vibe", DistributionComponentName::Vibe),
        ("vibe-app-proxy", DistributionComponentName::VibeAppProxy),
        ("vibe-index", DistributionComponentName::VibeIndex),
    ]
    .into_iter()
    .map(|(n, k)| DistributionComponent {
        name: k,
        path: n.into(),
        size: 4,
        digest: digest(b"code"),
    })
    .collect();
    let m = BundleDistributionManifest::new(
        "1.0.12".parse().unwrap(),
        "1".repeat(40),
        "x86_64-unknown-linux-gnu",
        components,
        DistributionSourceArchive {
            path: "vibevm-source.zip".into(),
            size: source.len() as u64,
            digest: digest(&source),
            tree_oid: "2".repeat(40),
        },
    )
    .unwrap();
    let manifest = m.to_json_bytes().unwrap();
    zip_entries(&[
        ("DISTRIBUTION.json", &manifest),
        ("vibe", b"code"),
        ("vibe-app-proxy", b"code"),
        ("vibe-index", b"code"),
        ("vibevm-source.zip", &source),
        ("LICENSE.md", b"license"),
        ("README.md", b"readme"),
    ])
}
#[test]
fn verified_bundle_refuses_outer_hash_and_wrong_target() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("bundle.zip");
    let b = bundle();
    std::fs::write(&p, &b).unwrap();
    let h = format!("{:x}", Sha256::digest(&b));
    assert!(verify_bundle(&p, &"0".repeat(64), "x86_64-unknown-linux-gnu").is_err());
    assert!(verify_bundle(&p, &h, "x86_64-pc-windows-msvc").is_err());
    let ok = verify_bundle(&p, &h, "x86_64-unknown-linux-gnu").unwrap();
    assert_eq!(std::fs::read(ok.directory.join("vibe")).unwrap(), b"code");
}
#[test]
fn checksum_paths_refuse_escape_and_duplicates() {
    assert!(parse_checksums(&format!("{}  ../bad\n", "a".repeat(64))).is_err());
    assert!(parse_checksums(&format!("{0}  A.zip\n{0}  a.zip\n", "a".repeat(64))).is_err());
    assert_eq!(
        parse_checksums(&format!("{}  drop/a.zip\n", "a".repeat(64)))
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn source_extract_refuses_collision_and_traversal_before_writing() {
    for rows in [
        vec![("../escape", &b"x"[..])],
        vec![("A", &b"x"[..]), ("a", &b"y"[..])],
        vec![("file", &b"x"[..]), ("file/child", &b"y"[..])],
    ] {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("s.zip");
        std::fs::write(&p, zip_entries(&rows)).unwrap();
        let out = t.path().join("out");
        assert!(extract_source_archive(&p, &out).is_err());
        assert!(!out.exists());
    }
}

#[test]
fn conflicting_settings_refuse_before_any_artifact_or_destination_write() {
    use clap::Parser;
    let t = tempfile::tempdir().unwrap();
    let kit = t.path().join("kit");
    std::fs::create_dir(&kit).unwrap();
    let declared = t.path().join("declared");
    let other = t.path().join("other");
    let profile = kit.join("profile.toml");
    std::fs::write(
        &profile,
        format!("schema=1\n[self]\nsettings_dir='{}'\n", declared.display()),
    )
    .unwrap();
    let args = vibe_bootstrap::kit::Args::try_parse_from([
        "vibe-installer",
        "--kit",
        kit.to_str().unwrap(),
        "--profile",
        profile.to_str().unwrap(),
        "--settings",
        other.to_str().unwrap(),
    ])
    .unwrap();
    let error = vibe_bootstrap::kit::run(args).unwrap_err();
    assert!(error.to_string().contains("conflicts"));
    assert!(!declared.exists());
    assert!(!other.exists());
}
