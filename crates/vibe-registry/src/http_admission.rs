//! Immutable HTTP-only admission for managed registry reads.
//! This constrains logical HTTP destinations, not DNS or process-wide egress.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION");

use reqwest::{
    Url,
    blocking::{Client, ClientBuilder, RequestBuilder},
    header::{AUTHORIZATION, COOKIE, LOCATION, PROXY_AUTHORIZATION, REFERER},
};
use std::collections::BTreeSet;
mod client;
mod response;
use client::HttpClient;
pub use client::{ClientBuildError, ManagedHttpClient};
pub use response::ManagedResponse;
use vibe_core::remap::endpoint::normalized_host_key;

/// An immutable set of exact, already URL-normalized HTTP host keys.
#[derive(Debug, Clone)]
pub struct HttpAdmission {
    denied: BTreeSet<String>,
}

/// A refusal contains no URL credentials, path, query or fragment.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
pub enum ContactRefusal {
    #[error(
        "controlled HTTP requires the retained managed client factory (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: build the controlled client through the operation's retained managed client factory)"
    )]
    ControlUnavailable,
    #[error(
        "controlled HTTP refuses an unsupported request body or bound-policy bypass (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: send a supported request through the bound managed client)"
    )]
    UnsupportedControlledRequest,

    #[error(
        "registry HTTP request cancelled before the next admitted network operation (violates spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW; fix: rerun the operation without requesting a stop)"
    )]
    OperationCancelled,

    #[error(
        "excluded HTTP endpoint {endpoint} (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: use an admitted HTTP endpoint or a separately supported transport capability)"
    )]
    ExcludedHost { endpoint: String },
    #[error(
        "unsupported HTTP endpoint {endpoint} (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: use an admitted HTTP endpoint or a separately supported transport capability)"
    )]
    UnsupportedScheme { endpoint: String },
    #[error(
        "invalid normalized HTTP host key (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: use an admitted HTTP endpoint or a separately supported transport capability)"
    )]
    InvalidHostKey,
    #[error(
        "native transport unavailable under HTTP-only admission (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: use an admitted HTTP endpoint or a separately supported transport capability)"
    )]
    NativeTransportUnavailable,
    #[error(
        "HTTP redirect limit exceeded (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: use an admitted HTTP endpoint or a separately supported transport capability)"
    )]
    RedirectLimit,
    #[error(
        "invalid HTTP redirect (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: use an admitted HTTP endpoint or a separately supported transport capability)"
    )]
    InvalidRedirect,
    #[error(
        "HTTPS downgrade refused (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: use an admitted HTTP endpoint or a separately supported transport capability)"
    )]
    HttpsDowngrade,
    #[error(
        "URL credentials unavailable under HTTP-only admission (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; fix: use an admitted HTTP endpoint or a separately supported transport capability)"
    )]
    UrlCredentials,
}

impl HttpAdmission {
    /// Require canonical keys: no terminal DNS root dot, URL/IP/IDNA-normalized.
    /// Report labels are never expanded into hosting families.
    pub fn new(hosts: impl IntoIterator<Item = impl Into<String>>) -> Result<Self, ContactRefusal> {
        let mut denied = BTreeSet::new();
        for host in hosts {
            let host = host.into();
            if normalized_host_key(&host).as_deref() != Some(host.as_str()) {
                return Err(ContactRefusal::InvalidHostKey);
            }
            denied.insert(host);
        }
        Ok(Self { denied })
    }

    /// Preserve refusal before URL-building APIs can turn unsupported syntax
    /// into an ordinary construction error.
    /// Check only a selected Git endpoint's host before transport/config lowering.
    /// This grants no HTTP request capability or alternate protocol execution.
    pub fn check_git_host(&self, address: &str) -> Result<(), crate::GitError> {
        let raw = address.strip_prefix("git+").unwrap_or(address);
        let parsed = if raw.contains("://") {
            raw.to_owned()
        } else {
            let authority = if let Some(end) = raw.find("]:") {
                &raw[..=end]
            } else {
                raw.split_once(':').map_or(raw, |(host, _)| host)
            };
            let host = authority
                .rsplit_once('@')
                .map_or(authority, |(_, host)| host);
            format!("ssh://{host}/")
        };
        let url = reqwest::Url::parse(&parsed).map_err(|_| crate::GitError::NativeContext {
            reason: "invalid selected Git endpoint",
        })?;
        self.admit_host(&url).map_err(Into::into)
    }
    pub fn check_address(&self, address: &str) -> Result<(), ContactRefusal> {
        self.admit_address(address)
    }

