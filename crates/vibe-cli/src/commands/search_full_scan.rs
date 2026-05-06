//! `vibe search --full-scan` fallback for GitHub-hosted registries
//! that do not run a `vibe-index` server.
//!
//! Walks `GET /orgs/{org}/repos` (Link-header pagination, same shape
//! as `services/vibe-index::scanner::from_github`), fetches each
//! repo's `vibe-package.toml` via the Contents API, parses the
//! manifest with `vibe_core::manifest::PackageManifest`, then runs a
//! lightweight token-based match against `name`, `description`,
//! `keywords`, and `provides.capabilities`. Score is the number of
//! distinct query tokens that hit any of those fields — same idea as
//! the index-server's term-overlap scoring, just on whatever the
//! org happens to surface today.
//!
//! Spec: [ROADMAP §M2.10](../../../../ROADMAP.md). The full-scan
//! mode is the explicit "naive at first" path the roadmap promised
//! to ship before per-org indexes proliferated; it stays available
//! after PROP-005 so operators on orgs without an index get useful
//! discovery without running their own infrastructure first.
//!
//! Token discipline ([PROP-000 §20](../../../../spec/common/PROP-000.md#token-secrecy)):
//! authentication is via `vibe_publish::token::load_token_for_host("github.com")`
//! when available — same precedence chain as `vibe registry publish`.
//! Anonymous calls work but the GitHub rate limit drops to 60 req/h.
//! The token bytes never enter logs, errors, or the JSON envelope.

use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;
use vibe_core::PackageKind;
use vibe_core::manifest::PackageManifest;

const REQUEST_TIMEOUT_SECS: u64 = 15;
const PER_PAGE: u32 = 100;
/// Hard cap on repos scanned per registry per invocation. Beyond this
/// the cost-per-search trends toward a small `git ls-remote` pass for
/// every package in a 200+-repo org, which is exactly the antipattern
/// the index-server path was built to avoid. Set high enough that
/// a single-digit-org case lands fine; operators with bigger orgs
/// should run an index.
const MAX_REPOS_PER_SCAN: usize = 500;

#[derive(Debug, Error)]
#[allow(dead_code)] // NotGitHub / NoOrgInUrl reserved for future host detection paths
pub enum FullScanError {
    #[error("registry URL `{url}` does not point at a GitHub organisation")]
    NotGitHub { url: String },
    #[error("could not extract org segment from `{url}`")]
    NoOrgInUrl { url: String },
    #[error("HTTP request to `{url}` failed: {message}")]
    Http { url: String, message: String },
    #[error("GitHub API returned status {status} on `{url}`")]
    Status { url: String, status: u16 },
    #[error("response body from `{url}` could not be parsed: {message}")]
    Malformed { url: String, message: String },
}

#[derive(Debug, Clone)]
pub struct FullScanHit {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,
    pub description: Option<String>,
    pub score: u32,
    pub matched_tokens: Vec<String>,
}

/// Extract `(api_base, org)` from a registry URL pointing at a GitHub
/// organisation root. The `api_base` is the GitHub REST API root —
/// configurable via `VIBEVM_GITHUB_API_BASE` for testing; defaults to
/// `https://api.github.com`. Returns `None` for non-GitHub URLs so
/// the caller can fall through to the registries-unsupported bucket
/// without raising an error.
pub fn detect_github_org(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    if parsed.host_str() != Some("github.com") {
        return None;
    }
    let mut segments = parsed.path_segments()?;
    let org = segments.next()?.trim();
    if org.is_empty() {
        return None;
    }
    Some(org.to_string())
}

