//! Public Linux fixture for remap (PROP-066 `#acceptance`, the
//! consent-free half): an application `sources` input served from a
//! directory by a `sha256` row with the server never touched, a `prefix`
//! row rewriting one server's URL to a second, a `deny` row refusing
//! before any connection, `[policy] unmatched = "deny"` refusing an
//! unlisted URL, `--remap` beating a settings row, and the two read-only
//! views.
//!
//! Every fetch here is verified against the same digest it always was.
//! That is what the hit counters are for: a row that redirected a fetch
//! and a row that merely failed look identical in the exit status, and
//! only "the server was never asked" distinguishes them.

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

/// Serve `body` on a loopback port and count every request that reached
/// it. The count is the whole point: "no connection was made" is the
/// claim `deny` and `sha256`-from-a-directory have to earn.
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

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// A one-step application package whose single `sources` input is the
/// declared URL, plus a recipe that copies it into the payload.
struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
    package: PathBuf,
}

impl Fixture {
    fn new(url: &str, digest: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let package = registry.join("org.example/remapped/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        let body = format!(
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
        );
        fs::write(package.join("vibe.toml"), body).unwrap();
        fs::write(
            package.join("build.sh"),
            "set -e\nmkdir -p \"$VIBE_OUT/bin\"\n\
             cp \"$VIBE_SOURCES/payload/payload.txt\" \"$VIBE_OUT/bin/remapped\"\n\
             chmod 755 \"$VIBE_OUT/bin/remapped\"\n",
        )
        .unwrap();
        Self {
            root,
            registry,
            package,
        }
    }

    fn vibe(&self, user: &UserScratch, args: &[&str]) -> Output {
        let mut command = user.vibe();
        command.current_dir(self.root.path()).args(args);
        for variable in ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
            command.env_remove(variable);
        }
        command.output().unwrap()
    }

    fn install(&self, user: &UserScratch, extra: &[&str]) -> Output {
        let registry = self.registry.to_str().unwrap();
        let mut args = extra.to_vec();
        args.extend_from_slice(&["install", "-g", "org.example/remapped"]);
        args.extend_from_slice(&["--registry", registry]);
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

/// Write `~/.vibe/settings.toml` with a `[[remap]]` body.
fn settings(user: &UserScratch, body: &str) {
    fs::write(user.settings.join("settings.toml"), body).unwrap();
}

/// Lay out a download-cache-shaped directory: `<dir>/<sha256>/<filename>`.
fn by_digest(dir: &Path, digest: &str, filename: &str, bytes: &[u8]) -> PathBuf {
    let entry = dir.join(digest);
    fs::create_dir_all(&entry).unwrap();
    fs::write(entry.join(filename), bytes).unwrap();
    dir.to_path_buf()
}

#[test]
fn a_sha256_row_serves_an_input_from_a_directory_without_touching_the_server() {
    let payload = b"payload-bytes\n";
    let digest = sha(payload);
    let (url, hits) = serve(payload.to_vec());
    let fixture = Fixture::new(&url, &digest);
    let user = UserScratch::new();

    let store = fixture.root.path().join("store");
    by_digest(&store, &digest, "payload.txt", payload);
    settings(
        &user,
        &format!(
            "[[remap]]\nsha256 = \"{digest}\"\npath = \"{}\"\n\
             reason = \"the air-gapped artifact store\"\n",
            store.display()
        ),
    );

    ok(&fixture.install(&user, &["--unattended"]));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "the row served the bytes; the server was never asked"
    );

    // The row that served the bytes is recorded beside them, and the
    // package's canonical URL is untouched.
    let manifest = fs::read_to_string(fixture.package.join("vibe.toml")).unwrap();
    assert!(manifest.contains(&url), "the manifest keeps its own URL");
}

#[test]
fn a_prefix_row_rewrites_one_servers_url_to_a_second_server() {
    let payload = b"payload-bytes\n";
    let digest = sha(payload);
    let (declared, declared_hits) = serve(b"never-served\n".to_vec());
    let (actual, actual_hits) = serve(payload.to_vec());
    // The declared URL's digest is the payload the *second* server has, so
    // only a successful rewrite can satisfy it.
    let fixture = Fixture::new(&declared, &digest);
    let user = UserScratch::new();

    let declared_base = declared.trim_end_matches("/dist/payload.txt").to_owned();
    let actual_base = actual.trim_end_matches("/dist/payload.txt").to_owned();
    settings(
        &user,
        &format!(
            "[[remap]]\nprefix = \"{declared_base}/\"\nurl = \"{actual_base}/\"\n\
             reason = \"the internal mirror\"\n"
        ),
    );

    ok(&fixture.install(&user, &["--unattended"]));
    assert_eq!(
        actual_hits.load(Ordering::SeqCst),
        1,
        "the rewrite was used"
    );
    assert_eq!(
        declared_hits.load(Ordering::SeqCst),
        0,
        "the declared host was never asked"
    );
}

