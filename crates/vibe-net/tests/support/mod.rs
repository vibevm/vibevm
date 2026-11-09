//! Local network fixtures for the route tests (`PROP-068#proxy`): a test
//! CA (OpenSSL CLI), an HTTPS server with a git smart-HTTP mount, and a
//! forwarding proxy that maps every host to 127.0.0.1 — so a request for
//! `vibe-fixture.test` can only succeed through the proxy (the name does
//! not resolve directly). Unix test hosts with `python3`, `openssl` and
//! `git` on PATH. Shared by `vibe-net` and `vibe-registry` tests.
//!
//! Each test binary compiles its own copy and uses a subset, so dead-code
//! analysis is silenced for the module as a whole (the convention of
//! `crates/vibe-cli/tests/common/mod.rs`).
#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// The host name every fixture certificate carries and no resolver knows.
pub const FIXTURE_HOST: &str = "vibe-fixture.test";

pub fn support_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vibe-net/tests/support")
        .canonicalize()
        .unwrap()
}

pub struct Ca {
    pub ca_pem: PathBuf,
    pub cert: PathBuf,
    pub key: PathBuf,
}

fn openssl(args: &[&str]) {
    let out = Command::new("openssl").args(args).output().unwrap();
    assert!(
        out.status.success(),
        "openssl {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A CA and a leaf for `vibe-fixture.test`, `localhost` and `127.0.0.1`.
pub fn test_ca(dir: &Path, name: &str) -> Ca {
    let p = |file: &str| dir.join(format!("{name}-{file}"));
    let (ca_pem, ca_key, cert, key, csr, ext) = (
        p("ca.pem"),
        p("ca-key.pem"),
        p("leaf.pem"),
        p("leaf-key.pem"),
        p("leaf.csr"),
        p("leaf.ext"),
    );
    std::fs::write(
        &ext,
        format!(
            "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\n\
             extendedKeyUsage=serverAuth\nsubjectAltName=DNS:{FIXTURE_HOST},DNS:localhost,IP:127.0.0.1\n"
        ),
    )
    .unwrap();
    let s = |path: &Path| path.to_str().unwrap().to_owned();
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        &s(&ca_key),
        "-out",
        &s(&ca_pem),
        "-days",
        "2",
        "-subj",
        &format!("/CN=vibe test {name} CA"),
        "-addext",
        "basicConstraints=critical,CA:TRUE",
        "-addext",
        "keyUsage=critical,keyCertSign,cRLSign",
    ]);
    openssl(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        &s(&key),
        "-out",
        &s(&csr),
        "-subj",
        &format!("/CN={FIXTURE_HOST}"),
    ]);
    openssl(&[
        "x509",
        "-req",
        "-in",
        &s(&csr),
        "-CA",
        &s(&ca_pem),
        "-CAkey",
        &s(&ca_key),
        "-CAcreateserial",
        "-out",
        &s(&cert),
        "-days",
        "2",
        "-extfile",
        &s(&ext),
    ]);
    Ca { ca_pem, cert, key }
}

/// A child process killed and reaped on drop.
pub struct Server {
    child: Child,
    pub port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_for(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(text) = std::fs::read_to_string(path)
            && !text.is_empty()
        {
            return text;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("fixture did not start: {}", path.display());
}

/// Serve `root` over HTTPS with `ca`'s leaf (plain HTTP without one).
pub fn https_server(root: &Path, ca: Option<&Ca>, scratch: &Path) -> Server {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let port_file = scratch.join(format!("server-{}-{n}.port", std::process::id()));
    let _ = std::fs::remove_file(&port_file);
    let mut command = Command::new("python3");
    command
        .arg("-I")
        .arg(support_dir().join("https_server.py"))
        .arg(format!("--root={}", root.display()))
        .arg(format!("--port-file={}", port_file.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(ca) = ca {
        command
            .arg(format!("--cert={}", ca.cert.display()))
            .arg(format!("--key={}", ca.key.display()));
    }
    let child = command.spawn().unwrap();
    let port = wait_for(&port_file).trim().parse().unwrap();
    Server { child, port }
}

/// The forwarding proxy (the fake px run as a plain proxy), logging every
/// request line to `log`.
pub fn proxy(log: &Path) -> Server {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let child = Command::new("python3")
        .arg("-I")
        .arg(support_dir().join("fake_px.py"))
        .arg("--listen=127.0.0.1")
        .arg(format!("--port={port}"))
        .arg(format!("--fake-log={}", log.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < deadline, "proxy did not start");
        std::thread::sleep(Duration::from_millis(20));
    }
    Server { child, port }
}

/// The request lines the proxy saw.
pub fn requests(log: &Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("request "))
        .map(str::to_owned)
        .collect()
}
