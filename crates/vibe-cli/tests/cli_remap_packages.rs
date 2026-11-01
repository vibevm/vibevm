//! Public Linux fixture for remap **packages** (PROP-066 `#acceptance`,
//! the consent half): a remap package installed from a `file://` registry
//! into a scratch home as an offer, the question an unattended run refuses
//! with, the flags that answer it, the provider map two packages share,
//! the uninstall question, `remap test`, and `export` → `import` →
//! `--offline` install.
//!
//! The hit counter is the whole point here as well. A row that applied and
//! a row that merely failed look identical in an exit status; only "the
//! server was never asked" tells an accepted offer from an ignored one.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Serve `body` on a loopback port, counting every request that arrived.
fn serve(body: Vec<u8>) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => request.extend_from_slice(&buffer[..read]),
                }
            }
            counter.fetch_add(1, Ordering::SeqCst);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    (format!("http://{address}/dist/payload.txt"), hits)
}

/// A loopback address nothing listens on — what an unreachable target is.
fn closed_port() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}/dist/payload.txt")
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The scratch world: a `file://` registry holding one application package
/// and as many remap packages as a test plants, plus a digest-laid-out
/// artifact store the rows point at.
struct World {
    root: tempfile::TempDir,
    registry: PathBuf,
    store: PathBuf,
    digest: String,
    payload: Vec<u8>,
}

impl World {
    /// An application package whose single `sources` input is `url`.
    fn new(url: &str) -> Self {
        let payload = b"payload-bytes\n".to_vec();
        let digest = sha(&payload);
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let package = registry.join("org.example/remapped/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("vibe.toml"),
            format!(
                r#"[package]
name = "remapped"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "remapped"

[[installer_options]]
id = "src"
name = "Build from source"
platform = "linux"
platform_name = "Linux"
arch = "{arch}"
[installer_options.installation]
type = "source_build"
[[installer_options.installation.sources]]
name = "payload"
url = "{url}"
sha256 = "{digest}"
[installer_options.installation.recipe]
interpreter = "posix_sh"
script = "build.sh"

[[installer_options.launchers]]
id = "main"
name = "Remapped"
executable = "$app_bin/remapped"
kind = "command"
default = true
"#,
                arch = std::env::consts::ARCH,
            ),
        )
        .unwrap();
        fs::write(
            package.join("build.sh"),
            "set -e\nmkdir -p \"$VIBE_OUT/bin\"\n\
             cp \"$VIBE_SOURCES/payload/payload.txt\" \"$VIBE_OUT/bin/remapped\"\n\
             chmod 755 \"$VIBE_OUT/bin/remapped\"\n",
        )
        .unwrap();

        // The artifact store the rows redirect to, laid out exactly as the
        // application download cache is.
        let store = root.path().join("store");
        let entry = store.join(&digest);
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("payload.txt"), &payload).unwrap();

