//! One operation's network: the route, the lazily started px and the
//! operation's trust bundle — released when the operation ends.

use crate::{NetError, PxError, TrustSet, px::PxProcess};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, PoisonError, Weak},
};
use vibe_core::cancellation::{CancellationToken, WeakCancellationToken};
use vibe_core::net::{ProxyChoice, ProxyRoute};

enum PxSlot {
    Idle,
    Running(PxProcess),
    Failed(PxError),
    Closed,
}

/// The network of one operation (`PROP-068#proxy`).
///
/// It is an operation resource of the invocation's cancellation token
/// ([`enter`]): the vibe-managed px is started by the first request or git
/// child that needs it, every client and child of the operation reaches it
/// through this session, and [`NetworkSession::close`] — run by the
/// [`NetworkGuard`] when the operation ends, and by `Drop` as a backstop —
/// kills and reaps it. A command that never needs the network never starts
/// px.
///
/// ```
/// use vibe_core::net::ProxyRoute;
/// use vibe_net::NetworkSession;
/// let session = NetworkSession::new(ProxyRoute::direct());
/// assert!(!session.px_started());
/// let url = url::Url::parse("https://example.test/").unwrap();
/// assert_eq!(session.proxy_for(&url).unwrap(), None);
/// ```
pub struct NetworkSession {
    route: ProxyRoute,
    control: Option<WeakCancellationToken>,
    px: Mutex<PxSlot>,
    scratch: Mutex<Option<tempfile::TempDir>>,
    /// The operation's trust set, read once: the configured certificate
    /// files are opened on the first TLS client the operation builds, not
    /// once per client.
    trust: OnceLock<Arc<TrustSet>>,
    starts: std::sync::atomic::AtomicUsize,
}

impl std::fmt::Debug for NetworkSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkSession")
            .field("mode", &self.route.mode_name())
            .field("px_started", &self.px_started())
            .finish_non_exhaustive()
    }
}

