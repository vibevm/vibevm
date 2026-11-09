//! Retained HTTP policy built once for blocking and cancellable execution.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use super::*;
use std::{sync::Arc, time::Duration};

/// One captured policy, two execution modes. No-control calls retain the blocking pool.
/// ```no_run
/// use std::{sync::Arc, time::Duration};
/// use vibe_registry::http_admission::{HttpAdmission, ManagedHttpClient};
/// let client = ManagedHttpClient::bound(Duration::from_secs(30), Arc::new(HttpAdmission::new([] as [&str; 0])?))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct ManagedHttpClient {
    blocking: Client,
    asynchronous: reqwest::Client,
    binding: Option<Arc<HttpAdmission>>,
}
impl ManagedHttpClient {
    /// Capture the existing unbound HTTP policy, for a separately authorized domain owner.
    pub fn legacy(timeout: Duration) -> Result<Self, ClientBuildError> {
        Self::captured(timeout, None, reqwest::header::HeaderMap::new(), None, None)
    }
    /// A legacy domain request cannot bypass a source client's immutable binding.
    pub fn execute_legacy(
        &self,
        method: reqwest::Method,
        address: &str,
        headers: reqwest::header::HeaderMap,
        body: Option<Vec<u8>>,
        token: &vibe_core::cancellation::CancellationToken,
    ) -> Result<ManagedResponse, HttpGetError> {
        if self.binding.is_some() {
            return Err(ContactRefusal::UnsupportedControlledRequest.into());
        }
        let mut request = self.blocking.request(method, address).headers(headers);
        if let Some(body) = body {
            request = request.body(body);
        }
        send(self, request, token)
    }

    /// Anonymous bound payload reads disable implicit redirects, proxies and retries.
    pub fn bound(timeout: Duration, binding: Arc<HttpAdmission>) -> Result<Self, ClientBuildError> {
        Self::captured(
            timeout,
            None,
            reqwest::header::HeaderMap::new(),
            Some(binding),
            None,
        )
    }
    pub(crate) fn captured(
        timeout: Duration,
        user_agent: Option<&str>,
        headers: reqwest::header::HeaderMap,
        binding: Option<Arc<HttpAdmission>>,
        roots: Option<Vec<reqwest::Certificate>>,
    ) -> Result<Self, ClientBuildError> {
        Self::captured_in(
            &vibe_net::current(),
            timeout,
            user_agent,
            headers,
            binding,
            roots,
        )
    }
    /// [`Self::captured`] under an explicit network session — the seam a
    /// proxy test drives without installing a process-wide route.
    pub fn captured_in(
        network: &Arc<vibe_net::NetworkSession>,
        timeout: Duration,
        user_agent: Option<&str>,
        headers: reqwest::header::HeaderMap,
        binding: Option<Arc<HttpAdmission>>,
        roots: Option<Vec<reqwest::Certificate>>,
    ) -> Result<Self, ClientBuildError> {
        let mut blocking = Client::builder()
            .timeout(timeout)
            .default_headers(headers.clone());
        let mut asynchronous = reqwest::Client::builder()
            .timeout(timeout)
            .default_headers(headers)
            .pool_max_idle_per_host(0);
        if let Some(agent) = user_agent {
            blocking = blocking.user_agent(agent);
            asynchronous = asynchronous.user_agent(agent);
        }
        if binding.is_some() {
            blocking = bound_builder(blocking);
            asynchronous = asynchronous
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .referer(false)
                .retry(reqwest::retry::never());
        }
        // PROP-068 `#proxy`: a bound client tunnels through the configured
        // proxy — admission has already judged the requested host name,
        // since the peer address is the proxy's — and never inherits an
        // ambient proxy; an unbound one keeps the environment without
        // `[proxy]`. Either way `[proxy] ca_file` joins the trust set.
        let ambient = if binding.is_some() {
            vibe_net::http::Ambient::Refuse
        } else {
            vibe_net::http::Ambient::Inherit
        };
        blocking = vibe_net::http::route_in(network, blocking, ambient)?;
        asynchronous = vibe_net::http::route_in(network, asynchronous, ambient)?;
        if let Some(roots) = roots {
            blocking = blocking.tls_built_in_root_certs(false);
            asynchronous = asynchronous.tls_built_in_root_certs(false);
            for root in roots {
                blocking = blocking.add_root_certificate(root.clone());
                asynchronous = asynchronous.add_root_certificate(root);
            }
        }
        // Both builders capture ambient default policy in this single factory,
        // before any operation. Controlled requests never create an unbound client.
        Ok(Self {
            blocking: blocking.build()?,
            asynchronous: asynchronous.build()?,
            binding,
        })
    }
    pub(crate) fn get(&self, url: impl reqwest::IntoUrl) -> RequestBuilder {
        self.blocking.get(url)
    }
    pub fn get_anonymous(
        &self,
        address: &str,
        cancellation: &vibe_core::cancellation::CancellationToken,
    ) -> Result<ManagedResponse, HttpGetError> {
        get_controlled(
            self,
            self.get(address),
            address,
            self.binding.as_deref(),
            cancellation,
        )
    }
}
impl std::fmt::Debug for ManagedHttpClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManagedHttpClient")
            .field("bound", &self.binding.is_some())
            .finish_non_exhaustive()
    }
}
/// Why a managed client could not be built: the HTTP stack, or the
/// network route (`[proxy] ca_file`, …) it must carry.
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#proxy")]
pub enum ClientBuildError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Route(#[from] vibe_net::NetError),
}
/// Adapter retained for existing no-control embedders; opaque clients cannot
/// acquire new cancellation policy after construction.
pub(crate) trait HttpClient {
    fn blocking(&self) -> &Client;
    fn asynchronous(&self) -> Option<&reqwest::Client>;
}
impl HttpClient for Client {
    fn blocking(&self) -> &Client {
        self
    }
    fn asynchronous(&self) -> Option<&reqwest::Client> {
        None
    }
}
impl HttpClient for ManagedHttpClient {
    fn blocking(&self) -> &Client {
        &self.blocking
    }
    fn asynchronous(&self) -> Option<&reqwest::Client> {
        Some(&self.asynchronous)
    }
}