        World {
            root,
            registry,
            store,
            digest,
            payload,
        }
    }

    /// Plant a remap package carrying one `sha256` row pointing at the
    /// store, and nothing else — the shape `kind = "remap"` allows.
    fn remap_package(&self, key: &str, description: &str) {
        let (group, name) = key.split_once('/').unwrap();
        let dir = self.registry.join(group).join(name).join("v1.0.0");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("vibe.toml"),
            format!(
                r#"[package]
name = "{name}"
group = "{group}"
kind = "remap"
version = "1.0.0"

[remap]
description = "{description}"

[[remap.rows]]
sha256 = "{digest}"
path = "{store}"
reason = "the air-gapped artifact store"
"#,
                digest = self.digest,
                store = self.store.display(),
            ),
        )
        .unwrap();
    }

    /// A remap package whose row points at a host nothing listens on.
    fn unreachable_package(&self, key: &str, target: &str) {
        let (group, name) = key.split_once('/').unwrap();
        let dir = self.registry.join(group).join(name).join("v1.0.0");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("vibe.toml"),
            format!(
                r#"[package]
name = "{name}"
group = "{group}"
kind = "remap"
version = "1.0.0"

[[remap]]
sha256 = "{digest}"
url = "{target}"
reason = "the mirror that is supposed to be up"
"#,
                digest = self.digest,
            ),
        )
        .unwrap();
    }

    fn vibe(&self, user: &UserScratch, args: &[&str]) -> Output {
        let mut command = user.vibe();
        command.current_dir(self.root.path()).args(args);
        for variable in ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
            command.env_remove(variable);
        }
        command.output().unwrap()
    }

    /// `vibe install -g <package> --registry <dir>`, plus extra flags.
    fn install(&self, user: &UserScratch, package: &str, extra: &[&str]) -> Output {
        let registry = self.registry.to_str().unwrap();
        let mut args: Vec<&str> = vec!["install", "-g", package, "--registry", registry];
        args.extend_from_slice(extra);
        self.vibe(user, &args)
    }
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn failed(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The `vibe-remap/1` listing, as `--json` emits it.
fn listing(world: &World, user: &UserScratch) -> Value {
    let out = ok(&world.vibe(user, &["remap", "list", "--json", "--offline"]));
    serde_json::from_str(&out).unwrap_or_else(|error| panic!("{error}: {out}"))
}

/// The source block of one package in a listing.
fn block<'a>(listing: &'a Value, label: &str) -> &'a Value {
    listing["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["source"] == label)
        .unwrap_or_else(|| panic!("no `{label}` block in {listing}"))
}

#[test]
fn an_installed_remap_package_is_an_offer_and_applies_nothing() {
    let (url, hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let user = UserScratch::new();

    let installed = ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));
    assert!(
        installed.contains("Nothing applies yet"),
        "an install records an offer, never an obligation: {installed}"
    );

    let listing = listing(&world, &user);
    let offered = block(&listing, "org.corp/intranet@1.0.0");
    assert_eq!(offered["state"], "offered");
    assert_eq!(offered["rows"][0]["kind"], "sha256");
    assert_eq!(
        offered["rows"][0]["providers"][0],
        "org.corp/intranet@1.0.0"
    );

    // Nothing applies: the application's input still comes from the
    // declared origin, so the server IS asked.
    ok(&world.install(
        &user,
        "org.example/remapped",
        &["--unattended", "--reject-remap"],
    ));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "an unaccepted offer redirects nothing"
    );
}

#[test]
fn an_unattended_run_refuses_an_unanswered_offer_and_names_the_verb() {
    let (url, hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));

    let output = world.install(&user, "org.example/remapped", &["--unattended"]);
    let error = failed(&output);
    assert_eq!(
        output.status.code(),
        Some(2),
        "a refusal before any effect exits 2; stderr={error}"
    );
    assert!(error.contains("consent_required"), "{error}");
    assert!(
        error.contains("vibe remap accept org.corp/intranet"),
        "the refusal names the command that answers it: {error}"
    );
    assert!(error.contains("PROP-066#consent"), "{error}");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "the refusal comes before any fetch"
    );
}

#[test]
fn accept_remap_on_the_install_applies_the_row_without_touching_the_server() {
    let (url, hits) = serve(b"never-served\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));

    // `--accept-remap` answers the question for this run and records it.
    ok(&world.install(
        &user,
        "org.example/remapped",
        &["--unattended", "--accept-remap"],
    ));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "the accepted row served the bytes from the store"
    );

    let listing = listing(&world, &user);
    assert_eq!(
        block(&listing, "org.corp/intranet@1.0.0")["state"],
        "accepted"
    );

    // And the answer is remembered: a second run neither asks nor refuses.
    let second = world.vibe(&user, &["remap", "list", "--offline"]);
    assert!(second.status.success());
}

