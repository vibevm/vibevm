//! Step-by-step diagnosis of the network route (`vibe self doctor`):
//! px present and started → `px --test` → `CONNECT` through the proxy →
//! TLS with the trust set. Each failed step names its cause and the fix.

use crate::{NetworkSession, http};
use base64::Engine;
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};

/// The outcome of one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Fail,
    /// Not applicable to this route (direct, bypassed, not px).
    Skip,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Fail => "fail",
            Status::Skip => "skip",
        }
    }
}

/// One diagnosed step.
///
/// ```
/// use vibe_net::doctor::{Check, Status};
/// let check = Check::skip("px", "the route does not use px");
/// assert_eq!(check.status, Status::Skip);
/// assert!(check.fix.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub step: &'static str,
    pub status: Status,
    pub detail: String,
    pub fix: Option<String>,
}

impl Check {
    pub fn ok(step: &'static str, detail: impl Into<String>) -> Self {
        Self {
            step,
            status: Status::Ok,
            detail: detail.into(),
            fix: None,
        }
    }
    pub fn skip(step: &'static str, detail: impl Into<String>) -> Self {
        Self {
            step,
            status: Status::Skip,
            detail: detail.into(),
            fix: None,
        }
    }
    pub fn fail(step: &'static str, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            step,
            status: Status::Fail,
            detail: detail.into(),
            fix: Some(fix.into()),
        }
    }
}

const FIX_407: &str = "the proxy wants credentials: the request is not going through px / SSO — \
     set `[proxy] mode = \"px\"` with `pac = \"<your PAC URL>\"` (or `upstream`), or point \
     `https` at a running px";
const FIX_CA: &str = "the certificate is not trusted (`unable to get local issuer`): install the \
     corporate CA in the system store, or set `[proxy] ca_file` to its PEM file";
const FIX_TIMEOUT: &str = "timed out: the direct route is blocked here — configure `[proxy]` \
     (`mode = \"px\"` with your PAC URL, or `https = \"http://host:port\"`)";
/// A Windows schannel handshake that could not complete the revocation
/// lookup: the inspecting CA's CRL / OCSP endpoints are outside the
/// network it inspects, so the check never finishes.
const FIX_REVOCATION: &str = "the certificate's revocation state could not be checked (the inspecting CA's CRL / OCSP \
     endpoint is unreachable from here): set `[proxy] revocation = \"best-effort\"` (or \
     `\"off\"`), or export `VIBEVM_PROXY_REVOCATION=best-effort`";

