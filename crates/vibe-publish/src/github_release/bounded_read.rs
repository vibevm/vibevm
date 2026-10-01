//! Bounded, no-redirect authenticated GitHub JSON reads for later owner
//! observation. These are independent metadata reads, never an atomic CAS or
//! a publication effect grant.

use std::collections::BTreeSet;
use std::io::Read;

use reqwest::header::{ACCEPT, AUTHORIZATION, LINK, USER_AGENT};
use reqwest::{Method, StatusCode, Url};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::{
    GITHUB_API_VERSION, GITHUB_JSON, GithubGitRef, GithubRelease, GithubReleaseAsset,
    GithubReleaseClient, user_agent, validate_asset_name, validate_release_tag,
};

const TAG_JSON_CAP: u64 = 64 * 1024;
const PAGE_JSON_CAP: u64 = 1024 * 1024;
const MAX_PAGES: usize = 32;
const PAGE_ITEMS: usize = 100;
const TOTAL_ITEMS: usize = MAX_PAGES * PAGE_ITEMS;
const LINK_CAP: usize = 4096;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum BoundedReadError {
    #[error("bounded GitHub observation refused before GET: {0}")]
    Refused(&'static str),
    #[error("bounded GitHub observation is uncertain: {0}")]
    Uncertain(&'static str),
}

struct Page<T> {
    value: Option<T>,
    next: Option<String>,
}

/// The legacy model defaults absent `draft` to false; an owner observation
/// requires GitHub to state that field explicitly.
#[derive(Deserialize)]
struct StrictRelease {
    id: u64,
    tag_name: String,
    draft: bool,
    #[serde(default)]
    target_commitish: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    upload_url: String,
}

impl From<StrictRelease> for GithubRelease {
    fn from(row: StrictRelease) -> Self {
        Self {
            id: row.id,
            tag_name: row.tag_name,
            draft: row.draft,
            target_commitish: row.target_commitish,
            name: row.name,
            body: row.body,
            prerelease: row.prerelease,
            html_url: row.html_url,
            upload_url: row.upload_url,
        }
    }
}

fn parse_next(header: &str) -> Result<Option<String>, BoundedReadError> {
    if header.len() > LINK_CAP {
        return Err(BoundedReadError::Uncertain(
            "pagination Link header exceeds bound",
        ));
    }
    let mut next = None;
    for part in header.split(',') {
        let (url, params) = part
            .trim()
            .split_once('>')
            .ok_or(BoundedReadError::Uncertain(
                "malformed pagination Link header",
            ))?;
        let url = url
            .strip_prefix('<')
            .ok_or(BoundedReadError::Uncertain("malformed pagination Link URL"))?;
        let mut relation = None;
        for param in params.split(';').map(str::trim) {
            if let Some(raw) = param.strip_prefix("rel=") {
                let parsed = raw
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                    .ok_or(BoundedReadError::Uncertain("malformed pagination relation"))?;
                if relation.replace(parsed).is_some() {
                    return Err(BoundedReadError::Uncertain("repeated pagination relation"));
                }
            }
        }
        let relation = relation.ok_or(BoundedReadError::Uncertain("pagination relation absent"))?;
        match relation {
            "next" => {
                if next.replace(url.to_owned()).is_some() {
                    return Err(BoundedReadError::Uncertain("multiple next-page links"));
                }
            }
            "prev" | "first" | "last" => {}
            _ => {
                return Err(BoundedReadError::Uncertain(
                    "unsupported pagination relation",
                ));
            }
        }
    }
    Ok(next)
}

fn next_page(current: &Url, initial: &Url, raw: &str) -> Result<Url, BoundedReadError> {
    fn canonical_page(url: &Url) -> Result<(u64, bool), BoundedReadError> {
        let mut per_page = false;
        let mut page = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "per_page" if !per_page && value == "100" => per_page = true,
                "page" if page.is_none() => {
                    let parsed = value
                        .parse::<u64>()
                        .map_err(|_| BoundedReadError::Uncertain("invalid page number"))?;
                    if parsed == 0 || parsed.to_string() != value {
                        return Err(BoundedReadError::Uncertain("noncanonical page number"));
                    }
                    page = Some(parsed);
                }
                _ => {
                    return Err(BoundedReadError::Uncertain(
                        "extra or repeated pagination parameter",
                    ));
                }
            }
        }
        if !per_page {
            return Err(BoundedReadError::Uncertain("per_page=100 is absent"));
        }
        Ok((page.unwrap_or(1), page.is_some()))
    }
    let next = Url::parse(raw).map_err(|_| BoundedReadError::Uncertain("invalid next-page URL"))?;
    if next.scheme() != initial.scheme()
        || next.host_str() != initial.host_str()
        || next.port_or_known_default() != initial.port_or_known_default()
        || next.path() != initial.path()
        || !next.username().is_empty()
        || next.password().is_some()
        || next.fragment().is_some()
        || next == *current
    {
        return Err(BoundedReadError::Uncertain(
            "next page escaped exact endpoint or looped",
        ));
    }
    let (current_page, _) = canonical_page(current)?;
    let (next_number, explicit) = canonical_page(&next)?;
    if !explicit || current_page.checked_add(1) != Some(next_number) {
        return Err(BoundedReadError::Uncertain(
            "next page skipped or repeated an index",
        ));
    }
    Ok(next)
}

