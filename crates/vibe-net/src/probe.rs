//! The diagnostic probe of `vibe dl` (`PROP-068#dl`): one request walked
//! phase by phase over the configured route — the route decision, DNS (on
//! a direct route), the TCP connection, the proxy's `CONNECT` answer, the
//! TLS handshake (version, cipher, the server's certificate chain and the
//! trust source that accepted it), the response head and every redirect.
//!
//! The probe verifies the server with the very trust set every vibe client
//! uses — the operating system's own verifier plus `[tls] extra_ca` — and
//! a server that fails verification never receives a request. It sends no credential of any kind. It only describes: the
//! bytes themselves are fetched by the controlled transport afterwards.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#dl");

use crate::NetworkSession;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod class;
mod der;
mod hop;
mod tls;
pub use crate::TrustSource;
pub use class::FailureClass;
pub use der::summarize;
use hop::one_hop;

/// One failure: its class and what was observed (never a credential).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub class: FailureClass,
    pub detail: String,
}

/// A certificate as an operator reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertSummary {
    pub subject: String,
    pub issuer: String,
}

/// The negotiated TLS session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsInfo {
    pub version: String,
    pub cipher: String,
    /// Leaf first, as the server sent it.
    pub chain: Vec<CertSummary>,
    pub trusted_by: Option<TrustSource>,
}

/// Milliseconds spent per phase of one hop (absent: the phase did not run).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Timings {
    pub dns_ms: Option<u64>,
    pub connect_ms: Option<u64>,
    pub proxy_ms: Option<u64>,
    pub tls_ms: Option<u64>,
    pub request_ms: Option<u64>,
}

/// One request of the redirect chain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hop {
    /// Without credentials, query or fragment.
    pub url: String,
    /// The printable proxy (`scheme://host:port`, or `px` with its port),
    /// absent for a direct connection.
    pub via: Option<String>,
    /// Whether `[proxy] no_proxy` (or loopback) sent this host direct
    /// while a proxy is configured.
    pub bypassed: bool,
    /// Resolved addresses (direct route only: through a proxy the proxy
    /// resolves).
    pub addresses: Vec<String>,
    /// The proxy's answer to `CONNECT`, e.g. `HTTP/1.1 200 Connection established`.
    pub connect_status: Option<String>,
    pub tls: Option<TlsInfo>,
    pub status_line: Option<String>,
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub timings: Timings,
}

/// The whole probe.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Probe {
    /// `direct`, `explicit` or `px`.
    pub mode: &'static str,
    pub hops: Vec<Hop>,
    pub failure: Option<Failure>,
}

impl Probe {
    /// The last hop's status, when the chain ended in an answer.
    pub fn final_status(&self) -> Option<u16> {
        self.hops.last().and_then(|hop| hop.status)
    }
}

/// How the probe runs.
#[derive(Debug, Clone)]
pub struct ProbeOptions {
    pub timeout: Duration,
    pub max_redirects: usize,
    pub user_agent: String,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_redirects: 10,
            user_agent: format!("vibevm/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

fn ms(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn fail(class: FailureClass, detail: impl Into<String>) -> Failure {
    Failure {
        class,
        detail: detail.into(),
    }
}

/// `scheme://host[:port]/path` — what a hop is displayed as.
pub fn display_url(url: &url::Url) -> String {
    let mut shown = url.clone();
    let _ = shown.set_username("");
    let _ = shown.set_password(None);
    shown.set_query(None);
    shown.set_fragment(None);
    shown.to_string()
}

/// Walk `url` over `session`'s route. `admit` is the operation's source
/// policy, asked before every hop; its `Err` is the refusal text.
pub fn probe(
    session: &Arc<NetworkSession>,
    url: &url::Url,
    options: &ProbeOptions,
    admit: &dyn Fn(&url::Url) -> Result<(), String>,
) -> Probe {
    let mut report = Probe {
        mode: session.route().mode_name(),
        ..Probe::default()
    };
    let mut current = url.clone();
    for followed in 0..=options.max_redirects {
        if let Err(reason) = admit(&current) {
            report.failure = Some(fail(FailureClass::Policy, reason));
            return report;
        }
        let mut hop = Hop {
            url: display_url(&current),
            ..Hop::default()
        };
        let outcome = one_hop(session, &current, options, &mut hop);
        report.hops.push(hop);
        if let Err(failure) = outcome {
            report.failure = Some(failure);
            return report;
        }
        let Some(hop) = report.hops.last() else {
            return report;
        };
        let status = hop.status.unwrap_or_default();
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            if followed == options.max_redirects {
                report.failure = Some(fail(
                    FailureClass::Redirect,
                    format!("more than {} redirects", options.max_redirects),
                ));
                return report;
            }
            let location = hop
                .headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("location"))
                .map(|(_, value)| value.clone());
            let Some(next) = location.and_then(|location| current.join(&location).ok()) else {
                report.failure = Some(fail(
                    FailureClass::Redirect,
                    format!("{status} without a usable Location"),
                ));
                return report;
            };
            if current.scheme() == "https" && next.scheme() != "https" {
                report.failure = Some(fail(
                    FailureClass::Redirect,
                    format!("redirect from HTTPS down to {}", display_url(&next)),
                ));
                return report;
            }
            current = next;
            continue;
        }
        if (400..500).contains(&status) {
            report.failure = Some(fail(
                FailureClass::HttpClient,
                hop.status_line.clone().unwrap_or_default(),
            ));
        } else if status >= 500 {
            report.failure = Some(fail(
                FailureClass::HttpServer,
                hop.status_line.clone().unwrap_or_default(),
            ));
        }
        return report;
    }
    report
}

/// The px steps of a proxy test (mode = `px`): located, started, and
/// `px --test=<target>` with the same route settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PxCheck {
    /// The executable vibe runs, or why none was found.
    pub program: Result<std::path::PathBuf, String>,
    /// The loopback address px listens on, or why it did not start.
    pub started: Option<Result<SocketAddr, String>>,
    /// `px --test` passed, or what it reported.
    pub self_test: Option<Result<(), String>>,
}

/// Run the px steps when the route uses px; `None` otherwise. px is
/// started through the session, so the operation reaps it.
pub fn px_checks(
    session: &NetworkSession,
    target: &url::Url,
    timeout: Duration,
) -> Option<PxCheck> {
    let settings = session.route().px()?;
    let program = crate::px::locate(settings).map_err(|error| error.to_string());
    let Ok(path) = &program else {
        return Some(PxCheck {
            program,
            started: None,
            self_test: None,
        });
    };
    let started = Some(session.px_address().map_err(|error| error.to_string()));
    let self_test = Some(match tempfile::tempdir() {
        Ok(dir) => crate::px::self_test(
            path,
            settings,
            session.route().no_proxy(),
            target.as_str(),
            timeout,
            dir.path(),
        ),
        Err(error) => Err(error.to_string()),
    });
    Some(PxCheck {
        program: program.clone(),
        started,
        self_test,
    })
}