#[test]
fn a_rejected_package_is_not_asked_about_again() {
    let (url, hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));
    ok(&world.vibe(&user, &["remap", "reject", "org.corp/intranet"]));

    let listing = listing(&world, &user);
    assert_eq!(
        block(&listing, "org.corp/intranet@1.0.0")["state"],
        "rejected"
    );

    // An unattended install now proceeds: the question has an answer.
    ok(&world.install(&user, "org.example/remapped", &["--unattended"]));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "a rejected row redirects nothing"
    );
}

#[test]
fn disable_and_enable_switch_an_acceptance_without_forgetting_it() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));
    ok(&world.vibe(&user, &["remap", "accept", "org.corp/intranet"]));

    ok(&world.vibe(&user, &["remap", "disable", "org.corp/intranet"]));
    let off = listing(&world, &user);
    assert_eq!(block(&off, "org.corp/intranet@1.0.0")["state"], "disabled");

    ok(&world.vibe(&user, &["remap", "enable", "org.corp/intranet"]));
    let on = listing(&world, &user);
    assert_eq!(block(&on, "org.corp/intranet@1.0.0")["state"], "accepted");
}

#[test]
fn two_packages_offering_one_row_both_appear_as_its_providers() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    world.remap_package("org.corp/second", "the same store, said twice");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));
    ok(&world.install(&user, "org.corp/second", &["--unattended"]));
    ok(&world.vibe(
        &user,
        &["remap", "accept", "org.corp/intranet", "org.corp/second"],
    ));

    let listing = listing(&world, &user);
    let providers = block(&listing, "org.corp/intranet@1.0.0")["rows"][0]["providers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        providers,
        ["org.corp/intranet@1.0.0", "org.corp/second@1.0.0"],
        "key, target and scope are equal, so it is one row with two providers"
    );

    // The provider map is on disk, derived and rebuilt.
    let map = user.settings.join("remap/providers.json");
    let body = fs::read_to_string(&map).unwrap();
    assert!(body.contains("vibe-remap-providers/1"), "{body}");
}

#[test]
fn uninstalling_one_of_two_providers_keeps_the_row_the_other_offers() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    world.remap_package("org.corp/second", "the same store, said twice");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));
    ok(&world.install(&user, "org.corp/second", &["--unattended"]));
    ok(&world.vibe(
        &user,
        &["remap", "accept", "org.corp/intranet", "org.corp/second"],
    ));

    // Nothing is orphaned, so nothing is asked and nothing is kept.
    let removed = ok(&world.vibe(
        &user,
        &["uninstall", "-g", "org.corp/second", "--unattended"],
    ));
    assert!(removed.contains("org.corp/second@1.0.0"), "{removed}");
    let listing = listing(&world, &user);
    assert_eq!(
        block(&listing, "org.corp/intranet@1.0.0")["rows"][0]["providers"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "the surviving provider is the only one left"
    );
}

#[test]
fn uninstalling_the_last_provider_refuses_unattended_and_keeps_rows_on_request() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));
    ok(&world.vibe(&user, &["remap", "accept", "org.corp/intranet"]));

    // Unattended, with neither flag: the question is refused, not guessed.
    let output = world.vibe(
        &user,
        &["uninstall", "-g", "org.corp/intranet", "--unattended"],
    );
    let error = failed(&output);
    assert_eq!(output.status.code(), Some(2), "stderr={error}");
    assert!(error.contains("consent_required"), "{error}");
    assert!(error.contains("--keep-remap"), "{error}");

    // `--keep-remap` copies the orphan into the user's own local layer,
    // carrying the package it outlived and the original reason.
    ok(&world.vibe(
        &user,
        &[
            "uninstall",
            "-g",
            "org.corp/intranet",
            "--unattended",
            "--keep-remap",
        ],
    ));
    let settings = fs::read_to_string(user.settings.join("settings.toml")).unwrap();
    assert!(settings.contains("[[remap]]"), "{settings}");
    assert!(settings.contains(&world.digest), "{settings}");
    assert!(
        settings.contains("from = \"org.corp/intranet@1.0.0\""),
        "the kept row says where it came from: {settings}"
    );
    assert!(
        settings.contains("the air-gapped artifact store"),
        "and keeps the package's own reason: {settings}"
    );

    // The row is now a local row, and it still applies — with no package
    // behind it and no question to answer.
    let listing = listing(&world, &user);
    let local = listing["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["layer"] == "local" && !source["rows"].as_array().unwrap().is_empty())
        .unwrap_or_else(|| panic!("no local block in {listing}"));
    assert_eq!(local["rows"][0]["from"], "org.corp/intranet@1.0.0");
}

