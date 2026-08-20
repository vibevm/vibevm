//! Optional HTTP client that lets `GitPerPackageRegistry` consult an
//! upstream index (PROP-005 §2.10) for cheap version enumeration
//! before falling back to `git ls-remote`. Slice 10.
//!
//! The client is resilient: any failure (4xx, 5xx, connect-fail,
//! malformed JSON) returns an error that the caller treats as a
//! fall-through trigger. Identity (`content_hash`) is verified at
//! fetch time per [PROP-002 §2.1] regardless of how versions were
//! enumerated, so a compromised index can at worst mislead the
//! version selector — never substitute content.
//!
//! **Authentication (A2-INDEXAUTH).** The index is THIS registry's
//! index, so the client authenticates with the registry's own
//! credentials — an optional bearer token resolved from the
//! `[[registry]]` `auth` / `token_env` the same way the git side
//! does (see [`auth`]). The token rides an `Authorization: Bearer`
//! header on every request via the client's `default_headers`, never
//! in the URL, and only over `https://`. Regimes that cannot serve
//! HTTP auth (`ssh`, `credential-helper`) carry no token and surface
//! [`IndexError::AuthIncapable`] on a 401/403 instead of looking like
//! "no index here". See [`IndexAuth`] and [`ProbeOutcome`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http");

mod auth;
mod handshake;
mod locate;
mod wire;

pub use auth::{BearerToken, IndexAuth};
pub use locate::{IndexUrlResolution, IndexUrlSource, index_url_for, resolve_index_url};
pub use wire::{BindingSite, PurlLookupHit, PurlLookupResults, SearchHit, SearchResults};

use std::time::Duration;

use semver::Version;
use specmark::spec;
use thiserror::Error;
use vibe_core::{Group, PackageKind};

use auth::refusal_reason;
use wire::NameEntryView;

const PROBE_TIMEOUT_SECS: u64 = 5;
const FETCH_TIMEOUT_SECS: u64 = 10;

/// Resolved client.
///
/// `file_base` is the URL prefix that, when joined with `repomd.json`
/// or `by-name/<name>.json`, addresses the per-file endpoints
/// (the static-mirror-friendly read surface from PROP-005 §2.4).
/// `server_base` is the URL prefix for structured live-server routes
/// (`/v1/packages`, `/v1/capabilities/{cap}`, etc. from PROP-005
/// §2.10). Built via [`IndexClient::probe`] which auto-detects
/// whether the supplied operator URL points at a vibe-index server
/// (`<base>/v1/index/...`) or a static raw-file root (`<base>/...`)
/// — `server_base` is always the bare `<base>` regardless, since the
/// structured routes only exist on a live server and never on a
/// static mirror.
///
/// `auth` is the index-access plan resolved from the registry's
/// credentials (see [`IndexAuth`]). Its [`Debug`](std::fmt::Debug)
/// representation redacts any bearer token, so this struct's derived
/// `Debug` never leaks the secret.
#[derive(Debug, Clone)]
pub struct IndexClient {
    file_base: String,
    server_base: String,
    auth: IndexAuth,
}

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http")]
pub enum IndexError {
    #[error(
        "HTTP request to `{url}` failed \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
          fix: check the index URL and network reachability): {message}"
    )]
    Http { url: String, message: String },
    #[error(
        "index at `{url}` returned status {status} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
          fix: check the index server health at that URL)"
    )]
    Status { url: String, status: u16 },
    #[error(
        "index at `{url}` returned status {status} — authentication required, but the \
         registry's `auth = \"{regime}\"` authorises git transport, not HTTP \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth; \
          fix: declare `auth = \"token-env\"` on this registry with a token the index accepts, \
          read from an env var named VIBEVM_REGISTRY_TOKEN_<HOST>)"
    )]
    AuthIncapable {
        url: String,
        regime: &'static str,
        status: u16,
    },
    #[error(
        "index at `{url}` returned malformed JSON \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
          fix: regenerate the index via reindex): {message}"
    )]
    Malformed { url: String, message: String },
}

