//! Local network fixtures for the proxy / trust / px tests (PROP-068#proxy):
//! a test certificate authority, HTTPS and HTTP file servers (with redirect,
//! status and `git http-backend` routes), a forwarding CONNECT proxy that maps
//! every host to numeric loopback, and the fake px executable.
//!
//! Each server is a `python3` helper (`net_fixture.py`, `fake-px.py` beside
//! this file) owned by the fixture value: it is killed and reaped on drop, so
//! no listener survives a test. Linux/macOS only (`openssl`, `python3`).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

/// The host name every fixture certificate carries and the proxy maps to
/// `127.0.0.1`. It does not resolve, so a direct connection to it fails.
pub const FIXTURE_HOST: &str = "vibe-fixture.test";

fn script(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("common")
        .join(name)
}

/// The fake px executable (executable bit set), for `[proxy] px_path`.
pub fn fake_px() -> PathBuf {
    let path = script("fake-px.py");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut mode = std::fs::metadata(&path).unwrap().permissions();
        mode.set_mode(0o755);
        std::fs::set_permissions(&path, mode).unwrap();
    }
    path
}

/// A CA and one leaf it signed (SAN `vibe-fixture.test`, `localhost`,
/// `127.0.0.1`).
#[derive(Debug, Clone)]
pub struct TestCa {
    pub ca_pem: PathBuf,
    pub cert: PathBuf,
    pub key: PathBuf,
}

impl TestCa {
    fn generate(dir: &Path, label: &str) -> Self {
        let out = Command::new("python3")
            .arg(script("net_fixture.py"))
            .args(["ca", dir.to_str().unwrap(), label])
            .output()
            .expect("python3 runs the CA fixture");
        assert!(
            out.status.success(),
            "CA fixture failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let path = |k: &str| PathBuf::from(v[k].as_str().unwrap());
        Self {
            ca_pem: path("ca_pem"),
            cert: path("cert"),
            key: path("key"),
        }
    }

    /// The trusted test CA (callers put `ca_pem` in `[proxy] ca_file`).
    pub fn new(dir: &Path) -> Self {
        Self::generate(dir, "trusted")
    }

    /// An unrelated CA's leaf: what an untrusted corporate MITM looks like.
    pub fn untrusted(dir: &Path) -> Self {
        Self::generate(dir, "untrusted")
    }
}

/// One owned helper process that printed `{"port": N}` when ready.
struct Helper {
    child: Child,
    stdin: Option<ChildStdin>,
    port: u16,
}

impl Helper {
    fn spawn(mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("python3 runs the network fixture");
        let stdin = child.stdin.take();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(line.trim())
            .unwrap_or_else(|_| panic!("network fixture did not start: {line:?}"));
        let port = v["port"].as_u64().unwrap() as u16;
        Self { child, stdin, port }
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// A file server over `root`; `root/git/<repo>.git` is served as smart HTTP
/// under `/git/<repo>.git`.
pub struct FileServer {
    helper: Helper,
    log: tempfile::NamedTempFile,
    scheme: &'static str,
}

impl FileServer {
    pub fn port(&self) -> u16 {
        self.helper.port
    }
    /// `<scheme>://<host>:<port>/<path>` (`path` without a leading slash).
    pub fn url(&self, host: &str, path: &str) -> String {
        format!("{}://{host}:{}/{path}", self.scheme, self.helper.port)
    }
    /// `METHOD /path host=<Host header>` per request served.
    pub fn requests(&self) -> Vec<String> {
        read_lines(self.log.path())
    }
}

pub struct HttpsServer;
impl HttpsServer {
    pub fn start(ca: &TestCa, root: &Path) -> FileServer {
        let log = tempfile::NamedTempFile::new().unwrap();
        let mut command = Command::new("python3");
        command.arg(script("net_fixture.py")).arg("https").args([
            &ca.cert,
            &ca.key,
            &root.to_path_buf(),
            &log.path().to_path_buf(),
        ]);
        FileServer {
            helper: Helper::spawn(command),
            log,
            scheme: "https",
        }
    }
}

pub struct HttpServer;
impl HttpServer {
    pub fn start(root: &Path) -> FileServer {
        let log = tempfile::NamedTempFile::new().unwrap();
        let mut command = Command::new("python3");
        command
            .arg(script("net_fixture.py"))
            .arg("http")
            .args([&root.to_path_buf(), &log.path().to_path_buf()]);
        FileServer {
            helper: Helper::spawn(command),
            log,
            scheme: "http",
        }
    }
}

/// A forwarding proxy (CONNECT + absolute-form HTTP) mapping every host to
/// `127.0.0.1`.
pub struct ConnectProxy {
    helper: Helper,
    log: tempfile::NamedTempFile,
}

impl ConnectProxy {
    fn spawn(require_auth: bool) -> Self {
        let log = tempfile::NamedTempFile::new().unwrap();
        let mut command = Command::new("python3");
        command
            .arg(fake_px())
            .arg("--listen=127.0.0.1")
            .arg("--port=0")
            .arg(format!("--request-log={}", log.path().display()));
        if require_auth {
            command.arg("--require-auth");
        }
        command.env_remove("FAKE_PX_FAIL").env_remove("FAKE_PX_LOG");
        Self {
            helper: Helper::spawn(command),
            log,
        }
    }
    pub fn start() -> Self {
        Self::spawn(false)
    }
    /// Answers `407 Proxy Authentication Required` to every request.
    pub fn start_requiring_auth() -> Self {
        Self::spawn(true)
    }
    pub fn port(&self) -> u16 {
        self.helper.port
    }
    /// `http://127.0.0.1:<port>`.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.helper.port)
    }
    /// Request lines seen, e.g. `CONNECT vibe-fixture.test:4443 HTTP/1.1`.
    pub fn requests(&self) -> Vec<String> {
        read_lines(self.log.path())
    }
}
