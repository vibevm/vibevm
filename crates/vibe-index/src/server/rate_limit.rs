//! Token-bucket rate limiter for the HTTP surface.
//!
//! Two parallel bucket pools:
//! - **Per-token** — keyed by the bearer token from the `Authorization`
//!   header. Mute abusive authenticated clients without proxy help.
//! - **Per-IP** — keyed by the connecting peer's IP. Mute anonymous
//!   reads (which are not auth-gated) when an attacker spams them.
//!
//! Each request picks ONE bucket: per-token if a Bearer header was
//! supplied, per-IP otherwise. A bucket is enabled only when the
//! corresponding `--rate-limit-per-{token,ip}` RPM is non-zero.
//!
//! Algorithm: classic token bucket. Capacity = configured RPM; refill
//! rate = RPM/60 tokens per second. So an idle client can burst up
//! to its full minute allowance, then is throttled to RPM/60 per
//! second steady-state.
//!
//! Eviction: when the per-IP map reaches `max_buckets`, idle entries
//! (tokens at >= 99% capacity) get dropped lazily on each `check`.
//! If even after that the map is full, the most-idle bucket gets
//! evicted to make room. PROP-005 §9 Q10 — this is the v1 surface
//! the PROP foresaw; v2 (per-route quotas, sliding window, dashmap
//! sharding) lands when scale demands it.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#open");

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use specmark::spec;

/// Poison-recovering lock: a panicking holder leaves the bucket maps
/// structurally intact (worst case an approximate token count), so
/// recovering the guard beats poisoning every later request into a
/// panic of its own.
fn locked<K>(pool: &Mutex<HashMap<K, Bucket>>) -> MutexGuard<'_, HashMap<K, Bucket>> {
    pool.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Default maximum tracked unique keys per bucket pool. Caps memory
/// at ~2 MB for the IP pool under sustained adversarial load.
pub const DEFAULT_MAX_BUCKETS: usize = 10_000;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Authenticated requests per minute, per token. Zero disables
    /// the per-token bucket.
    pub per_token_rpm: u32,
    /// Anonymous requests per minute, per IP. Zero disables the
    /// per-IP bucket.
    pub per_ip_rpm: u32,
    /// Soft cap on tracked keys per pool.
    pub max_buckets: usize,
}

impl RateLimitConfig {
    pub fn disabled() -> Self {
        RateLimitConfig {
            per_token_rpm: 0,
            per_ip_rpm: 0,
            max_buckets: DEFAULT_MAX_BUCKETS,
        }
    }

    pub fn enabled(&self) -> bool {
        self.per_token_rpm > 0 || self.per_ip_rpm > 0
    }
}

/// Which key the request is being rate-limited against.
#[derive(Debug)]
pub enum RateLimitKey<'a> {
    /// Authenticated request — bucket is per bearer token.
    Token(&'a str),
    /// Anonymous request — bucket is per peer IP.
    Ip(IpAddr),
}

#[derive(Debug)]
pub struct RateDecision {
    pub allowed: bool,
    /// Bucket capacity, surfaced in the `X-RateLimit-Limit` header.
    pub limit: u32,
    /// Tokens left in the bucket after this check (or before the
    /// rejection, when allowed=false). Surfaced in
    /// `X-RateLimit-Remaining`.
    pub remaining: u32,
    /// Seconds until at least one token replenishes. Used for the
    /// `Retry-After` header on 429 responses; 0 when allowed.
    pub retry_after_seconds: f64,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl Bucket {
    fn refill(&mut self, capacity: f64, rate_per_sec: f64, now: Instant) {
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * rate_per_sec).min(capacity);
            self.last_refill = now;
        }
    }

    fn try_consume(&mut self, capacity: f64, rate_per_sec: f64, now: Instant) -> RateDecision {
        self.refill(capacity, rate_per_sec, now);
        let limit = capacity as u32;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            RateDecision {
                allowed: true,
                limit,
                remaining: self.tokens.floor() as u32,
                retry_after_seconds: 0.0,
            }
        } else {
            // Tokens left is < 1; how long until 1 token?
            let needed = 1.0 - self.tokens;
            let retry = if rate_per_sec > 0.0 {
                needed / rate_per_sec
            } else {
                f64::INFINITY
            };
            RateDecision {
                allowed: false,
                limit,
                remaining: 0,
                retry_after_seconds: retry,
            }
        }
    }
}

