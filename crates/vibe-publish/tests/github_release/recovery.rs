use super::*;

#[test]
fn authenticated_tag_ref_read_uses_the_exact_singular_endpoint() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    mock.json(
        StatusCode::OK,
        json!({
            "ref": "refs/tags/v1.2.3",
            "object": { "sha": "def456", "type": "commit", "url": "https://example.test/object" }
        }),
    );

    let reference = client.get_tag_ref_authenticated("v1.2.3").unwrap();
    assert_eq!(reference.object.sha, "def456");
    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests[0].method, "GET");
    assert_eq!(
        state.requests[0].uri,
        "/api/repos/vibevm/vibe/git/ref/tags/v1.2.3"
    );
    assert_eq!(
        state.requests[0].headers["authorization"],
        "Bearer release-token"
    );
}

#[test]
fn configured_endpoints_reject_and_redact_user_info() {
    let rendered = GithubReleaseClient::with_endpoints(
        Token::from_explicit("password"),
        "vibevm",
        "vibe",
        "https://publisher:password@example.test/api",
        "https://uploads.example.test",
    )
    .err()
    .expect("credentialed API base must be refused")
    .to_string();
    assert!(!rendered.contains("password"));
    assert!(rendered.contains("https://***@example.test/api"));
}

#[test]
fn explicit_final_temp_cleanup_removes_only_the_canonical_namespace() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    let public_url = format!("{}/downloads/temp", mock.base);
    mock.json(
        StatusCode::OK,
        json!([
            asset(
                91,
                ".vibe-upload-stale-0123456789ab-DISTRIBUTIONS.json",
                b"stale",
                &public_url
            ),
            asset(
                92,
                ".vibe-upload-stale-0123456789ab-other-DISTRIBUTIONS.json",
                b"other",
                &public_url
            )
        ]),
    );
    mock.bytes(StatusCode::NO_CONTENT, b"");
    client
        .cleanup_temporary_assets(7, &["DISTRIBUTIONS.json"])
        .unwrap();
    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 2);
    assert_eq!(
        state.requests[1].uri,
        "/api/repos/vibevm/vibe/releases/assets/91"
    );
}

#[test]
fn explicit_transfer_timeout_seam_aborts_a_slow_release_response() {
    let mock = MockGithub::spawn();
    let client = GithubReleaseClient::with_endpoints_and_timeouts(
        Token::from_explicit("release-token"),
        "vibevm",
        "vibe",
        &format!("{}/api", mock.base),
        &format!("{}/uploads", mock.base),
        Duration::from_secs(1),
        Duration::from_millis(10),
    )
    .unwrap();
    mock.delayed_json(Duration::from_millis(100), release(90, "v1.0.0"));
    assert!(matches!(
        client.find_release_authenticated("v1.0.0"),
        Err(GithubReleaseError::Transport { .. })
    ));
}

#[test]
fn publishing_uses_legacy_latest_selection_instead_of_forcing_an_older_version_latest() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    mock.json(StatusCode::OK, release(41, "v1.0.0"));
    client
        .update_release(
            41,
            &UpdateGithubRelease {
                draft: Some(false),
                make_latest: Some(GithubMakeLatest::Legacy),
                ..UpdateGithubRelease::default()
            },
        )
        .unwrap();
    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 1);
    assert_eq!(state.requests[0].method, "PATCH");
    assert_eq!(state.requests[0].uri, "/api/repos/vibevm/vibe/releases/41");
    assert_eq!(
        serde_json::from_slice::<Value>(&state.requests[0].body).unwrap(),
        json!({"draft": false, "make_latest": "legacy"})
    );
}

#[test]
fn replace_uploads_verifies_deletes_old_then_renames() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    let old = b"old";
    let new = b"new bundle";
    let public_url = format!("{}/downloads/vibe.zip", mock.base);
    mock.json(
        StatusCode::OK,
        json!([asset(21, "vibe.zip", old, &public_url)]),
    );
    mock.json(
        StatusCode::CREATED,
        asset(22, "temporary", new, &public_url),
    );
    mock.bytes(StatusCode::NO_CONTENT, b"");
    mock.json(StatusCode::OK, asset(22, "vibe.zip", new, &public_url));
    let replaced = client
        .publish_asset(4, "vibe.zip", "application/zip", new)
        .unwrap();
    assert_eq!(replaced.id, 22);
    assert_eq!(replaced.name, "vibe.zip");
    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 4);
    assert_eq!(state.requests[0].method, "GET");
    assert_eq!(state.requests[1].method, "POST");
    assert!(state.requests[1].uri.contains("name=.vibe-upload-"));
    assert_eq!(
        state.requests[2].uri,
        "/api/repos/vibevm/vibe/releases/assets/21"
    );
    assert_eq!(state.requests[2].method, "DELETE");
    assert_eq!(state.requests[3].method, "PATCH");
    let rename_body: Value = serde_json::from_slice(&state.requests[3].body).unwrap();
    assert_eq!(rename_body, json!({"name": "vibe.zip"}));
}

#[test]
fn replacement_verification_failure_keeps_the_old_asset() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    let public_url = format!("{}/downloads/vibe.zip", mock.base);
    mock.json(
        StatusCode::OK,
        json!([asset(30, "vibe.zip", b"old", &public_url)]),
    );
    let mut wrong = asset(31, "temporary", b"new", &public_url);
    wrong["digest"] = json!(sha256_digest(b"different"));
    mock.json(StatusCode::CREATED, wrong);
    mock.json(
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({"message": "cleanup also failed"}),
    );
    let error = client
        .replace_asset(4, "vibe.zip", "application/zip", b"new")
        .unwrap_err();
    assert!(matches!(
        error,
        GithubReleaseError::AssetVerification { .. }
    ));
    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 3);
    assert_eq!(state.requests[2].method, "DELETE");
    assert_eq!(
        state.requests[2].uri, "/api/repos/vibevm/vibe/releases/assets/31",
        "only the invalid temporary upload is cleaned; the old asset remains"
    );
}

