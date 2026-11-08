//! Release listings and directories per source (`PROP-019#release-sources`),
//! against loopback fakes only — never the real network.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use vibe_core::remap::drop::release_source_row;
use vibe_core::remap::{Layer, LoadedSource, RemapTarget};

use super::*;
use crate::commands::vvm::bundle::release_source::SourceOrigin;

type Routes = BTreeMap<String, (u16, String)>;

/// Serve `routes` (path with query → status and body) on loopback,
/// recording every request head.
fn serve(routes: Routes) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut head = Vec::new();
            let mut byte = [0_u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                head.push(byte[0]);
            }
            let head = String::from_utf8_lossy(&head).into_owned();
            let target = head.split_whitespace().nth(1).unwrap_or("/").to_owned();
            record.lock().unwrap().push(head);
            let (status, body) = routes.get(&target).cloned().unwrap_or((404, String::new()));
            let reply = format!(
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    (base, seen)
}

fn source(source: ReleaseSource) -> EffectiveReleaseSource {
    EffectiveReleaseSource {
        source,
        origin: SourceOrigin::SelfTable(None),
    }
}

fn locator_for(effective: &EffectiveReleaseSource) -> Locator {
    let rows = effective.row().into_iter().collect();
    Locator::from_sources(
        vec![LoadedSource {
            layer: Layer::Local,
            source: "[self] source".into(),
            rows,
            policy: Default::default(),
        }],
        Vec::new(),
    )
}

const TARGET: &str = "x86_64-unknown-linux-gnu";

#[test]
fn a_drop_lists_release_directories_and_bare_bundles_for_this_target() {
    let drop = tempfile::tempdir().unwrap();
    let root = drop.path();
    std::fs::create_dir_all(root.join("v1.0.11")).unwrap();
    std::fs::write(root.join("v1.0.11/DISTRIBUTIONS.json"), b"{}").unwrap();
    std::fs::create_dir_all(root.join("v1.0.12")).unwrap();
    std::fs::write(
        root.join("v1.0.12")
            .join(bundle_file_name("1.0.12", TARGET)),
        b"z",
    )
    .unwrap();
    std::fs::write(root.join(bundle_file_name("1.0.13", TARGET)), b"z").unwrap();
    // Not releases: an empty version directory, another target's zip, junk.
    std::fs::create_dir_all(root.join("v2.0.0")).unwrap();
    std::fs::write(
        root.join(bundle_file_name("3.0.0", "aarch64-apple-darwin")),
        b"z",
    )
    .unwrap();
    std::fs::write(root.join("notes.txt"), b"z").unwrap();
    let mut versions = scan(root, TARGET).unwrap();
    versions.sort();
    assert_eq!(
        versions,
        ["1.0.11", "1.0.12", "1.0.13"]
            .map(|v| semver::Version::parse(v).unwrap())
            .to_vec()
    );
    let missing = scan(&root.join("nowhere"), TARGET).unwrap_err().to_string();
    assert!(
        missing.contains("vibe self source set localdir"),
        "{missing}"
    );
}

#[test]
fn the_listing_accepts_its_three_spellings() {
    let expect = vec![
        semver::Version::new(1, 0, 13),
        semver::Version::new(1, 0, 12),
    ];
    for body in [
        r#"{"schema_version":1,"releases":[{"version":"1.0.13"},{"version":"v1.0.12"}]}"#,
        r#"{"releases":["1.0.13","1.0.12"]}"#,
        r#"["1.0.13",{"version":"1.0.12"}]"#,
    ] {
        assert_eq!(parse_listing(body.as_bytes()).unwrap(), expect, "{body}");
    }
}

#[test]
fn nexus_repository_paths_name_their_versions() {
    let (origin, repository, prefix) =
        nexus_repository("https://nexus.corp.example:8081/nexus/repository/raw-vibe/vibevm")
            .unwrap();
    assert_eq!(origin, "https://nexus.corp.example:8081/nexus");
    assert_eq!(repository, "raw-vibe");
    assert_eq!(prefix, "vibevm");
    assert_eq!(
        nexus_version("/vibevm/v1.0.13/DISTRIBUTIONS.json", "vibevm"),
        Some(semver::Version::new(1, 0, 13))
    );
    assert_eq!(
        nexus_version("vibevm/v1.0.13/vibevm-1.0.13-x.zip", "vibevm"),
        None
    );
    assert_eq!(
        nexus_version("other/v1.0.13/DISTRIBUTIONS.json", "vibevm"),
        None
    );
    assert!(nexus_repository("https://web.example/releases").is_none());
}

