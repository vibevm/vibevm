//! Artifact drops for `install -g` (PROP-066 `#drops`): an application
//! input served from the `[artifacts] packages` drop by the package
//! convention with its server never asked, a wrong file refused by its
//! digest, a miss that names the exact file and the searched paths, the
//! author-side `drop = "packages"` input, and `vibe artifacts where`.

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
use sha2::{Digest, Sha256};

/// A loopback server that counts requests: "the server was never asked"
/// is what distinguishes a drop that served the bytes from one that failed.
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

struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
    user: UserScratch,
}

impl Fixture {
    /// `org.example/dropped@1.0.0` whose one `sources` input is `input`
    /// (a TOML body after `name = "payload"`).
    fn new(input: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let package = registry.join("org.example/dropped/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        let body = format!(
            r#"[package]
name = "dropped"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "dropped"

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
{input}
[installer_options.installation.recipe]
interpreter = "posix_sh"
script = "build.sh"

[[installer_options.launchers]]
id = "main"
name = "Dropped"
executable = "$app_bin/dropped"
kind = "command"
default = true
"#,
            arch = std::env::consts::ARCH,
        );
        fs::write(package.join("vibe.toml"), body).unwrap();
        fs::write(
            package.join("build.sh"),
            "set -e\nmkdir -p \"$VIBE_OUT/bin\"\n\
             cp \"$VIBE_SOURCES\"/payload/* \"$VIBE_OUT/bin/dropped\"\n\
             chmod 755 \"$VIBE_OUT/bin/dropped\"\n",
        )
        .unwrap();
        Self {
            root,
            registry,
            user: UserScratch::new(),
        }
    }

    fn drop_dir(&self) -> PathBuf {
        self.root.path().join("drop")
    }

    /// Declare the package drop in `config.toml`.
    fn configure_drop(&self) {
        fs::write(
            self.user.settings.join("config.toml"),
            format!(
                "[artifacts]\npackages = \"{}\"\n",
                self.drop_dir().display()
            ),
        )
        .unwrap();
    }

    fn place(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.drop_dir().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    fn vibe(&self, args: &[&str]) -> Output {
        let mut command = self.user.vibe();
        command.current_dir(self.root.path()).args(args);
        for variable in ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
            command.env_remove(variable);
        }
        command.output().unwrap()
    }

    fn install(&self, extra: &[&str]) -> Output {
        let registry = self.registry.to_str().unwrap();
        let mut args = extra.to_vec();
        args.extend_from_slice(&["install", "-g", "org.example/dropped", "--unattended"]);
        args.extend_from_slice(&["--registry", registry]);
        self.vibe(&args)
    }

    fn installed_payload(&self) -> Vec<u8> {
        let found = walk(&self.user.settings)
            .into_iter()
            .find(|path| path.ends_with("bin/dropped"))
            .expect("the installed launcher payload");
        fs::read(found).unwrap()
    }
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found
}

fn text(output: &Output) -> String {
    format!(
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn an_input_is_served_from_the_package_drop_without_asking_its_server() {
    let payload = b"from-the-drop\n";
    let digest = sha(payload);
    let (url, hits) = serve(b"from-the-server\n".to_vec());
    let fixture = Fixture::new(&format!("url = \"{url}\"\nsha256 = \"{digest}\""));
    fixture.configure_drop();
    fixture.place("org.example/dropped/1.0.0/payload.txt", payload);

    let output = fixture.install(&[]);
    assert!(output.status.success(), "{}", text(&output));
    assert_eq!(hits.load(Ordering::SeqCst), 0, "the drop served the bytes");
    assert_eq!(fixture.installed_payload(), payload);
}

#[test]
fn a_wrong_file_is_refused_and_a_miss_names_the_file_and_the_paths() {
    let payload = b"expected\n";
    let digest = sha(payload);
    // A closed port: the origin is never reachable, and `--offline`
    // keeps the run from trying.
    let fixture = Fixture::new(&format!(
        "url = \"http://127.0.0.1:9/dist/payload.txt\"\nsha256 = \"{digest}\""
    ));
    fixture.configure_drop();

    let missing = fixture.install(&["--offline"]);
    assert!(!missing.status.success(), "{}", text(&missing));
    let stderr = text(&missing);
    assert!(stderr.contains("holds no `payload.txt`"), "{stderr}");
    let first = fixture
        .drop_dir()
        .join("org.example/dropped/1.0.0/payload.txt");
    assert!(stderr.contains(&first.display().to_string()), "{stderr}");

    fixture.place("payload.txt", b"tampered\n");
    let wrong = fixture.install(&["--offline"]);
    assert!(!wrong.status.success(), "{}", text(&wrong));
    assert!(
        text(&wrong).contains("does not match its declared sha256"),
        "{}",
        text(&wrong)
    );
}

#[test]
fn artifacts_where_prints_the_expected_locations_and_their_state() {
    let payload = b"expected\n";
    let digest = sha(payload);
    let fixture = Fixture::new(&format!(
        "url = \"https://dist.example/rel/payload.txt\"\nsha256 = \"{digest}\""
    ));
    fixture.configure_drop();
    let registry = fixture.registry.to_str().unwrap().to_owned();
    let args = [
        "artifacts",
        "where",
        "org.example/dropped",
        "--registry",
        &registry,
        "--json",
    ];
    let before = fixture.vibe(&args);
    assert!(before.status.success(), "{}", text(&before));
    let report: serde_json::Value = serde_json::from_slice(&before.stdout).unwrap();
    assert_eq!(report["packages"], fixture.drop_dir().display().to_string());
    let input = &report["inputs"][0];
    assert_eq!(input["file"], "payload.txt");
    assert_eq!(input["status"], "missing");
    assert_eq!(
        input["searched"][0],
        fixture
            .drop_dir()
            .join("org.example/dropped/1.0.0/payload.txt")
            .display()
            .to_string()
    );
    fixture.place(&format!("{digest}/payload.txt"), payload);
    let after = fixture.vibe(&args);
    let report: serde_json::Value = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(report["inputs"][0]["status"], "present");

    // Without a package it still says where releases and payloads go.
    let plain = fixture.vibe(&["artifacts", "where"]);
    assert!(plain.status.success(), "{}", text(&plain));
    assert!(text(&plain).contains("<drop>/G/N/V/F"), "{}", text(&plain));
}

#[test]
fn an_author_side_drop_input_is_read_from_the_configured_drop_only() {
    let payload = b"built-inside-the-network\n";
    let digest = sha(payload);
    let fixture = Fixture::new(&format!(
        "drop = \"packages\"\nfile = \"payload.txt\"\nsha256 = \"{digest}\""
    ));

    let unconfigured = fixture.install(&["--offline"]);
    assert!(!unconfigured.status.success(), "{}", text(&unconfigured));
    assert!(
        text(&unconfigured).contains("[artifacts] packages"),
        "{}",
        text(&unconfigured)
    );

    fixture.configure_drop();
    fixture.place("org.example/dropped/payload.txt", payload);
    let output = fixture.install(&["--offline"]);
    assert!(output.status.success(), "{}", text(&output));
    assert_eq!(fixture.installed_payload(), payload);

    let registry = fixture.registry.to_str().unwrap().to_owned();
    let listed = fixture.vibe(&[
        "artifacts",
        "where",
        "org.example/dropped",
        "--registry",
        &registry,
        "--json",
    ]);
    let report: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(report["inputs"][0]["file"], "payload.txt");
    assert_eq!(report["inputs"][0]["status"], "present");
}