/// Run the full-scan against a GitHub org. Pulls every public repo,
/// fetches each `vibe-package.toml`, scores against the supplied
/// query tokens. Returns a flat list of hits sorted by score (desc)
/// then `(kind, name)` lex.
pub fn full_scan_github_org(
    org: &str,
    api_base: &str,
    token: Option<&str>,
    query_tokens: &[String],
    kind_filter: Option<PackageKind>,
) -> std::result::Result<Vec<FullScanHit>, FullScanError> {
    if query_tokens.is_empty() {
        return Ok(Vec::new());
    }
    let client = build_client()?;
    let repos = list_org_repos(&client, api_base, org, token)?;
    let mut hits: Vec<FullScanHit> = Vec::new();
    for repo in repos.into_iter().take(MAX_REPOS_PER_SCAN) {
        if repo.fork || repo.archived {
            continue;
        }
        let manifest = match fetch_package_manifest(&client, api_base, org, &repo.name, token) {
            Ok(Some(m)) => m,
            Ok(None) => continue,            // not a vibevm package — skip
            Err(FullScanError::Status { status: 404, .. }) => continue,
            Err(FullScanError::Status { status, url }) if status == 403 || status == 429 => {
                // Rate-limited — surface upward so the caller can
                // attribute the failure to this registry instead of
                // silently truncating results.
                return Err(FullScanError::Status { status, url });
            }
            Err(_) => continue,             // transient — skip this repo
        };
        if let Some(k) = kind_filter
            && manifest.package.kind != k
        {
            continue;
        }
        let (score, matched) = score_manifest(&manifest, query_tokens);
        if score == 0 {
            continue;
        }
        hits.push(FullScanHit {
            kind: manifest.package.kind,
            name: manifest.package.name.clone(),
            version: manifest.package.version.clone(),
            description: manifest.package.description.clone(),
            score,
            matched_tokens: matched,
        });
    }
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.kind.as_str().cmp(b.kind.as_str()))
            .then(a.name.cmp(&b.name))
    });
    Ok(hits)
}

fn build_client() -> std::result::Result<reqwest::blocking::Client, FullScanError> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("vibe-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| FullScanError::Http {
            url: "<client-build>".into(),
            message: e.to_string(),
        })
}

#[derive(Debug, Deserialize, Clone)]
struct RepoEntry {
    name: String,
    #[serde(default)]
    fork: bool,
    #[serde(default)]
    archived: bool,
}

fn list_org_repos(
    client: &reqwest::blocking::Client,
    api_base: &str,
    org: &str,
    token: Option<&str>,
) -> std::result::Result<Vec<RepoEntry>, FullScanError> {
    let mut page = 1u32;
    let mut all = Vec::new();
    loop {
        let url = format!(
            "{}/orgs/{}/repos?per_page={}&page={}",
            api_base.trim_end_matches('/'),
            org,
            PER_PAGE,
            page
        );
        let mut req = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        let resp = req.send().map_err(|e| FullScanError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(FullScanError::Status {
                url,
                status: status.as_u16(),
            });
        }
        let link_header = resp
            .headers()
            .get("link")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let bytes = resp.bytes().map_err(|e| FullScanError::Http {
            url: url.clone(),
            message: e.to_string(),
        })?;
        let page_repos: Vec<RepoEntry> = serde_json::from_slice(&bytes).map_err(|e| {
            FullScanError::Malformed {
                url: url.clone(),
                message: e.to_string(),
            }
        })?;
        all.extend(page_repos);
        if all.len() >= MAX_REPOS_PER_SCAN {
            break;
        }
        match link_header.as_deref().and_then(parse_next_link) {
            Some(_) => {
                page += 1;
            }
            None => break,
        }
    }
    Ok(all)
}

/// Lift the `next` URL out of a GitHub `Link:` header. Returns `None`
/// when there is no `rel="next"` entry.
pub fn parse_next_link(link_header: &str) -> Option<String> {
    for part in link_header.split(',') {
        let trimmed = part.trim();
        if !trimmed.contains("rel=\"next\"") {
            continue;
        }
        let lt = trimmed.find('<')?;
        let gt = trimmed[lt + 1..].find('>')? + lt + 1;
        return Some(trimmed[lt + 1..gt].to_string());
    }
    None
}

#[derive(Debug, Deserialize)]
struct ContentsResponse {
    encoding: String,
    content: String,
}

