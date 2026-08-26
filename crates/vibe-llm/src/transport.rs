use std::io::Read;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use specmark::spec;

use crate::{ApiKey, Endpoint};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy");

/// Maximum accepted 2xx Chat Completions body. The transport reads at most
/// one byte beyond it to distinguish an exact-limit body from overflow.
pub const MAX_CHAT_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// Default bound on establishing one chat connection, TCP plus the TLS
/// handshake. A reachable provider answers connection attempts quickly, so
/// ten seconds tolerates a slow handshake while still turning a hung connect
/// into a [`TransportError::RequestFailed`] instead of an indefinite block.
pub const CHAT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default bound on one complete chat exchange — connect, request, server
/// thinking time, and reading the response body up to
/// [`MAX_CHAT_RESPONSE_BYTES`]. Synchronous chat completions legitimately
/// take minutes on long generations, so the bound is five minutes rather
/// than a millisecond-scale default; when it fires the call surfaces
/// [`TransportError::RequestFailed`] instead of hanging the calling thread.
pub const CHAT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Injectable HTTP boundary used by the provider and its deterministic tests.
///
/// ```
/// use vibe_llm::{ApiKey, ChatTransport, Endpoint, TransportError, TransportResponse};
///
/// struct StaticTransport;
/// impl ChatTransport for StaticTransport {
///     fn post_json(
///         &self,
///         _: &Endpoint,
///         _: Option<&ApiKey>,
///         _: &[u8],
///     ) -> Result<TransportResponse, TransportError> {
///         Ok(TransportResponse::new(200, None, b"{}".to_vec()))
///     }
/// }
/// let endpoint = Endpoint::parse("http://localhost/v1/chat/completions", false).unwrap();
/// assert_eq!(StaticTransport.post_json(&endpoint, None, b"{}").unwrap().status(), 200);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub trait ChatTransport: Send + Sync {
    fn post_json(
        &self,
        endpoint: &Endpoint,
        api_key: Option<&ApiKey>,
        body: &[u8],
    ) -> Result<TransportResponse, TransportError>;
}

/// Production blocking transport. Redirects are disabled at client creation,
/// once, for every request this transport can issue, and every client this
/// type builds is time-bounded: [`CHAT_CONNECT_TIMEOUT`] on establishing the
/// connection and [`CHAT_REQUEST_TIMEOUT`] on the whole exchange, connection
/// through the completed response body.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES")]
pub struct ReqwestChatTransport {
    remote_client: reqwest::blocking::Client,
    direct_loopback_client: reqwest::blocking::Client,
}

impl ReqwestChatTransport {
    /// Build the two blocking clients (remote/proxy-capable and direct
    /// loopback), both with redirects disabled and both bounded by the
    /// default [`CHAT_CONNECT_TIMEOUT`] / [`CHAT_REQUEST_TIMEOUT`] pair.
    /// `.no_proxy()` stays direct-loopback-only.
    ///
    /// # Panics
    ///
    /// Upstream `reqwest::blocking` may panic when a blocking client is
    /// constructed or dropped inside an async runtime. Async/MCP callers must
    /// construct and own this transport from a dedicated blocking boundary.
    pub fn new() -> Result<Self, TransportError> {
        Self::with_timeouts(CHAT_CONNECT_TIMEOUT, CHAT_REQUEST_TIMEOUT)
    }

    /// The one client construction both clients share: redirects disabled,
    /// the injected connect and total request timeouts applied. The seam the
    /// bounded-timeout proof injects short durations through; production
    /// callers reach it only through [`Self::new`] and the `CHAT_*` defaults.
    fn with_timeouts(
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self, TransportError> {
        let build = |no_proxy: bool| {
            let builder = reqwest::blocking::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(connect_timeout)
                .timeout(request_timeout);
            if no_proxy {
                builder.no_proxy()
            } else {
                builder
            }
            .build()
            .map_err(|_| TransportError::ClientBuild)
        };
        Ok(Self {
            remote_client: build(false)?,
            direct_loopback_client: build(true)?,
        })
    }
}

impl ChatTransport for ReqwestChatTransport {
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES")]
    fn post_json(
        &self,
        endpoint: &Endpoint,
        api_key: Option<&ApiKey>,
        body: &[u8],
    ) -> Result<TransportResponse, TransportError> {
        let client = if endpoint.is_literal_loopback() {
            &self.direct_loopback_client
        } else {
            &self.remote_client
        };
        let mut request = client
            .post(endpoint.as_url().clone())
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_vec());

