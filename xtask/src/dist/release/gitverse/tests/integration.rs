//! End-to-end runs of `mirror_with`, `write_dry_run` and the GitHub source
//! against the loopback fakes.

use super::fake::*;
use super::*;

#[test]
fn fresh_mirror_creates_the_release_uploads_everything_and_reruns_as_a_no_op() {
    let (server, state) = gitverse_fake();
    let source = fixture();
    let (report, _) = mirror(&server, &source, false).unwrap();
    assert!(report.created);
    assert_eq!(report.uploaded.len(), source.assets.len());
    let requests = server.requests();
    let names = uploads(&requests);
    assert_eq!(
        names,
        source
            .assets
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(names.last().unwrap(), super::super::source::COMPANION_NAME);
    for request in &requests {
        assert_eq!(request.accept.as_deref(), Some(GITVERSE_ACCEPT));
        assert_eq!(
            request.authorization.as_deref(),
            Some(&*format!("Bearer {TOKEN}"))
        );
    }
    {
        let state = state.lock().unwrap();
        assert_eq!(state.releases.len(), 1);
        assert!(!state.releases[0].draft);
        for asset in &source.assets {
            let stored = state.releases[0]
                .assets
                .iter()
                .find(|a| a.1 == asset.name)
                .unwrap();
            assert_eq!(stored.2, asset.bytes, "{} stored byte-for-byte", asset.name);
        }
    }

    server.clear();
    let (again, _) = mirror(&server, &source, false).unwrap();
    assert!(!again.created);
    assert_eq!(again.release_id, report.release_id);
    assert_eq!(again.skipped.len(), source.assets.len());
    assert!(again.uploaded.is_empty() && again.replaced.is_empty());
    assert!(
        server.requests().iter().all(|r| r.method == "GET"),
        "a rerun only reads"
    );
}

/// Opaque, already-final attachments: this exercises mirror planning only,
/// not the production packer or ZIP validity.
fn six_asset_mirror_fixture() -> SourceRelease {
    let mut source = fixture();
    source.assets.retain(|asset| {
        asset.name.ends_with(".zip") && asset.name != super::super::source::COMPANION_NAME
    });
    assert_eq!(source.assets.len(), 5);
    for (index, asset) in source.assets.iter_mut().enumerate() {
        asset.bytes = vec![b'P', index as u8, 0, 255];
        asset.digest = sha256_digest(&asset.bytes);
    }
    let bytes = b"opaque ready companion\0\xff".to_vec();
    source.assets.push(SourceAsset {
        name: "vibevm-release-companion.zip".to_owned(),
        content_type: "application/zip",
        digest: sha256_digest(&bytes),
        bytes,
    });
    source
}

#[test]
fn six_asset_mirror_repeat_is_read_only_and_companion_hash_refusal_is_atomic() {
    let (server, state) = gitverse_fake();
    let source = six_asset_mirror_fixture();
    let (first, _) = mirror(&server, &source, false).unwrap();
    assert_eq!(first.uploaded.len(), 6);
    let before = {
        let state = state.lock().unwrap();
        (state.next_id, state.releases[0].assets.clone())
    };

    server.clear();
    let (repeat, _) = mirror(&server, &source, false).unwrap();
    assert!(!repeat.created);
    assert_eq!(repeat.release_id, first.release_id);
    assert_eq!(repeat.skipped.len(), 6);
    assert!(repeat.uploaded.is_empty() && repeat.replaced.is_empty());
    let requests = server.requests();
    assert!(!requests.is_empty());
    assert!(requests.iter().all(|request| request.method == "GET"));
    {
        let state = state.lock().unwrap();
        assert_eq!((state.next_id, state.releases[0].assets.clone()), before);
    }

    let mut changed = source.clone();
    let companion = changed.assets.last_mut().unwrap();
    companion.bytes[0] ^= 1; // Same size: refusal must compare hashes.
    companion.digest = sha256_digest(&companion.bytes);
    assert_eq!(
        companion.bytes.len(),
        source.assets.last().unwrap().bytes.len()
    );
    assert_ne!(companion.digest, source.assets.last().unwrap().digest);
    server.clear();
    let error = mirror(&server, &changed, false).unwrap_err().to_string();
    assert!(error.contains("vibevm-release-companion.zip"), "{error}");
    assert!(error.contains("nothing was uploaded"), "{error}");
    assert!(error.contains("--replace"), "{error}");
    let requests = server.requests();
    assert!(!requests.is_empty());
    assert!(requests.iter().all(|request| request.method == "GET"));
    let state = state.lock().unwrap();
    assert_eq!(state.releases.len(), 1);
    assert_eq!((state.next_id, state.releases[0].assets.clone()), before);
}

#[test]
fn six_asset_mirror_refuses_a_legacy_standalone_manifest_before_mutation() {
    let (server, state) = gitverse_fake();
    let source = six_asset_mirror_fixture();
    let legacy = b"INVALID OLD MANIFEST".to_vec();
    {
        let mut state = state.lock().unwrap();
        state.releases.push(GvRelease {
            id: 1,
            tag: source.tag.clone(),
            draft: false,
            assets: vec![(
                2,
                DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME.to_owned(),
                legacy.clone(),
            )],
        });
    }
    let before = {
        let state = state.lock().unwrap();
        (state.next_id, state.releases[0].assets.clone())
    };
    for replace in [false, true] {
        server.clear();
        let error = mirror(&server, &source, replace).unwrap_err().to_string();
        assert!(error.contains(&source.tag), "{error}");
        assert!(
            error.contains(DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME),
            "{error}"
        );
        assert!(error.contains("vibevm-release-companion.zip"), "{error}");
        assert!(error.contains("conflicts with this tag"), "{error}");
        assert!(error.contains("Nothing was uploaded or deleted"), "{error}");
        assert!(error.contains("`--replace` does not remove"), "{error}");
        let requests = server.requests();
        assert!(!requests.is_empty());
        assert!(requests.iter().all(|request| request.method == "GET"));
        let state = state.lock().unwrap();
        assert_eq!(state.releases.len(), 1);
        assert_eq!(state.releases[0].id, 1);
        assert_eq!(state.releases[0].tag, source.tag);
        assert!(!state.releases[0].draft);
        assert_eq!((state.next_id, state.releases[0].assets.clone()), before);
    }
}

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
        if let Some(name) = request.path.strip_prefix("/captured/") {
            if let Some(bytes) = read_assets.get(name) {
                return (200, bytes.clone());
            }
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

#[test]
fn a_draft_hidden_from_the_tag_route_is_found_in_the_list_and_reused() {
    let (server, state) = gitverse_fake();
    {
        let mut state = state.lock().unwrap();
        state.by_tag_hides_drafts = true;
        for index in 0..RELEASES_PER_PAGE {
            state.releases.push(GvRelease {
                id: u64::from(index) + 1,
                tag: format!("v0.0.{index}"),
                ..GvRelease::default()
            });
        }
        state.releases.push(GvRelease {
            id: 77,
            tag: "v1.0.0".to_string(),
            draft: true,
            assets: Vec::new(),
        });
    }
    let (report, out) = mirror(&server, &fixture(), false).unwrap();
    assert!(!report.created);
    assert_eq!(report.release_id, 77);
    assert!(out.contains("is a draft"));
    let pages = server
        .requests()
        .iter()
        .filter(|r| r.method == "GET" && r.path == format!("{REPO}/releases"))
        .count();
    assert_eq!(pages, 2, "the second list page holds the draft");
}

#[test]
fn a_differing_asset_is_refused_without_replace_and_replaced_with_it() {
    let (server, state) = gitverse_fake();
    let source = fixture();
    mirror(&server, &source, false).unwrap();
    let changed = source.assets[1].name.clone();
    {
        let mut state = state.lock().unwrap();
        let asset = state.releases[0]
            .assets
            .iter_mut()
            .find(|a| a.1 == changed)
            .unwrap();
        // Same size, other bytes: only the digest comparison can tell.
        asset.2[0] ^= 0xff;
    }
    server.clear();
    let error = mirror(&server, &source, false).unwrap_err().to_string();
    assert!(
        error.contains(&changed) && error.contains("--replace"),
        "{error}"
    );
    assert!(
        server.requests().iter().all(|r| r.method == "GET"),
        "a refused run writes nothing"
    );

    server.clear();
    let (report, _) = mirror(&server, &source, true).unwrap();
    assert_eq!(report.replaced, vec![changed.clone()]);
    assert_eq!(report.skipped.len(), source.assets.len() - 1);
    assert_eq!(uploads(&server.requests()), vec![changed.clone()]);
    let state = state.lock().unwrap();
    let stored = state.releases[0]
        .assets
        .iter()
        .find(|a| a.1 == changed)
        .unwrap();
    assert_eq!(stored.2, source.assets[1].bytes);
}

#[test]
fn replacing_deletes_the_manifest_before_any_other_asset() {
    let (server, state) = gitverse_fake();
    let source = fixture();
    mirror(&server, &source, false).unwrap();
    let old_manifest_id = {
        let mut state = state.lock().unwrap();
        for asset in state.releases[0].assets.iter_mut() {
            if asset.1 == super::super::source::COMPANION_NAME || asset.1 == source.assets[0].name {
                asset.2.push(b'!');
            }
        }
        state.releases[0]
            .assets
            .iter()
            .find(|a| a.1 == super::super::source::COMPANION_NAME)
            .unwrap()
            .0
    };
    server.clear();
    mirror(&server, &source, true).unwrap();
    let requests = server.requests();
    let deletes = requests
        .iter()
        .filter(|r| r.method == "DELETE")
        .collect::<Vec<_>>();
    assert_eq!(deletes.len(), 2);
    let names = uploads(&requests);
    assert_eq!(names.first().unwrap(), &source.assets[0].name);
    assert_eq!(names.last().unwrap(), super::super::source::COMPANION_NAME);
    assert!(
        deletes[0]
            .path
            .ends_with(&format!("/assets/{old_manifest_id}")),
        "the old manifest goes first"
    );
}

#[test]
fn duplicate_remote_names_stop_the_run() {
    let (server, state) = gitverse_fake();
    let source = fixture();
    mirror(&server, &source, false).unwrap();
    {
        let mut state = state.lock().unwrap();
        let copy = state.releases[0].assets[0].clone();
        state.releases[0].assets.push((999, copy.1, copy.2));
    }
    let error = mirror(&server, &source, true).unwrap_err().to_string();
    assert!(error.contains("more than once"), "{error}");
}

#[test]
fn server_errors_never_echo_the_token() {
    let server = FakeServer::start(Arc::new(|request, _| {
        (
            500,
            request
                .authorization
                .clone()
                .unwrap_or_default()
                .into_bytes(),
        )
    }));
    let error = client_for(&server)
        .find_release("v1.0.0")
        .unwrap_err()
        .to_string();
    assert!(error.contains("500"), "{error}");
    assert!(!error.contains(TOKEN), "{error}");
    assert!(error.contains("Bearer ***"), "{error}");
}

#[test]
fn dry_run_lists_every_request_without_a_token_or_a_connection() {
    let (server, _) = gitverse_fake();
    let source = fixture();
    let endpoint = Endpoint::new(&server.base, "vibevm", "vibevm").unwrap();
    let mut out = Vec::new();
    write_dry_run(&mut out, &endpoint, &source, false).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert!(server.requests().is_empty(), "dry run sends nothing");
    assert!(!out.contains(TOKEN));
    assert!(out.contains(GITVERSE_ACCEPT));
    assert!(out.contains(&format!(
        "GET    {}{REPO}/releases/tags/v1.0.0",
        server.base
    )));
    assert!(out.contains("\"is_authorized_only\":false"));
    for asset in &source.assets {
        assert!(
            out.contains(&format!(
                "POST   {}{REPO}/releases/{{release_id}}/assets?name={} multipart attachment=@{}",
                server.base, asset.name, asset.name
            )),
            "{out}"
        );
    }
    assert!(out.contains("stops the run before any upload"));
    assert!(!out.contains("has no file extension"));
    let mut with_replace = Vec::new();
    write_dry_run(&mut with_replace, &endpoint, &source, true).unwrap();
    assert!(
        String::from_utf8(with_replace)
            .unwrap()
            .contains("/releases/{release_id}/assets/{asset_id}")
    );
}

#[test]
fn source_mirrors_the_published_github_release() {
    let files = Arc::new(fixture_files());
    let names = files.keys().cloned().collect::<Vec<_>>();
    let handler_files = files.clone();
    let server = FakeServer::start(Arc::new(move |request, _| {
        assert!(
            request.authorization.is_none(),
            "GitHub reads are anonymous"
        );
        let path = request.path.as_str();
        if path == "/repos/vibevm/vibevm/releases/tags/v1.0.0" {
            return (
                200,
                json(serde_json::json!({"id": 5, "tag_name": "v1.0.0", "draft": false})),
            );
        }
        if path == "/repos/vibevm/vibevm/releases/5/assets" {
            let items = names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let bytes = &handler_files[name];
                    serde_json::json!({
                        "id": index + 1, "name": name, "size": bytes.len(),
                        "digest": sha256_digest(bytes),
                    })
                })
                .collect::<Vec<_>>();
            return (200, json(serde_json::Value::Array(items)));
        }
        if let Some(id) = path.strip_prefix("/repos/vibevm/vibevm/releases/assets/") {
            let index: usize = id.parse().unwrap();
            return (200, handler_files[&names[index - 1]].clone());
        }
        not_found()
    }));
    let github = GithubReleaseClient::anonymous_with_endpoints(
        "vibevm",
        "vibevm",
        &server.base,
        &server.base,
    )
    .unwrap();
    assert_eq!(from_github(&github, &version()).unwrap(), fixture());
}

#[test]
fn a_release_list_answering_not_found_on_an_empty_repository_still_creates_the_release() {
    let (server, state) = gitverse_fake();
    state.lock().unwrap().empty_list_is_not_found = true;
    let source = fixture();
    let (report, _) = mirror(&server, &source, false).unwrap();
    assert!(report.created);
    assert_eq!(report.uploaded.len(), source.assets.len());
    assert_eq!(state.lock().unwrap().releases.len(), 1);
}
