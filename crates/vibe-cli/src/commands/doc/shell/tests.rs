//! The install lane, without a network.
//!
//! Every test here drives [`super::install_into`] through a [`Fetcher`]
//! that reads a directory this test wrote — which is the whole point of
//! the seam: the one command in this surface that would touch the network
//! is exercised without touching it, and the checks it makes on what
//! arrives are what is under test.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::Digest as _;
use vibe_doc_shell::{digest, index::Index};

use super::*;

/// A release on disk: `<root>/v<version>/<file>`.
struct LocalRelease(PathBuf);

impl Fetcher for LocalRelease {
    fn fetch(&self, url: &str, destination: &Path, maximum: u64) -> anyhow::Result<()> {
        let name = url.rsplit('/').next().unwrap_or_default();
        let from = self.0.join(name);
        let bytes = std::fs::read(&from)?;
        if bytes.len() as u64 > maximum {
            anyhow::bail!("`{url}` exceeds its {maximum}-byte limit");
        }
        std::fs::write(destination, bytes)?;
        Ok(())
    }
}

/// Build a shell, zip it, and write the manifest beside it. Returns the
/// digest of the shell tree — what a pin would carry.
fn publish(into: &Path, template: &str, version: &str) -> String {
    std::fs::create_dir_all(into).unwrap();
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    files.insert(
        vibe_doc_shell::index::PAGE_TEMPLATE.to_string(),
        template.as_bytes().to_vec(),
    );
    files.insert("assets/style.css".to_string(), b"body{}".to_vec());
    let sha256 = digest::of(&files);
    let index = Index {
        schema: vibe_doc_shell::index::INDEX_SCHEMA,
        package: "org.vibevm.doc/web".to_string(),
        version: "0.1.0".to_string(),
        base: vibe_doc_shell::DEFAULT_BASE.to_string(),
        island_marker: vibe_doc_shell::ISLAND_MARKER.to_string(),
        files: files.len() as u32,
        sha256: sha256.clone(),
    };

    let name = format!("vibevm-doc-shell-{version}.zip");
    let archive_path = into.join(&name);
    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut packed = files.clone();
    packed.insert(
        vibe_doc_shell::index::INDEX_FILE.to_string(),
        index.to_json().into_bytes(),
    );
    for (relative, bytes) in &packed {
        writer.start_file(relative.clone(), options).unwrap();
        std::io::Write::write_all(&mut writer, bytes).unwrap();
    }
    writer.finish().unwrap();

    let archive = std::fs::read(&archive_path).unwrap();
    let manifest = serde_json::json!({
        "schema_version": 1,
        "product": "vibevm",
        "repository": "https://github.com/vibevm/vibevm",
        "version": version,
        "tag": format!("v{version}"),
        "source_commit": "0".repeat(40),
        "asset": {
            "name": name,
            "size": archive.len(),
            "digest": format!("sha256:{}", hex(&sha2::Sha256::digest(&archive))),
        }
    });
    std::fs::write(
        into.join(fetch::MANIFEST_FILENAME),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    sha256
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn pin_for(sha256: &str) -> Pin {
    Pin {
        schema: vibe_doc_shell::pin::PIN_SCHEMA,
        package: "org.vibevm.doc/web".to_string(),
        version: "0.1.0".to_string(),
        sha256: sha256.to_string(),
    }
}

#[test]
fn a_release_that_matches_the_pin_is_unpacked_into_the_store() {
    let tmp = tempfile::tempdir().unwrap();
    let release = tmp.path().join("v9.9.9");
    let sha256 = publish(&release, "<!--vibe-doc-island-->", "9.9.9");
    let into = tmp.path().join("store").join(&sha256);

    super::install_into(
        &tmp.path().join("staging"),
        &into,
        &LocalRelease(release),
        "ignored",
        "9.9.9",
        &pin_for(&sha256),
        &vibe_core::progress::Progress::default(),
    )
    .unwrap();

    assert!(into.join(vibe_doc_shell::index::INDEX_FILE).is_file());
    let shell = Shell::inspect(Some(&into)).unwrap();
    assert!(
        shell
            .page_template()
            .contains(vibe_doc_shell::ISLAND_MARKER)
    );
    assert_eq!(shell.measured_digest(), sha256);
}

#[test]
fn a_release_for_another_version_is_refused_before_anything_is_downloaded() {
    let tmp = tempfile::tempdir().unwrap();
    let release = tmp.path().join("v9.9.9");
    let sha256 = publish(&release, "<!--vibe-doc-island-->", "9.9.9");

    let outcome = super::install_into(
        &tmp.path().join("staging"),
        &tmp.path().join("store").join(&sha256),
        &LocalRelease(release),
        "ignored",
        "8.8.8",
        &pin_for(&sha256),
        &vibe_core::progress::Progress::default(),
    );
    let message = outcome.unwrap_err().to_string();
    assert!(
        message.contains("does not travel between them"),
        "{message}"
    );
}

#[test]
fn the_registered_shell_release_manifest_corpus_is_the_accepted_shape() {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/doc-shell-release-manifest/e1/valid/manifest.json"
    ));
    let manifest = fetch::parse_manifest(bytes, "1.2.3").unwrap();
    assert_eq!(manifest.schema_version, fetch::MANIFEST_SCHEMA_VERSION);
    assert_eq!(manifest.asset.name, "vibevm-doc-shell-1.2.3.zip");
    let mut encoded = serde_json::to_string_pretty(&manifest).unwrap();
    encoded.push('\n');
    assert_eq!(encoded.as_bytes(), bytes);
}

