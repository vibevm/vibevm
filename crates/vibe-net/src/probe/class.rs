//! The failure classes of the probe: code, exit status, label and fix.

/// What kind of failure stopped a request — each with a stable code, a
/// distinct `vibe dl` exit status and the fix an operator can act on.
///
/// ```
/// use vibe_net::probe::FailureClass;
/// assert_eq!(FailureClass::ProxyAuth.code(), "dl_proxy_auth");
/// assert_eq!(FailureClass::ProxyAuth.exit_code(), 9);
/// assert!(FailureClass::UntrustedCertificate.fix().contains("extra_ca"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// Source policy refused the host (exit 4).
    Policy,
    /// The vibe-managed px is missing or did not start (exit 5).
    Px,
    /// The name did not resolve (exit 6).
    Dns,
    /// The connection timed out — typically a direct route where only a
    /// proxy may leave the network (exit 7).
    ConnectTimeout,
    /// The connection was refused or reset (exit 8).
    ConnectFailed,
    /// The proxy answered 407 (exit 9).
    ProxyAuth,
    /// The proxy refused the tunnel with another status (exit 10).
    ConnectRejected,
    /// The server's chain does not end at a trusted root (exit 11).
    UntrustedCertificate,
    /// The certificate is not valid for the requested name (exit 12).
    NameMismatch,
    /// Any other TLS failure (exit 13).
    Tls,
    /// The server answered 4xx (exit 14).
    HttpClient,
    /// The server answered 5xx (exit 15).
    HttpServer,
    /// A redirect vibe will not follow: too many, a downgrade, no
    /// `Location` (exit 20).
    Redirect,
    /// The server did not speak HTTP or the read failed (exit 1).
    Protocol,
}

impl FailureClass {
    pub fn code(self) -> &'static str {
        match self {
            Self::Policy => "dl_source_policy",
            Self::Px => "dl_px",
            Self::Dns => "dl_dns",
            Self::ConnectTimeout => "dl_connect_timeout",
            Self::ConnectFailed => "dl_connect_failed",
            Self::ProxyAuth => "dl_proxy_auth",
            Self::ConnectRejected => "dl_connect_rejected",
            Self::UntrustedCertificate => "dl_untrusted_certificate",
            Self::NameMismatch => "dl_certificate_name",
            Self::Tls => "dl_tls",
            Self::HttpClient => "dl_http_client_error",
            Self::HttpServer => "dl_http_server_error",
            Self::Redirect => "dl_redirect",
            Self::Protocol => "dl_failed",
        }
    }

    /// What the failure is, in words, for a row of the phase list.
    pub fn label(self) -> &'static str {
        match self {
            Self::Policy => "refused by source policy",
            Self::Px => "px unavailable",
            Self::Dns => "the name does not resolve",
            Self::ConnectTimeout => "timed out",
            Self::ConnectFailed => "connection refused",
            Self::ProxyAuth => "the proxy wants authentication",
            Self::ConnectRejected => "the proxy refused the tunnel",
            Self::UntrustedCertificate => "untrusted certificate",
            Self::NameMismatch => "certificate for another name",
            Self::Tls => "TLS failed",
            Self::HttpClient => "client error",
            Self::HttpServer => "server error",
            Self::Redirect => "redirect refused",
            Self::Protocol => "not an HTTP answer",
        }
    }

    pub fn exit_code(self) -> u8 {
        match self {
            Self::Policy => 4,
            Self::Px => 5,
            Self::Dns => 6,
            Self::ConnectTimeout => 7,
            Self::ConnectFailed => 8,
            Self::ProxyAuth => 9,
            Self::ConnectRejected => 10,
            Self::UntrustedCertificate => 11,
            Self::NameMismatch => 12,
            Self::Tls => 13,
            Self::HttpClient => 14,
            Self::HttpServer => 15,
            Self::Redirect => 20,
            Self::Protocol => 1,
        }
    }

    pub fn fix(self) -> &'static str {
        match self {
            Self::Policy => {
                "the source policy excludes this host: use an admitted mirror, or change `[source_policy] exclude_hosting`"
            }
            Self::Px => {
                "run `vibe self doctor` (it checks px, runs `px --test` and names the cause); to stop using px, set `[proxy] mode = \"none\"` or export `VIBEVM_PROXY_MODE=none`"
            }
            Self::Dns => {
                "check the host name; on a network where only the proxy resolves outside names, configure `[proxy]`"
            }
            Self::ConnectTimeout => {
                "the direct route is blocked here: configure `[proxy]` (`mode = \"px\"` with your PAC URL, or `https = \"http://host:port\"`)"
            }
            Self::ConnectFailed => {
                "check that the server (or the proxy) runs and listens on that port"
            }
            Self::ProxyAuth => {
                "the proxy wants credentials: set `[proxy] mode = \"px\"` with `pac = \"<your PAC URL>\"` (or `upstream`) so px signs in with your single sign-on"
            }
            Self::ConnectRejected => {
                "the proxy refuses this destination: ask for it to be allowed, or add the host to `[proxy] no_proxy` if it is internal"
            }
            // The operating system's own verifier already read every
            // store this machine has, so the fix is never "install it
            // again": it is to export the issuer and name it, and the
            // text says how on this operating system (PROP-068 `#proxy`).
            Self::UntrustedCertificate => crate::TrustSet::untrusted_help(),
            Self::NameMismatch => {
                "the certificate is for another name: use the host name the certificate names, or ask the server's owner to fix it"
            }
            Self::Tls => {
                "check the server's TLS configuration and the system clock; a connection closed during the handshake usually means a filter on this network refuses the host: configure `[proxy]`, or take the file from an artifact drop"
            }
            Self::HttpClient => "check the URL and whether it needs a token",
            Self::HttpServer => "the server failed; retry later or ask its owner",
            Self::Redirect => "download the final URL directly",
            Self::Protocol => "check that the URL names an HTTP(S) server",
        }
    }
}
