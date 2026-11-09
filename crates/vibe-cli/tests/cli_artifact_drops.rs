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
use vibe_wire::generated::artifacts_where_report::{
    ArtifactsWhereReport, DropStatus, ReleaseSourceKind,
};

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

/// Every registered `--json` document these tests read goes through its
/// generated reader (`common::wire`) before anything is asserted about it.
fn where_report(stdout: &[u8]) -> serde_json::Value {
    common::wire::checked(&String::from_utf8_lossy(stdout))
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
    let report = where_report(&before.stdout);
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
    let report = where_report(&after.stdout);
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

/// `vibe-artifacts-where/1` is the document its registered schema
/// describes, with its closed `status` vocabulary and the release source
/// that decided where releases go.
#[test]
fn the_where_report_reads_as_its_registered_wire_type() {
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
    let missing = fixture.vibe(&args);
    assert!(missing.status.success(), "{}", text(&missing));
    let report: ArtifactsWhereReport =
        common::wire::typed(&String::from_utf8_lossy(&missing.stdout));
    assert_eq!(report.protocol, "vibe-artifacts-where/1");
    // No `[self]` and no `[artifacts] vibe`: releases are still downloaded.
    assert_eq!(report.release.source, ReleaseSourceKind::Github);
    assert_eq!(report.release.example_dir, None);
    assert!(report.release.files.is_empty());
    assert_eq!(
        report.packages.as_deref(),
        Some(fixture.drop_dir().display().to_string().as_str())
    );
    let input = report.inputs.first().expect("one downloaded input");
    assert_eq!(input.file.as_deref(), Some("payload.txt"));
    assert_eq!(input.status, DropStatus::Missing);
    assert_eq!(input.sha256, digest);

    fixture.place(&format!("{digest}/payload.txt"), payload);
    let present: ArtifactsWhereReport =
        common::wire::typed(&String::from_utf8_lossy(&fixture.vibe(&args).stdout));
    assert_eq!(present.inputs[0].status, DropStatus::Present);
}

/// The plan prices only what will be transferred (`PROP-065#sizes`,
/// `PROP-066#drops`): an input the drop already holds is named as the
/// drop's and costs nothing, while the same input absent from the drop is
/// priced at its declared size.
#[test]
fn the_plan_counts_a_dropped_input_as_local_and_names_the_drop() {
    let payload = b"from-the-drop\n";
    let digest = sha(payload);
    let fixture = Fixture::new(&format!(
        "url = \"https://dist.example/rel/payload.txt\"\nsha256 = \"{digest}\"\nsize = {}",
        payload.len()
    ));
    fixture.configure_drop();
    let registry = fixture.registry.to_str().unwrap().to_owned();
    let args = [
        "app",
        "plan",
        "org.example/dropped",
        "--registry",
        &registry,
    ];

    let priced = fixture.vibe(&args);
    assert!(priced.status.success(), "{}", text(&priced));
    let priced = text(&priced);
    assert!(priced.contains("download"), "{priced}");
    assert!(
        !priced.contains("[artifacts] packages drop"),
        "nothing is in the drop yet: {priced}"
    );

    fixture.place("org.example/dropped/1.0.0/payload.txt", payload);
    let served = fixture.vibe(&args);
    assert!(served.status.success(), "{}", text(&served));
    let served = text(&served);
    assert!(
        served.contains("1 input(s) from the [artifacts] packages drop"),
        "{served}"
    );
    assert!(served.contains("0 B"), "{served}");
}

/// A drop miss whose declared origin the source policy excludes points at
/// the drop, not at another endpoint (`PROP-066#drops`): the fix names the
/// exact first rung, the admission law's own "use an admitted HTTP
/// endpoint" advice is gone, and the excluded origin is never contacted.
#[test]
fn an_excluded_origin_behind_a_drop_miss_is_fixed_by_filling_the_drop() {
    let payload = b"expected\n";
    let digest = sha(payload);
    let (url, hits) = serve(payload.to_vec());
    let fixture = Fixture::new(&format!(
        "url = \"{url}/payload.txt\"\nsha256 = \"{digest}\""
    ));
    fixture.configure_drop();
    fs::write(
        fixture.user.settings.join("registry.toml"),
        "[source_policy]\nexclude_hosting=['host:127.0.0.1']\n",
    )
    .unwrap();

    let refused = fixture.install(&[]);
    assert!(!refused.status.success(), "{}", text(&refused));
    let stderr = text(&refused);
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "an excluded origin is refused before any connection: {stderr}"
    );
    let first = fixture
        .drop_dir()
        .join("org.example/dropped/1.0.0/payload.txt");
    assert!(
        stderr.contains(&format!("put the file at {}", first.display())),
        "the fix names the exact rung: {stderr}"
    );
    assert!(
        !stderr.contains("use an admitted HTTP endpoint"),
        "the fix no longer points away from the drop: {stderr}"
    );
}