/// Diagnose the route to `target` (an `https://` URL) in order. Stops after
/// the first failed step: what follows cannot be judged without it.
pub fn diagnose(session: &Arc<NetworkSession>, target: &url::Url, timeout: Duration) -> Vec<Check> {
    let mut checks = Vec::new();
    let route = session.route();
    match session.extra_roots() {
        Ok(extra) => {
            let (native, errors) = crate::trust::native_root_count();
            let mut detail = format!("{native} operating-system roots");
            if errors > 0 {
                detail.push_str(&format!(" ({errors} unreadable entries ignored)"));
            }
            if let Some(path) = route.ca_file() {
                detail.push_str(&format!(" + {} from `{}`", extra.len(), path.display()));
            }
            checks.push(Check::ok("trust", detail));
        }
        Err(error) => {
            checks.push(Check::fail(
                "trust",
                error.to_string(),
                "fix `[proxy] ca_file`",
            ));
            return checks;
        }
    }
    if let Some(check) = pac_check(session, timeout) {
        // A PAC that cannot be read is reported, and px still tried: px
        // falls back to its own handling, and the remaining steps show it.
        checks.push(check);
    }
    if let Some(settings) = route.px() {
        match crate::px::locate(settings) {
            Err(error) => {
                checks.push(Check::fail(
                    "px",
                    error.to_string(),
                    "install px or set `[proxy] px_path`",
                ));
                return checks;
            }
            Ok(program) => match session.px_address() {
                Ok(address) => checks.push(Check::ok(
                    "px",
                    format!("`{}` started on {address}", program.display()),
                )),
                Err(error) => {
                    checks.push(Check::fail("px", error.to_string(), "see the cause above"));
                    return checks;
                }
            },
        }
        let scratch = tempfile::tempdir();
        let tested = match &scratch {
            Ok(dir) => crate::px::locate(settings)
                .map_err(|e| e.to_string())
                .and_then(|program| {
                    crate::px::self_test(
                        &program,
                        settings,
                        route.no_proxy(),
                        target.as_str(),
                        Duration::from_secs(90),
                        dir.path(),
                    )
                }),
            Err(error) => Err(error.to_string()),
        };
        match tested {
            Ok(()) => checks.push(Check::ok("px --test", format!("px reached {target}"))),
            Err(reason) => {
                checks.push(Check::fail(
                    "px --test",
                    format!("px could not reach {target}: {reason}"),
                    "single sign-on or the PAC route failed: check that you are logged on to the \
                     domain, that the PAC URL answers, and that `px_args` names the right `--auth`",
                ));
                return checks;
            }
        }
    } else {
        checks.push(Check::skip("px", "the route does not use px"));
    }
    match session.proxy_for(target) {
        Ok(Some(proxy)) => match connect(&proxy, target, timeout) {
            Ok(()) => checks.push(Check::ok(
                "connect",
                format!(
                    "CONNECT through {} answered 200",
                    vibe_core::net::printable(&proxy)
                ),
            )),
            Err(Connect::Status(407)) => {
                checks.push(Check::fail("connect", "the proxy answered 407", FIX_407));
                return checks;
            }
            Err(Connect::Status(code)) => {
                checks.push(Check::fail(
                    "connect",
                    format!("the proxy answered {code} to CONNECT"),
                    "the proxy refuses this destination: ask for it to be allowed, or bypass it with `no_proxy`",
                ));
                return checks;
            }
            Err(Connect::Io(reason)) => {
                checks.push(Check::fail(
                    "connect",
                    format!(
                        "cannot reach the proxy {}: {reason}",
                        vibe_core::net::printable(&proxy)
                    ),
                    "check the proxy address and that it runs",
                ));
                return checks;
            }
        },
        Ok(None) => checks.push(Check::skip(
            "connect",
            "direct connection (no proxy for this host)",
        )),
        Err(error) => {
            checks.push(Check::fail(
                "connect",
                error.to_string(),
                "see the cause above",
            ));
            return checks;
        }
    }
    checks.push(tls(session, target, timeout));
    checks
}

enum Connect {
    Status(u16),
    Io(String),
}

fn connect(proxy: &url::Url, target: &url::Url, timeout: Duration) -> Result<(), Connect> {
    let io = |error: std::io::Error| Connect::Io(error.to_string());
    let host = proxy
        .host_str()
        .ok_or(Connect::Io("no proxy host".into()))?;
    let port = proxy.port_or_known_default().unwrap_or(3128);
    let address = (host, port)
        .to_socket_addrs()
        .map_err(io)?
        .next()
        .ok_or(Connect::Io("the proxy name does not resolve".into()))?;
    let mut stream = TcpStream::connect_timeout(&address, timeout).map_err(io)?;
    stream.set_read_timeout(Some(timeout)).map_err(io)?;
    let authority = format!(
        "{}:{}",
        target.host_str().unwrap_or_default(),
        target.port_or_known_default().unwrap_or(443)
    );
    let mut request = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n");
    if !proxy.username().is_empty() {
        let user = percent_decode(proxy.username());
        let password = percent_decode(proxy.password().unwrap_or_default());
        let token = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
        request.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).map_err(io)?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line).map_err(io)?;
    let code = line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| Connect::Io("the proxy did not answer HTTP".into()))?;
    if code == 200 {
        Ok(())
    } else {
        Err(Connect::Status(code))
    }
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = raw.get(i + 1..i + 3)
            && let Ok(value) = u8::from_str_radix(hex, 16)
        {
            out.push(value);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn tls(session: &Arc<NetworkSession>, target: &url::Url, timeout: Duration) -> Check {
    let client = http::route_in(
        session,
        reqwest::blocking::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none()),
        http::Ambient::Inherit,
    )
    .map_err(|error| error.to_string())
    .and_then(|builder| builder.build().map_err(|error| error.to_string()));
    let client = match client {
        Ok(client) => client,
        Err(reason) => return Check::fail("tls", reason, "fix the cause above"),
    };
    match client.head(target.clone()).send() {
        Ok(response) if response.status().as_u16() == 407 => {
            Check::fail("tls", "the proxy answered 407", FIX_407)
        }
        Ok(response) => Check::ok(
            "tls",
            format!("{target} answered {} over verified TLS", response.status()),
        ),
        Err(error) => classify(&error),
    }
}

