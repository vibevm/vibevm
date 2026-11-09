//! The one owner of vibevm's network route (`PROP-068#proxy`).
//!
//! [`vibe_core::net`] decides where a URL goes. This crate acts on that
//! decision for everything vibe itself connects with:
//!
//! - [`http`] configures every `reqwest` client builder: the configured
//!   proxy (HTTP `CONNECT` for `https://`), the bypass list, and the trust
//!   set — the operating system's root store (reqwest's native-roots
//!   feature) plus `[proxy] ca_file`. Verification is never disabled.
//! - [`NetworkSession`] is one operation's network: it starts the
//!   vibe-managed px lazily at the first request that needs it, routes the
//!   operation's clients and git children through it, and kills and reaps
//!   it when the operation ends ([`enter`] / [`NetworkGuard`]).
//! - [`git`] is the explicit proxy and CA configuration a child `git`
//!   receives; the user's git configuration is never written.
//! - [`doctor`] diagnoses the route step by step.
//! - [`system_proxy`] reads the logged-in Windows user's proxy
//!   configuration, so `[proxy] pac = "system"` becomes the explicit
//!   `--pac=<url>` px is given instead of a detection px has to do
//!   itself (`PROP-068#proxy-px`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy");

pub mod doctor;
pub mod git;
pub mod http;
pub mod probe;
mod px;
pub mod px_store;
#[cfg(test)]
mod px_tests;
mod session;
pub mod system_proxy;
mod trust;

pub use px::{PX_EXE, PX_READY_TIMEOUT, PxError, bundled_px_candidates};
pub use session::{NetworkGuard, NetworkSession, current, enter};
pub use trust::TrustError;

/// Why the network route could not be applied. Each message names the
/// cause and the fix, and never carries a credential.
///
/// ```
/// use vibe_net::{NetError, TrustError};
/// let error = NetError::from(TrustError::Empty { path: "/etc/corp-ca.pem".into() });
/// assert!(error.to_string().contains("PROP-068#proxy"));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#proxy")]
pub enum NetError {
    #[error(transparent)]
    Trust(#[from] TrustError),
    #[error(transparent)]
    Px(#[from] PxError),
    #[error(
        "could not prepare the operation's network scratch: {0} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: make the temporary directory writable)"
    )]
    Scratch(std::io::Error),
}
