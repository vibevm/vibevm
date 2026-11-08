use super::client::{
    ASSETS_PER_PAGE, GITVERSE_ACCEPT, PlannedBody, RELEASES_PER_PAGE, multipart_body,
};
use super::source::{expected_assets, from_dir, from_github, verify};
use super::*;
use crate::dist::release::tests::{COMMIT, platform};

use std::io::{BufRead, BufReader, Read as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use vibe_publish::release_manifest::{
    DISTRIBUTION_BASH_INSTALLER_FILENAME, DISTRIBUTION_CMD_INSTALLER_FILENAME,
    DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
};
use vibe_publish::{AggregateDistributionManifest, SUPPORTED_DISTRIBUTION_TARGETS};

const TOKEN: &str = "gv-test-token-0f9e8d7c";
const REPO: &str = "/repos/vibevm/vibevm";

fn version() -> Version {
    Version::parse("1.0.0").unwrap()
}

fn fixture_files() -> BTreeMap<String, Vec<u8>> {
    let version = version();
    let mut files = BTreeMap::new();
    let mut fragments = Vec::new();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
        files.insert(fragment.asset.name.clone(), bundle);
        files.insert(fragment.bootstrap.name.clone(), bootstrap);
        files.insert(
            crate::dist::build::fragment_asset_name(&version, target),
            fragment_bytes,
        );
        fragments.push(fragment);
    }
    let aggregate = AggregateDistributionManifest::from_platforms(fragments).unwrap();
    files.insert(
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME.to_string(),
        aggregate.to_json_bytes().unwrap(),
    );
    files.insert(
        DISTRIBUTION_BASH_INSTALLER_FILENAME.to_string(),
        b"#!/bin/sh\n".to_vec(),
    );
    files.insert(
        DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME.to_string(),
        b"Write-Host\n".to_vec(),
    );
    files.insert(
        DISTRIBUTION_CMD_INSTALLER_FILENAME.to_string(),
        b"@echo off\n".to_vec(),
    );
    files
}

fn fixture() -> SourceRelease {
    verify(&version(), fixture_files()).unwrap()
}

// ---------------------------------------------------------------- requests

fn endpoint() -> Endpoint {
    Endpoint::new("https://api.gitverse.ru/", "vibevm", "vibevm").unwrap()
}

#[test]
fn release_requests_follow_the_documented_paths() {
    let endpoint = endpoint();
    let by_tag = endpoint.release_by_tag("v1.0.0");
    assert_eq!(by_tag.method, "GET");
    assert_eq!(
        by_tag.url.as_str(),
        "https://api.gitverse.ru/repos/vibevm/vibevm/releases/tags/v1.0.0"
    );
    assert_eq!(
        endpoint.list_releases(2).url.as_str(),
        format!(
            "https://api.gitverse.ru/repos/vibevm/vibevm/releases?page=2&per_page={RELEASES_PER_PAGE}"
        )
    );
    assert_eq!(
        endpoint.list_assets(7, 1).url.as_str(),
        format!(
            "https://api.gitverse.ru/repos/vibevm/vibevm/releases/7/assets?page=1&per_page={ASSETS_PER_PAGE}"
        )
    );
    let delete = endpoint.delete_asset(7, 9);
    assert_eq!(delete.method, "DELETE");
    assert_eq!(
        delete.url.as_str(),
        "https://api.gitverse.ru/repos/vibevm/vibevm/releases/7/assets/9"
    );
}

#[test]
fn tag_path_segments_are_percent_encoded() {
    let url = endpoint().release_by_tag("v1/../x").url;
    assert_eq!(
        url.as_str(),
        "https://api.gitverse.ru/repos/vibevm/vibevm/releases/tags/v1%2F..%2Fx"
    );
}

#[test]
fn create_release_body_is_public_and_bound_to_the_commit() {
    let source = fixture();
    let planned = endpoint()
        .create_release(&release_request(&source))
        .unwrap();
    assert_eq!(planned.method, "POST");
    assert_eq!(
        planned.url.as_str(),
        "https://api.gitverse.ru/repos/vibevm/vibevm/releases"
    );
    let PlannedBody::Json(body) = planned.body else {
        panic!("create carries JSON");
    };
    assert_eq!(body["tag_name"], "v1.0.0");
    assert_eq!(body["name"], "vibevm 1.0.0");
    assert_eq!(body["target_commitish"], COMMIT);
    assert_eq!(body["draft"], false);
    assert_eq!(body["prerelease"], false);
    assert_eq!(body["is_authorized_only"], false);
}

