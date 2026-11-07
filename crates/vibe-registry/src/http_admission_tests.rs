//! Bounded numeric-loopback contact oracle for HTTP admission.
use crate::{ContactRefusal, HttpAdmission, IndexAuth, IndexClient, IndexError, ProbeOutcome};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

struct Server {
    url: String,
    count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(host: &str, respond: impl Fn(&str, usize) -> String + Send + 'static) -> Self {
        let listener = TcpListener::bind((host, 0)).expect("numeric loopback bind");
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (c, s) = (count.clone(), stop.clone());
        let worker = thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut bytes = [0; 8192];
                        let n = stream.read(&mut bytes).unwrap_or(0);
                        let seen = c.fetch_add(1, Ordering::SeqCst);
                        let answer = respond(&String::from_utf8_lossy(&bytes[..n]), seen);
                        let _ = stream.write_all(answer.as_bytes());
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("loopback accept: {e}"),
                }
            }
        });
        Self {
            url,
            count,
            stop,
            worker: Some(worker),
        }
    }
    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}
fn response(status: u16, body: &str, extra: &str) -> String {
    format!(
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",
        body.len()
    )
}
fn binding(hosts: &[&str]) -> Arc<HttpAdmission> {
    Arc::new(HttpAdmission::new(hosts.iter().copied()).unwrap())
}
fn excluded(error: IndexError) {
    assert!(
        matches!(
            error,
            IndexError::ContactRefused(ContactRefusal::ExcludedHost { .. })
        ),
        "{error:?}"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn bound_probe_redirect_denies_before_second_contact() {
    let b = Server::new("127.0.0.2", |_, _| response(200, "{}", ""));
    let target = b.url.clone();
    let a = Server::new("127.0.0.1", move |_, _| {
        response(302, "", &format!("Location: {target}/hello.json\r\n"))
    });
    let result =
        IndexClient::probe_with_admission(&a.url, IndexAuth::None, binding(&["127.0.0.2"]));
    assert_eq!(a.count(), 1);
    assert_eq!(b.count(), 0);
    excluded(result.unwrap_err());
}
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn bound_probe_initial_deny_contacts_nothing() {
    let a = Server::new("127.0.0.1", |_, _| response(200, "{}", ""));
    excluded(
        IndexClient::probe_with_admission(&a.url, IndexAuth::None, binding(&["127.0.0.1"]))
            .unwrap_err(),
    );
    assert_eq!(a.count(), 0);
}
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn allowed_bound_probe_and_raw_read_succeed() {
    let a = Server::new("127.0.0.1", |request, _| {
        if request.contains("hello.json") {
            response(404, "", "")
        } else {
            response(200, "exact bytes", "")
        }
    });
    assert!(matches!(
        IndexClient::probe_with_admission(&a.url, IndexAuth::None, binding(&[])).unwrap(),
        ProbeOutcome::Found(_)
    ));
    let backend = crate::ShellGit::new()
        .with_raw_base(&a.url)
        .with_http_admission(binding(&[]));
    assert_eq!(
        crate::GitBackend::fetch_file_at_ref(
            &backend,
            "https://github.com/o/r",
            "v1.0.0",
            "vibe.toml"
        )
        .unwrap(),
        b"exact bytes"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn allowed_redirect_proves_second_server_is_reachable() {
    let b = Server::new("127.0.0.2", |_, _| response(200, "", ""));
    let target = b.url.clone();
    let a = Server::new("127.0.0.1", move |_, _| {
        response(302, "", &format!("Location: {target}/primary.jsonl\r\n"))
    });
    let client = IndexClient::at_with_admission(&a.url, IndexAuth::None, binding(&[]));
    assert!(client.primary().unwrap().unwrap().is_empty());
    assert_eq!(a.count(), 1);
    assert_eq!(b.count(), 1);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn every_index_get_family_denies_and_pools_are_isolated() {
    let a = Server::new("127.0.0.1", |_, _| response(200, "", ""));
    let allowed = IndexClient::at_with_admission(&a.url, IndexAuth::None, binding(&[]));
    assert!(allowed.primary().unwrap().unwrap().is_empty());
    let denied = IndexClient::at_with_admission(&a.url, IndexAuth::None, binding(&["127.0.0.1"]));
    let group = vibe_core::Group::parse("org.example").unwrap();
    excluded(denied.list_version_entries(&group, "p").unwrap_err());
    excluded(denied.name_candidates("p").unwrap_err());
    excluded(denied.lookup_purl("pkg:cargo/test").unwrap_err());
    excluded(denied.search("test", None, None).unwrap_err());
    excluded(denied.primary().unwrap_err());
    excluded(denied.repomd().unwrap_err());
    excluded(denied.by_name_is_current("p", b"{}").unwrap_err());
    excluded(denied.clone().primary().unwrap_err());
    assert_eq!(a.count(), 1);
    assert!(allowed.primary().unwrap().unwrap().is_empty());
    assert_eq!(a.count(), 2);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn freshness_and_static_search_preserve_redirect_refusal() {
    let b = Server::new("127.0.0.2", |_, _| response(200, "{}", ""));
    let target = b.url.clone();
    let a = Server::new("127.0.0.1", move |request, _| {
        if request.contains("/v1/packages?") {
            response(404, "", "")
        } else if request.contains("/by-name/") {
            response(200, "{}", "")
        } else {
            response(302, "", &format!("Location: {target}/data\r\n"))
        }
    });
    let client = IndexClient::at_with_admission(&a.url, IndexAuth::None, binding(&["127.0.0.2"]));
    excluded(
        client
            .list_version_entries(&vibe_core::Group::parse("org.example").unwrap(), "p")
            .unwrap_err(),
    );
    excluded(client.by_name_is_current("p", b"{}").unwrap_err());
    excluded(client.search("test", None, None).unwrap_err());
    assert_eq!(a.count(), 4);
    assert_eq!(b.count(), 0);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn raw_rewrite_denial_is_terminal_and_anonymous_binding_is_retained() {
    use crate::{GitBackend, GitError, ShellGit};
    let a = Server::new("127.0.0.1", |_, _| response(200, "bytes", ""));
    let sleeps = Arc::new(AtomicUsize::new(0));
    let copy = sleeps.clone();
    let backend = ShellGit::new()
        .with_raw_base(&a.url)
        .with_raw_sleeper(move |_| {
            copy.fetch_add(1, Ordering::SeqCst);
        })
        .with_http_admission(binding(&["127.0.0.1"]));
    let anonymous = backend.anonymized_for_public().unwrap();
    for backend in [&backend as &dyn GitBackend, anonymous.as_ref()] {
        assert!(matches!(
            backend.fetch_file_at_ref("https://github.com/o/r", "v1.0.0", "vibe.toml"),
            Err(GitError::ContactRefused(
                ContactRefusal::ExcludedHost { .. }
            ))
        ));
    }
    assert_eq!(a.count(), 0);
    assert_eq!(sleeps.load(Ordering::SeqCst), 0);
    // Actual production rewrite, denied before DNS or transport.
    let backend = ShellGit::new().with_http_admission(binding(&["raw.githubusercontent.com"]));
    assert!(matches!(
        backend.fetch_file_at_ref("https://github.com/o/r", "v1.0.0", "vibe.toml"),
        Err(GitError::ContactRefused(
            ContactRefusal::ExcludedHost { .. }
        ))
    ));
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn raw_retry_rechecks_redirect_and_stops_on_denial() {
    use crate::{GitBackend, GitError, ShellGit};
    let b = Server::new("127.0.0.2", |_, _| response(200, "bytes", ""));
    let target = b.url.clone();
    let a = Server::new("127.0.0.1", move |_, n| {
        if n == 0 {
            response(503, "", "")
        } else {
            response(302, "", &format!("Location: {target}/data\r\n"))
        }
    });
    let sleeps = Arc::new(AtomicUsize::new(0));
    let copy = sleeps.clone();
    let backend = ShellGit::new()
        .with_raw_base(&a.url)
        .with_raw_sleeper(move |_| {
            copy.fetch_add(1, Ordering::SeqCst);
        })
        .with_http_admission(binding(&["127.0.0.2"]));
    assert!(matches!(
        backend.fetch_file_at_ref("https://github.com/o/r", "v1.0.0", "vibe.toml"),
        Err(GitError::ContactRefused(
            ContactRefusal::ExcludedHost { .. }
        ))
    ));
    assert_eq!(a.count(), 2);
    assert_eq!(b.count(), 0);
    assert_eq!(sleeps.load(Ordering::SeqCst), 1);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn redirects_have_finite_limit_and_invalid_locations_are_typed() {
    for hops in [10, 11] {
        let a = Server::new("127.0.0.1", move |_, n| {
            if n < hops {
                response(302, "", "Location: /next\r\n")
            } else {
                response(200, "", "")
            }
        });
        let client = IndexClient::at_with_admission(&a.url, IndexAuth::None, binding(&[]));
        let result = client.primary();
        if hops == 10 {
            assert!(result.unwrap().unwrap().is_empty());
        } else {
            assert!(matches!(
                result,
                Err(IndexError::ContactRefused(ContactRefusal::RedirectLimit))
            ));
        }
        assert_eq!(a.count(), 11);
    }
    let a = Server::new("127.0.0.1", |_, _| response(302, "", ""));
    assert!(matches!(
        IndexClient::at_with_admission(&a.url, IndexAuth::None, binding(&[])).primary(),
        Err(IndexError::ContactRefused(ContactRefusal::InvalidRedirect))
    ));
    assert_eq!(a.count(), 1);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn resolver_binds_before_probe_and_keeps_index_denial_through_git_fallback() {
    let b = Server::new("127.0.0.2", |_, _| response(200, "{}", ""));
    let target = b.url.clone();
    let a = Server::new("127.0.0.1", move |request, _| {
        if request.contains("hello.json") {
            response(404, "", "")
        } else if request.contains("repomd.json") {
            response(200, "{}", "")
        } else {
            response(302, "", &format!("Location: {target}/data\r\n"))
        }
    });
    let reg: vibe_core::manifest::RegistrySection = toml::from_str(&format!(
        "name='http-admission-unique-test'\nurl='https://github.com/o'\nindex_url='{}'",
        a.url
    ))
    .unwrap();
    let cache = tempfile::tempdir().unwrap();
    let denied = crate::MultiRegistryResolver::from_manifest_with_http_admission(
        std::slice::from_ref(&reg),
        &[],
        &[],
        cache.path().to_owned(),
        0,
        binding(&["127.0.0.1"]),
    );
    assert!(matches!(
        denied,
        Err(crate::RegistryError::ContactRefused(
            ContactRefusal::ExcludedHost { .. }
        ))
    ));
    assert_eq!(a.count(), 0);
    let resolver = crate::MultiRegistryResolver::from_manifest_with_http_admission(
        &[reg],
        &[],
        &[],
        cache.path().to_owned(),
        0,
        binding(&["127.0.0.2"]),
    )
    .unwrap();
    let result = resolver.registries()[0]
        .list_versions(&vibe_core::Group::parse("org.example").unwrap(), "p");
    assert!(matches!(
        result,
        Err(crate::RegistryError::ContactRefused(
            ContactRefusal::ExcludedHost { .. }
        ))
    ));
    assert!(matches!(
        resolver.resolve_name_candidates("p"),
        Err(crate::RegistryError::ContactRefused(
            ContactRefusal::ExcludedHost { .. }
        ))
    ));
    assert_eq!(a.count(), 5);
    assert_eq!(b.count(), 0);
}

#[cfg(unix)]
pub(crate) fn sentinel_git() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("git-sentinel");
    let record = dir.path().join("spawned");
    std::fs::write(
        &binary,
        format!(
            "#!/bin/sh\nprintf invoked >> '{}'\nexit 0\n",
            record.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    (dir, binary, record)
}

#[cfg(unix)]
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn bound_raw_decline_refuses_native_before_sentinel_spawn() {
    use crate::{GitBackend, GitError, ShellGit};
    let (_dir, binary, record) = sentinel_git();
    let a = Server::new("127.0.0.1", |_, _| response(404, "", ""));
    let backend = ShellGit::new()
        .with_binary(binary)
        .with_raw_base(&a.url)
        .with_http_admission(binding(&[]));
    for url in [
        "https://github.com/o/r",
        "ssh://git@example.invalid/o/r",
        "file:///tmp/unused",
    ] {
        assert!(matches!(
            backend.fetch_file_at_ref(url, "v1.0.0", "vibe.toml"),
            Err(GitError::ContactRefused(
                ContactRefusal::NativeTransportUnavailable
            ))
        ));
    }
    assert_eq!(a.count(), 1);
    assert!(!record.exists());
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn bound_owners_never_attach_plaintext_bearer_or_url_credentials() {
    let a = Server::new("127.0.0.1", |request, _| {
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
        response(200, "", "")
    });
    let auth = IndexAuth::Bearer(crate::BearerToken::new("fixture".into()));
    IndexClient::at_with_admission(&a.url, auth, binding(&[]))
        .primary()
        .unwrap();
    use crate::GitBackend;
    crate::ShellGit::new()
        .with_raw_base(&a.url)
        .with_http_admission(binding(&[]))
        .fetch_file_at_ref("https://user:fixture@github.com/o/r", "v1.0.0", "vibe.toml")
        .unwrap();
    let credentialed = a.url.replace("http://", "http://user:fixture@");
    assert!(matches!(
        IndexClient::at_with_admission(credentialed, IndexAuth::None, binding(&[])).primary(),
        Err(IndexError::ContactRefused(ContactRefusal::UrlCredentials))
    ));
    assert_eq!(a.count(), 2);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn unbound_driver_preserves_legacy_redirect_behavior_with_isolated_test_client() {
    let b = Server::new("127.0.0.2", |_, _| response(200, "legacy", ""));
    let target = b.url.clone();
    let a = Server::new("127.0.0.1", move |_, _| {
        response(302, "", &format!("Location: {target}/data\r\n"))
    });
    // Test-only client excludes ambient proxies; no process environment changes.
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .unwrap();
    let response = crate::http_admission::get(&client, client.get(&a.url), &a.url, None).unwrap();
    assert_eq!(response.text().unwrap(), "legacy");
    assert_eq!(a.count(), 1);
    assert_eq!(b.count(), 1);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn bound_raw_transient_status_retries_still_recover() {
    use crate::GitBackend;
    let a = Server::new("127.0.0.1", |_, n| {
        if n < 2 {
            response(503, "", "")
        } else {
            response(200, "recovered", "")
        }
    });
    let sleeps = Arc::new(AtomicUsize::new(0));
    let copy = sleeps.clone();
    let backend = crate::ShellGit::new()
        .with_raw_base(&a.url)
        .with_raw_sleeper(move |_| {
            copy.fetch_add(1, Ordering::SeqCst);
        })
        .with_http_admission(binding(&[]));
    assert_eq!(
        backend
            .fetch_file_at_ref("https://github.com/o/r", "v1.0.0", "vibe.toml")
            .unwrap(),
        b"recovered"
    );
    assert_eq!(a.count(), 3);
    assert_eq!(sleeps.load(Ordering::SeqCst), 2);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn bound_auth_and_format_errors_keep_their_existing_discriminants() {
    let a = Server::new("127.0.0.1", |_, _| response(401, "", ""));
    let client =
        IndexClient::at_with_admission(&a.url, IndexAuth::HttpIncapable("ssh"), binding(&[]));
    assert!(matches!(
        client.primary(),
        Err(IndexError::AuthIncapable { status: 401, .. })
    ));
    assert!(matches!(
        IndexClient::probe_with_admission(&a.url, IndexAuth::HttpIncapable("ssh"), binding(&[])),
        Ok(ProbeOutcome::Refused { .. })
    ));
    let b = Server::new("127.0.0.1", |_, _| response(200, "not json", ""));
    assert!(matches!(
        IndexClient::at_with_admission(&b.url, IndexAuth::None, binding(&[])).repomd(),
        Err(IndexError::Malformed { .. })
    ));
    assert_eq!(a.count(), 2);
    assert_eq!(b.count(), 1);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn bound_probe_refuses_hostless_and_opaque_addresses_before_request_build() {
    let untouched = Server::new("127.0.0.1", |_, _| {
        panic!("unsupported address must never contact HTTP")
    });
    for address in [
        "file:///tmp/vibe-http-never-read",
        "mailto:test@example.invalid",
        "data:text/plain,fixture",
        "not a URL",
    ] {
        let result = IndexClient::probe_with_admission(address, IndexAuth::None, binding(&[]));
        assert!(
            matches!(
                result,
                Err(IndexError::ContactRefused(
                    ContactRefusal::UnsupportedScheme { .. }
                ))
            ),
            "{address}: {result:?}"
        );
    }
    assert_eq!(untouched.count(), 0);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn bound_purl_refuses_unsupported_base_before_url_construction() {
    for address in [
        "file:///tmp/vibe-http-never-read",
        "mailto:test@example.invalid",
        "data:text/plain,fixture",
        "not a URL",
    ] {
        let result = IndexClient::at_with_admission(address, IndexAuth::None, binding(&[]))
            .lookup_purl("pkg:cargo/test");
        assert!(
            matches!(
                result,
                Err(IndexError::ContactRefused(
                    ContactRefusal::UnsupportedScheme { .. }
                ))
            ),
            "{address}: {result:?}"
        );
    }
    // These unbound constructor failures preserve their ordinary error owner.
    for address in ["mailto:test@example.invalid", "not a URL"] {
        assert!(matches!(
            IndexClient::at(address).lookup_purl("pkg:cargo/test"),
            Err(IndexError::Http { .. })
        ));
    }
}

#[path = "http_admission_observation_tests.rs"]
mod observation;
