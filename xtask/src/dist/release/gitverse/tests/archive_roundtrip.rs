//! Archive-only publication and reader acceptance over loopback.

use super::fake::*;
use super::*;

/// Production publication followed by real self-update over the captured bytes.
/// Supply VIBE_TEST_CURRENT_CLI with a built companion-aware CLI.
#[cfg(target_os = "linux")]
#[test]
fn gitverse_archive_only_roundtrip() {
    use crate::dist::archive::{read_zip, write_zip};
    use vibe_publish::{AggregateDistributionManifest, PlatformDistributionFragment};
    let mut files = fixture_files();
    let target = "x86_64-unknown-linux-gnu";
    let fragment_name = crate::dist::build::fragment_asset_name(&version(), target);
    let mut fragment: serde_json::Value = serde_json::from_slice(&files[&fragment_name]).unwrap();
    let bundle_name = fragment["asset"]["name"].as_str().unwrap().to_owned();
    let stub = b"#!/bin/sh\nprintf 'vibe 1.0.0 (stub)\\n'\n".to_vec();
    let mut entries = read_zip(&files[&bundle_name]).unwrap();
    let mut bundle = fragment["bundle"].clone();
    let component = bundle["components"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|component| component["name"] == "vibe")
        .unwrap();
    component["size"] = serde_json::json!(stub.len());
    component["digest"] = serde_json::json!(sha256_digest(&stub));
    for entry in &mut entries {
        if entry.name == "vibe" {
            entry.bytes = stub.clone();
        }
        if entry.name == "DISTRIBUTION.json" {
            entry.bytes = serde_json::to_vec(&bundle).unwrap();
        }
    }
    let zip = write_zip(&entries).unwrap();
    fragment["bundle"] = bundle;
    fragment["asset"]["size"] = serde_json::json!(zip.len());
    fragment["asset"]["digest"] = serde_json::json!(sha256_digest(&zip));
    fragment["bootstrap"]["size"] = serde_json::json!(stub.len());
    fragment["bootstrap"]["digest"] = serde_json::json!(sha256_digest(&stub));
    let bootstrap = fragment["bootstrap"]["name"].as_str().unwrap().to_owned();
    files.insert(bundle_name.clone(), zip);
    files.insert(bootstrap, stub);
    files.insert(fragment_name, serde_json::to_vec(&fragment).unwrap());
    let platforms = SUPPORTED_DISTRIBUTION_TARGETS
        .iter()
        .map(|target| {
            PlatformDistributionFragment::from_json_slice(
                &files[&crate::dist::build::fragment_asset_name(&version(), target)],
            )
            .unwrap()
        })
        .collect();
    let aggregate = AggregateDistributionManifest::from_platforms(platforms).unwrap();
    files.insert(
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME.to_owned(),
        aggregate.to_json_bytes().unwrap(),
    );
    let originals = files.clone();
    let source = verify(&version(), files).unwrap();
    let (publisher, state) = gitverse_fake();
    mirror(&publisher, &source, false).expect("production publication succeeds");
    let captured = state.lock().unwrap().releases[0]
        .assets
        .iter()
        .map(|asset| (asset.1.clone(), asset.2.clone()))
        .collect::<BTreeMap<_, _>>();
    let companion = "vibevm-release-companion.zip";
    let archives = originals
        .keys()
        .filter(|name| name.ends_with(".zip"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected = archives
        .iter()
        .cloned()
        .chain([companion.to_owned()])
        .collect::<BTreeSet<_>>();
    assert_eq!(
        captured.keys().cloned().collect::<BTreeSet<_>>(),
        expected,
        "publisher must supply archive-only output before the reader starts"
    );
    for name in &archives {
        assert_eq!(captured[name], originals[name]);
    }
    let expected_metadata = originals
        .iter()
        .filter(|(name, _)| !archives.contains(*name))
        .map(|(name, bytes)| (name.clone(), bytes.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(expected_metadata.len(), 14);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&captured[companion])).unwrap();
    let mut members = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        assert!(members.insert(entry.name().to_owned(), bytes).is_none());
    }
    assert_eq!(members, expected_metadata);
    // Adapt only the local read API; every downloadable body is the exact
    // byte vector saved by the publisher fake, never a reader-side companion.
    let captured = Arc::new(captured);
    let read_assets = captured.clone();
    let reader = FakeServer::start(Arc::new(move |request, base| {
        if request.path == format!("{REPO}/releases") {
            return (
                200,
                json(serde_json::json!([{"tag_name":"v1.0.0","draft":false}])),
            );
        }
        if request.path == format!("{REPO}/releases/tags/v1.0.0") {
            let assets = read_assets
                .keys()
                .map(|name| {
                    serde_json::json!({
                        "name":name,"browser_download_url":format!("{base}/captured/{name}")
                    })
                })
                .collect::<Vec<_>>();
            return (
                200,
                json(serde_json::json!({"tag_name":"v1.0.0","draft":false,"assets":assets})),
            );
        }
        if let Some(name) = request.path.strip_prefix("/captured/")
            && let Some(bytes) = read_assets.get(name)
        {
            return (200, bytes.clone());
        }
        not_found()
    }));
    let cli = std::env::var_os("VIBE_TEST_CURRENT_CLI")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let exe = std::env::current_exe().unwrap();
            exe.parent().unwrap().parent().unwrap().join("vibe")
        });
    assert!(
        cli.is_file(),
        "build the workspace CLI or set VIBE_TEST_CURRENT_CLI"
    );
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let settings = root.path().join("settings");
    let base = root.path().join("base");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&settings).unwrap();
    let run = |binary: &std::path::Path, args: &[&str]| {
        std::process::Command::new(binary)
            .args(["--unattended"])
            .args(args)
            .env_clear()
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("VIBE_SETTINGS", &settings)
            .env("VIBEVM_INSTALL_ROOT", &base)
            .env("PATH", "/usr/bin:/bin")
            .current_dir(root.path())
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    };
    let imported = run(
        &cli,
        &[
            "self",
            "import",
            cli.to_str().unwrap(),
            "--tag",
            "0.0.0",
            "--use",
        ],
    );
    assert!(
        imported.status.success(),
        "{}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let active = base.join("opt/bin/vibe");
    let source_url = format!("{}/vibevm/vibevm", reader.base);
    let configured = run(
        &active,
        &[
            "self",
            "source",
            "set",
            "gitverse",
            "--url",
            &source_url,
            "--api",
            &reader.base,
        ],
    );
    assert!(
        configured.status.success(),
        "{}",
        String::from_utf8_lossy(&configured.stderr)
    );
    let updated = run(&active, &["self", "update"]);
    assert!(
        updated.status.success(),
        "{}",
        String::from_utf8_lossy(&updated.stderr)
    );
    let activated = run(&active, &["--version"]);
    assert!(activated.status.success());
    assert_eq!(
        String::from_utf8(activated.stdout).unwrap(),
        "vibe 1.0.0 (stub)\n"
    );
    let bundle_path = format!("/captured/{bundle_name}");
    let count = || {
        reader
            .requests()
            .iter()
            .filter(|request| request.path == bundle_path)
            .count()
    };
    assert_eq!(
        count(),
        1,
        "the uploaded platform bundle was downloaded once"
    );
    let original = base.join("opt/vibevm/versions/tag/0.0.0/1/bin/vibe");
    let repeat = run(&original, &["self", "update"]);
    assert!(
        repeat.status.success(),
        "{}",
        String::from_utf8_lossy(&repeat.stderr)
    );
    assert_eq!(
        count(),
        1,
        "repeat does not download the installed bundle again"
    );
}
