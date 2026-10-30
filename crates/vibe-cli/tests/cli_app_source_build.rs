//! Public Linux fixture for the `source_build` method (PROP-059
//! `#source-build`, `#acceptance`): a package-local recipe and two sources —
//! one `.tar.gz` served by a local HTTP server, one package-local file —
//! install → run → PATH proxy → offline cache hit and miss → a failing
//! recipe keeps its work directory → a wrong digest refuses before the
//! recipe → uninstall.

#![cfg(target_os = "linux")]

mod common;

use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};

const KEY: &str = "org.example/built-app@1.0.0";

/// Serve `body` to every request on a loopback port; count the requests.
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
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/gzip\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    (format!("http://{address}/dist/tool-src.tar.gz"), hits)
}

fn tar_gz(entries: &[(&str, &str)]) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);
    for (name, body) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, body.as_bytes()).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
    package: PathBuf,
    log: PathBuf,
    url: String,
    url_sha: String,
}

impl Fixture {
    fn new(url: String, url_sha: String) -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let package = registry.join("org.example/built-app/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        let log = root.path().join("steps.log");
        Self {
            root,
            registry,
            package,
            log,
            url,
            url_sha,
        }
    }

    fn manifest(&self, greeting_sha: &str) {
        let log = self.log.display();
        let body = format!(
            r#"[package]
name = "built-app"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "tool"

[[installer_options]]
id = "src"
name = "Build from source"
platform = "linux"
platform_name = "Linux"
arch = "{arch}"
pre_install = "echo PRE >> '{log}'"
post_install = "test -x \"$VIBE_APP_ROOT/bin/tool\" && echo POST >> '{log}'"
[installer_options.installation]
type = "source_build"
[[installer_options.installation.sources]]
name = "upstream"
url = "{url}"
sha256 = "{url_sha}"
[[installer_options.installation.sources]]
name = "greeting"
path = "greeting.txt"
sha256 = "{greeting_sha}"
[installer_options.installation.recipe]
interpreter = "posix_sh"
script = "build.sh"

[[installer_options.launchers]]
id = "main"
name = "Tool"
executable = "$app_bin/tool"
args = ["fixed"]
default = true

[[installer_options.public_commands]]
name = "tool"
launcher = "main"
"#,
            arch = std::env::consts::ARCH,
            url = self.url,
            url_sha = self.url_sha,
        );
        fs::write(self.package.join("vibe.toml"), body).unwrap();
    }

    fn recipe(&self, body: &str) {
        let path = self.package.join("build.sh");
        fs::write(&path, body).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
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
        args.extend_from_slice(&["install", "-g", "org.example/built-app"]);
        args.extend_from_slice(&["--registry", registry]);
        self.vibe(user, &args)
    }

    fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
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

fn index(user: &UserScratch) -> Value {
    serde_json::from_slice(&fs::read(user.settings.join("applications/index.json")).unwrap())
        .unwrap()
}

fn entries(directory: &Path) -> Vec<String> {
    fs::read_dir(directory)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// `tool` resolved through PATH with `opt/bin` first, as a shell would.
fn run_proxy(user: &UserScratch) -> String {
    let mut path = OsString::from(user.settings.join("opt/bin"));
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    ok(&Command::new("tool")
        .arg("arg")
        .env("PATH", path)
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap())
}

fn good_recipe(log: &Path) -> String {
    format!(
        r#"set -e
echo RECIPE >> '{log}'
test "$(pwd -P)" = "$(cd "$VIBE_WORK" && pwd -P)"
test -z "$VIBE_APP_ROOT"
test -f "$VIBE_PACKAGE/vibe.toml"
mkdir -p "$VIBE_OUT/bin"
{{ echo '#!/bin/sh'; cat "$VIBE_SOURCES/upstream/tool.sh.in" "$VIBE_SOURCES/greeting/greeting.txt"; }} > "$VIBE_OUT/bin/tool"
chmod 755 "$VIBE_OUT/bin/tool"
echo "$VIBE_PLATFORM/$VIBE_ARCH" > "$VIBE_OUT/built-for"
echo RECIPE_RAN
"#,
        log = log.display()
    )
}

#[test]
fn source_build_installs_runs_caches_fails_visibly_and_uninstalls() {
    let tarball = tar_gz(&[("tool.sh.in", "echo \"TOOL_RAN $*\"\n")]);
    let url_sha = sha(&tarball);
    let (url, hits) = serve(tarball);
    let fixture = Fixture::new(url, url_sha.clone());
    let greeting = b"echo GREETING_FROM_PATH\n";
    fs::write(fixture.package.join("greeting.txt"), greeting).unwrap();
    fixture.manifest(&sha(greeting));
    fixture.recipe(&good_recipe(&fixture.log));
    let user = UserScratch::new();

    // install -g: hooks around the recipe, payload in the default library.
    let installed = fixture.install(&user, &["--json"]);
    let report: Value = serde_json::from_str(&ok(&installed)).unwrap();
    let payload = user
        .settings
        .join("opt/apps/tool/org.example.built-app@1.0.0");
    assert_eq!(report["installation"], KEY);
    assert_eq!(report["appRoot"], payload.display().to_string());
    assert!(String::from_utf8_lossy(&installed.stderr).contains("RECIPE_RAN"));
    assert_eq!(fixture.log(), "PRE\nRECIPE\nPOST\n");
    assert!(payload.join("bin/tool").is_file());
    assert_eq!(
        fs::read_to_string(payload.join("built-for")).unwrap(),
        format!("linux/{}\n", std::env::consts::ARCH)
    );
    let record = &index(&user)["installations"][KEY];
    assert_eq!(record["method"], "source_build");
    assert_eq!(record["library"], "default");
    assert_eq!(record["directory"], "apps/tool/org.example.built-app@1.0.0");
    assert_eq!(record["portable"], true);
    let cached = user
        .settings
        .join(format!("cache/app-sources/{url_sha}/tool-src.tar.gz"));
    assert!(cached.is_file(), "the URL source is cached by digest");
    assert!(
        entries(&user.settings.join("cache/app-build")).is_empty(),
        "a successful build deletes its work directory"
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1);

    // app run and the PATH proxy.
    let ran = ok(&fixture.vibe(&user, &["app", "run", "org.example/built-app", "main"]));
    assert!(ran.contains("TOOL_RAN fixed"), "{ran}");
    assert!(ran.contains("GREETING_FROM_PATH"), "{ran}");
    let proxied = run_proxy(&user);
    assert!(proxied.contains("TOOL_RAN fixed arg"), "{proxied}");

    // --offline: a fresh root misses the cache; the cached root succeeds.
    let fresh = UserScratch::new();
    let error = failed(&fixture.install(&fresh, &["--offline"]));
    assert!(error.contains("--offline is set"), "{error}");
    assert!(!fresh.settings.join("applications/index.json").exists());
    assert!(!fresh.settings.join("cache/app-build").exists());
    assert_eq!(
        fixture.log(),
        "PRE\nRECIPE\nPOST\nPRE\n",
        "pre_install runs before fetching; the miss stops before the recipe"
    );
    ok(&fixture.install(&user, &["--offline"]));
    assert_eq!(hits.load(Ordering::SeqCst), 1, "the cached file is reused");
    assert!(fixture.log().ends_with("PRE\nPRE\nRECIPE\nPOST\n"));
    let before = index(&user);

    // A failing recipe keeps its work directory and the index.
    fixture.recipe(&format!(
        "echo RECIPE >> '{}'\nmkdir -p \"$VIBE_OUT/partial\"\nexit 3\n",
        fixture.log.display()
    ));
    let error = failed(&fixture.install(&user, &[]));
    assert!(error.contains("exited with code 3"), "{error}");
    assert!(error.contains("kept for inspection"), "{error}");
    let build_root = user.settings.join("cache/app-build");
    let kept = entries(&build_root);
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert!(
        kept[0].starts_with("org.example.built-app@1.0.0-"),
        "{kept:?}"
    );
    assert!(
        error.contains(&kept[0]),
        "the error names the work directory"
    );
    let work = build_root.join(&kept[0]);
    assert!(work.join("sources/greeting/greeting.txt").is_file());
    assert!(work.join("sources/upstream/tool.sh.in").is_file());
    assert_eq!(index(&user), before, "the index is unchanged");
    assert!(fixture.log().ends_with("PRE\nRECIPE\n"));
    let ran = ok(&fixture.vibe(&user, &["app", "run", "org.example/built-app", "main"]));
    assert!(
        ran.contains("TOOL_RAN"),
        "the previous payload stays: {ran}"
    );

    // A wrong digest refuses before the recipe runs.
    fixture.recipe(&good_recipe(&fixture.log));
    fixture.manifest(&"0".repeat(64));
    let log_before = fixture.log();
    let error = failed(&fixture.install(&user, &[]));
    assert!(
        error.contains("does not match its declared sha256"),
        "{error}"
    );
    assert_eq!(fixture.log(), format!("{log_before}PRE\n"), "no recipe ran");
    assert_eq!(entries(&build_root), kept, "no work directory was created");
    assert_eq!(index(&user), before);

    // uninstall -g deletes the payload and the proxy.
    ok(&fixture.vibe(&user, &["uninstall", "-g", "org.example/built-app"]));
    assert!(!payload.exists());
    assert!(!user.settings.join("opt/bin/tool").exists());
    assert_eq!(index(&user)["installations"], serde_json::json!({}));
}