#[derive(Debug)]
#[spec(implements = "spec://vibevm/modules/vibe-index/PROP-005#http", r = 1)]
pub struct RateLimiter {
    config: RateLimitConfig,
    by_token: Mutex<HashMap<String, Bucket>>,
    by_ip: Mutex<HashMap<IpAddr, Bucket>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        RateLimiter {
            config,
            by_token: Mutex::new(HashMap::new()),
            by_ip: Mutex::new(HashMap::new()),
        }
    }

    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Probe `key` against its bucket. Always returns a decision —
    /// `allowed = true` when the bucket is disabled (no quota
    /// configured) or when a token was successfully consumed.
    pub fn check(&self, key: RateLimitKey<'_>) -> RateDecision {
        self.check_at(key, Instant::now())
    }

    /// Same as [`check`] but with an injectable clock for tests.
    pub fn check_at(&self, key: RateLimitKey<'_>, now: Instant) -> RateDecision {
        match key {
            RateLimitKey::Token(token) => {
                if self.config.per_token_rpm == 0 {
                    return RateDecision {
                        allowed: true,
                        limit: 0,
                        remaining: 0,
                        retry_after_seconds: 0.0,
                    };
                }
                let capacity = self.config.per_token_rpm as f64;
                let rate = capacity / 60.0;
                let mut buckets = locked(&self.by_token);
                evict_if_full(&mut buckets, self.config.max_buckets, capacity);
                let bucket = buckets.entry(token.to_string()).or_insert_with(|| Bucket {
                    tokens: capacity,
                    last_refill: now,
                });
                bucket.try_consume(capacity, rate, now)
            }
            RateLimitKey::Ip(ip) => {
                if self.config.per_ip_rpm == 0 {
                    return RateDecision {
                        allowed: true,
                        limit: 0,
                        remaining: 0,
                        retry_after_seconds: 0.0,
                    };
                }
                let capacity = self.config.per_ip_rpm as f64;
                let rate = capacity / 60.0;
                let mut buckets = locked(&self.by_ip);
                evict_if_full(&mut buckets, self.config.max_buckets, capacity);
                let bucket = buckets.entry(ip).or_insert_with(|| Bucket {
                    tokens: capacity,
                    last_refill: now,
                });
                bucket.try_consume(capacity, rate, now)
            }
        }
    }

    /// Diagnostic accessor for tests.
    pub fn token_pool_size(&self) -> usize {
        locked(&self.by_token).len()
    }

    pub fn ip_pool_size(&self) -> usize {
        locked(&self.by_ip).len()
    }
}

