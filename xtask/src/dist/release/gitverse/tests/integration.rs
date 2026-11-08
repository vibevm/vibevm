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
    assert_eq!(
        names.last().unwrap(),
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME
    );
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
            if asset.1 == DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME
                || asset.1 == source.assets[0].name
            {
                asset.2.push(b'!');
            }
        }
        state.releases[0]
            .assets
            .iter()
            .find(|a| a.1 == DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME)
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
    assert_eq!(
        names.last().unwrap(),
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME
    );
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
    assert!(out.contains("has no file extension"));
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