#[test]
fn drop_remap_lets_the_orphaned_rows_go_with_the_package() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));
    ok(&world.vibe(&user, &["remap", "accept", "org.corp/intranet"]));
    ok(&world.vibe(
        &user,
        &[
            "uninstall",
            "-g",
            "org.corp/intranet",
            "--unattended",
            "--drop-remap",
        ],
    ));
    let settings = user.settings.join("settings.toml");
    assert!(
        !settings.exists() || !fs::read_to_string(&settings).unwrap().contains("[[remap]]"),
        "nothing was kept"
    );
    let listing = listing(&world, &user);
    assert!(
        listing["sources"]
            .as_array()
            .unwrap()
            .iter()
            .all(|source| source["source"] != "org.corp/intranet@1.0.0"),
        "the package is gone from the catalog: {listing}"
    );
}

#[test]
fn remap_test_reports_an_unreachable_target_and_exits_one() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    let closed = closed_port();
    world.unreachable_package("org.corp/down", &closed);
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/down", &["--unattended"]));
    ok(&world.vibe(&user, &["remap", "accept", "org.corp/down"]));
    // One installed application, so the home walk has an input to resolve
    // through the row.
    ok(&world.install(
        &user,
        "org.example/remapped",
        &["--unattended", "--accept-remap"],
    ));

    let output = world.vibe(&user, &["remap", "test", "--json"]);
    let body = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unreachable target exits 1; stdout={body} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_str(&body).unwrap_or_else(|e| panic!("{e}: {body}"));
    assert_eq!(document["protocol"], "vibe-remap-test/1");
    assert_eq!(document["scope"], "home");
    assert!(
        document["unreachable"].as_u64().unwrap() >= 1,
        "the closed port is reported: {document}"
    );
    // The probe of interest is the one the accepted row chose — the
    // closed port — not whatever else a host without a network cannot
    // reach.
    let probe = document["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|probe| {
            probe["row"]
                .as_str()
                .is_some_and(|row| row.contains("org.corp/down"))
        })
        .unwrap_or_else(|| panic!("no probe chose the row: {document}"));
    assert_eq!(probe["outcome"], "unreachable");
    assert_eq!(probe["probe"], "head");
    assert_eq!(probe["target"], closed);
    assert!(probe["error"].is_string(), "the error text is reported");

    // Every view then states what the target answered, and when
    // (`PROP-066#diagnostics-reachability`).
    let listing = listing(&world, &user);
    let row = &block(&listing, "org.corp/down@1.0.0")["rows"][0];
    assert!(
        row["probed"]
            .as_str()
            .is_some_and(|note| note.starts_with("unreachable at ")),
        "the row carries the last probe: {listing}"
    );
}