impl GithubReleaseClient {
    fn bounded_json_get<T: DeserializeOwned>(
        &self,
        url: Url,
        cap: u64,
    ) -> Result<Page<T>, BoundedReadError> {
        let token = self
            .token
            .as_ref()
            .ok_or(BoundedReadError::Refused("publish token absent"))?;
        // Unlike the legacy read client, this client never follows a 3xx to
        // another host; Authorization is sent only to the exact API URL.
        let request = self
            .no_redirect_write_client
            .request(Method::GET, url)
            .header(ACCEPT, GITHUB_JSON)
            .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
            .header(USER_AGENT, user_agent())
            .header(AUTHORIZATION, format!("Bearer {}", token.value()));
        let mut response = request
            .send()
            .map_err(|_| BoundedReadError::Uncertain("GET transport failed"))?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Page {
                value: None,
                next: None,
            });
        }
        if !response.status().is_success() {
            return Err(BoundedReadError::Uncertain(
                "GET returned non-success status or redirect",
            ));
        }
        let next = response
            .headers()
            .get(LINK)
            .map(|header| {
                header
                    .to_str()
                    .map_err(|_| BoundedReadError::Uncertain("invalid Link header"))
            })
            .transpose()?
            .map(parse_next)
            .transpose()?
            .flatten();
        if response
            .content_length()
            .is_some_and(|declared| declared > cap)
        {
            return Err(BoundedReadError::Uncertain(
                "GET declared JSON size exceeds cap",
            ));
        }
        let mut bytes =
            Vec::with_capacity(usize::try_from(cap.min(64 * 1024)).unwrap_or(64 * 1024));
        response
            .take(cap + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| BoundedReadError::Uncertain("GET JSON body failed"))?;
        if bytes.len() as u64 > cap {
            return Err(BoundedReadError::Uncertain("GET JSON body exceeds cap"));
        }
        let value = serde_json::from_slice(&bytes)
            .map_err(|_| BoundedReadError::Uncertain("GET JSON body malformed"))?;
        Ok(Page {
            value: Some(value),
            next,
        })
    }

    /// 404 is an exact absent tag observation. All other read failures are
    /// uncertain, including 3xx (which is never followed with the token).
    pub(crate) fn read_tag_ref_bounded(
        &self,
        tag: &str,
    ) -> Result<Option<GithubGitRef>, BoundedReadError> {
        validate_release_tag(tag, self.token_value())
            .map_err(|_| BoundedReadError::Refused("invalid release tag"))?;
        let url = self
            .api_url(&["git", "ref", "tags", tag])
            .map_err(|_| BoundedReadError::Refused("invalid tag endpoint"))?;
        let page: Page<GithubGitRef> = self.bounded_json_get(url, TAG_JSON_CAP)?;
        let Some(reference) = page.value else {
            return Ok(None);
        };
        if page.next.is_some()
            || reference.reference != format!("refs/tags/{tag}")
            || reference.object.kind != "commit"
            || reference.object.sha.len() != 40
            || !reference
                .object
                .sha
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(BoundedReadError::Uncertain("tag ref identity differs"));
        }
        Ok(Some(reference))
    }

    /// The tag endpoint may 404 for a draft. Only an exhausted authenticated
    /// release list can establish absence of that draft-aware lookup.
    pub(crate) fn read_release_by_tag_bounded(
        &self,
        tag: &str,
    ) -> Result<Option<GithubRelease>, BoundedReadError> {
        validate_release_tag(tag, self.token_value())
            .map_err(|_| BoundedReadError::Refused("invalid release tag"))?;
        let direct = self
            .api_url(&["releases", "tags", tag])
            .map_err(|_| BoundedReadError::Refused("invalid release endpoint"))?;
        let page: Page<StrictRelease> = self.bounded_json_get(direct, PAGE_JSON_CAP)?;
        if let Some(release) = page.value {
            let release: GithubRelease = release.into();
            if release.id == 0 || release.tag_name != tag || page.next.is_some() {
                return Err(BoundedReadError::Uncertain(
                    "direct release identity differs",
                ));
            }
            return Ok(Some(release));
        }
        let mut url = self
            .api_url(&["releases"])
            .map_err(|_| BoundedReadError::Refused("invalid release list endpoint"))?;
        url.query_pairs_mut().append_pair("per_page", "100");
        let initial = url.clone();
        let mut seen = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut total = 0;
        let mut found = None;
        for _ in 0..MAX_PAGES {
            if !seen.insert(url.as_str().to_owned()) {
                return Err(BoundedReadError::Uncertain("release pagination loop"));
            }
            let page: Page<Vec<StrictRelease>> =
                self.bounded_json_get(url.clone(), PAGE_JSON_CAP)?;
            let Some(rows) = page.value else {
                return Err(BoundedReadError::Uncertain("release page was absent"));
            };
            if rows.len() > PAGE_ITEMS || total + rows.len() > TOTAL_ITEMS {
                return Err(BoundedReadError::Uncertain(
                    "release list exceeds item bound",
                ));
            }
            total += rows.len();
            for release in rows {
                let release: GithubRelease = release.into();
                if release.id == 0 || release.tag_name.is_empty() || !ids.insert(release.id) {
                    return Err(BoundedReadError::Uncertain(
                        "release list has invalid or repeated ID",
                    ));
                }
                if release.tag_name == tag && found.replace(release).is_some() {
                    return Err(BoundedReadError::Uncertain(
                        "release list repeats selected tag",
                    ));
                }
            }
            let Some(raw) = page.next else {
                return Ok(found);
            };
            url = next_page(&url, &initial, &raw)?;
        }
        Err(BoundedReadError::Uncertain(
            "release pagination exceeds page bound",
        ))
    }

    /// Complete bounded list of one exact release ID, sorted for deterministic
    /// downstream comparison. A 404 is uncertain, not an empty inventory.
    pub(crate) fn read_all_assets_bounded(
        &self,
        release_id: u64,
    ) -> Result<Vec<GithubReleaseAsset>, BoundedReadError> {
        if release_id == 0 {
            return Err(BoundedReadError::Refused("invalid release ID"));
        }
        let mut url = self
            .api_url(&["releases", &release_id.to_string(), "assets"])
            .map_err(|_| BoundedReadError::Refused("invalid asset list endpoint"))?;
        url.query_pairs_mut().append_pair("per_page", "100");
        let initial = url.clone();
        let mut seen = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut assets = Vec::new();
        for _ in 0..MAX_PAGES {
            if !seen.insert(url.as_str().to_owned()) {
                return Err(BoundedReadError::Uncertain("asset pagination loop"));
            }
            let page: Page<Vec<GithubReleaseAsset>> =
                self.bounded_json_get(url.clone(), PAGE_JSON_CAP)?;
            let Some(rows) = page.value else {
                return Err(BoundedReadError::Uncertain("asset page was absent"));
            };
            if rows.len() > PAGE_ITEMS || assets.len() + rows.len() > TOTAL_ITEMS {
                return Err(BoundedReadError::Uncertain("asset list exceeds item bound"));
            }
            for asset in rows {
                let digest_valid = asset.digest.as_deref().is_none_or(|value| {
                    value.strip_prefix("sha256:").is_some_and(|hex| {
                        hex.len() == 64
                            && hex
                                .bytes()
                                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                    })
                });
                if asset.id == 0
                    || validate_asset_name(&asset.name, self.token_value()).is_err()
                    || !digest_valid
                    || !ids.insert(asset.id)
                    || !names.insert(asset.name.clone())
                {
                    return Err(BoundedReadError::Uncertain(
                        "asset list has invalid or repeated identity",
                    ));
                }
                assets.push(asset);
            }
            let Some(raw) = page.next else {
                assets
                    .sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
                return Ok(assets);
            };
            url = next_page(&url, &initial, &raw)?;
        }
        Err(BoundedReadError::Uncertain(
            "asset pagination exceeds page bound",
        ))
    }
}

#[cfg(test)]
#[path = "bounded_read_tests.rs"]
mod tests;
