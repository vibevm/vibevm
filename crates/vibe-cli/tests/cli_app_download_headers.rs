//! `vibe install -g … --download-header-file <path>` (PROP-075
//! `#DOWNLOAD-HEADERS`): the declared headers reach this install's own
//! input downloads, their values reach nothing a human or a machine can
//! read afterwards, and a file that is not well formed stops the command
//! before the first request.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Arc, Mutex};

use common::UserScratch;
use sha2::{Digest, Sha256};

/// The credential the header file carries. It must appear in the request
/// the server receives and nowhere else at all.
const SECRET: &str = "s3cr3t-gateway-token";

/// A loopback server recording every request head, answering `reply(n)`.
fn serve(reply: impl Fn(usize) -> Vec<u8> + Send + 'static) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    std::thread::spawn(move || {
        for (index, stream) in listener.incoming().flatten().enumerate() {
            let mut stream = stream;
            let mut request = Vec::new();
            let mut buffer = [0u8; 2048];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => request.extend_from_slice(&buffer[..read]),
                }
            }
            log.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&request).into_owned());
            let _ = stream.write_all(&reply(index));
        }
    });
    (format!("http://{address}/dist/payload.txt"), seen)
}

fn ok(body: &[u8]) -> Vec<u8> {
    let mut answer = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    answer.extend_from_slice(body);
    answer
}

struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
    user: UserScratch,
}

impl Fixture {
    /// `org.example/headed@1.0.0`: one `copy` step over one declared input.
    fn new(url: &str, digest: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let package = registry.join("org.example/headed/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("vibe.toml"),
            format!(
                r#"[package]
name = "headed"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "headed"

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
name = "Headed"
executable = "$app_bin/headed"
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
             cp \"$VIBE_SOURCES\"/payload/* \"$VIBE_OUT/bin/headed\"\n\
             chmod 755 \"$VIBE_OUT/bin/headed\"\n",
        )
        .unwrap();
        Self {
            root,
            registry,
            user: UserScratch::new(),
        }
    }

    /// A header file as the shim writes it: `mktemp`, mode 600.
    fn header_file(&self, body: &str) -> PathBuf {
        let path = self.root.path().join("headers.txt");
        fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        path
    }

    fn install(&self, extra: &[&str]) -> Output {
        let mut command = self.user.vibe();
        command.current_dir(self.root.path());
        for variable in ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
            command.env_remove(variable);
        }
        command.args(["install", "-g", "org.example/headed", "--unattended"]);
        command.args(["--registry", self.registry.to_str().unwrap()]);
        command.args(extra);
        command.output().unwrap()
    }

    /// Every byte the run left behind in the user's settings root, the
    /// transfer journal among them (`PROP-068#journal`).
    fn settings_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut stack = vec![self.user.settings.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(read) = fs::read(&path) {
                    bytes.extend_from_slice(&read);
                }
            }
        }
        bytes
    }
}

fn text(output: &Output) -> String {
    format!(
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// The headers reach the input's own download, and their values reach
/// neither stream nor the journal — at `-v`, where everything else does.
#[test]
fn an_install_sends_the_declared_headers_and_writes_none_of_their_values() {
    let payload = b"from-the-gateway\n";
    let (url, seen) = serve(move |_| ok(payload));
    let fixture = Fixture::new(&url, &format!("{:x}", Sha256::digest(payload)));
    let headers = fixture.header_file(&format!(
        "# the mirror's gateway\nAuthorization: Bearer {SECRET}\n\nX-Mirror: eu\n"
    ));

    let output = fixture.install(&["-v", "--download-header-file", headers.to_str().unwrap()]);
    assert!(output.status.success(), "{}", text(&output));

    let requests = seen.lock().unwrap().clone();
    assert!(
        requests.iter().any(|request| request.contains(SECRET)),
        "the download carried the header: {requests:?}"
    );
    assert!(!contains(&output.stdout, SECRET), "{}", text(&output));
    assert!(!contains(&output.stderr, SECRET), "{}", text(&output));
    assert!(
        !contains(&fixture.settings_bytes(), SECRET),
        "a value reached the settings root"
    );
}

/// A download that fails says so without the credential, on either
/// surface — the human one and `--json`.
#[test]
fn a_failing_download_with_headers_leaks_no_value_to_any_surface() {
    let (url, seen) = serve(|_| {
        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec()
    });
    let fixture = Fixture::new(&url, &format!("{:x}", Sha256::digest(b"never-served")));
    let headers = fixture.header_file(&format!("Authorization: Bearer {SECRET}\n"));
    let path = headers.to_str().unwrap().to_owned();

    for extra in [
        vec!["-v", "--download-header-file", &path],
        vec!["--json", "--download-header-file", &path],
    ] {
        let output = fixture.install(&extra);
        assert!(!output.status.success(), "{}", text(&output));
        assert!(
            contains(&output.stdout, "503") || contains(&output.stderr, "503"),
            "the failure names itself: {}",
            text(&output)
        );
        assert!(!contains(&output.stdout, SECRET), "{}", text(&output));
        assert!(!contains(&output.stderr, SECRET), "{}", text(&output));
    }
    assert!(!seen.lock().unwrap().is_empty(), "the server was asked");
    assert!(
        !contains(&fixture.settings_bytes(), SECRET),
        "a value reached the settings root"
    );
}

/// A header file that is not well formed is refused by line, and the
/// refusal happens before the first request of the run.
#[test]
fn a_malformed_header_file_is_refused_by_line_before_anything_is_contacted() {
    let payload = b"from-the-gateway\n";
    let (url, seen) = serve(move |_| ok(payload));
    let fixture = Fixture::new(&url, &format!("{:x}", Sha256::digest(payload)));
    let headers = fixture.header_file(&format!("Authorization: ok\nBearer {SECRET}\n"));

    let output = fixture.install(&["--download-header-file", headers.to_str().unwrap()]);
    assert!(!output.status.success(), "{}", text(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("line 2"), "{stderr}");
    assert!(stderr.contains("Name: value"), "{stderr}");
    assert!(!stderr.contains(SECRET), "{stderr}");
    assert!(
        seen.lock().unwrap().is_empty(),
        "nothing was contacted: {:?}",
        seen.lock().unwrap()
    );
}

/// A file that does not exist is named, and an empty one is refused as
/// empty rather than read as "no headers".
#[test]
fn a_missing_or_empty_header_file_is_refused_by_name() {
    let payload = b"from-the-gateway\n";
    let (url, _) = serve(move |_| ok(payload));
    let fixture = Fixture::new(&url, &format!("{:x}", Sha256::digest(payload)));

    let absent = fixture.root.path().join("absent.txt");
    let output = fixture.install(&["--download-header-file", absent.to_str().unwrap()]);
    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("absent.txt"),
        "{}",
        text(&output)
    );

    let empty = fixture.header_file("# only a comment\n");
    let output = fixture.install(&["--download-header-file", empty.to_str().unwrap()]);
    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("declares no header"),
        "{}",
        text(&output)
    );
}

/// The flag belongs to `install -g`: a project install does not take it,
/// so there is no spelling by which it reaches a registry or index read.
#[test]
fn the_flag_is_refused_without_the_global_plane() {
    let project = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    let headers = project.path().join("headers.txt");
    fs::write(&headers, "Authorization: Bearer x\n").unwrap();
    let output = user
        .vibe()
        .current_dir(project.path())
        .args(["install", "--download-header-file"])
        .arg(&headers)
        .output()
        .unwrap();
    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--global"),
        "{}",
        text(&output)
    );
    let _: &Path = project.path();
}
