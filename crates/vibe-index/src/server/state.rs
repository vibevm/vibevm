//! Shared server state — the in-memory [`Index`] under an async
//! `RwLock`, a configuration snapshot, and per-process counters used
//! by the `/metrics` endpoint.
//!
//! The two swappable server dependencies — the [`TokenStore`] auth
//! authority and the [`RateLimiter`] — are held as trait objects, so a
//! handler consumes a seam, not a concrete type. The convenience
//! constructors build the production variants ([`FileTokenStore`],
//! [`TokenBucketRateLimiter`]); [`AppState::with_seams`] injects either
//! directly, which is how a test drives a handler through a fake.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use tokio::sync::RwLock;

use crate::index::Index;
use crate::server::auth::{FileTokenStore, TokenStore};
use crate::server::rate_limit::{RateLimitConfig, RateLimiter, TokenBucketRateLimiter};

#[derive(Debug)]
pub struct AppState {
    pub data_dir: PathBuf,
    pub read_only: bool,
    pub started_at: Instant,
    pub generator: String,
    pub index: RwLock<Index>,
    pub stats: Stats,
    pub tokens: Box<dyn TokenStore>,
    pub rate_limiter: Box<dyn RateLimiter>,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub requests_total: AtomicU64,
    pub mutations_total: AtomicU64,
}

impl Stats {
    pub fn note_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn note_mutation(&self) {
        self.mutations_total.fetch_add(1, Ordering::Relaxed);
    }
}

impl AppState {
    pub fn new(data_dir: PathBuf, read_only: bool, index: Index) -> Self {
        AppState::with_tokens(data_dir, read_only, index, FileTokenStore::default())
    }

    pub fn with_tokens(
        data_dir: PathBuf,
        read_only: bool,
        index: Index,
        tokens: impl TokenStore + 'static,
    ) -> Self {
        Self::with_tokens_and_rate_limit(
            data_dir,
            read_only,
            index,
            tokens,
            RateLimitConfig::disabled(),
        )
    }

    pub fn with_tokens_and_rate_limit(
        data_dir: PathBuf,
        read_only: bool,
        index: Index,
        tokens: impl TokenStore + 'static,
        rate_limit: RateLimitConfig,
    ) -> Self {
        Self::with_seams(
            data_dir,
            read_only,
            index,
            Box::new(tokens),
            Box::new(TokenBucketRateLimiter::new(rate_limit)),
        )
    }

    /// The most general constructor — both server seams injected as
    /// trait objects. The constructors above build the production
    /// variants; a test reaches here to inject a fake [`TokenStore`] or
    /// [`RateLimiter`] and drive a handler through it.
    pub fn with_seams(
        data_dir: PathBuf,
        read_only: bool,
        index: Index,
        tokens: Box<dyn TokenStore>,
        rate_limiter: Box<dyn RateLimiter>,
    ) -> Self {
        AppState {
            generator: index.generator.clone(),
            data_dir,
            read_only,
            started_at: Instant::now(),
            index: RwLock::new(index),
            stats: Stats::default(),
            tokens,
            rate_limiter,
        }
    }
}