#[test]
fn a_raw_repository_lists_through_releases_json_then_nexus_search() {
    let page1 = r#"{"items":[{"path":"vibevm/v1.0.12/DISTRIBUTIONS.json"},{"path":"vibevm/v1.0.12/x.zip"}],"continuationToken":"next"}"#;
    let page2 =
        r#"{"items":[{"path":"vibevm/v1.0.13/DISTRIBUTIONS.json"}],"continuationToken":null}"#;
    let mut routes = Routes::new();
    routes.insert(
        "/service/rest/v1/search/assets?repository=raw".into(),
        (200, page1.into()),
    );
    routes.insert(
        "/service/rest/v1/search/assets?repository=raw&continuationToken=next".into(),
        (200, page2.into()),
    );
    let (base, _) = serve(routes);
    let effective = source(ReleaseSource::Nexus {
        base: format!("{base}/repository/raw/vibevm"),
    });
    let locator = locator_for(&effective);
    let api = ApiClient::new(None).unwrap();
    let lane = Releases::new(&effective, &api, &locator).unwrap();
    assert_eq!(
        lane.list().unwrap(),
        vec![
            semver::Version::new(1, 0, 13),
            semver::Version::new(1, 0, 12)
        ]
    );
    assert_eq!(
        lane.dir(&semver::Version::new(1, 0, 13)).unwrap(),
        ReleaseDir::Base(format!("{base}/repository/raw/vibevm/v1.0.13"))
    );

    let mut listed = Routes::new();
    listed.insert(
        "/rel/RELEASES.json".into(),
        (200, r#"{"releases":["1.0.11"]}"#.into()),
    );
    let (plain, _) = serve(listed);
    let effective = source(ReleaseSource::Nexus {
        base: format!("{plain}/rel"),
    });
    let locator = locator_for(&effective);
    let lane = Releases::new(&effective, &api, &locator).unwrap();
    assert!(
        matches!(lane.newest().unwrap(), Newest::Listed(v) if v == semver::Version::new(1, 0, 11))
    );

    // Neither a listing nor a Nexus search: a clear refusal, not a guess.
    let (bare, _) = serve(Routes::new());
    let effective = source(ReleaseSource::Nexus {
        base: format!("{bare}/rel"),
    });
    let locator = locator_for(&effective);
    let lane = Releases::new(&effective, &api, &locator).unwrap();
    let error = lane.list().unwrap_err().to_string();
    assert!(error.contains("vibe self install X.Y.Z"), "{error}");
}

#[test]
fn gitverse_lists_and_locates_through_its_api_with_the_token() {
    let release = |tag: &str, draft: bool| {
        format!(
            r#"{{"id":1,"tag_name":"{tag}","draft":{draft},"assets":[{{"name":"DISTRIBUTIONS.json","size":2,"browser_download_url":"https://gitverse.example/dl/{tag}/DISTRIBUTIONS.json"}}]}}"#
        )
    };
    let mut routes = Routes::new();
    routes.insert(
        "/repos/vibevm/vibevm/releases?page=1&per_page=30".into(),
        (
            200,
            format!(
                "[{},{},{}]",
                release("v1.0.13", false),
                release("v1.0.14", true),
                release("not-a-version", false)
            ),
        ),
    );
    routes.insert(
        "/repos/vibevm/vibevm/releases/tags/v1.0.13".into(),
        (200, release("v1.0.13", false)),
    );
    let (api_base, seen) = serve(routes);
    let effective = source(ReleaseSource::Gitverse {
        web: "https://gitverse.example/vibevm/vibevm".into(),
        api: api_base,
        owner: "vibevm".into(),
        repo: "vibevm".into(),
    });
    let locator = locator_for(&effective);
    let api = ApiClient::new(None).unwrap();
    let mut lane = Releases::new(&effective, &api, &locator).unwrap();
    lane.token = Some(Token::from_explicit("canary-gitverse-token"));
    assert_eq!(lane.list().unwrap(), vec![semver::Version::new(1, 0, 13)]);
    let dir = lane.dir(&semver::Version::new(1, 0, 13)).unwrap();
    assert_eq!(
        dir.asset("DISTRIBUTIONS.json").unwrap(),
        "https://gitverse.example/dl/v1.0.13/DISTRIBUTIONS.json"
    );
    let missing = dir.asset("vibevm-1.0.13-x.zip").unwrap_err().to_string();
    assert!(missing.contains("upload every file"), "{missing}");
    let heads = seen.lock().unwrap();
    assert!(heads.iter().all(|head| {
        let head = head.to_ascii_lowercase();
        head.contains("accept: application/vnd.gitverse.object+json;version=1")
            && head.contains("authorization: bearer canary-gitverse-token")
    }));
    // The token never reaches a display.
    assert!(!format!("{:?}", lane.token).contains("canary"));
}

#[test]
fn a_local_drop_is_located_as_a_file_url_and_listed_by_scan() {
    let drop = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(drop.path().join("v1.0.12")).unwrap();
    std::fs::write(drop.path().join("v1.0.12/DISTRIBUTIONS.json"), b"{}").unwrap();
    let effective = source(ReleaseSource::Localdir {
        path: drop.path().to_path_buf(),
    });
    let locator = locator_for(&effective);
    let api = ApiClient::new(None).unwrap();
    let lane = Releases::new(&effective, &api, &locator).unwrap();
    let dir = lane.dir(&semver::Version::new(1, 0, 12)).unwrap();
    assert_eq!(dir.local(), Some(drop.path().join("v1.0.12")));
    assert!(
        dir.asset("DISTRIBUTIONS.json")
            .unwrap()
            .starts_with("file://")
    );
    if super::super::current_target().is_ok() {
        assert_eq!(lane.list().unwrap(), vec![semver::Version::new(1, 0, 12)]);
    }
    // The release row lowers the GitHub download root only.
    let row = release_source_row(
        RemapTarget::Path(PathBuf::from("/srv")),
        Layer::Local,
        "[self] source",
        "test",
    );
    assert_eq!(row.key.what(), GITHUB_RELEASE_ROOT);
}