fn fetch_package_manifest(
    client: &reqwest::blocking::Client,
    api_base: &str,
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> std::result::Result<Option<PackageManifest>, FullScanError> {
    let url = format!(
        "{}/repos/{}/{}/contents/vibe-package.toml",
        api_base.trim_end_matches('/'),
        owner,
        repo
    );
    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().map_err(|e| FullScanError::Http {
        url: url.clone(),
        message: e.to_string(),
    })?;
    let status = resp.status();
    if status.as_u16() == 404 {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(FullScanError::Status {
            url,
            status: status.as_u16(),
        });
    }
    let body: ContentsResponse =
        resp.json::<ContentsResponse>().map_err(|e| FullScanError::Malformed {
            url: url.clone(),
            message: e.to_string(),
        })?;
    if body.encoding != "base64" {
        return Err(FullScanError::Malformed {
            url,
            message: format!("expected base64-encoded body, got `{}`", body.encoding),
        });
    }
    let cleaned: String = body
        .content
        .chars()
        .filter(|c: &char| !c.is_whitespace())
        .collect();
    let bytes = decode_base64(&cleaned).map_err(|e| FullScanError::Malformed {
        url: url.clone(),
        message: format!("base64 decode: {e}"),
    })?;
    let toml_str = std::str::from_utf8(&bytes).map_err(|e| FullScanError::Malformed {
        url: url.clone(),
        message: format!("utf-8 decode: {e}"),
    })?;
    let manifest: PackageManifest = toml::from_str(toml_str).map_err(|e| {
        FullScanError::Malformed {
            url: url.clone(),
            message: format!("toml parse: {e}"),
        }
    })?;
    Ok(Some(manifest))
}

/// Tiny standard-base64 decoder. Avoids pulling the `base64` crate
/// for one call site. Pads `=`-trailing input as expected; rejects
/// any non-base64 character.
fn decode_base64(input: &str) -> std::result::Result<Vec<u8>, &'static str> {
    fn val(c: u8) -> Result<u32, &'static str> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err("invalid character"),
        }
    }
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err("length not multiple of 4");
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut i = 0;
    while i < bytes.len() {
        let q = &bytes[i..i + 4];
        let pad0 = q[2] == b'=';
        let pad1 = q[3] == b'=';
        let v0 = val(q[0])?;
        let v1 = val(q[1])?;
        let v2 = if pad0 { 0 } else { val(q[2])? };
        let v3 = if pad1 { 0 } else { val(q[3])? };
        let n = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        out.push((n >> 16) as u8);
        if !pad0 {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if !pad1 {
            out.push((n & 0xff) as u8);
        }
        i += 4;
    }
    Ok(out)
}

/// Score a manifest against a list of (already-lowercase) query
/// tokens. Each distinct token that hits any of `name`,
/// `description`, `keywords`, or `provides.capabilities` adds one to
/// the score. Returns the score plus the actual tokens matched.
fn score_manifest(manifest: &PackageManifest, query_tokens: &[String]) -> (u32, Vec<String>) {
    let mut haystack = String::new();
    haystack.push_str(&manifest.package.name);
    haystack.push(' ');
    if let Some(d) = &manifest.package.description {
        haystack.push_str(d);
        haystack.push(' ');
    }
    for kw in &manifest.package.keywords {
        haystack.push_str(kw);
        haystack.push(' ');
    }
    for cap in &manifest.provides.capabilities {
        haystack.push_str(&cap.to_string());
        haystack.push(' ');
    }
    let lc = haystack.to_lowercase();
    let mut score = 0u32;
    let mut matched = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for tok in query_tokens {
        if !seen.insert(tok.clone()) {
            continue;
        }
        if lc.contains(tok.as_str()) {
            score += 1;
            matched.push(tok.clone());
        }
    }
    (score, matched)
}

/// Tokenise a free-text query the same way the server does:
/// lowercase ASCII alphanumeric runs, drop tokens shorter than 2
/// characters, drop trivial English stopwords. Kept inline to avoid
/// pulling the server-side index crate.
pub fn tokenise_query(query: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is",
        "it", "its", "of", "on", "or", "she", "that", "the", "this", "to", "was", "were", "with",
        "you", "your",
    ];
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in query.chars() {
        if c.is_ascii_alphanumeric() {
            buf.push(c.to_ascii_lowercase());
        } else if !buf.is_empty() {
            push_if_keepable(&mut out, std::mem::take(&mut buf), STOPWORDS);
        }
    }
    if !buf.is_empty() {
        push_if_keepable(&mut out, buf, STOPWORDS);
    }
    out
}

