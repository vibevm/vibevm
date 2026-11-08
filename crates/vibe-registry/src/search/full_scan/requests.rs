//! GitHub pagination and manifest GETs share the retained scan client.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");
use super::client::scan_http_error;
use super::*;
use serde::Deserialize;
#[derive(Debug, Deserialize, Clone)]
pub(super) struct RepoEntry {
    pub(super) name: String,
    #[serde(default)]
    pub(super) fork: bool,
    #[serde(default)]
    pub(super) archived: bool,
}

pub(super) fn list_org_repos(
    client: &ScanClient,
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
        if let Some(t) = token.filter(|_| client.permits_token(&url)) {
            req = req.bearer_auth(t);
        }
        let resp = client.send(req, &url)?;
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
        let bytes = resp.bytes().map_err(|e| scan_http_error(&url, e))?;
        let page_repos: Vec<RepoEntry> =
            serde_json::from_slice(&bytes).map_err(|e| FullScanError::Malformed {
                url: url.clone(),
                message: e.to_string(),
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

#[derive(Debug, Deserialize)]
struct ContentsResponse {
    encoding: String,
    content: String,
}

pub(super) fn fetch_package_manifest(
    client: &ScanClient,
    api_base: &str,
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> std::result::Result<Option<Manifest>, FullScanError> {
    let url = format!(
        "{}/repos/{}/{}/contents/vibe.toml",
        api_base.trim_end_matches('/'),
        owner,
        repo
    );
    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(t) = token.filter(|_| client.permits_token(&url)) {
        req = req.bearer_auth(t);
    }
    let resp = client.send(req, &url)?;
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
    let body: ContentsResponse = resp
        .json::<ContentsResponse>()
        .map_err(|e| scan_http_error(&url, e))?;
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
    let manifest = Manifest::parse_str(toml_str).map_err(|e| FullScanError::Malformed {
        url: url.clone(),
        message: format!("toml parse: {e}"),
    })?;
    Ok(Some(manifest))
}

/// Tiny standard-base64 decoder. Avoids pulling the `base64` crate
/// for one call site. Pads `=`-trailing input as expected; rejects
/// any non-base64 character.
pub(super) fn decode_base64(input: &str) -> std::result::Result<Vec<u8>, &'static str> {
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