/// When the pool is at or above `max_buckets`, drop entries that
/// are at >=99% capacity (idle since last activity). If still over
/// the cap, drop the single most-replenished bucket. Cheap because
/// most calls don't trip the threshold.
fn evict_if_full<K: Eq + std::hash::Hash + Clone>(
    buckets: &mut HashMap<K, Bucket>,
    max: usize,
    capacity: f64,
) {
    if buckets.len() < max {
        return;
    }
    let threshold = capacity * 0.99;
    buckets.retain(|_, b| b.tokens < threshold);
    if buckets.len() < max {
        return;
    }
    // Still full — drop the most-replenished entry (it's been most
    // recently quiescent). total_cmp gives floats a total order, so
    // no Option to unwrap even if a NaN ever crept into a bucket.
    if let Some(victim) = buckets
        .iter()
        .max_by(|a, b| a.1.tokens.total_cmp(&b.1.tokens))
        .map(|(k, _)| k.clone())
    {
        buckets.remove(&victim);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn token_only(rpm: u32) -> RateLimiter {
        RateLimiter::new(RateLimitConfig {
            per_token_rpm: rpm,
            per_ip_rpm: 0,
            max_buckets: DEFAULT_MAX_BUCKETS,
        })
    }

    #[test]
    fn disabled_allows_everything() {
        let r = RateLimiter::new(RateLimitConfig::disabled());
        for _ in 0..100 {
            let d = r.check(RateLimitKey::Token("alpha"));
            assert!(d.allowed);
        }
    }

    #[test]
    fn token_bucket_burst_then_throttle() {
        let r = token_only(60);
        let now = Instant::now();
        // Burst up to capacity (60).
        for i in 0..60 {
            let d = r.check_at(RateLimitKey::Token("alpha"), now);
            assert!(d.allowed, "request {i} should be allowed in initial burst");
        }
        // 61st request: bucket empty → reject.
        let d = r.check_at(RateLimitKey::Token("alpha"), now);
        assert!(!d.allowed);
        assert!(d.retry_after_seconds > 0.0);
        assert_eq!(d.remaining, 0);
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let r = token_only(60);
        let t0 = Instant::now();
        for _ in 0..60 {
            assert!(r.check_at(RateLimitKey::Token("alpha"), t0).allowed);
        }
        // 60 RPM = 1 per second. After 1.5s we should have 1 token.
        let t1 = t0 + Duration::from_millis(1500);
        let d = r.check_at(RateLimitKey::Token("alpha"), t1);
        assert!(d.allowed, "expected at least one refilled token at +1.5s");
    }

    #[test]
    fn token_keys_are_independent() {
        let r = token_only(2);
        let now = Instant::now();
        // Drain alpha.
        assert!(r.check_at(RateLimitKey::Token("alpha"), now).allowed);
        assert!(r.check_at(RateLimitKey::Token("alpha"), now).allowed);
        assert!(!r.check_at(RateLimitKey::Token("alpha"), now).allowed);
        // Beta has its own bucket — still allowed.
        assert!(r.check_at(RateLimitKey::Token("beta"), now).allowed);
    }

    #[test]
    fn ip_keys_track_separately_from_tokens() {
        let r = RateLimiter::new(RateLimitConfig {
            per_token_rpm: 5,
            per_ip_rpm: 2,
            max_buckets: DEFAULT_MAX_BUCKETS,
        });
        let now = Instant::now();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        // IP bucket is 2; token bucket is 5. Drain IP first.
        assert!(r.check_at(RateLimitKey::Ip(ip), now).allowed);
        assert!(r.check_at(RateLimitKey::Ip(ip), now).allowed);
        assert!(!r.check_at(RateLimitKey::Ip(ip), now).allowed);
        // Same IP but presenting a token → token bucket consulted, allowed.
        assert!(r.check_at(RateLimitKey::Token("alpha"), now).allowed);
    }

    #[test]
    fn limit_field_reflects_capacity() {
        let r = token_only(42);
        let d = r.check(RateLimitKey::Token("alpha"));
        assert_eq!(d.limit, 42);
    }

    #[test]
    fn retry_after_decreases_with_quota() {
        let r = token_only(60); // 1 token/sec
        let t0 = Instant::now();
        for _ in 0..60 {
            r.check_at(RateLimitKey::Token("alpha"), t0);
        }
        let d = r.check_at(RateLimitKey::Token("alpha"), t0);
        assert!(!d.allowed);
        assert!((d.retry_after_seconds - 1.0).abs() < 0.001);
    }

    #[test]
    fn ip_pool_eviction_caps_memory() {
        let r = RateLimiter::new(RateLimitConfig {
            per_token_rpm: 0,
            per_ip_rpm: 60,
            max_buckets: 4,
        });
        let now = Instant::now();
        // Fill 4 buckets, each touched once → all near full capacity.
        for i in 0..4 {
            let ip: IpAddr = format!("10.0.0.{i}").parse().unwrap();
            r.check_at(RateLimitKey::Ip(ip), now);
        }
        assert_eq!(r.ip_pool_size(), 4);
        // 5th distinct IP triggers eviction. Idle entries (>=99%
        // full) get dropped first; one remaining bucket gets pruned
        // to make room.
        let new_ip: IpAddr = "10.0.0.99".parse().unwrap();
        r.check_at(RateLimitKey::Ip(new_ip), now);
        assert!(
            r.ip_pool_size() <= 4,
            "expected eviction to keep pool ≤ max"
        );
    }
}