fn push_if_keepable(out: &mut Vec<String>, tok: String, stopwords: &[&str]) {
    if tok.len() < 2 {
        return;
    }
    if stopwords.contains(&tok.as_str()) {
        return;
    }
    out.push(tok);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_github_org_extracts_first_path_segment() {
        assert_eq!(
            detect_github_org("https://github.com/vibespecs"),
            Some("vibespecs".into())
        );
        assert_eq!(
            detect_github_org("https://github.com/vibespecs/"),
            Some("vibespecs".into())
        );
        assert_eq!(
            detect_github_org("https://github.com/vibespecs/extra/path"),
            Some("vibespecs".into())
        );
    }

    #[test]
    fn detect_github_org_returns_none_for_non_github_hosts() {
        assert!(detect_github_org("https://gitverse.ru/anarchic").is_none());
        assert!(detect_github_org("https://gitlab.com/foo").is_none());
    }

    #[test]
    fn detect_github_org_returns_none_when_org_segment_is_missing() {
        assert!(detect_github_org("https://github.com").is_none());
        assert!(detect_github_org("https://github.com/").is_none());
    }

    #[test]
    fn parse_next_link_extracts_next_url() {
        let header = r#"<https://api.github.com/orgs/x/repos?page=2>; rel="next", <https://api.github.com/orgs/x/repos?page=5>; rel="last""#;
        assert_eq!(
            parse_next_link(header).as_deref(),
            Some("https://api.github.com/orgs/x/repos?page=2")
        );
    }

    #[test]
    fn parse_next_link_returns_none_when_no_next_rel() {
        let header = r#"<https://api.github.com/orgs/x/repos?page=5>; rel="last""#;
        assert!(parse_next_link(header).is_none());
    }

    #[test]
    fn decode_base64_roundtrips_simple_input() {
        let body = b"hello world";
        // standard base64 of "hello world" is "aGVsbG8gd29ybGQ="
        let encoded = "aGVsbG8gd29ybGQ=";
        let decoded = decode_base64(encoded).unwrap();
        assert_eq!(decoded, body);
    }

    #[test]
    fn decode_base64_handles_double_pad() {
        let body = b"any";
        let encoded = "YW55"; // 3 bytes => no pad
        assert_eq!(decode_base64(encoded).unwrap(), body);
        let body2 = b"a";
        let encoded2 = "YQ=="; // 1 byte => double pad
        assert_eq!(decode_base64(encoded2).unwrap(), body2);
    }

    /// Manual encoder mirrors the integration-test fixture so that a
    /// regression in either side surfaces locally instead of in
    /// tests/cli_search.rs.
    fn encode_base64_for_test(input: &[u8]) -> String {
        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
        let mut i = 0;
        while i < input.len() {
            let b0 = input[i];
            let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
            let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };
            let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
            out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
            if i + 1 < input.len() {
                out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
            if i + 2 < input.len() {
                out.push(TABLE[(n & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
            i += 3;
        }
        out
    }

    #[test]
    fn decode_base64_round_trips_arbitrary_byte_ranges() {
        for body in [
            b"".to_vec(),
            b"a".to_vec(),
            b"hi".to_vec(),
            b"abc".to_vec(),
            b"hello world".to_vec(),
            b"vibe-package.toml".to_vec(),
            (0..=255u8).collect::<Vec<u8>>(),
        ] {
            let encoded = encode_base64_for_test(&body);
            let decoded = decode_base64(&encoded).unwrap();
            assert_eq!(decoded, body, "round-trip failed for {body:?}");
        }
    }

    #[test]
    fn tokenise_query_drops_stopwords_and_short_tokens() {
        let toks = tokenise_query("the WAL log discipline");
        assert!(!toks.contains(&"the".into()));
        assert!(toks.contains(&"wal".into()));
        assert!(toks.contains(&"log".into()));
        assert!(toks.contains(&"discipline".into()));
    }
}
