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
    /// Whether the pools follow a redirect by themselves. A bound pool
    /// never does, and neither does a [`Self::direct`] one; the walk that
    /// decides what a hop may carry needs the chain in its own hands.
    follows_redirects: bool,
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
        trust: Option<Vec<u8>>,
    ) -> Result<Self, ClientBuildError> {
        Self::captured_in(
            &vibe_net::current(),
            timeout,
            user_agent,
            headers,
            binding,
            trust,
        )
    }
    /// The client of a request that carries caller-supplied headers
    /// (`PROP-075#DOWNLOAD-HEADERS`): exactly the pool [`Self::bound`] or
    /// [`Self::legacy`] would build for this `binding` — same admission,
    /// same proxy and trust — except that it never follows a redirect on
    /// its own. The chain belongs to [`Self::get_with_headers`], because
    /// only a walk that sees each hop can keep those headers on the origin
    /// the address named.
    pub fn direct(
        timeout: Duration,
        binding: Option<Arc<HttpAdmission>>,
    ) -> Result<Self, ClientBuildError> {
        Self::build(
            &vibe_net::current(),
            timeout,
            None,
            reqwest::header::HeaderMap::new(),
            binding,
            None,
            true,
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
        trust: Option<Vec<u8>>,
    ) -> Result<Self, ClientBuildError> {
        Self::build(network, timeout, user_agent, headers, binding, trust, false)
    }
    #[allow(clippy::too_many_arguments)]
    fn build(
        network: &Arc<vibe_net::NetworkSession>,
        timeout: Duration,
        user_agent: Option<&str>,
        headers: reqwest::header::HeaderMap,
        binding: Option<Arc<HttpAdmission>>,
        trust: Option<Vec<u8>>,
        direct: bool,
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
        } else if direct {
            // Nothing else of the incumbent unbound policy moves — only
            // the redirects, which this client's caller walks itself.
            blocking = blocking.redirect(reqwest::redirect::Policy::none());
            asynchronous = asynchronous.redirect(reqwest::redirect::Policy::none());
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
        // A captured registry trust REPLACES the installation's trust set
        // rather than adding to it — that is what capturing one means —
        // so it is applied after the route, over the configuration the
        // route installed, and the two can never be in force at once.
        if let Some(pem) = trust {
            let config =
                vibe_net::TrustSet::pinned_roots_config(&pem).map_err(vibe_net::NetError::from)?;
            blocking = blocking.use_preconfigured_tls(config.clone());
            asynchronous = asynchronous.use_preconfigured_tls(config);
        }
        // Both builders capture ambient default policy in this single factory,
        // before any operation. Controlled requests never create an unbound client.
        let follows_redirects = binding.is_none() && !direct;
        Ok(Self {
            blocking: blocking.build()?,
            asynchronous: asynchronous.build()?,
            binding,
            follows_redirects,
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
    /// [`Self::get_anonymous`] with the caller's own request headers
    /// attached (`PROP-075#DOWNLOAD-HEADERS`). The admission runs first
    /// and unchanged — a header decides nothing about *whether* an
    /// endpoint may be contacted — and the headers are origin-bound: they
    /// go to the origin `address` names and to no other, whether or not a
    /// binding is in force. Only a [`Self::direct`] client may do this,
    /// because any other pool would follow a redirect before this side
    /// could decide.
    pub fn get_with_headers(
        &self,
        address: &str,
        cancellation: &vibe_core::cancellation::CancellationToken,
        headers: &vibe_core::download_headers::DownloadHeaders,
    ) -> Result<ManagedResponse, HttpGetError> {
        if self.follows_redirects {
            return Err(ContactRefusal::UnsupportedControlledRequest.into());
        }
        let mut map = reqwest::header::HeaderMap::new();
        let mut bound = Vec::with_capacity(headers.len());
        for header in headers {
            // The file reader has already refused everything `http` would
            // refuse here, so this cannot fail on a validated file; a
            // refusal rather than a panic keeps that a statement about the
            // reader and not an assumption about this crate.
            let name = HeaderName::from_bytes(header.name().as_bytes())
                .map_err(|_| ContactRefusal::UnsupportedControlledRequest)?;
            let value = reqwest::header::HeaderValue::from_str(header.value())
                .map_err(|_| ContactRefusal::UnsupportedControlledRequest)?;
            bound.push(name.clone());
            map.append(name, value);
        }
        Ok(walk(
            self,
            self.get(address).headers(map),
            address,
            self.binding.as_deref(),
            &bound,
            cancellation,
        )?
        .response)
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