        // The sole Authorization attachment point: immediately before send.
        if let Some(api_key) = api_key {
            if !endpoint.accepts_api_key() {
                return Err(TransportError::CredentialRequiresHttps);
            }
            let header = api_key
                .bearer_header()
                .map_err(|_| TransportError::InvalidAuthorizationHeader)?;
            request = request.header(AUTHORIZATION, header);
        }

        let mut response = request.send().map_err(|_| TransportError::RequestFailed)?;
        let status = response.status().as_u16();
        let body = if (200..300).contains(&status) {
            let mut body = Vec::with_capacity(16 * 1024);
            response
                .by_ref()
                .take((MAX_CHAT_RESPONSE_BYTES + 1) as u64)
                .read_to_end(&mut body)
                .map_err(|_| TransportError::ResponseReadFailed)?;
            if body.len() > MAX_CHAT_RESPONSE_BYTES {
                return Err(TransportError::ResponseTooLarge {
                    limit: MAX_CHAT_RESPONSE_BYTES,
                });
            }
            body
        } else {
            Vec::new()
        };
        Ok(TransportResponse::new(status, None, body))
    }
}

/// Raw response bytes remain private and are omitted from `Debug`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-NEVER-PRINTED")]
pub struct TransportResponse {
    status: u16,
    body: Vec<u8>,
}

impl TransportResponse {
    /// Build a mockable response. `request_id` is accepted only so tests can
    /// plant reflected-secret canaries; provider-controlled header bytes are
    /// intentionally discarded and never enter diagnostics.
    pub fn new(status: u16, _request_id: Option<String>, body: Vec<u8>) -> Self {
        Self { status, body }
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub(crate) fn body(&self) -> &[u8] {
        &self.body
    }
}

impl std::fmt::Debug for TransportResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransportResponse")
            .field("status", &self.status)
            .field("body_len", &self.body.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES")]
pub enum TransportError {
    #[error(
        "could not construct the blocking LLM HTTP client \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: verify the platform TLS installation)"
    )]
    ClientBuild,
    #[error(
        "the selected API key cannot be attached as an Authorization header \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: replace the token at its configured source)"
    )]
    InvalidAuthorizationHeader,
    #[error(
        "refused to attach an LLM credential to a non-HTTPS endpoint \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: configure https or remove the credential)"
    )]
    CredentialRequiresHttps,
    #[error(
        "the LLM HTTP request failed \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: verify endpoint reachability)"
    )]
    RequestFailed,
    #[error(
        "the LLM HTTP response could not be read \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: retry the provider request)"
    )]
    ResponseReadFailed,
    #[error(
        "the LLM HTTP response exceeded the {limit}-byte safety limit \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-NEVER-PRINTED; \
         fix: reduce provider output size or select a bounded text model)"
    )]
    ResponseTooLarge { limit: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    /// A loopback server that accepts the connection and then never answers
    /// must surface a bounded `RequestFailed` through the production request
    /// path. Short durations are injected at client construction, so the
    /// proof runs in well under a second, never sleeps for the production
    /// five-minute bound, and touches no process environment.
    #[test]
    fn a_stalling_loopback_server_yields_a_bounded_request_failed() {
        const REQUEST_CANARY: &str = "timeout-request-canary";
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _read = stream.read(&mut buffer).unwrap_or(0);
            // Hold the connection open without ever answering.
            let _ = stop_rx.recv();
        });

        let transport = ReqwestChatTransport::with_timeouts(
            Duration::from_secs(10),
            Duration::from_millis(750),
        )
        .unwrap();
        let endpoint =
            Endpoint::parse(&format!("http://{address}/v1/chat/completions"), false).unwrap();
        let started = Instant::now();
        let error = transport
            .post_json(&endpoint, None, REQUEST_CANARY.as_bytes())
            .unwrap_err();
        let elapsed = started.elapsed();

        assert!(matches!(error, TransportError::RequestFailed), "{error:?}");
        // The deadline fired (not an incidental instant failure) and the call
        // was bounded: near the injected 750 ms, nowhere near hanging.
        assert!(elapsed >= Duration::from_millis(700), "{elapsed:?}");
        assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
        // The sent request payload is not echoed by the failure.
        assert!(!format!("{error:?}\n{error}").contains(REQUEST_CANARY));

        drop(stop_tx);
        server.join().unwrap();
    }
}