#[test]
fn a_deny_row_refuses_before_any_connection() {
    let payload = b"payload-bytes\n";
    let digest = sha(payload);
    let (url, hits) = serve(payload.to_vec());
    let fixture = Fixture::new(&url, &digest);
    let user = UserScratch::new();
    settings(
        &user,
        &format!(
            "[[remap]]\nsha256 = \"{digest}\"\ndeny = true\n\
             reason = \"policy: this artifact is not approved\"\n"
        ),
    );

    let output = fixture.install(&user, &["--unattended"]);
    let error = failed(&output);
    assert_eq!(
        output.status.code(),
        Some(2),
        "a refusal before any effect exits 2; stderr={error}"
    );
    assert!(error.contains("remap_denied"), "{error}");
    assert!(
        error.contains("policy: this artifact is not approved"),
        "the refusal carries the row's reason: {error}"
    );
    assert!(error.contains("PROP-066#row"), "{error}");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "a deny refuses before any connection"
    );
}

#[test]
fn an_unmatched_deny_policy_refuses_an_unlisted_url() {
    let payload = b"payload-bytes\n";
    let digest = sha(payload);
    let (url, hits) = serve(payload.to_vec());
    let fixture = Fixture::new(&url, &digest);
    let user = UserScratch::new();
    // One row that matches nothing in this run, and the strict policy.
    settings(
        &user,
        "[policy]\nunmatched = \"deny\"\n\
         [[remap]]\nprefix = \"https://nowhere.invalid/\"\nurl = \"https://elsewhere.invalid/\"\n",
    );

    let output = fixture.install(&user, &["--unattended"]);
    let error = failed(&output);
    assert_eq!(output.status.code(), Some(2), "stderr={error}");
    assert!(error.contains("remap_denied"), "{error}");
    assert!(error.contains("unmatched"), "{error}");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "the policy refuses before any connection"
    );
}

#[test]
fn a_remap_flag_beats_a_settings_row() {
    let payload = b"payload-bytes\n";
    let digest = sha(payload);
    let (url, hits) = serve(payload.to_vec());
    let fixture = Fixture::new(&url, &digest);
    let user = UserScratch::new();

    // The settings row points at a directory that holds nothing, so only
    // the invocation row can satisfy the digest.
    let empty = fixture.root.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let store = fixture.root.path().join("store");
    by_digest(&store, &digest, "payload.txt", payload);
    settings(
        &user,
        &format!(
            "[[remap]]\nsha256 = \"{digest}\"\npath = \"{}\"\nreason = \"the machine default\"\n",
            empty.display()
        ),
    );
    let invocation = fixture.root.path().join("closed.toml");
    fs::write(
        &invocation,
        format!(
            "[[remap]]\nsha256 = \"{digest}\"\npath = \"{}\"\nreason = \"this run only\"\n",
            store.display()
        ),
    )
    .unwrap();

    ok(&fixture.install(
        &user,
        &["--unattended", "--remap", invocation.to_str().unwrap()],
    ));
    assert_eq!(hits.load(Ordering::SeqCst), 0);

    // And the chain says why, in winning order.
    let explain = ok(&fixture.vibe(
        &user,
        &[
            "--offline",
            "--remap",
            invocation.to_str().unwrap(),
            "remap",
            "explain",
            &format!("sha256:{digest}"),
        ],
    ));
    let this_run = explain.find("this run only").expect("the invocation row");
    let machine = explain
        .find("the machine default")
        .expect("the settings row");
    assert!(
        this_run < machine,
        "the invocation layer wins and is printed first:\n{explain}"
    );
}

#[test]
fn explain_lists_two_competing_rows_in_winning_order() {
    let fixture = Fixture::new("https://example.invalid/a", &"ab".repeat(32));
    let user = UserScratch::new();
    settings(
        &user,
        "[[remap]]\nprefix = \"https://github.com/\"\nurl = \"https://wide.invalid/\"\n\
         reason = \"everything on github\"\n\
         [[remap]]\nprefix = \"https://github.com/vibespecs/\"\nurl = \"https://narrow.invalid/\"\n\
         reason = \"just the registry\"\n",
    );
    let explain = ok(&fixture.vibe(
        &user,
        &[
            "--offline",
            "remap",
            "explain",
            "https://github.com/vibespecs/index",
        ],
    ));
    let narrow = explain
        .find("just the registry")
        .expect("the longer prefix");
    let wide = explain
        .find("everything on github")
        .expect("the shorter prefix");
    assert!(narrow < wide, "the longest prefix wins:\n{explain}");
    assert!(
        explain.contains("chosen  https://narrow.invalid/index"),
        "{explain}"
    );
    assert!(
        explain.contains("origin   https://github.com/vibespecs/index"),
        "{explain}"
    );
}

