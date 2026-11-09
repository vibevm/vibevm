//! Host-API GET requests that need headers (`PROP-019#release-sources`).
//!
//! Release bytes go through the operation's controlled transport
//! (`download.rs`). The release-list endpoints of GitHub and GitVerse need
//! what that anonymous payload client cannot send — a `User-Agent`, a
//! versioned `Accept`, and for GitVerse a bearer token — so this small
//! client sends them, under the same contact law: every address, the first
//! and every redirect hop, is admitted by the operation's source policy
//! before a connection is opened, redirects are followed by hand, a token
//! is never sent past the origin it was meant for, and the token value is
//! never printed or logged.
//!
//! The same three needs arise wherever vibe asks a Git host's REST API a
//! question, so this is the one such client in the CLI: `vibe registry
//! mirror` lists an organisation's repositories through it
//! (`commands::registry::mirror::org_api`). Callers that must see a
//! non-success status rather than a message about it use [`ApiClient::request`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#release-sources");

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use vibe_publish::token::Token;
use vibe_registry::HttpAdmission;

const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_REDIRECTS: usize = 5;
/// A release list is small; anything larger is not one.
pub(crate) const MAX_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;

/// An admitted, header-carrying GET client for host-API documents.
pub(crate) struct ApiClient {
    admission: Option<Arc<HttpAdmission>>,
    client: reqwest::blocking::Client,
}

/// What a document request answered.
pub(crate) enum Fetched {
    Body(Vec<u8>),
    /// The server answered, and the document is not there (404/410).
    Absent,
}

/// What one admitted request answered, status and all — the form a caller
/// that decides on the status itself needs (a refusal it must explain in
/// its own words, say).
pub(crate) enum Answer {
    /// A success, with the address it came from (safe to print).
    Ok(reqwest::blocking::Response, String),
    /// Any other status, with the address it came from.
    Status(reqwest::StatusCode, String),
}

impl ApiClient {
    /// A client under `admission` (the operation's source policy); `None`
    /// is the uncontrolled posture, where the ambient `[proxy]` applies.
    pub(crate) fn new(admission: Option<Arc<HttpAdmission>>) -> Result<Self> {
        let mut builder = reqwest::blocking::Client::builder()
            .user_agent(format!("vibevm/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(super::download::CONNECT_TIMEOUT)
            .timeout(TIMEOUT)
            .redirect(reqwest::redirect::Policy::none());
        // The controlled transport's posture: no implicit proxy hop the
        // policy could not see — only the configured `[proxy]` route, whose
        // requested host admission has judged (PROP-068 `#proxy`).
        builder = vibe_net::http::route(
            builder,
            if admission.is_some() {
                vibe_net::http::Ambient::Refuse
            } else {
                vibe_net::http::Ambient::Inherit
            },
        )?;
        Ok(Self {
            admission,
            client: builder
                .build()
                .context("building the release-list client")?,
        })
    }

    /// GET a document of at most [`MAX_DOCUMENT_BYTES`].
    pub(crate) fn get(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        token: Option<&Token>,
    ) -> Result<Fetched> {
        let Some((response, at)) = self.send(url, headers, token)? else {
            return Ok(Fetched::Absent);
        };
        Ok(Fetched::Body(document(response, &at)?))
    }

    /// GET `url` with `headers`, sending `token` as a bearer only to the
    /// origin of `url`. Redirects are followed by hand, each hop admitted.
    /// `None` is a 404/410 answer; the address is returned for messages.
    pub(crate) fn send(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        token: Option<&Token>,
    ) -> Result<Option<(reqwest::blocking::Response, String)>> {
        match self.request(url, headers, token)? {
            Answer::Ok(response, at) => Ok(Some((response, at))),
            Answer::Status(status, _)
                if status == reqwest::StatusCode::NOT_FOUND
                    || status == reqwest::StatusCode::GONE =>
            {
                Ok(None)
            }
            Answer::Status(status, at)
                if status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN =>
            {
                bail!(
                    "`{at}` answered {status}: the release list needs a token \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
                      fix: put a read token in the per-host file `~/.vibe/<host-prefix>.publish.token`, \
                      e.g. `~/.vibe/gitverse.publish.token`)"
                )
            }
            Answer::Status(status, at) => bail!("`{at}` answered {status}"),
        }
    }

    /// [`ApiClient::send`] without the status interpretation: the caller
    /// receives the status it did not expect and says what it means.
    pub(crate) fn request(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        token: Option<&Token>,
    ) -> Result<Answer> {
        let mut current = reqwest::Url::parse(url).with_context(|| format!("parsing `{url}`"))?;
        let origin = current.origin();
        for _ in 0..=MAX_REDIRECTS {
            if !matches!(current.scheme(), "https" | "http") {
                bail!(
                    "release list `{}` is not an HTTP(S) address \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-066#locations)",
                    safe(&current)
                );
            }
            if let Some(admission) = &self.admission {
                admission
                    .check_address(current.as_str())
                    .with_context(|| format!("contacting `{}`", safe(&current)))?;
            }
            let mut request = self.client.get(current.clone());
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            if let Some(token) = token.filter(|_| current.origin() == origin) {
                request = request.bearer_auth(token.value());
            }
            let response = request
                .send()
                .with_context(|| format!("requesting `{}`", safe(&current)))?;
            let status = response.status();
            if status.is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .with_context(|| format!("`{}` redirected nowhere", safe(&current)))?;
                current = current.join(location).with_context(|| {
                    format!("`{}` redirected to an invalid address", safe(&current))
                })?;
                continue;
            }
            if !status.is_success() {
                return Ok(Answer::Status(status, safe(&current)));
            }
            return Ok(Answer::Ok(response, safe(&current)));
        }
        bail!("`{url}` redirected more than {MAX_REDIRECTS} times")
    }
}