/// Name the cause of a failed request the way an operator can act on it.
///
/// ```
/// let text = "error sending request: invalid peer certificate: UnknownIssuer";
/// assert_eq!(vibe_net::doctor::cause_of(text, false).0, "untrusted-ca");
/// assert_eq!(vibe_net::doctor::cause_of("operation timed out", true).0, "timeout");
/// // A Windows schannel revocation failure names the setting that fixes it.
/// let schannel = "schannel: next InitializeSecurityContext failed: \
///     CRYPT_E_NO_REVOCATION_CHECK (0x80092012) - The revocation function was unable to \
///     check revocation for the certificate.";
/// let (cause, fix) = vibe_net::doctor::cause_of(schannel, false);
/// assert_eq!(cause, "revocation");
/// assert!(fix.contains("`[proxy] revocation = \"best-effort\"`"), "{fix}");
/// assert_eq!(vibe_net::doctor::cause_of("CRYPT_E_REVOCATION_OFFLINE (0x80092013)", false).0, "revocation");
/// ```
pub fn cause_of(chain: &str, timed_out: bool) -> (&'static str, &'static str) {
    let lower = chain.to_ascii_lowercase();
    // Judged before the trust causes: schannel reports a revocation
    // failure as a certificate problem, and the fix is a different key.
    if REVOCATION_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return ("revocation", FIX_REVOCATION);
    }
    if lower.contains("unknownissuer")
        || lower.contains("unknown issuer")
        || lower.contains("local issuer")
        || lower.contains("invalid peer certificate")
    {
        ("untrusted-ca", FIX_CA)
    } else if lower.contains("407") {
        ("proxy-auth", FIX_407)
    } else if timed_out || lower.contains("timed out") {
        ("timeout", FIX_TIMEOUT)
    } else {
        (
            "other",
            "check the address, then the proxy and trust settings",
        )
    }
}

/// What a schannel (or libcurl) handshake says when the revocation
/// lookup could not be completed, lowercase. `CRYPT_E_NO_REVOCATION_CHECK`
/// is `0x80092012` and `CRYPT_E_REVOCATION_OFFLINE` is `0x80092013`; git
/// and curl print the name, the code, or both.
pub const REVOCATION_MARKERS: &[&str] = &[
    "crypt_e_no_revocation_check",
    "crypt_e_revocation_offline",
    "0x80092012",
    "0x80092013",
    "revocation function was unable",
    "revocation server was offline",
    "ssl-no-revoke",
];

fn classify(error: &reqwest::Error) -> Check {
    let mut chain = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(next) = source {
        chain.push_str(": ");
        chain.push_str(&next.to_string());
        source = next.source();
    }
    let (_, fix) = cause_of(&chain, error.is_timeout());
    Check::fail("tls", chain, fix)
}

