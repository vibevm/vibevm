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

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#reindex");

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderValue, LINK, USER_AGENT};
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::scanner::git_cli;

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
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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
pub fn list_repos(opts: &FromGithubOptions) -> Result<Vec<Repo>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT_VAL)
        .timeout(opts.timeout)
        .build()
        .map_err(|e| Error::Malformed(format!("could not build HTTP client: {e}")))?;

    let mut url = format!(
        "{}/orgs/{}/repos?per_page=100",
        opts.api_base.trim_end_matches('/'),
        opts.org
    );
    let mut out = Vec::new();
    loop {
        let mut req = client
            .get(&url)
            .header(USER_AGENT, USER_AGENT_VAL)
            .header(ACCEPT, "application/vnd.github+json");
        if let Some(token) = &opts.token {
            req = req.header(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}")).map_err(|e| {
                    Error::InvalidInput(format!("token is not a valid header value: {e}"))
                })?,
            );
        }
        let resp = req
            .send()
            .map_err(|e| Error::Malformed(format!("GitHub API: {e}")))?;
        let status = resp.status();
        let next = resp
            .headers()
            .get(LINK)
            .and_then(|h| h.to_str().ok())
            .and_then(parse_next_link);
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
        let page: Vec<Repo> = resp
            .json()
            .map_err(|e| Error::Malformed(format!("GitHub API JSON: {e}")))?;
        out.extend(page.into_iter().filter(|r| !(opts.skip_forks && r.fork)));
        match next {
            Some(n) => url = n,
            None => break,
        }
    }
    Ok(out)
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
/// warm cache survives across runs.
pub fn clone_org(opts: &FromGithubOptions) -> Result<PathBuf> {
    let repos = list_repos(opts)?;
    std::fs::create_dir_all(&opts.clone_into).map_err(|e| Error::Io {
        path: opts.clone_into.clone(),
        message: e.to_string(),
    })?;
    for repo in &repos {
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