/// Outcome of [`IndexClient::probe`]. Distinguishes the three cases an
/// operator must tell apart (PROP-005 §2.10, PROP-002 §2.2.1):
///
/// - [`ProbeOutcome::Found`] — the index answered; the client is ready.
/// - [`ProbeOutcome::Absent`] — nothing answered here (404, connect-fail,
///   5xx): silent fall-through to `git ls-remote`, as before.
/// - [`ProbeOutcome::Refused`] — the index is there but this build
///   cannot use it: it refused us (401/403), OR its handshake
///   publishes no world this build reads (an unparseable body, an
///   unknown handshake format, foreign epochs — each reason names
///   the facts and the fix). Surfaced via `UnreachableRegistry.reason`
///   so a private or newer-than-us index is **not** indistinguishable
///   from a missing one.
#[derive(Debug)]
pub enum ProbeOutcome {
    Found(IndexClient),
    Absent,
    Refused { reason: String },
}

impl IndexClient {
    /// Probe the operator-supplied base URL with the registry's
    /// [`IndexAuth`] plan. The eternal handshake `hello.json` is
    /// asked FIRST, at BOTH candidate bases (`<base>/v1/index` then
    /// `<base>`), before any `repomd.json` probe: the handshake's
    /// `successor` key is the in-band forwarding pointer for a moved
    /// index, and it is readable exactly when the old address no
    /// longer serves a catalog (PROP-044 `##ONE-ETERNAL-FILE`) — a
    /// handshake sought only beside a found `repomd` would never be
    /// read precisely when it matters. On HTTP 200 the handshake is
    /// parsed by the generated type and its worlds matched against
    /// the epoch this build reads (`FormatId::IndexRepomd.epoch()`
    /// from the generated registry — the client mints no constant of
    /// its own); the winning world's `path` refines `file_base`
    /// (`"."` keeps the candidate untouched, no `/.` tail). A
    /// handshake this build cannot use — an unparseable body, an
    /// unknown handshake format, no world of its epoch — is
    /// [`ProbeOutcome::Refused`] naming the offered epochs, this
    /// build's epoch, and the fix (`successor` named, never
    /// followed). The price of asking first is paid only by
    /// handshake-less indexes: up to two extra GETs.
    ///
    /// When neither candidate answers `hello.json` with 200, the
    /// probe keeps today's path unchanged — the compatibility with
    /// pre-handshake indexes the format family requires: returns
    /// [`ProbeOutcome::Found`] if `<base>/repomd.json` OR
    /// `<base>/v1/index/repomd.json` responds HTTP 200;
    /// [`ProbeOutcome::Refused`] if any probe step responds 401/403
    /// (the index is private — the reason carries regime-specific
    /// guidance); [`ProbeOutcome::Absent`] for anything else (404,
    /// connect-fail, 5xx — no index there). Probe timeout is short
    /// (5s) so a misconfigured URL does not stall every install. The
    /// probe request itself carries the bearer token when the plan is
    /// [`IndexAuth::Bearer`], so a private index's probe authenticates.
    pub fn probe(base: &str, auth: IndexAuth) -> ProbeOutcome {
        let trimmed = base.trim_end_matches('/');
        let client = match Self::build_client(
            Duration::from_secs(PROBE_TIMEOUT_SECS),
            &auth,
            trimmed,
        ) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(target: "vibe_registry::index_client", "could not build probe client: {e}");
                return ProbeOutcome::Absent;
            }
        };
        let candidates = [format!("{trimmed}/v1/index"), trimmed.to_string()];
        // The eternal handshake, asked first at every candidate —
        // including the ones where no `repomd` will ever answer
        // again (a moved index answers `successor` here, or nothing).
        for candidate in &candidates {
            match handshake::probe_candidate(&client, candidate, &auth) {
                handshake::HandshakeProbe::Found { file_base } => {
                    tracing::debug!(
                        target: "vibe_registry::index_client",
                        "probe succeeded via handshake at {candidate}"
                    );
                    return ProbeOutcome::Found(IndexClient {
                        file_base,
                        server_base: trimmed.to_string(),
                        auth,
                    });
                }
                handshake::HandshakeProbe::Refused { reason } => {
                    return ProbeOutcome::Refused { reason };
                }
                handshake::HandshakeProbe::Absent => {}
            }
        }
        // No handshake anywhere — today's `repomd.json` path, byte
        // for byte: the compatibility surface for indexes without a
        // handshake.
        for candidate in candidates {
            let url = format!("{candidate}/repomd.json");
            match client.get(&url).send() {
                Ok(resp) if resp.status().is_success() => {
                    tracing::debug!(target: "vibe_registry::index_client", "probe succeeded at {url}");
                    return ProbeOutcome::Found(IndexClient {
                        file_base: candidate,
                        server_base: trimmed.to_string(),
                        auth,
                    });
                }
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if matches!(status, 401 | 403) {
                        let reason = refusal_reason(trimmed, status, &auth);
                        tracing::warn!(
                            target: "vibe_registry::index_client",
                            "index probe at `{url}` refused: {reason}"
                        );
                        return ProbeOutcome::Refused { reason };
                    }
                    // 404 / 5xx / other non-success — try the next
                    // candidate, then fall through to Absent.
                    tracing::debug!(
                        target: "vibe_registry::index_client",
                        "probe at `{url}` non-success ({status}); trying next candidate"
                    );
                }
                Err(e) => {
                    tracing::debug!(target: "vibe_registry::index_client", "probe at `{url}` errored: {e}");
                }
            }
        }
        tracing::debug!(target: "vibe_registry::index_client", "no index found at base `{base}`");
        ProbeOutcome::Absent
    }

    /// Construct directly without probing, with no auth plan. Used by
    /// tests where the caller has set up the server and knows its
    /// layout. Both `file_base` and `server_base` are set to the
    /// supplied URL — suitable for the in-tree `tests/` mock servers
    /// that mount raw-file routes (`/repomd.json`, `/by-name/...`) and
    /// the structured server routes (`/v1/packages`) on the same root.
    /// Equivalent to [`IndexClient::at_with_auth`] with
    /// [`IndexAuth::None`].
    pub fn at(base: impl Into<String>) -> IndexClient {
        Self::at_with_auth(base, IndexAuth::None)
    }

    /// Construct directly without probing, carrying an explicit
    /// [`IndexAuth`] plan. The scheme gate for the bearer token lives
    /// in [`IndexAuth::for_registry`] (a `Bearer` plan is produced only
    /// for an `https://` base); this constructor is the low-level
    /// escape hatch for tests / direct use that already know the plan,
    /// mirroring how [`crate::git_package_registry::inject_token`] is
    /// public for external integrations while staying scheme-gated
    /// internally.
    pub fn at_with_auth(base: impl Into<String>, auth: IndexAuth) -> IndexClient {
        let trimmed = base.into().trim_end_matches('/').to_string();
        IndexClient {
            file_base: trimmed.clone(),
            server_base: trimmed,
            auth,
        }
    }

    pub fn file_base(&self) -> &str {
        &self.file_base
    }

    pub fn server_base(&self) -> &str {
        &self.server_base
    }

    /// The [`IndexAuth`] plan this client authenticates with.
    pub fn auth(&self) -> &IndexAuth {
        &self.auth
    }

    /// Fetch the `by-name/<name>.json` candidate set and return the
    /// versions of the `(group, name)` package in ascending semver
    /// order. Returns `Ok(None)` when the file is absent (404) **or**
    /// the candidate set carries no package for `group` — both mean
    /// "fall through to `git ls-remote`". `Ok(Some(versions))` on a
    /// hit; `Err(...)` for any other failure.
    ///
    /// The `by-name/` layer is keyed by bare `name` and holds the whole
    /// candidate set — every group that publishes a package of that
    /// name (PROP-008 §2.8). The lookup selects the candidate whose
    /// `group` matches the requested `(group, name)` identity.
    pub fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Option<Vec<Version>>, IndexError> {
        let url = format!("{}/by-name/{}.json", self.file_base, name);
        let client = Self::build_client(
            Duration::from_secs(FETCH_TIMEOUT_SECS),
            &self.auth,
            &self.file_base,
        )
        .map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let resp = client.get(&url).send().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(self.classify_failure(url, status.as_u16()));
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let parsed: NameEntryView =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        let Some(pkg) = parsed.packages.into_iter().find(|p| &p.group == group) else {
            return Ok(None);
        };
        let mut versions: Vec<Version> = pkg.versions.into_iter().map(|v| v.version).collect();
        versions.sort();
        Ok(Some(versions))
    }

    /// Fetch the `by-name/<name>.json` candidate set and return every
    /// `group` that publishes a package of this bare name (PROP-008
    /// §2.8). This is the primitive index-backed short-name resolution
    /// (PROP-008 §2.6) walks: one GET per registry enumerates the
    /// `(*, name)` candidates, so a collision (PROP-008 §2.7) — two
    /// groups under one bare name — is visible at once.
    ///
    /// `Ok(vec![])` when the file is absent (404) — the name is simply
    /// not carried by this index. `Err(...)` for any other failure;
    /// the caller decides whether to treat it as fatal or skip the
    /// registry. Groups are returned in on-disk order; de-duplication
    /// and sorting are the caller's job (it unions across registries).
    pub fn name_candidates(&self, name: &str) -> Result<Vec<Group>, IndexError> {
        let url = format!("{}/by-name/{}.json", self.file_base, name);
        let client = Self::build_client(
            Duration::from_secs(FETCH_TIMEOUT_SECS),
            &self.auth,
            &self.file_base,
        )
        .map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let resp = client.get(&url).send().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(Vec::new());
        }
        if !status.is_success() {
            return Err(self.classify_failure(url, status.as_u16()));
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let parsed: NameEntryView =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        Ok(parsed.packages.into_iter().map(|p| p.group).collect())
    }

    /// Direct PURL lookup against the live-server route
    /// `<server_base>/v1/purls/{purl}` from PROP-005 §2.10. Returns
    /// every package whose top-level `describes` or any subskill's
    /// `describes` equals the supplied PURL, with the `binding_site`
    /// surfaced so consumers see whether the match originated at the
    /// package or subskill level.
    ///
    /// Non-2xx surfaces as [`IndexError::Status`] (or
    /// [`IndexError::AuthIncapable`] under an HTTP-incapable regime);
    /// 404 here means the URL points at a raw-file mirror without the
    /// live server. Empty `hits` is the "no match" case (HTTP 200 with
    /// 0-length list), not 404. Path-segment encoding is delegated to
    /// `reqwest::Url` so PURL punctuation (`:`, `/`, `@`) is escaped
    /// correctly.
    pub fn lookup_purl(&self, purl: &str) -> Result<PurlLookupResults, IndexError> {
        let base_url = format!("{}/v1/purls/", self.server_base);
        let mut parsed = reqwest::Url::parse(&base_url).map_err(|e| IndexError::Http {
            url: base_url.clone(),
            message: e.to_string(),
        })?;
        parsed
            .path_segments_mut()
            .map_err(|_| IndexError::Http {
                url: base_url.clone(),
                message: "base URL is not hierarchical".into(),
            })?
            .pop_if_empty()
            .push(purl);
        let url = parsed.to_string();
        let client = Self::build_client(
            Duration::from_secs(FETCH_TIMEOUT_SECS),
            &self.auth,
            &self.server_base,
        )
        .map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let resp = client.get(&url).send().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(self.classify_failure(url, status.as_u16()));
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let parsed: PurlLookupResults =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        Ok(parsed)
    }

    /// Run a full-text search against the live-server route
    /// `<server_base>/v1/packages?q=<query>[&kind=&limit=]` from
    /// PROP-005 §2.10. Returns the structured response on 200; any
    /// non-2xx status surfaces as [`IndexError::Status`] (or
    /// [`IndexError::AuthIncapable`] under an HTTP-incapable regime)
    /// so the caller can decide whether to fall through to another
    /// registry or surface the error. A 404 here means the URL is a
    /// raw-file mirror (no live server), not "package absent" — there
    /// is no "package absent" case for this endpoint, since search
    /// returns an empty `hits` array on no matches. Identity /
    /// integrity invariants are unaffected: search is metadata-only
    /// and never resolves into a fetch without the consumer running
    /// through the regular `MultiRegistryResolver` path that
    /// re-verifies `content_hash` per [PROP-002 §2.1].
    pub fn search(
        &self,
        query: &str,
        kind: Option<PackageKind>,
        limit: Option<usize>,
    ) -> Result<SearchResults, IndexError> {
        let url = format!("{}/v1/packages", self.server_base);
        let client = Self::build_client(
            Duration::from_secs(FETCH_TIMEOUT_SECS),
            &self.auth,
            &self.server_base,
        )
        .map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let mut req = client.get(&url).query(&[("q", query)]);
        if let Some(k) = kind {
            req = req.query(&[("kind", k.as_str())]);
        }
        if let Some(lim) = limit {
            req = req.query(&[("limit", lim.to_string())]);
        }
        let resp = req.send().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(self.classify_failure(url, status.as_u16()));
        }
        let body = resp.bytes().map_err(|e| IndexError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let parsed: SearchResults =
            serde_json::from_slice(&body).map_err(|e| IndexError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            })?;
        Ok(parsed)
    }

    /// Map a non-success status to the right error. A 401/403 under an
    /// HTTP-incapable regime ([`IndexAuth::HttpIncapable`]) becomes
    /// [`IndexError::AuthIncapable`] — naming the regime and the fix —
    /// so the operator is told what to do instead of seeing a bare
    /// status. Every other case keeps the generic [`IndexError::Status`]
    /// (the status code itself is the signal).
    fn classify_failure(&self, url: String, status: u16) -> IndexError {
        if matches!(status, 401 | 403)
            && let IndexAuth::HttpIncapable(regime) = &self.auth
        {
            return IndexError::AuthIncapable {
                url,
                regime,
                status,
            };
        }
        IndexError::Status { url, status }
    }

    /// Build the blocking client for one request. The single chokepoint
    /// every request funnels through, so the bearer token (when the
    /// plan is [`IndexAuth::Bearer`]) is attached once here via
    /// `default_headers` and rides every request — including the probe
    /// — without touching the individual `.send()` call sites. A fresh
    /// client per call preserves the per-call timeout (5s probe /
    /// 10s fetch).
    ///
    /// **Р3 layer 2 — the attachment refuses plaintext.** The token is
    /// attached only when this client's `base_url` starts with
    /// `https://`. Layer 1 (`IndexAuth::plan`, reached via
    /// [`IndexAuth::for_registry`]) never births a `Bearer` plan for a
    /// non-https base, but that gate alone could be bypassed by a public
    /// constructor (`at_with_auth("http://…", Bearer)`); this one
    /// cannot, because every request — from every constructor — funnels
    /// through here and is checked against its own base. Mirrors
    /// [`crate::git_package_registry::inject_token`], which checks
    /// `https://` inside the function the caller cannot sidestep.
    fn build_client(
        timeout: Duration,
        auth: &IndexAuth,
        base_url: &str,
    ) -> Result<reqwest::blocking::Client, reqwest::Error> {
        let mut builder = reqwest::blocking::Client::builder()
            .user_agent(concat!("vibe-registry/", env!("CARGO_PKG_VERSION")))
            .timeout(timeout);
        if attaches_authorization(base_url, auth)
            && let Some(headers) = auth.header_map()
        {
            builder = builder.default_headers(headers);
        }
        builder.build()
    }
}

/// Does this client attach `Authorization` — the whole decision, in one
/// place, so it can be asserted directly.
///
/// Extracted from [`IndexClient::build_client`] because once the scheme
/// gate moved into the attachment step, the positive case became
/// untestable end to end: this crate's mock servers are plain HTTP, so
/// every integration test can now exercise only the suppressing rows. A
/// truth table nobody can assert is a truth table that drifts, and the
/// row that would drift silently is the one that matters — a token that
/// quietly stops being attached breaks private-index reads while every
/// «no header here» test stays green.
fn attaches_authorization(base_url: &str, auth: &IndexAuth) -> bool {
    base_url.starts_with("https://") && matches!(auth, IndexAuth::Bearer(_))
}

#[cfg(test)]
mod tests;