/// Whether the PAC file the route names can be fetched (directly — a PAC
/// URL must be reachable without the proxy it configures) or read, and
/// looks like one. `None` when the route has no PAC.
pub fn pac_check(session: &Arc<NetworkSession>, timeout: Duration) -> Option<Check> {
    use vibe_core::user_config::PacSource;
    let settings = session.route().px()?;
    let origin = settings.pac_origin.label(vibe_core::net::PAC_ENV);
    // `system` is resolved by vibe, here and again when px starts, so the
    // check reports the URL px will be given and where it came from —
    // never "px reads the settings itself" (`PROP-068#proxy-px`).
    let resolved = match settings.pac.as_ref()? {
        PacSource::System => match crate::system_proxy::resolve() {
            Ok(resolved) => Some(resolved),
            Err(error) => {
                return Some(Check::fail(
                    "pac",
                    format!("{error} · pac from {origin}"),
                    "set `[proxy] pac` to your PAC URL or an absolute PAC file path, or                      `[proxy] upstream` to the proxy's `host:port`",
                ));
            }
        },
        _ => None,
    };
    let system = resolved.as_ref().map(|resolved| resolved.describe());
    let source = match &resolved {
        // A PAC URL the Windows settings named is fetched and read like
        // any other, under the phrase that says where it came from.
        Some(resolved) => match &resolved.pac_url {
            Some(url) => PacSource::Url(url.clone()),
            // The settings name a fixed proxy instead: there is no PAC to
            // read, and px is given `--proxy`.
            None => {
                return Some(Check::skip(
                    "pac",
                    format!("{} · pac from {origin}", system.unwrap_or_default()),
                ));
            }
        },
        None => settings.pac.clone()?,
    };
    let body = match &source {
        // Resolved above; a `system` that reaches here is impossible.
        PacSource::System => return None,
        PacSource::File(location) => {
            let path = PacSource::File(location.clone())
                .px_location()
                .unwrap_or_else(|| location.clone());
            match std::fs::read(&path) {
                Ok(bytes) => Ok((format!("file `{path}`"), bytes)),
                Err(error) => Err((
                    format!("file `{path}`"),
                    error.to_string(),
                    "check the path in `[proxy] pac`",
                )),
            }
        }
        PacSource::Url(location) => {
            let shown = vibe_core::progress::transfer::safe_url(location);
            let fetched = session
                .extra_roots()
                .map_err(|error| error.to_string())
                .and_then(|roots| {
                    let mut builder = reqwest::blocking::Client::builder()
                        .no_proxy()
                        .timeout(timeout);
                    for root in roots {
                        builder = builder.add_root_certificate(root);
                    }
                    builder.build().map_err(|error| error.to_string())
                })
                .and_then(|client| {
                    client
                        .get(location)
                        .send()
                        .map_err(|error| error.to_string())
                })
                .and_then(|response| {
                    let status = response.status();
                    if !status.is_success() {
                        return Err(format!("answered {status}"));
                    }
                    response
                        .bytes()
                        .map(|b| b.to_vec())
                        .map_err(|error| error.to_string())
                });
            match fetched {
                Ok(bytes) => Ok((format!("URL {shown}"), bytes)),
                Err(reason) => Err((
                    format!("URL {shown}"),
                    reason,
                    "a PAC URL must be reachable without the proxy it configures: check the address, \
                     or copy the PAC file locally and set `pac` to its absolute path",
                )),
            }
        }
    };
    // What the row calls the PAC: the resolution phrase when `system`
    // answered, else the location as configured.
    let name = |what: String| match &system {
        Some(resolved) => resolved.clone(),
        None => what,
    };
    Some(match body {
        Ok((what, bytes)) if String::from_utf8_lossy(&bytes).contains("FindProxyForURL") => {
            Check::ok(
                "pac",
                format!(
                    "{} read, {} bytes · pac from {origin}",
                    name(what),
                    bytes.len()
                ),
            )
        }
        Ok((what, _)) => Check::fail(
            "pac",
            format!(
                "{} has no `FindProxyForURL` · pac from {origin}",
                name(what)
            ),
            "point `[proxy] pac` at the PAC file itself",
        ),
        Err((what, reason, fix)) => Check::fail(
            "pac",
            format!(
                "{} could not be read: {reason} · pac from {origin}",
                name(what)
            ),
            fix,
        ),
    })
}