#[test]
fn list_json_carries_the_rows_and_their_computed_diagnostics() {
    let fixture = Fixture::new("https://example.invalid/a", &"ab".repeat(32));
    let user = UserScratch::new();
    let store = fixture.root.path().join("store");
    fs::create_dir_all(&store).unwrap();
    settings(
        &user,
        &format!(
            "[policy]\nunmatched = \"deny\"\n\
             [[remap]]\nprefix = \"http://10.1.0.5/\"\nurl = \"http://10.1.0.6/\"\n\
             reason = \"the one host the proxy allows\"\n\
             [[remap]]\nsha256 = \"{digest}\"\npath = \"{store}\"\n\
             [[remap]]\nregistry = \"vibespecs\"\ndeny = true\nreason = \"policy\"\n",
            digest = "ab".repeat(32),
            store = store.display(),
        ),
    );
    // `--offline` is what keeps the diagnostics free of any DNS lookup.
    let raw = ok(&fixture.vibe(&user, &["--offline", "remap", "list", "--json"]));
    let document: Value = serde_json::from_str(&raw).unwrap_or_else(|error| {
        panic!("not JSON: {error}\n{raw}");
    });
    let document = document.get("data").unwrap_or(&document).clone();
    assert_eq!(document["protocol"], "vibe-remap/1");
    assert_eq!(document["policy"], "deny");
    assert_eq!(document["denies"], 1);

    let settings_block = document["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| {
            source["source"]
                .as_str()
                .is_some_and(|name| name.ends_with("settings.toml"))
        })
        .expect("the settings source block")
        .clone();
    let rows = settings_block["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);

    // A plain-http target over a private literal: unencrypted, private,
    // and not broad — the diagnostics are computed, never declared.
    assert_eq!(rows[0]["kind"], "prefix");
    assert_eq!(rows[0]["scheme"], "http");
    assert_eq!(rows[0]["reach"], "private");
    assert_eq!(rows[0]["host"], "10.1.0.6");
    assert_eq!(rows[0]["notes"], serde_json::json!(["unencrypted"]));
    assert_eq!(rows[0]["why"], "the one host the proxy allows");

    assert_eq!(rows[1]["kind"], "sha256");
    assert_eq!(rows[1]["scheme"], "path");
    assert_eq!(rows[1]["reach"], "local");

    assert_eq!(rows[2]["kind"], "registry");
    assert_eq!(rows[2]["deny"], true);

    // The footer groups by host and reports the refusal separately. It
    // covers every source, so the two built-in single-file endpoints are
    // in it too — the settings rows add exactly these two entries.
    let hosts = document["hosts"].as_array().unwrap();
    let rewrite = hosts
        .iter()
        .find(|host| host["host"] == "10.1.0.6")
        .expect("the rewritten host");
    assert_eq!(
        (rewrite["rows"].as_u64(), &rewrite["reach"]),
        (Some(1), &Value::from("private"))
    );
    let local = hosts
        .iter()
        .find(|host| host["host"] == "local path")
        .expect("the path target");
    assert_eq!(local["rows"].as_u64(), Some(1));
}

#[test]
fn the_two_views_print_the_one_layout() {
    let fixture = Fixture::new("https://example.invalid/a", &"ab".repeat(32));
    let user = UserScratch::new();
    settings(
        &user,
        "[[remap]]\nprefix = \"https://github.com/\"\nurl = \"http://10.1.0.5/gh/\"\n\
         reason = \"the only host the proxy allows\"\n",
    );
    let listed = ok(&fixture.vibe(&user, &["--offline", "remap", "list"]));
    assert!(
        listed.contains("1. prefix    https://github.com/"),
        "{listed}"
    );
    assert!(
        listed.contains("to       http://10.1.0.5/gh/  [http, private]"),
        "{listed}"
    );
    assert!(
        listed.contains("why      the only host the proxy allows"),
        "{listed}"
    );
    assert!(
        listed.contains("note     the target is plain http"),
        "a plaintext target is called out: {listed}"
    );
    assert!(
        listed.contains("1 row → 10.1.0.5 (http, private)"),
        "the per-host footer: {listed}"
    );
    // The built-in single-file endpoints are rows too, and they are shown.
    assert!(
        listed.contains("built-in  [user-package, unmatched = allow]"),
        "the default raw-file rows are listed: {listed}"
    );
    assert!(
        listed.contains("https://raw.githubusercontent.com"),
        "{listed}"
    );
}

/// `PROP-066#commands`: a verb vibe cannot honour does not appear in its
/// help — and now that every verb of `#commands-packages` is honoured,
/// every one of them does.
#[test]
fn the_whole_remap_surface_is_offered() {
    let fixture = Fixture::new("https://example.invalid/a", &"ab".repeat(32));
    let user = UserScratch::new();
    let help = ok(&fixture.vibe(&user, &["remap", "--help"]));
    for verb in [
        "list", "explain", "accept", "reject", "enable", "disable", "test", "export", "import",
    ] {
        assert!(
            help.contains(&format!("  {verb}")),
            "`remap {verb}` is built and must appear in the help:\n{help}"
        );
    }
}