#[test]
fn export_then_import_carries_the_package_and_its_artifacts_into_a_second_home() {
    let (url, hits) = serve(b"never-served\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let first = UserScratch::new();
    ok(&world.install(&first, "org.corp/intranet", &["--unattended"]));
    ok(&world.vibe(&first, &["remap", "accept", "org.corp/intranet"]));
    // Warm the first home's download cache, so the export has bytes to
    // carry: the row's digest is what names them.
    ok(&world.install(
        &first,
        "org.example/remapped",
        &["--unattended", "--accept-remap"],
    ));
    assert_eq!(hits.load(Ordering::SeqCst), 0);

    let bundle = world.root.path().join("bundle");
    let exported = ok(&world.vibe(
        &first,
        &[
            "remap",
            "export",
            "org.corp/intranet",
            "--out",
            bundle.to_str().unwrap(),
            "--with-artifacts",
        ],
    ));
    assert!(exported.contains("file://"), "{exported}");
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(bundle.join("MANIFEST.json")).unwrap()).unwrap();
    assert_eq!(manifest["protocol"], "vibe-remap-bundle/1");
    assert_eq!(manifest["package"], "org.corp/intranet");
    assert!(
        manifest["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact["sha256"] == world.digest),
        "the row's artifact travels with the package: {manifest}"
    );
    // The bundle is a registry as well as an artifact tree.
    assert!(
        bundle.join("org.corp/intranet/v1.0.0/vibe.toml").is_file(),
        "the bundle is a `file://` registry"
    );

    // A second, empty home imports it and accepts it in one go.
    let second = UserScratch::new();
    let imported = ok(&world.vibe(
        &second,
        &[
            "remap",
            "import",
            bundle.to_str().unwrap(),
            "--accept-remap",
        ],
    ));
    assert!(imported.contains("org.corp/intranet@1.0.0"), "{imported}");
    let listing = listing(&world, &second);
    assert_eq!(
        block(&listing, "org.corp/intranet@1.0.0")["state"],
        "accepted"
    );

    // And now an `--offline` install succeeds in the second home with the
    // network never touched: the row points at the store directory, and
    // the artifact is in the download cache the import filled.
    let before = hits.load(Ordering::SeqCst);
    ok(&world.install(
        &second,
        "org.example/remapped",
        &["--unattended", "--offline"],
    ));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        before,
        "an offline install from the bundle asks no server"
    );
}

#[test]
fn a_remap_kind_package_may_carry_nothing_executable() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    let dir = world.registry.join("org.corp/bad/v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            r#"[package]
name = "bad"
group = "org.corp"
kind = "remap"
version = "1.0.0"

[[binary]]
name = "mischief"
crate = "mischief"

[[remap]]
sha256 = "{digest}"
path = "{store}"
"#,
            digest = world.digest,
            store = world.store.display(),
        ),
    )
    .unwrap();
    let user = UserScratch::new();
    let error = failed(&world.install(&user, "org.corp/bad", &["--unattended"]));
    assert!(error.contains("[[binary]]"), "{error}");
    assert!(error.contains("PROP-066#package"), "{error}");
}

/// `PROP-066#provenance`: the sidecar records only what was redirected, so
/// an ordinary install writes none at all.
#[test]
fn an_install_from_the_canonical_origin_writes_no_provenance_sidecar() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    let user = UserScratch::new();
    let project = world.root.path().join("project");
    fs::create_dir_all(&project).unwrap();
    user.init_project(&project);
    assert!(
        !project.join("vibe.lock.provenance").exists(),
        "nothing was redirected, so nothing is recorded"
    );
}

/// The three files of a home live under `<home>/remap/`, and the ledger is
/// the `vibe-remap-consent/1` document `PROP-066#consent` names.
#[test]
fn the_homes_remap_state_lives_in_three_named_files() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    world.remap_package("org.corp/intranet", "the corp network");
    let user = UserScratch::new();
    ok(&world.install(&user, "org.corp/intranet", &["--unattended"]));
    ok(&world.vibe(&user, &["remap", "accept", "org.corp/intranet"]));

    let remap = user.settings.join("remap");
    for name in ["consent.json", "providers.json", "installed.json"] {
        assert!(remap.join(name).is_file(), "`{name}` is written");
    }
    let ledger = fs::read_to_string(remap.join("consent.json")).unwrap();
    assert!(ledger.contains("vibe-remap-consent/1"), "{ledger}");
    assert!(ledger.contains("org.corp/intranet"), "{ledger}");
    // The package's own tree is kept, so `export` works with no registry
    // in reach.
    assert!(
        remap
            .join("packages/org.corp__intranet/vibe.toml")
            .is_file(),
        "the home keeps the package tree"
    );
}

