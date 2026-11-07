//! Pure observations of source spellings, never transport admission or routing.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#HOSTING-OBSERVATION");

use url::{Host, Url};

/// Transport syntax only; Git rewrites and SSH configuration can change routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#HOSTING-OBSERVATION")]
pub enum TransportSpelling {
    Http,
    Https,
    Git,
    Ssh,
    Scp,
}

/// A normalized spelled authority, containing no userinfo, path, query or fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#HOSTING-OBSERVATION")]
pub struct NetworkAuthority {
    pub host: String,
    /// URL-normalized port: explicit default HTTP(S) ports are omitted.
    pub port: Option<u16>,
    pub transport: TransportSpelling,
    pub hosting: Option<&'static str>,
}

/// Classification is not a claim of contact, safety, authorization or reachability.
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#HOSTING-OBSERVATION")]
pub enum EndpointObservation {
    Local,
    /// UNC or a file URL with non-local authority; never called contact-free.
    NonLocalFile,
    /// Unsupported syntax, malformed remote or an unresolved SSH alias.
    Opaque,
    Network(NetworkAuthority),
}

/// Observe supported URLs and conservative SCP spellings without consulting state.
/// One `git+` wrapper is supported. SSH aliases stay opaque; custom fully
/// qualified hosts are observed but unlabeled. No DNS/configuration is consulted.
///
/// ```
/// use vibe_core::remap::endpoint::{observe_endpoint, EndpointObservation};
/// let EndpointObservation::Network(host) = observe_endpoint("git@github.com:org/repo.git") else { panic!() };
/// assert_eq!(host.hosting, Some("github"));
/// assert_eq!(observe_endpoint("git@my-alias:repo"), EndpointObservation::Opaque);
/// ```
pub fn observe_endpoint(raw: &str) -> EndpointObservation {
    let raw = raw.strip_prefix("git+").unwrap_or(raw);
    if raw.is_empty() || raw.chars().any(char::is_control) {
        return EndpointObservation::Opaque;
    }
    if raw.starts_with("\\\\") || raw.starts_with("//") {
        return EndpointObservation::NonLocalFile;
    }
    if raw.as_bytes().get(1) == Some(&b':') && raw.as_bytes()[0].is_ascii_alphabetic() {
        return EndpointObservation::Local;
    }
    if raw.starts_with('/')
        || raw.starts_with("./")
        || raw.starts_with("../")
        || raw.starts_with("~/")
    {
        return EndpointObservation::Local;
    }
    if raw.contains("://") || raw.starts_with("file:") {
        return observe_url(raw);
    }
    if raw.contains(':') {
        return observe_scp(raw);
    }
    EndpointObservation::Local
}

fn observe_url(raw: &str) -> EndpointObservation {
    let Ok(url) = Url::parse(raw) else {
        return EndpointObservation::Opaque;
    };
    let transport = match url.scheme() {
        "file" => {
            return if url.host_str().is_none_or(|host| host == "localhost") {
                EndpointObservation::Local
            } else {
                EndpointObservation::NonLocalFile
            };
        }
        "http" => TransportSpelling::Http,
        "https" => TransportSpelling::Https,
        "git" => TransportSpelling::Git,
        "ssh" => TransportSpelling::Ssh,
        _ => return EndpointObservation::Opaque,
    };
    let Some(host) = url.host_str() else {
        return EndpointObservation::Opaque;
    };
    authority(host, url.port(), transport)
}

fn observe_scp(raw: &str) -> EndpointObservation {
    // A remote helper (`helper::...`) is not SCP; neither is a local path
    // with a colon after a slash. Bracketed IPv6 has its own delimiter.
    let remote = raw.rsplit_once('@').map_or(raw, |(user, remote)| {
        if user.is_empty() || user.contains(['/', '\\', ':', '@']) {
            ""
        } else {
            remote
        }
    });
    let split = if remote.starts_with('[') {
        remote.find("]:").map(|i| (&remote[..=i], &remote[i + 2..]))
    } else {
        remote.split_once(':')
    };
    let Some((host, path)) = split else {
        return EndpointObservation::Opaque;
    };
    if path.is_empty()
        || path.starts_with(':')
        || host.contains(['/', '\\', '@', '?', '#'])
        || host.chars().any(char::is_whitespace)
    {
        return EndpointObservation::Opaque;
    }
    authority(host, None, TransportSpelling::Scp)
}

fn authority(host: &str, port: Option<u16>, transport: TransportSpelling) -> EndpointObservation {
    let Ok(host) = Host::parse(host) else {
        return EndpointObservation::Opaque;
    };
    if matches!(transport, TransportSpelling::Ssh | TransportSpelling::Scp)
        && matches!(&host, Host::Domain(domain) if !domain.contains('.'))
    {
        return EndpointObservation::Opaque;
    }
    let host = host.to_string();
    let hosting = match host.as_str() {
        "github.com" => Some("github"),
        "gitverse.ru" => Some("gitverse"),
        _ => None,
    };
    EndpointObservation::Network(NetworkAuthority {
        host,
        port,
        transport,
        hosting,
    })
}

#[cfg(test)]
#[path = "endpoint_tests.rs"]
mod tests;