#[test]
fn the_generated_shell_release_manifest_reader_keeps_the_unknown_field_refusal() {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/doc-shell-release-manifest/e1/invalid/unknown_field.json"
    ));
    let message = fetch::parse_manifest(bytes, "1.2.3")
        .unwrap_err()
        .to_string();
    assert!(message.contains("reading the shell's release manifest"));
}

#[test]
fn a_shell_that_does_not_hash_to_the_pin_is_refused_after_it_arrives() {
    let tmp = tempfile::tempdir().unwrap();
    let release = tmp.path().join("v9.9.9");
    publish(&release, "<!--vibe-doc-island-->", "9.9.9");
    let wrong = "0".repeat(64);

    let outcome = super::install_into(
        &tmp.path().join("staging"),
        &tmp.path().join("store").join(&wrong),
        &LocalRelease(release),
        "ignored",
        "9.9.9",
        &pin_for(&wrong),
        &vibe_core::progress::Progress::default(),
    );
    let message = outcome.unwrap_err().to_string();
    assert!(message.contains("is pinned to"), "{message}");
    assert!(!tmp.path().join("store").join(&wrong).exists());
}

#[test]
fn a_corrupted_asset_is_refused_on_the_digest() {
    let tmp = tempfile::tempdir().unwrap();
    let release = tmp.path().join("v9.9.9");
    let sha256 = publish(&release, "<!--vibe-doc-island-->", "9.9.9");
    // One byte of the archive changed; the manifest still says what it
    // said.
    let archive = release.join("vibevm-doc-shell-9.9.9.zip");
    let mut bytes = std::fs::read(&archive).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(&archive, bytes).unwrap();

    let outcome = super::install_into(
        &tmp.path().join("staging"),
        &tmp.path().join("store").join(&sha256),
        &LocalRelease(release),
        "ignored",
        "9.9.9",
        &pin_for(&sha256),
        &vibe_core::progress::Progress::default(),
    );
    let message = outcome.unwrap_err().to_string();
    assert!(message.contains("integrity mismatch"), "{message}");
}

#[test]
fn an_archive_entry_that_walks_out_of_the_shell_is_refused() {
    for name in [
        "../outside.txt",
        "assets/../../outside.txt",
        "/absolute.txt",
        "C:/absolute.txt",
        "assets\\style.css",
    ] {
        assert!(
            fetch::plain_relative(name).is_none(),
            "`{name}` must not be unpacked"
        );
    }
    assert_eq!(
        fetch::plain_relative("assets/style.css"),
        Some("assets/style.css")
    );
}

#[test]
fn the_store_directory_is_named_by_the_pin() {
    let env = DocEnv {
        install_root: Some(PathBuf::from("/root/opt")),
        ..DocEnv::default()
    };
    let pin = Pin::compiled_in().unwrap();
    match super::store_dir(&env) {
        Some(dir) => assert!(dir.ends_with(&pin.sha256)),
        // A checkout that has never built a shell pins nothing, and
        // there is no directory to name.
        None => assert!(!pin.is_set()),
    }
}
