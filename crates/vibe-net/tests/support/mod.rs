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

/// A two-level authority: a root that signs an intermediate, and a leaf
/// signed by the intermediate. `leaf_only` is the leaf by itself — a
/// server configured with it sends an incomplete chain, which is the
/// corporate shape a trust set has to be able to complete on its own.
pub struct Chain {
    pub root_pem: PathBuf,
    pub intermediate_pem: PathBuf,
    pub leaf_only: PathBuf,
    pub leaf_with_intermediate: PathBuf,
    pub key: PathBuf,
}

/// A root, an intermediate it signs, and a leaf for the fixture host.
pub fn test_chain(dir: &Path, name: &str) -> Chain {
    let p = |file: &str| dir.join(format!("{name}-{file}"));
    let s = |path: &PathBuf| path.to_str().unwrap().to_owned();
    let (root_pem, root_key) = (p("root.pem"), p("root-key.pem"));
    let (int_pem, int_key, int_csr, int_ext) =
        (p("int.pem"), p("int-key.pem"), p("int.csr"), p("int.ext"));
    let (leaf, leaf_key, leaf_csr, leaf_ext) = (
        p("leaf.pem"),
        p("leaf-key.pem"),
        p("leaf.csr"),
        p("leaf.ext"),
    );
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        &s(&root_key),
        "-out",
        &s(&root_pem),
        "-days",
        "2",
        "-subj",
        &format!("/CN=vibe test {name} root"),
        "-addext",
        "basicConstraints=critical,CA:TRUE",
        "-addext",
        "keyUsage=critical,keyCertSign,cRLSign",
    ]);
    std::fs::write(
        &int_ext,
        "basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign,cRLSign\n",
    )
    .unwrap();
    openssl(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        &s(&int_key),
        "-out",
        &s(&int_csr),
        "-subj",
        &format!("/CN=vibe test {name} subordinate"),
    ]);
    openssl(&[
        "x509",
        "-req",
        "-in",
        &s(&int_csr),
        "-CA",
        &s(&root_pem),
        "-CAkey",
        &s(&root_key),
        "-CAcreateserial",
        "-out",
        &s(&int_pem),
        "-days",
        "2",
        "-extfile",
        &s(&int_ext),
    ]);
    std::fs::write(
        &leaf_ext,
        format!(
            "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\n\
             extendedKeyUsage=serverAuth\nsubjectAltName=DNS:{FIXTURE_HOST},DNS:localhost,IP:127.0.0.1\n"
        ),
    )
    .unwrap();
    openssl(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        &s(&leaf_key),
        "-out",
        &s(&leaf_csr),
        "-subj",
        &format!("/CN={FIXTURE_HOST}"),
    ]);
    openssl(&[
        "x509",
        "-req",
        "-in",
        &s(&leaf_csr),
        "-CA",
        &s(&int_pem),
        "-CAkey",
        &s(&int_key),
        "-CAcreateserial",
        "-out",
        &s(&leaf),
        "-days",
        "2",
        "-extfile",
        &s(&leaf_ext),
    ]);
    let full = p("leaf-full.pem");
    let mut bytes = std::fs::read(&leaf).unwrap();
    bytes.extend(std::fs::read(&int_pem).unwrap());
    std::fs::write(&full, bytes).unwrap();
    Chain {
        root_pem,
        intermediate_pem: int_pem,
        leaf_only: leaf,
        leaf_with_intermediate: full,
        key: leaf_key,
    }
}

/// A self-signed end-entity certificate for `host` — no authority above
/// it, so nothing can ever build a path to it and only naming its bytes
/// makes it trusted.
pub fn self_signed(dir: &Path, name: &str, host: &str) -> Ca {
    let p = |file: &str| dir.join(format!("{name}-{file}"));
    let s = |path: &PathBuf| path.to_str().unwrap().to_owned();
    let (cert, key) = (p("self.pem"), p("self-key.pem"));
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        &s(&key),
        "-out",
        &s(&cert),
        "-days",
        "2",
        "-subj",
        &format!("/CN={host}"),
        "-addext",
        "basicConstraints=critical,CA:FALSE",
        "-addext",
        "keyUsage=critical,digitalSignature,keyEncipherment",
        "-addext",
        "extendedKeyUsage=serverAuth",
        "-addext",
        &format!("subjectAltName=DNS:{host},IP:127.0.0.1"),
    ]);
    Ca {
        ca_pem: cert.clone(),
        cert,
        key,
    }
}

/// A PEM certificate written out again as DER, as the Windows and macOS
/// export dialogs write it.
pub fn as_der(pem: &Path, out: &Path) -> PathBuf {
    openssl(&[
        "x509",
        "-in",
        pem.to_str().unwrap(),
        "-outform",
        "DER",
        "-out",
        out.to_str().unwrap(),
    ]);
    out.to_path_buf()
}
