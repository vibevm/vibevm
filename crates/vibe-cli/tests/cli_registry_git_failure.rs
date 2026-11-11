//! What an operator reads when a registry git operation is answered by
//! a redirect — the shape a TLS-terminating corporate gateway produces
//! for an external host (PROP-002 `#failure-discriminator`).
//!
//! The fixture is a loopback server that answers the smart-HTTP
//! endpoint with `302`, exactly as such a gateway does. Git then fails
//! with `The requested URL returned error: 302` and nothing else; the
//! test asserts that `vibe` says what it was doing, where, for which
//! package, why, and what to change — and that git's own words are
//! still there, below that explanation.

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{UserScratch, git_available};

/// A loopback server that answers every request with `302 Found`.
///
/// Bounded and owned: the listener thread stops and is joined on drop,
/// so no detached listener survives the test (the standing shape of the
/// fixtures in `tests/common/`).
struct RedirectingHost {
    /// `http://127.0.0.1:<port>` — no trailing slash.
    url: String,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl RedirectingHost {
    fn new() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let worker = thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        socket.set_nonblocking(false).unwrap();
                        socket
                            .set_read_timeout(Some(Duration::from_millis(500)))
                            .unwrap();
                        let mut request = [0u8; 8192];
                        let _ = socket.read(&mut request);
                        // What an intercepting gateway answers the
                        // smart-HTTP ref listing with: a redirect to its
                        // own sign-in page, never the ref stream.
                        let _ = socket.write_all(
                            b"HTTP/1.1 302 Found\r\n\
                              Location: https://gateway.invalid/sign-in\r\n\
                              Content-Length: 0\r\nConnection: close\r\n\r\n",
                        );
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
        });
        RedirectingHost {
            url,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for RedirectingHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
)]
fn a_redirected_registry_read_explains_itself_before_quoting_git() {
    if !git_available() {
        return;
    }
    let host = RedirectingHost::new();
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let mirror = project.path().join("local-mirror");
    fs::create_dir_all(&mirror).unwrap();
    let mirror_url = format!("file://{}", mirror.to_string_lossy().replace('\\', "/"));
    // Two declared registries: the intercepted one the read goes to,
    // and a local mirror — so the failure has a profile to report.
    fs::write(
        project.path().join("vibe.toml"),
        format!(
            "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n\
             [[registry]]\nname = \"corp\"\nurl = \"{}/org\"\nindex_url = \"none\"\n\n\
             [[registry]]\nname = \"sitemirror\"\ntype = \"localdir\"\nurl = \"{mirror_url}\"\n",
            host.url
        ),
    )
    .unwrap();

    let output = user
        .vibe()
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        // What makes the corporate case the corporate case: the 3xx is
        // the answer, not a hop. Git follows a redirect on an initial
        // ref listing by default, so without this the fixture's
        // `Location` would be chased and the failure would be about the
        // host it points at. The real gateway hits the RPC leg, where
        // git does not follow — the same stderr this produces.
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "http.followRedirects")
        .env("GIT_CONFIG_VALUE_0", "false")
        .args(["install", "org.example/thing", "--assume-yes", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "install should have failed");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // The errand, in words — not an argv, and not `origin`.
    assert!(
        text.contains("listing the versions of `org.example/thing` failed"),
        "{text}"
    );
    // The registry alias and the endpoint actually contacted.
    assert!(text.contains("registry: corp"), "{text}");
    assert!(
        text.contains(&format!("{}/org/org.example.thing.git", host.url)),
        "{text}"
    );
    // The translated cause and the fix that answers it.
    assert!(text.contains("the server redirected (HTTP 302)"), "{text}");
    assert!(text.contains("exclude_hosting"), "{text}");
    assert!(text.contains("vibe dl --test-proxy"), "{text}");
    // The registries the profile allows, so the unused mirror is visible.
    assert!(text.contains("allowed registries:"), "{text}");
    assert!(text.contains("sitemirror (localdir)"), "{text}");
    // Git's own words survive — below the explanation, not instead of it.
    let said = text.find("git said:").unwrap_or_else(|| panic!("{text}"));
    assert!(text.find("cause:").unwrap() < said, "{text}");
    assert!(text[said..].contains("302"), "{text}");
}
