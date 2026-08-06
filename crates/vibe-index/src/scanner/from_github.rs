//! Walk a GitHub org via the REST API + clone every listed repo
//! into a scratch directory, then run `from_clones::scan_org_dir` on
//! the result. PROP-005 §2.8 / slice 8.
//!
//! Public-org orgs (default vibevm posture) clone over HTTPS without
//! auth. When `token` is supplied, the REST API call uses it for
//! higher rate limits + access to private repos; the clone URL is
//! rewritten to embed credentials for the duration of the clone
//! (matches the discipline `vibe-publish::github` follows for HTTPS
//! token-auth pushes — token never appears in logs or process output).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#reindex");

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use reqwest::header::{
    ACCEPT, AUTHORIZATION, ETAG, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
    LINK, USER_AGENT,
};
use serde::{Deserialize, Serialize};
use specmark::{cell, spec};

use crate::error::{Error, Result};
use crate::index::checkpoint::Checkpoint;
use crate::scanner::PackageScanner;
use crate::scanner::git_cli;
use crate::scanner::org_cache::{self, OrgCache, Validator};
use crate::scanner::org_walk::{FromClonesOptions, ScanReport, scan_org_dir_with_filter};

const DEFAULT_API_BASE: &str = "https://api.github.com";
const USER_AGENT_VAL: &str = concat!("vibe-index/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct FromGithubOptions {
    pub api_base: String,
    pub org: String,
    pub token: Option<String>,
    pub clone_into: PathBuf,
    pub timeout: Duration,
    /// Skip forks (default `true`) — index entries for downstream
    /// forks would collide with the upstream's. Set `false` if the
    /// org curates forks deliberately.
    pub skip_forks: bool,
    /// Path to the org-image cache file
    /// (`<data-dir>/state/org-cache.json`, computed by the
    /// composition root via [`org_cache::path`]). When `Some`, the
    /// scanner persists the enumerated image here after every run,
    /// and — when `probe_freshness` is true — consults it first to
    /// skip re-enumerating an unchanged org (Р2, Р3). `None` ⇒
    /// caching is off entirely: no read, no write, behaviour
    /// identical to pre-A3 (Р6 — only the `--from-github` path sets
    /// this; `--from-clones` never does).
    pub org_cache_path: Option<PathBuf>,
    /// When true and `org_cache_path` is set, send the stored
    /// validator and short-circuit the enumeration on a 304. When
    /// false (`rescan-org`, or the image is unusable), enumerate
    /// unconditionally — the image is still re-persisted afterwards
    /// so the next run benefits (Р4).
    pub probe_freshness: bool,
}

impl FromGithubOptions {
    pub fn new(org: impl Into<String>, clone_into: PathBuf) -> Self {
        FromGithubOptions {
            api_base: DEFAULT_API_BASE.into(),
            org: org.into(),
            token: None,
            clone_into,
            timeout: Duration::from_secs(30),
            skip_forks: true,
            org_cache_path: None,
            probe_freshness: false,
        }
    }
}

/// The `from-github` scanner cell — the org is cloned via the REST
/// API into `opts.clone_into` first, then walked exactly like a
/// local org-dir. The composition root owns the clone directory's
/// lifetime (a scratch temp dir or an operator-supplied warm cache);
/// the cell only fills and walks it.
#[cell(seam = "PackageScanner", variant = "from-github")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#reindex")]
pub struct FromGithubPackageScanner {
    /// API endpoint, org, auth, and clone destination for the fetch
    /// half; the walk half shares [`FromClonesOptions`] through the
    /// seam signature.
    pub opts: FromGithubOptions,
}