/// A sanity check on the fixture itself: the payload really is what the
/// digest says, so a "the server was never asked" assertion means what it
/// claims.
#[test]
fn the_fixture_payload_matches_its_declared_digest() {
    let (url, _hits) = serve(b"payload-bytes\n".to_vec());
    let world = World::new(&url);
    assert_eq!(sha(&world.payload), world.digest);
    assert!(
        world
            .store
            .join(&world.digest)
            .join("payload.txt")
            .is_file()
    );
    let _: &Path = world.registry.as_path();
}

/// `PROP-066#locator`: an `archive` input is cached across runs by its
/// digest exactly as a `sources` input is, so a second install after the
/// work directory is wiped makes no server hit at all.
#[test]
fn an_archive_input_is_cached_across_runs_by_its_digest() {
    let body = tar_gz("bin/tool", "#!/bin/sh\necho tool\n");
    let digest = sha(&body);
    let (url, hits) = serve_archive(body);
    let root = tempfile::tempdir().unwrap();
    let registry = root.path().join("registry");
    let package = registry.join("org.example/archived/v1.0.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            r#"[package]
name = "archived"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "archived"

[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "{arch}"
[installer_options.installation]
type = "tar_gz"
source = "https://archive.invalid/tool.tar.gz"
sha256 = "{digest}"

[[installer_options.launchers]]
id = "main"
name = "Archived"
executable = "$app_bin/tool"
kind = "command"
default = true
"#,
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();

    let user = UserScratch::new();
    // An `archive` source must be an HTTPS URL or a package-relative path,
    // so the loopback server is reached through a local `sha256` row —
    // which is the shape a closed network uses anyway.
    fs::write(
        user.settings.join("settings.toml"),
        format!("[[remap]]\nsha256 = \"{digest}\"\nurl = \"{url}\"\n"),
    )
    .unwrap();
    let run = |args: &[&str]| {
        let mut command = user.vibe();
        command.current_dir(root.path()).args(args);
        for variable in ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
            command.env_remove(variable);
        }
        command.output().unwrap()
    };
    let registry = registry.to_str().unwrap().to_owned();
    ok(&run(&[
        "install",
        "-g",
        "org.example/archived",
        "--registry",
        &registry,
        "--unattended",
    ]));
    assert_eq!(hits.load(Ordering::SeqCst), 1, "the first install fetched");
    // The bytes are in the shared download cache, keyed by digest alone.
    assert!(
        user.settings
            .join("cache/app-sources")
            .join(&digest)
            .is_dir(),
        "the archive input is cached by digest"
    );

    // Wipe the work directory and the installation, then install again.
    ok(&run(&["app", "work", "--all", "--clean", "--unattended"]));
    ok(&run(&[
        "uninstall",
        "-g",
        "org.example/archived",
        "--unattended",
    ]));
    ok(&run(&[
        "install",
        "-g",
        "org.example/archived",
        "--registry",
        &registry,
        "--unattended",
    ]));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "the second install read the cache and asked no server"
    );
}

/// A tar.gz holding one file.
fn tar_gz(name: &str, body: &str) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    tar.append_data(&mut header, name, body.as_bytes()).unwrap();
    tar.into_inner().unwrap().finish().unwrap()
}

/// The same counting server, serving an archive under an archive name.
fn serve_archive(body: Vec<u8>) -> (String, Arc<AtomicUsize>) {
    let (url, hits) = serve(body);
    (url.replace("payload.txt", "tool.tar.gz"), hits)
}