impl NetworkSession {
    /// A session over `route`, observing no operation.
    pub fn new(route: ProxyRoute) -> Self {
        Self {
            route,
            control: None,
            px: Mutex::new(PxSlot::Idle),
            scratch: Mutex::new(None),
            trust: OnceLock::new(),
            starts: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn observing(route: ProxyRoute, control: WeakCancellationToken) -> Self {
        let mut session = Self::new(route);
        session.control = Some(control);
        session
    }

    pub fn route(&self) -> &ProxyRoute {
        &self.route
    }

    fn scratch(&self) -> Result<PathBuf, NetError> {
        let mut slot = self.scratch.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(dir) = slot.as_ref() {
            return Ok(dir.path().to_path_buf());
        }
        let dir = tempfile::Builder::new()
            .prefix("vibe-net-")
            .tempdir()
            .map_err(NetError::Scratch)?;
        let path = dir.path().to_path_buf();
        *slot = Some(dir);
        Ok(path)
    }

    fn stopped(&self) -> bool {
        self.control
            .as_ref()
            .and_then(WeakCancellationToken::upgrade)
            .is_some_and(|token| token.is_cancelled())
    }

    /// The address of this operation's px, starting it on first use.
    /// A failure is remembered: the operation does not retry px on every
    /// request, and [`Self::px_failure`] reports the cause once.
    pub fn px_address(&self) -> Result<SocketAddr, PxError> {
        let Some(settings) = self.route.px() else {
            return Err(PxError::Closed);
        };
        let mut slot = self.px.lock().unwrap_or_else(PoisonError::into_inner);
        match &*slot {
            PxSlot::Running(process) => return Ok(process.address()),
            PxSlot::Failed(error) => return Err(error.clone()),
            PxSlot::Closed => return Err(PxError::Closed),
            PxSlot::Idle => {}
        }
        if self.stopped() {
            return Err(PxError::Cancelled);
        }
        let started = crate::px::locate(settings).and_then(|program| {
            let scratch = self.scratch().map_err(|error| PxError::Spawn {
                program: program.clone(),
                reason: error.to_string(),
            })?;
            self.starts
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            PxProcess::start(&program, settings, self.route.no_proxy(), &scratch, &|| {
                self.stopped()
            })
        });
        match started {
            Ok(process) => {
                let address = process.address();
                tracing::info!(%address, pid = process.id(), "started vibe-managed px");
                *slot = PxSlot::Running(process);
                Ok(address)
            }
            Err(error) => {
                *slot = PxSlot::Failed(error.clone());
                Err(error)
            }
        }
    }

    /// Whether this session started px (it may since have been closed).
    pub fn px_started(&self) -> bool {
        self.starts.load(std::sync::atomic::Ordering::SeqCst) > 0
    }

    /// The process id of the running px, if one runs.
    pub fn px_pid(&self) -> Option<u32> {
        match &*self.px.lock().unwrap_or_else(PoisonError::into_inner) {
            PxSlot::Running(process) => Some(process.id()),
            _ => None,
        }
    }

    /// The cause px could not serve this operation, once it failed.
    pub fn px_failure(&self) -> Option<PxError> {
        match &*self.px.lock().unwrap_or_else(PoisonError::into_inner) {
            PxSlot::Failed(error) => Some(error.clone()),
            _ => None,
        }
    }

    /// The proxy a request to `url` goes through: `None` is direct.
    pub fn proxy_for(&self, url: &url::Url) -> Result<Option<url::Url>, PxError> {
        match self.route.choice_for(url) {
            None => Ok(None),
            Some(ProxyChoice::Explicit(proxy)) => Ok(Some(proxy.clone())),
            Some(ProxyChoice::ManagedPx) => {
                let address = self.px_address()?;
                url::Url::parse(&format!("http://{address}"))
                    .map(Some)
                    .map_err(|error| PxError::Port {
                        reason: error.to_string(),
                    })
            }
        }
    }

    /// The operation's trust set: the operating system's verifier plus
    /// `[tls] extra_ca` / `[proxy] ca_file`, read once.
    pub fn trust(&self) -> Result<&Arc<TrustSet>, NetError> {
        if let Some(set) = self.trust.get() {
            return Ok(set);
        }
        let set = Arc::new(TrustSet::of_route(&self.route)?);
        Ok(self.trust.get_or_init(|| set))
    }

    /// The TLS configuration every client of this operation is built from.
    pub fn tls_config(&self) -> Result<rustls::ClientConfig, NetError> {
        Ok(self.trust()?.client_config()?)
    }

    /// The combined PEM bundle a child `git` reads as `http.sslCAInfo` —
    /// this machine's own certificates followed by the configured extra
    /// ones, in the content-addressed cache. `None` when nothing is
    /// configured: the child then uses the platform's own trust, which on
    /// Windows is the whole certificate store rather than a file.
    pub fn ca_bundle(&self) -> Result<Option<PathBuf>, NetError> {
        Ok(self.trust()?.git_bundle()?)
    }

    /// Kill and reap px, and drop the scratch. Later requests that need px
    /// fail with [`PxError::Closed`] instead of starting another one.
    pub fn close(&self) {
        let previous = std::mem::replace(
            &mut *self.px.lock().unwrap_or_else(PoisonError::into_inner),
            PxSlot::Closed,
        );
        if let PxSlot::Running(mut process) = previous {
            let pid = process.id();
            match process.kill_and_reap() {
                Ok(()) => tracing::info!(pid, "stopped vibe-managed px"),
                Err(error) => tracing::warn!(pid, "could not stop the vibe-managed px: {error}"),
            }
        }
        self.scratch
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
    }
}

impl Drop for NetworkSession {
    fn drop(&mut self) {
        self.close();
    }
}

static CURRENT: Mutex<Weak<NetworkSession>> = Mutex::new(Weak::new());

/// Ends the operation's network when dropped: px is killed and reaped
/// whether the operation succeeded, failed or was stopped.
#[must_use = "dropping the guard ends the operation's network at once"]
pub struct NetworkGuard {
    session: Arc<NetworkSession>,
}

impl NetworkGuard {
    pub fn session(&self) -> &Arc<NetworkSession> {
        &self.session
    }
}

impl Drop for NetworkGuard {
    fn drop(&mut self) {
        self.session.close();
        let mut current = CURRENT.lock().unwrap_or_else(PoisonError::into_inner);
        if current
            .upgrade()
            .is_some_and(|s| Arc::ptr_eq(&s, &self.session))
        {
            *current = Weak::new();
        }
    }
}

/// Begin the network of the operation `token` controls: the session
/// becomes that operation's resource and this process's current session.
///
/// ```
/// use vibe_core::cancellation::CancellationToken;
/// use vibe_core::net::ProxyRoute;
/// let token = CancellationToken::new();
/// let guard = vibe_net::enter(&token, ProxyRoute::direct());
/// assert!(std::sync::Arc::ptr_eq(guard.session(), &vibe_net::current()));
/// drop(guard);
/// ```
pub fn enter(token: &CancellationToken, route: ProxyRoute) -> NetworkGuard {
    let session = token.operation_resource(|| NetworkSession::observing(route, token.downgrade()));
    *CURRENT.lock().unwrap_or_else(PoisonError::into_inner) = Arc::downgrade(&session);
    NetworkGuard { session }
}

/// The current operation's session; outside one, a process-wide session
/// over the installed route ([`vibe_core::net::route`]).
pub fn current() -> Arc<NetworkSession> {
    if let Some(session) = CURRENT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .upgrade()
    {
        return session;
    }
    static FALLBACK: OnceLock<Arc<NetworkSession>> = OnceLock::new();
    FALLBACK
        .get_or_init(|| Arc::new(NetworkSession::new(vibe_core::net::route().clone())))
        .clone()
}