impl PackageScanner for FromGithubPackageScanner {
    fn scan(&self, walk: &FromClonesOptions, prior: Option<&Checkpoint>) -> Result<ScanReport> {
        // The fetch half is cache-aware: `resolve_repos` consults the
        // org-image cache and the host's conditional validator before
        // deciding whether to re-walk the API (Р2). The clone half
        // (`clone_repos_into`) is idempotent and reuses a warm cache.
        let (repos, org_cache_hit) = resolve_repos(&self.opts)?;
        let org_dir = clone_repos_into(&self.opts, &repos)?;
        let mut report = scan_org_dir_with_filter(&org_dir, walk, prior)?;
        report.org_cache_hit = org_cache_hit;
        Ok(report)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    pub clone_url: String,
    #[serde(default)]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub fork: bool,
}

/// Enumerate every (non-fork by default) repo in `<org>` via the
/// GitHub REST API. Follows `Link: rel="next"` until exhausted.
/// Unconditional — delegates to [`list_repos_conditional`] with no
/// validator (which never takes the 304 path), so this is exactly the
/// plain walk, not duplicated.
pub fn list_repos(opts: &FromGithubOptions) -> Result<Vec<Repo>> {
    match list_repos_conditional(opts, None)? {
        CondOutcome::Modified { repos, .. } => Ok(repos),
        // Probe is `None` ⇒ no validator is sent ⇒ the host cannot
        // answer 304; `Fresh` is unreachable here.
        CondOutcome::Fresh { .. } => {
            unreachable!("list_repos_conditional returned Fresh with no validator supplied")
        }
    }
}

/// `Link: <…>; rel="next", <…>; rel="last"` → the URL bound to `rel="next"`.
pub fn parse_next_link(link: &str) -> Option<String> {
    for part in link.split(',') {
        let part = part.trim();
        let (url_part, rel_part) = part.split_once(';')?;
        let url = url_part
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>');
        let rel = rel_part.trim();
        if rel == "rel=\"next\"" {
            return Some(url.to_string());
        }
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

/// List the org via the API, then `git clone` every repo into
/// `clone_into/<repo.name>`. Returns the path to `clone_into` so the
/// caller can pass it straight into `from_clones::scan_org_dir`.
/// Idempotent: directories that already exist are left alone, so a
/// warm cache survives across runs. This is the **unconditional**
/// list+clone entry point — it ignores the org-image cache. The
/// scanner cell uses [`resolve_repos`] + [`clone_repos_into`] instead
/// so it can short-circuit on a cache hit.
pub fn clone_org(opts: &FromGithubOptions) -> Result<PathBuf> {
    let repos = list_repos(opts)?;
    clone_repos_into(opts, &repos)
}

/// Clone every repo in `repos` into `clone_into/<repo.name>`.
/// Idempotent: directories that already exist are left alone, so a
/// warm cache survives across runs. Factored out of [`clone_org`] so
/// the cache-aware path can reuse the clone half with a repo list it
/// already holds — possibly served from the cache without re-listing.
pub fn clone_repos_into(opts: &FromGithubOptions, repos: &[Repo]) -> Result<PathBuf> {
    std::fs::create_dir_all(&opts.clone_into).map_err(|e| Error::Io {
        path: opts.clone_into.clone(),
        message: e.to_string(),
    })?;
    for repo in repos {
        let dest = opts.clone_into.join(&repo.name);
        if dest.exists() {
            continue;
        }
        let url = clone_url_with_token(&repo.clone_url, opts.token.as_deref());
        let dest_str = dest.to_str().ok_or_else(|| {
            Error::InvalidInput(format!("clone dest `{}` is not UTF-8", dest.display()))
        })?;
        let status = Command::new(git_cli::binary())
            .args(["clone", "--quiet"])
            .arg(&url)
            .arg(dest_str)
            .status()
            .map_err(|e| Error::Io {
                path: dest.clone(),
                message: format!("git clone: {e}"),
            })?;
        if !status.success() {
            return Err(Error::Malformed(format!(
                "git clone of `{}` failed",
                repo.clone_url
            )));
        }
    }
    Ok(opts.clone_into.clone())
}

/// Build the blocking HTTP client shared by the unconditional and
/// conditional enumerators (same user-agent + timeout).
fn build_client(opts: &FromGithubOptions) -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT_VAL)
        .timeout(opts.timeout)
        .build()
        .map_err(|e| Error::Malformed(format!("could not build HTTP client: {e}")))
}

/// Stamp the auth header on a request when a token is configured.
fn apply_auth(
    mut req: reqwest::blocking::RequestBuilder,
    opts: &FromGithubOptions,
) -> Result<reqwest::blocking::RequestBuilder> {
    if let Some(token) = &opts.token {
        req = req.header(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|e| {
                Error::InvalidInput(format!("token is not a valid header value: {e}"))
            })?,
        );
    }
    Ok(req)
}

/// Extract the conditional-request validator pair from a response's
/// headers. Either slot is `None` when the host did not supply it.
fn validator_from_headers(headers: &reqwest::header::HeaderMap) -> Validator {
    Validator {
        etag: headers
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned),
        last_modified: headers
            .get(LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned),
    }
}

/// Merge a validator read from a 304 response (usually empty — GitHub
/// does not re-emit `ETag` on a 304) into the one we stored: keep the
/// stored value when the host did not re-emit it.
fn refresh_validator(stored: Validator, refreshed: Validator) -> Validator {
    Validator {
        etag: refreshed.etag.or(stored.etag),
        last_modified: refreshed.last_modified.or(stored.last_modified),
    }
}