#[test]
fn upload_request_names_the_asset_in_the_query_and_the_attachment_field() {
    let planned = endpoint().upload_asset(
        7,
        "vibevm-1.0.0-x86_64-unknown-linux-musl.zip",
        "application/zip",
        3,
        "sha256:abc",
    );
    assert_eq!(planned.method, "POST");
    assert_eq!(
        planned.url.as_str(),
        "https://api.gitverse.ru/repos/vibevm/vibevm/releases/7/assets?name=vibevm-1.0.0-x86_64-unknown-linux-musl.zip"
    );
    assert_eq!(
        planned.body,
        PlannedBody::Multipart {
            field: "attachment",
            file_name: "vibevm-1.0.0-x86_64-unknown-linux-musl.zip".to_string(),
            content_type: "application/zip".to_string(),
            size: 3,
            digest: "sha256:abc".to_string(),
        }
    );
}

#[test]
fn multipart_body_is_one_attachment_part() {
    let (content_type, body) = multipart_body(
        "a.zip",
        "application/zip",
        "sha256:0123456789abcdef0123456789abcdef0123",
        b"xyz",
    );
    let boundary = "vibevm-xtask-0123456789abcdef0123456789abcdef";
    assert_eq!(
        content_type,
        format!("multipart/form-data; boundary={boundary}")
    );
    assert_eq!(
        String::from_utf8(body).unwrap(),
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"attachment\"; \
             filename=\"a.zip\"\r\nContent-Type: application/zip\r\n\r\nxyz\r\n--{boundary}--\r\n"
        )
    );
}

#[test]
fn endpoint_rejects_unsafe_configuration() {
    assert!(Endpoint::new("ftp://api.gitverse.ru", "vibevm", "vibevm").is_err());
    assert!(Endpoint::new("https://user:pw@api.gitverse.ru", "vibevm", "vibevm").is_err());
    assert!(Endpoint::new("https://api.gitverse.ru?x=1", "vibevm", "vibevm").is_err());
    assert!(Endpoint::new("https://api.gitverse.ru", "..", "vibevm").is_err());
    assert!(Endpoint::new("https://api.gitverse.ru", "vibevm", "a/b").is_err());
    let onprem = Endpoint::new("https://git.corp.example/api/", "tools", "vibevm").unwrap();
    assert_eq!(
        onprem.release_by_tag("v1.0.0").url.as_str(),
        "https://git.corp.example/api/repos/tools/vibevm/releases/tags/v1.0.0"
    );
}

#[test]
fn presence_is_decided_by_name_then_size() {
    let source = fixture();
    let asset = &source.assets[0];
    assert_eq!(presence(asset, None), Presence::Absent);
    let same = GitverseAsset {
        id: 4,
        name: asset.name.clone(),
        size: asset.bytes.len() as u64,
        browser_download_url: None,
    };
    assert_eq!(
        presence(asset, Some(&same)),
        Presence::SameSize { asset_id: 4 }
    );
    let other = GitverseAsset { size: 1, ..same };
    assert_eq!(
        presence(asset, Some(&other)),
        Presence::DifferentSize {
            asset_id: 4,
            remote_size: 1
        }
    );
}

// ------------------------------------------------------------------ source

#[test]
fn source_set_is_complete_with_the_manifest_last() {
    let source = fixture();
    assert_eq!(
        source.assets.len(),
        SUPPORTED_DISTRIBUTION_TARGETS.len() * 3 + 4
    );
    assert_eq!(
        source.assets.last().unwrap().name,
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME
    );
    assert_eq!(source.tag, "v1.0.0");
    assert_eq!(source.source_commit, COMMIT);
    assert_eq!(expected_assets(&version()).len(), source.assets.len());
    for asset in &source.assets {
        assert_eq!(asset.digest, sha256_digest(&asset.bytes));
    }
}

#[test]
fn source_rejects_tampered_missing_or_foreign_assets() {
    let mut tampered = fixture_files();
    let bundle =
        crate::dist::build::bundle_asset_name(&version(), SUPPORTED_DISTRIBUTION_TARGETS[0]);
    tampered.get_mut(&bundle).unwrap().push(0);
    assert!(verify(&version(), tampered).is_err());

    let mut missing = fixture_files();
    missing.remove(DISTRIBUTION_CMD_INSTALLER_FILENAME);
    assert!(verify(&version(), missing).is_err());

    assert!(verify(&Version::parse("1.0.1").unwrap(), fixture_files()).is_err());
}

#[test]
fn source_reads_a_local_release_directory() {
    let temp = tempfile::tempdir().unwrap();
    for (name, bytes) in fixture_files() {
        std::fs::write(temp.path().join(name), bytes).unwrap();
    }
    std::fs::write(temp.path().join("unrelated.txt"), b"ignored").unwrap();
    assert_eq!(from_dir(temp.path(), &version()).unwrap(), fixture());
}

mod fake;
mod integration;