/// Read a response body of at most [`MAX_DOCUMENT_BYTES`]; `at` is the
/// address it came from, for messages.
pub(crate) fn document(response: reqwest::blocking::Response, at: &str) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    response
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut body)
        .with_context(|| format!("reading `{at}`"))?;
    if body.len() as u64 > MAX_DOCUMENT_BYTES {
        bail!("`{at}` is larger than a release list");
    }
    Ok(body)
}

/// The address without userinfo, query or fragment.
fn safe(url: &reqwest::Url) -> String {
    vibe_core::progress::transfer::safe_remote(url.as_str())
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    use super::*;

    /// Serve canned responses in order, recording each request head.
    fn serve(responses: Vec<String>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&seen);
        std::thread::spawn(move || {
            for (stream, response) in listener.incoming().zip(responses) {
                let Ok(mut stream) = stream else { continue };
                let mut head = Vec::new();
                let mut byte = [0_u8; 1];
                while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                    head.push(byte[0]);
                }
                record
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&head).into_owned());
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (address, seen)
    }

    fn ok(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn headers_and_the_token_reach_the_origin_only() {
        let (other, other_seen) = serve(vec![ok("[]")]);
        let (origin, seen) = serve(vec![format!(
            "HTTP/1.1 302 Found\r\nLocation: {other}/elsewhere\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )]);
        let client = ApiClient::new(None).unwrap();
        let token = Token::from_explicit("canary-token-value");
        let Fetched::Body(body) = client
            .get(
                &format!("{origin}/list"),
                &[("Accept", "application/x-test")],
                Some(&token),
            )
            .unwrap()
        else {
            panic!("a body")
        };
        assert_eq!(body, b"[]");
        let first = seen.lock().unwrap()[0].to_ascii_lowercase();
        assert!(
            first.contains("authorization: bearer canary-token-value"),
            "{first}"
        );
        assert!(first.contains("accept: application/x-test"), "{first}");
        assert!(first.contains("user-agent: vibevm/"), "{first}");
        let hop = other_seen.lock().unwrap()[0].to_ascii_lowercase();
        assert!(
            !hop.contains("authorization"),
            "the token stays with its origin: {hop}"
        );
    }

    #[test]
    fn an_excluded_host_is_refused_before_any_connection() {
        let admission = Arc::new(HttpAdmission::new(["github.com"]).unwrap());
        let client = ApiClient::new(Some(admission)).unwrap();
        let error = client
            .get("https://github.com/vibevm/vibevm/releases", &[], None)
            .err()
            .expect("refused");
        assert!(format!("{error:#}").contains("github.com"), "{error:#}");
    }

    #[test]
    fn a_missing_document_and_a_refused_token_are_told_apart() {
        let (base, _) = serve(vec![
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        ]);
        let client = ApiClient::new(None).unwrap();
        assert!(matches!(
            client.get(&format!("{base}/a"), &[], None).unwrap(),
            Fetched::Absent
        ));
        let error = client.get(&format!("{base}/b"), &[], None).err().unwrap();
        assert!(
            error.to_string().contains("gitverse.publish.token"),
            "{error}"
        );
    }
}