/// Outcome of a conditional org enumeration.
#[derive(Debug)]
pub enum CondOutcome {
    /// The host answered `304 Not Modified` against the stored
    /// validator — the cached repo list is still current. No body was
    /// transferred and (on GitHub) no rate-limit token was spent.
    /// `validator` is carried forward for the next run.
    Fresh { validator: Validator },
    /// The host answered `200` (or no validator was available to
    /// send) — a fresh body was read. The repo list and validator
    /// must be re-persisted.
    Modified {
        repos: Vec<Repo>,
        validator: Validator,
    },
}

/// Enumerate the org's repos, optionally conditionally. When
/// `validator` is present it is sent back as `If-None-Match` /
/// `If-Modified-Since` on the **first** page; a `304` there means the
/// org's first page — the cheap proxy for the whole org — is
/// unchanged, so the cached list is still good and no further pages
/// are walked. When `validator` is `None` (or carries neither header)
/// the request is unconditional and always yields
/// [`CondOutcome::Modified`] (ИЗМЕРЬ-2: no validator ⇒ cannot probe
/// ⇒ enumerate, never "consider fresh").
///
/// The validator stored is the **first page's**: a 304 on page 1 is
/// the standard "did anything change?" probe GitHub clients use. It is
/// not a hard guarantee — a change confined to a later page that
/// leaves page 1 byte-identical would be missed. That is exactly why
/// `rescan-org` exists (Р4): only a full traversal is certain, and
/// the cheap probe is a default optimisation, not a guarantee.
pub fn list_repos_conditional(
    opts: &FromGithubOptions,
    validator: Option<&Validator>,
) -> Result<CondOutcome> {
    let client = build_client(opts)?;
    let probe = validator.filter(|v| v.has_any()).cloned();

    let mut url = format!(
        "{}/orgs/{}/repos?per_page=100",
        opts.api_base.trim_end_matches('/'),
        opts.org
    );
    let mut out = Vec::new();
    // The validator captured from the canonical first page — what we
    // persist so the next run can probe.
    let mut captured = Validator::default();
    let mut first = true;
    loop {
        let mut req = client
            .get(&url)
            .header(USER_AGENT, USER_AGENT_VAL)
            .header(ACCEPT, "application/vnd.github+json");
        req = apply_auth(req, opts)?;
        if first && let Some(v) = &probe {
            if let Some(etag) = &v.etag {
                req = req.header(IF_NONE_MATCH, etag.as_str());
            }
            if let Some(lm) = &v.last_modified {
                req = req.header(IF_MODIFIED_SINCE, lm.as_str());
            }
        }
        let resp = req
            .send()
            .map_err(|e| Error::Malformed(format!("GitHub API: {e}")))?;
        let status = resp.status();
        // Read the headers we need (Link for pagination, validator
        // for persistence / probing) as owned values before the body
        // is consumed.
        let (next, header_validator) = {
            let h = resp.headers();
            let next = h
                .get(LINK)
                .and_then(|hv| hv.to_str().ok())
                .and_then(parse_next_link);
            (next, validator_from_headers(h))
        };
        // 304 only matters on the conditional first page, and only when
        // we actually sent a validator (a probe we did not send cannot
        // be answered 304).
        if first
            && status.as_u16() == 304
            && let Some(p) = probe.as_ref()
        {
            return Ok(CondOutcome::Fresh {
                validator: refresh_validator(p.clone(), header_validator),
            });
        }
        if !status.is_success() {
            let hint = if status.as_u16() == 401 || status.as_u16() == 403 {
                " (auth failure or rate limit — pass --token-file with a PAT to raise the limit)"
            } else {
                ""
            };
            let body = resp.text().unwrap_or_default();
            return Err(Error::Malformed(format!(
                "GitHub API returned {status} for `{url}`{hint}: {}",
                truncate(&body, 256)
            )));
        }
        if first {
            captured = header_validator;
        }
        let page: Vec<Repo> = resp
            .json()
            .map_err(|e| Error::Malformed(format!("GitHub API JSON: {e}")))?;
        out.extend(page.into_iter().filter(|r| !(opts.skip_forks && r.fork)));
        first = false;
        match next {
            Some(n) => url = n,
            None => break,
        }
    }
    Ok(CondOutcome::Modified {
        repos: out,
        validator: captured,
    })
}