    pub(crate) fn admit_address(&self, address: &str) -> Result<(), ContactRefusal> {
        let parsed = Url::parse(address).map_err(|_| ContactRefusal::UnsupportedScheme {
            endpoint: "<invalid HTTP address>".into(),
        })?;
        self.admit(&parsed)
    }

    fn admit(&self, url: &Url) -> Result<(), ContactRefusal> {
        let host = url.host_str().and_then(normalized_host_key);
        let endpoint = format!(
            "{}://{}{}",
            url.scheme(),
            host.as_deref().unwrap_or("<opaque>"),
            url.port().map(|p| format!(":{p}")).unwrap_or_default()
        );
        if !matches!(url.scheme(), "http" | "https") || host.is_none() {
            return Err(ContactRefusal::UnsupportedScheme { endpoint });
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ContactRefusal::UrlCredentials);
        }
        self.admit_host(url)
    }

    /// Shared exact normalized-host predicate. Transport capabilities retain
    /// their own scheme, credential and execution validation before using it.
    pub(crate) fn admit_host(&self, url: &Url) -> Result<(), ContactRefusal> {
        let host = url.host_str().and_then(normalized_host_key);
        let endpoint = format!(
            "{}://{}{}",
            url.scheme(),
            host.as_deref().unwrap_or("<opaque>"),
            url.port().map(|p| format!(":{p}")).unwrap_or_default()
        );
        let host = host.ok_or_else(|| ContactRefusal::UnsupportedScheme {
            endpoint: endpoint.clone(),
        })?;
        if self.denied.contains(&host) {
            return Err(ContactRefusal::ExcludedHost { endpoint });
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
pub enum HttpGetError {
    #[error(transparent)]
    Refused(#[from] ContactRefusal),
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    #[error(transparent)]
    Io(std::io::Error),
}
impl From<ClientBuildError> for HttpGetError {
    fn from(error: ClientBuildError) -> Self {
        match error {
            ClientBuildError::Http(error) => Self::Transport(error),
            ClientBuildError::Route(error) => Self::Io(std::io::Error::other(error)),
        }
    }
}

/// Bound pools never follow a hidden redirect or inherit a proxy/retry policy;
/// the configured route is applied after this (`PROP-068#proxy`).
pub(crate) fn bound_builder(builder: ClientBuilder) -> ClientBuilder {
    builder
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .referer(false)
        .retry(reqwest::retry::never())
}

/// Successful managed GET plus facts observed across its whole redirect chain.
/// No credentials or request headers are retained in this observation.
pub(crate) struct BoundGet {
    pub(crate) response: ManagedResponse,
    /// Sticky even if a later redirect returns to the initial exact URL origin.
    /// True means subsequent requests must not reattach the initial credentials.
    pub(crate) crossed_origin: bool,
    pub(crate) redirects_followed: usize,
}

/// Execute a fully addressed GET, admitting each explicit network attempt.
pub(crate) fn get(
    client: &impl HttpClient,
    request: RequestBuilder,
    address: &str,
    admission: Option<&HttpAdmission>,
) -> Result<ManagedResponse, HttpGetError> {
    let token = crate::git_backend::transfers::current()
        .progress
        .unwrap_or_default()
        .cancellation()
        .clone();
    get_controlled(client, request, address, admission, &token)
}
fn get_controlled(
    client: &impl HttpClient,
    request: RequestBuilder,
    address: &str,
    admission: Option<&HttpAdmission>,
    token: &vibe_core::cancellation::CancellationToken,
) -> Result<ManagedResponse, HttpGetError> {
    if let Some(admission) = admission {
        return Ok(get_observed_controlled(client, request, address, admission, token)?.response);
    }
    send(client, request, token)
}
fn send(
    client: &impl HttpClient,
    request: RequestBuilder,
    token: &vibe_core::cancellation::CancellationToken,
) -> Result<ManagedResponse, HttpGetError> {
    token
        .checkpoint()
        .map_err(|_| ContactRefusal::OperationCancelled)?;
    if !token.has_control() {
        return request
            .send()
            .map(ManagedResponse::Blocking)
            .map_err(Into::into);
    }
    let asynchronous = client
        .asynchronous()
        .ok_or(ContactRefusal::ControlUnavailable)?;
    response::send(asynchronous, request.build()?, token)
}
pub(crate) fn get_observed(
    client: &impl HttpClient,
    request: RequestBuilder,
    address: &str,
    admission: &HttpAdmission,
) -> Result<BoundGet, HttpGetError> {
    let token = crate::git_backend::transfers::current()
        .progress
        .unwrap_or_default()
        .cancellation()
        .clone();
    get_observed_controlled(client, request, address, admission, &token)
}
fn get_observed_controlled(
    client: &impl HttpClient,
    request: RequestBuilder,
    address: &str,
    admission: &HttpAdmission,
    token: &vibe_core::cancellation::CancellationToken,
) -> Result<BoundGet, HttpGetError> {
    admission.admit_address(address)?;
    let request = request.build()?;
    let mut url = request.url().clone();
    let mut headers = request.headers().clone();
    headers.remove(REFERER);
    let mut crossed_origin = false;
    for followed in 0..=10 {
        token
            .checkpoint()
            .map_err(|_| ContactRefusal::OperationCancelled)?;
        admission.admit(&url)?;
        let response = send(
            client,
            client.blocking().get(url.clone()).headers(headers.clone()),
            token,
        )?;
        if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            return Ok(BoundGet {
                response,
                crossed_origin,
                redirects_followed: followed,
            });
        }
        if followed == 10 {
            return Err(ContactRefusal::RedirectLimit.into());
        }
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(ContactRefusal::InvalidRedirect)?;
        let next = response
            .url()
            .join(location)
            .map_err(|_| ContactRefusal::InvalidRedirect)?;
        crossed_origin |= redirect_headers(&url, &next, &mut headers)?;
        // Drop joins the prior response worker before another request starts.
        drop(response);
        url = next;
    }
    Err(ContactRefusal::RedirectLimit.into())
}
impl HttpGetError {
    pub fn from_io(error: std::io::Error) -> Self {
        let kind = error.kind();
        match error.into_inner() {
            Some(inner) if inner.is::<vibe_core::cancellation::Cancelled>() => {
                Self::Refused(ContactRefusal::OperationCancelled)
            }
            Some(inner) => match inner.downcast::<Self>() {
                Ok(error) => *error,
                Err(error) => Self::Io(std::io::Error::new(kind, error)),
            },
            None => Self::Io(std::io::Error::new(kind, "managed response read failed")),
        }
    }
}

fn redirect_headers(
    from: &Url,
    to: &Url,
    headers: &mut reqwest::header::HeaderMap,
) -> Result<bool, ContactRefusal> {
    if from.scheme() == "https" && to.scheme() == "http" {
        return Err(ContactRefusal::HttpsDowngrade);
    }
    let crossed_origin = from.origin() != to.origin();
    if crossed_origin {
        headers.remove(AUTHORIZATION);
        headers.remove(COOKIE);
        headers.remove(PROXY_AUTHORIZATION);
    }
    Ok(crossed_origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
    fn normalized_keys_are_exact_and_errors_reveal_only_authority() {
        for invalid in [
            "",
            "EXAMPLE.COM",
            "example.com/path",
            "example.com:443",
            "user@example.com",
            "example.com?x",
            "127.1",
        ] {
            assert!(HttpAdmission::new([invalid]).is_err(), "{invalid}");
        }
        let binding = HttpAdmission::new(["example.com"]).unwrap();
        let url = Url::parse("https://example.com/path?secret=1#secret").unwrap();
        let refusal = binding.admit(&url).unwrap_err();
        assert_eq!(
            refusal,
            ContactRefusal::ExcludedHost {
                endpoint: "https://example.com".into()
            }
        );
        assert!(!format!("{refusal:?}").contains("secret"));
        assert!(
            binding
                .admit(&Url::parse("https://other.example.com").unwrap())
                .is_ok()
        );
        assert!(matches!(
            binding.admit(&Url::parse("ftp://example.com").unwrap()),
            Err(ContactRefusal::UnsupportedScheme { .. })
        ));
    }
    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
    fn same_origin_retains_auth_cross_origin_strips_permanently_and_downgrade_refuses() {
        let origin = Url::parse("https://example.com/a").unwrap();
        let same = Url::parse("https://example.com:443/b").unwrap();
        let other = Url::parse("https://example.com:444/c").unwrap();
        let mut headers = reqwest::header::HeaderMap::new();
        for key in [AUTHORIZATION, COOKIE, PROXY_AUTHORIZATION] {
            headers.insert(key, "secret".parse().unwrap());
        }
        redirect_headers(&origin, &same, &mut headers).unwrap();
        assert_eq!(headers.len(), 3);
        redirect_headers(&same, &other, &mut headers).unwrap();
        assert!(headers.is_empty());
        redirect_headers(&other, &origin, &mut headers).unwrap();
        assert!(headers.is_empty());
        assert_eq!(
            redirect_headers(
                &origin,
                &Url::parse("http://example.com").unwrap(),
                &mut headers
            ),
            Err(ContactRefusal::HttpsDowngrade)
        );
    }
    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
    fn dns_root_dot_cannot_bypass_denied_host_key() {
        let admission = HttpAdmission::new(["example.invalid"]).unwrap();
        assert!(matches!(
            admission
                .admit(&Url::parse("https://example.invalid./private?secret=fixture").unwrap()),
            Err(ContactRefusal::ExcludedHost { .. })
        ));
    }
    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
    fn bound_get_denies_dotted_authority_before_connect_and_preserves_lookalikes() {
        let admission = std::sync::Arc::new(HttpAdmission::new(["example.invalid"]).unwrap());
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let socket = listener.local_addr().unwrap();
        // Both spellings resolve locally even if admission regresses; no DNS.
        let client = bound_builder(Client::builder())
            .resolve("example.invalid", socket)
            .resolve("example.invalid.", socket)
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap();
        let address = format!("http://EXAMPLE.invalid.:{}/", socket.port());
        let result = get(&client, client.get(&address), &address, Some(&admission));
        assert!(matches!(
            result,
            Err(HttpGetError::Refused(ContactRefusal::ExcludedHost { .. }))
        ));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        for key in [
            "example.invalid.",
            "example.invalid..",
            ".example.invalid",
            "example..invalid",
        ] {
            assert!(HttpAdmission::new([key]).is_err());
        }
        for host in ["example.invalid.attacker.invalid", "evilexample.invalid"] {
            assert!(
                admission
                    .admit(&Url::parse(&format!("https://{host}/")).unwrap())
                    .is_ok()
            );
        }
        assert!(matches!(
            admission.admit(&Url::parse("https://example.invalid../").unwrap()),
            Err(ContactRefusal::UnsupportedScheme { .. })
        ));
    }
    #[test]
    fn equivalent_dns_keys_do_not_relax_credential_origin_comparison() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(AUTHORIZATION, "fixture".parse().unwrap());
        redirect_headers(
            &Url::parse("https://example.invalid/").unwrap(),
            &Url::parse("https://example.invalid./").unwrap(),
            &mut headers,
        )
        .unwrap();
        assert!(headers.is_empty());
    }
}

#[cfg(test)]
#[path = "http_admission/cancellation_tests.rs"]
mod cancellation_tests;
