//! Accepted remote effects must survive cancellation in the owner's report.
use vibe_core::cancellation::CancellationToken;
use vibe_publish::{
    CreateOpts, PublishConfig, PublishError, Publisher, RepoCreator, RepoInfo, ValidatedOrg,
};
struct AcceptThenCancel(CancellationToken);
impl RepoCreator for AcceptThenCancel {
    fn repo_exists_controlled(
        &self,
        org: &ValidatedOrg,
        name: &str,
        _: &CancellationToken,
    ) -> Result<bool, PublishError> {
        self.repo_exists(org, name)
    }
    fn create_repo_controlled(
        &self,
        org: &ValidatedOrg,
        name: &str,
        opts: &CreateOpts,
        _: &CancellationToken,
    ) -> Result<RepoInfo, PublishError> {
        self.create_repo(org, name, opts)
    }

    fn host_name(&self) -> &str {
        "example.test"
    }
    fn repo_exists(&self, _: &ValidatedOrg, _: &str) -> Result<bool, PublishError> {
        Ok(false)
    }
    fn create_repo(
        &self,
        _: &ValidatedOrg,
        _: &str,
        _: &CreateOpts,
    ) -> Result<RepoInfo, PublishError> {
        self.0.cancel();
        Ok(RepoInfo {
            html_url: "https://example.test/org/repo".into(),
            clone_url: "https://example.test/org/repo.git".into(),
        })
    }
    fn push_url(&self, _: &ValidatedOrg, _: &str) -> String {
        panic!("cancelled publication must never start a later push")
    }
}
#[test]
fn accepted_repo_creation_is_reported_and_no_later_push_starts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[package]\ngroup='org.example'\nname='test'\nversion='1.0.0'\nkind='flow'\n",
    )
    .unwrap();
    let token = CancellationToken::new();
    let host = AcceptThenCancel(token.clone());
    let result = Publisher::new(&host).publish_controlled(
        &PublishConfig::with_defaults(dir.path().into(), "https://example.test/org".into()),
        &token,
    );
    match result.unwrap_err() {
        PublishError::Cancelled {
            repository_creation,
            release_push,
            ..
        } => {
            assert_eq!(repository_creation, "completed");
            assert_eq!(release_push, "not-started");
        }
        other => panic!("wrong outcome: {other}"),
    }
}

#[test]
fn accepted_post_without_response_is_unknown_and_socket_is_closed() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
        time::{Duration, Instant},
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (accepted, ready) = mpsc::channel();
    let server = std::thread::spawn(move || {
        for number in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&chunk[..n]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            if number == 0 {
                assert!(request.starts_with(b"GET "));
                stream
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
            } else {
                assert!(request.starts_with(b"POST "));
                accepted.send(()).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_millis(50)))
                    .unwrap();
                let deadline = Instant::now() + Duration::from_secs(1);
                while Instant::now() < deadline {
                    match stream.read(&mut chunk) {
                        Ok(0) => return true,
                        Ok(_) => {}
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) => {}
                        Err(_) => return true,
                    }
                }
                return false;
            }
        }
        false
    });
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[package]\ngroup='org.example'\nname='test'\nversion='1.0.0'\nkind='flow'\n",
    )
    .unwrap();
    let token = CancellationToken::new();
    let signal = token.clone();
    let cancel = std::thread::spawn(move || {
        ready.recv().unwrap();
        signal.cancel();
    });
    let host = vibe_publish::GithubRepoCreator::with_endpoint(
        vibe_publish::Token::from_explicit("fixture"),
        "org",
        &endpoint,
        "example.test",
    )
    .unwrap();
    let result = Publisher::new(&host).publish_controlled(
        &PublishConfig::with_defaults(dir.path().into(), "https://example.test/org".into()),
        &token,
    );
    cancel.join().unwrap();
    assert!(
        server.join().unwrap(),
        "POST network worker survived cancellation"
    );
    match result.unwrap_err() {
        PublishError::Cancelled {
            repository_creation,
            release_push,
            ..
        } => {
            assert_eq!(repository_creation, "unknown");
            assert_eq!(release_push, "not-started");
        }
        error => panic!("wrong outcome: {error}"),
    }
}