#[test]
fn replacement_cleans_new_upload_when_old_delete_or_rename_fails() {
    let old_delete = MockGithub::spawn();
    let client = old_delete.client("release-token");
    let public_url = format!("{}/downloads/vibe.zip", old_delete.base);
    old_delete.json(
        StatusCode::OK,
        json!([asset(40, "vibe.zip", b"old", &public_url)]),
    );
    old_delete.json(
        StatusCode::CREATED,
        asset(41, "temporary", b"new", &public_url),
    );
    old_delete.json(
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({"message": "old delete failed"}),
    );
    old_delete.bytes(StatusCode::NO_CONTENT, b"");
    assert!(
        client
            .replace_asset(4, "vibe.zip", "application/zip", b"new")
            .is_err()
    );
    let state = old_delete.state.lock().unwrap();
    assert_eq!(
        state.requests[2].uri,
        "/api/repos/vibevm/vibe/releases/assets/40"
    );
    assert_eq!(
        state.requests[3].uri,
        "/api/repos/vibevm/vibe/releases/assets/41"
    );

    let rename = MockGithub::spawn();
    let client = rename.client("release-token");
    let public_url = format!("{}/downloads/vibe.zip", rename.base);
    rename.json(StatusCode::OK, json!([]));
    rename.json(
        StatusCode::CREATED,
        asset(51, "temporary", b"new", &public_url),
    );
    rename.json(
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({"message": "rename failed"}),
    );
    rename.bytes(StatusCode::NO_CONTENT, b"");
    assert!(
        client
            .replace_asset(4, "vibe.zip", "application/zip", b"new")
            .is_err()
    );
    let state = rename.state.lock().unwrap();
    assert_eq!(state.requests[2].method, "PATCH");
    assert_eq!(
        state.requests[3].uri,
        "/api/repos/vibevm/vibe/releases/assets/51"
    );
}

#[test]
fn replacement_removes_stale_different_digest_temporary_asset_before_upload() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    let bytes = b"new";
    let digest = sha256_digest(b"different interrupted payload");
    let prefix = digest.strip_prefix("sha256:").unwrap();
    let stale_name = format!(".vibe-upload-stale-{}-vibe.zip", &prefix[..12]);
    let other_name = format!(".vibe-upload-stale-{}-other-vibe.zip", &prefix[..12]);
    let public_url = format!("{}/downloads/vibe.zip", mock.base);
    mock.json(
        StatusCode::OK,
        json!([
            asset(60, &stale_name, bytes, &public_url),
            asset(61, "vibe.zip", b"old", &public_url),
            asset(63, &other_name, b"other", &public_url)
        ]),
    );
    mock.bytes(StatusCode::NO_CONTENT, b"");
    mock.json(
        StatusCode::CREATED,
        asset(62, "temporary", bytes, &public_url),
    );
    mock.bytes(StatusCode::NO_CONTENT, b"");
    mock.json(StatusCode::OK, asset(62, "vibe.zip", bytes, &public_url));
    client
        .replace_asset(4, "vibe.zip", "application/zip", bytes)
        .unwrap();
    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests[1].method, "DELETE");
    assert_eq!(
        state.requests[1].uri,
        "/api/repos/vibevm/vibe/releases/assets/60"
    );
    assert_eq!(state.requests[2].method, "POST");
    assert!(
        state
            .requests
            .iter()
            .all(|request| !request.uri.ends_with("/assets/63")),
        "a temporary upload for another canonical name must not be deleted"
    );
}

#[test]
fn asset_listing_follows_authenticated_same_endpoint_next_links() {
    let mock = MockGithub::spawn();
    let client = mock.client("draft-token");
    let next = format!(
        "{}/api/repos/vibevm/vibe/releases/70/assets?per_page=100&page=2",
        mock.base
    );
    mock.json_with_link(
        StatusCode::OK,
        json!([asset(71, "one", b"1", "ignored")]),
        format!("<{next}>; rel=\"next\""),
    );
    mock.json(StatusCode::OK, json!([asset(72, "two", b"2", "ignored")]));
    let assets = client.list_assets_authenticated(70).unwrap();
    assert_eq!(
        assets.iter().map(|asset| asset.id).collect::<Vec<_>>(),
        [71, 72]
    );
    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 2);
    assert!(state.requests[1].uri.ends_with("per_page=100&page=2"));
    for request in &state.requests {
        assert_eq!(request.headers["authorization"], "Bearer draft-token");
    }
}

#[test]
fn bounded_download_refuses_declared_and_chunked_overflow_without_full_buffering() {
    let mock = MockGithub::spawn();
    let client = mock.client("draft-token");
    let declared = client
        .download_asset_authenticated_bounded(80, 9, 8)
        .unwrap_err();
    assert!(matches!(
        declared,
        GithubReleaseError::DownloadSize {
            expected_size: 9,
            actual_size: 9,
            max_size: 8,
            ..
        }
    ));
    assert!(mock.state.lock().unwrap().requests.is_empty());
    mock.chunked_bytes(StatusCode::OK, b"body-that-is-much-longer-than-expected");
    let streamed = client
        .download_asset_authenticated_bounded(81, 3, 8)
        .unwrap_err();
    assert!(matches!(
        streamed,
        GithubReleaseError::DownloadSize {
            expected_size: 3,
            actual_size: 4,
            max_size: 8,
            ..
        }
    ));
}