/// Resolve the org's repo list for cloning, consulting the org-image
/// cache when the cell is configured for it (Р2, Р3). Returns the
/// repos to clone plus the cache outcome for visibility (Р5):
/// - `Some(true)` — served from a fresh cache (304 hit);
/// - `Some(false)` — re-enumerated (200, or no usable cache, or
///   `rescan-org` skipping the probe) and the image re-persisted;
/// - `None` — caching is off (`--no-cache-org`): enumerated without
///   any read or write, indistinguishable from pre-A3 (Р6).
pub fn resolve_repos(opts: &FromGithubOptions) -> Result<(Vec<Repo>, Option<bool>)> {
    let Some(cache_path) = opts.org_cache_path.as_deref() else {
        // Caching entirely off — plain enumeration, no image persisted.
        let repos = list_repos(opts)?;
        return Ok((repos, None));
    };
    let cached = org_cache::load(cache_path)?;
    // ПРОВЕРЬ-4 — never serve an image taken for another org/endpoint.
    let usable = cached
        .as_ref()
        .is_some_and(|c| c.matches(&opts.org, &opts.api_base));
    let prior = if opts.probe_freshness && usable {
        cached.as_ref().and_then(OrgCache::validator)
    } else {
        None
    };
    match list_repos_conditional(opts, prior.as_ref())? {
        CondOutcome::Fresh { .. } => {
            // 304 — the cached repos are still current. Nothing
            // changed, so the image is not rewritten.
            //
            // A `Fresh` outcome is only reachable when a validator was
            // sent, and a validator only exists when the image is
            // usable — so the image is present by construction. It is
            // still resolved fallibly rather than unwrapped: the
            // invariant lives three bindings away, and the case it
            // rules out is one a MISBEHAVING HOST can produce (a 304
            // answering a request that carried no validator). Trusting
            // the far end to respect the protocol is not an invariant,
            // it is an assumption, and this one is cheap to check.
            let image = cached.ok_or_else(|| {
                Error::Malformed(
                    "host answered 304 Not Modified to a request that carried no validator \
                     — there is no cached organisation image to serve; re-run with \
                     `--no-cache-org` or `rescan-org` to enumerate unconditionally"
                        .to_string(),
                )
            })?;
            Ok((image.repos, Some(true)))
        }
        CondOutcome::Modified { repos, validator } => {
            // Р4 — always refresh the image after an unconditional or
            // modified enumeration, even under `rescan-org` (which
            // arrives here with `prior = None`).
            let image = OrgCache {
                schema_version: 1,
                org: opts.org.clone(),
                api_base: opts.api_base.clone(),
                etag: validator.etag,
                last_modified: validator.last_modified,
                repos: repos.clone(),
            };
            org_cache::save(cache_path, &image)?;
            Ok((repos, Some(false)))
        }
    }
}

/// Inject a GitHub PAT into a `https://github.com/...` URL for the
/// duration of a single `git clone`. Modern git (≥ 2.31) redacts URL
/// passwords in its own log output, so the credentialised URL is
/// safe to pass on the command line; vibevm itself MUST NEVER print
/// it to stdout / stderr / JSON / log lines per [PROP-000 §20].
pub fn clone_url_with_token(url: &str, token: Option<&str>) -> String {
    let Some(token) = token else {
        return url.to_string();
    };
    if let Some(rest) = url.strip_prefix("https://") {
        return format!("https://x-access-token:{token}@{rest}");
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_next_link_with_next_present() {
        let link = r#"<https://api.github.com/orgs/foo/repos?page=2>; rel="next", <https://api.github.com/orgs/foo/repos?page=5>; rel="last""#;
        assert_eq!(
            parse_next_link(link),
            Some("https://api.github.com/orgs/foo/repos?page=2".to_string())
        );
    }

    #[test]
    fn parse_next_link_without_next() {
        let link = r#"<https://api.github.com/orgs/foo/repos?page=1>; rel="prev", <https://api.github.com/orgs/foo/repos?page=1>; rel="first""#;
        assert_eq!(parse_next_link(link), None);
    }

    #[test]
    fn parse_next_link_handles_extra_whitespace() {
        let link = r#" <https://api.github.com/orgs/foo/repos?page=2>;rel="next" "#;
        assert_eq!(
            parse_next_link(link),
            Some("https://api.github.com/orgs/foo/repos?page=2".to_string())
        );
    }

    #[test]
    fn clone_url_with_token_injects_credentials_for_https() {
        assert_eq!(
            clone_url_with_token("https://github.com/foo/bar.git", Some("ghp_abc")),
            "https://x-access-token:ghp_abc@github.com/foo/bar.git"
        );
    }

    #[test]
    fn clone_url_with_token_passes_through_when_no_token() {
        assert_eq!(
            clone_url_with_token("https://github.com/foo/bar.git", None),
            "https://github.com/foo/bar.git"
        );
    }

    #[test]
    fn clone_url_with_token_passes_through_for_non_https() {
        assert_eq!(
            clone_url_with_token("git@github.com:foo/bar.git", Some("token")),
            "git@github.com:foo/bar.git"
        );
        assert_eq!(
            clone_url_with_token("/local/path", Some("token")),
            "/local/path"
        );
    }
}
