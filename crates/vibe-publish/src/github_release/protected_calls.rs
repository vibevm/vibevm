//! Single-call, no-redirect transport for a future private publication owner.
//! These response values are not journal Landed evidence. The owner still
//! owes exact pre-effect reobservation, Pending and independent post-effect
//! observation. There is no public or CLI route to these methods.

use std::io::Read;

use reqwest::blocking::{RequestBuilder, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::{Method, StatusCode, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{
    GITHUB_API_VERSION, GITHUB_JSON, GithubGitRef, GithubRelease, GithubReleaseClient, user_agent,
};
use crate::release_manifest::DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME;

const TAG: &str = "v1.1.0";
const REF: &str = "refs/tags/v1.1.0";
const DRAFT_NAME: &str = "vibevm 1.1.0";
const TAG_RESPONSE_CAP: u64 = 64 * 1024;
const RELEASE_RESPONSE_CAP: u64 = 1024 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum ProtectedCallError {
    #[error("protected GitHub call refused before dispatch: {0}")]
    Refused(&'static str),
    #[error(
        "protected GitHub call may have taken effect; inspect owner journal and remote state: {0}"
    )]
    Uncertain(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagResponseEvidence {
    pub reference: String,
    pub object_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeleteReleaseResponseEvidence {
    pub release_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DraftResponseEvidence {
    pub release_id: u64,
    pub tag: String,
    pub target_commitish: String,
}

fn valid_commit(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn fixed_draft_body() -> String {
    format!(
        "Mutable vibevm 1.1.0 distribution. Verify downloads through \
         `{DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME}`."
    )
}

fn bounded_json<T: DeserializeOwned>(
    response: Response,
    cap: u64,
) -> Result<T, ProtectedCallError> {
    if response
        .content_length()
        .is_some_and(|declared| declared > cap)
    {
        return Err(ProtectedCallError::Uncertain(
            "response declared JSON size exceeds cap",
        ));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(cap.min(64 * 1024)).unwrap_or(64 * 1024));
    response
        .take(cap + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ProtectedCallError::Uncertain("response body read failed"))?;
    if bytes.len() as u64 > cap {
        return Err(ProtectedCallError::Uncertain(
            "response JSON body exceeds cap",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| ProtectedCallError::Uncertain("response JSON malformed"))
}

impl GithubReleaseClient {
    fn protected_call_request(
        &self,
        method: Method,
        url: Url,
    ) -> Result<RequestBuilder, ProtectedCallError> {
        if !self.is_fixed_publication_target() {
            #[cfg(test)]
            if !self.is_fixed_publication_loopback() {
                return Err(ProtectedCallError::Refused(
                    "client is not fixed to the publication repository",
                ));
            }
            #[cfg(not(test))]
            return Err(ProtectedCallError::Refused(
                "client is not fixed to the publication repository",
            ));
        }
        let token = self
            .token
            .as_ref()
            .ok_or(ProtectedCallError::Refused("publish token absent"))?;
        Ok(self
            .no_redirect_write_client
            .request(method, url)
            .header(ACCEPT, GITHUB_JSON)
            .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
            .header(USER_AGENT, user_agent())
            .header(AUTHORIZATION, format!("Bearer {}", token.value())))
    }

    /// One POST of the fixed version ref. No upsert or force move.
    pub(crate) fn create_fixed_tag_ref_explicit(
        &self,
        source_commit: &str,
    ) -> Result<TagResponseEvidence, ProtectedCallError> {
        if !valid_commit(source_commit) {
            return Err(ProtectedCallError::Refused(
                "invalid selected source commit",
            ));
        }
        #[derive(Serialize)]
        struct CreateRef<'a> {
            r#ref: &'static str,
            sha: &'a str,
        }
        let url = self
            .api_url(&["git", "refs"])
            .map_err(|_| ProtectedCallError::Refused("invalid tag endpoint"))?;
        let response = self
            .protected_call_request(Method::POST, url)?
            .json(&CreateRef {
                r#ref: REF,
                sha: source_commit,
            })
            .send()
            .map_err(|_| ProtectedCallError::Uncertain("create tag transport failed"))?;
        if response.status() != StatusCode::CREATED {
            return Err(ProtectedCallError::Uncertain(
                "create tag returned non-201 status or redirect",
            ));
        }
        let observed: GithubGitRef = bounded_json(response, TAG_RESPONSE_CAP)?;
        if observed.reference != REF
            || observed.object.kind != "commit"
            || observed.object.sha != source_commit
        {
            return Err(ProtectedCallError::Uncertain(
                "create tag response identity differs",
            ));
        }
        Ok(TagResponseEvidence {
            reference: observed.reference,
            object_sha: observed.object.sha,
        })
    }

    /// One DELETE of an exact positive prior release ID. A 204 response is
    /// only callback evidence; the owner must independently observe absence.
    pub(crate) fn delete_prior_release_explicit(
        &self,
        release_id: u64,
    ) -> Result<DeleteReleaseResponseEvidence, ProtectedCallError> {
        if release_id == 0 {
            return Err(ProtectedCallError::Refused("invalid prior release ID"));
        }
        let url = self
            .api_url(&["releases", &release_id.to_string()])
            .map_err(|_| ProtectedCallError::Refused("invalid release endpoint"))?;
        let response = self
            .protected_call_request(Method::DELETE, url)?
            .send()
            .map_err(|_| ProtectedCallError::Uncertain("delete release transport failed"))?;
        if response.status() != StatusCode::NO_CONTENT {
            return Err(ProtectedCallError::Uncertain(
                "delete release returned non-204 status or redirect",
            ));
        }
        Ok(DeleteReleaseResponseEvidence { release_id })
    }

    /// One POST of a new 1.1.0 draft at the exact selected commit. No
    /// release search/update, asset operation or visibility transition.
    pub(crate) fn create_fixed_draft_explicit(
        &self,
        source_commit: &str,
    ) -> Result<DraftResponseEvidence, ProtectedCallError> {
        if !valid_commit(source_commit) {
            return Err(ProtectedCallError::Refused(
                "invalid selected source commit",
            ));
        }
        #[derive(Serialize)]
        struct Draft<'a> {
            tag_name: &'static str,
            target_commitish: &'a str,
            name: &'static str,
            body: &'a str,
            draft: bool,
            prerelease: bool,
            generate_release_notes: bool,
        }
        let url = self
            .api_url(&["releases"])
            .map_err(|_| ProtectedCallError::Refused("invalid create release endpoint"))?;
        let body = fixed_draft_body();
        let response = self
            .protected_call_request(Method::POST, url)?
            .json(&Draft {
                tag_name: TAG,
                target_commitish: source_commit,
                name: DRAFT_NAME,
                body: &body,
                draft: true,
                prerelease: false,
                generate_release_notes: false,
            })
            .send()
            .map_err(|_| ProtectedCallError::Uncertain("create draft transport failed"))?;
        if response.status() != StatusCode::CREATED {
            return Err(ProtectedCallError::Uncertain(
                "create draft returned non-201 status or redirect",
            ));
        }
        let observed: GithubRelease = bounded_json(response, RELEASE_RESPONSE_CAP)?;
        if observed.id == 0
            || observed.tag_name != TAG
            || !observed.draft
            || observed.target_commitish != source_commit
            || observed.name.as_deref() != Some(DRAFT_NAME)
            || observed.body.as_deref() != Some(body.as_str())
            || observed.prerelease
        {
            return Err(ProtectedCallError::Uncertain(
                "create draft response identity differs",
            ));
        }
        Ok(DraftResponseEvidence {
            release_id: observed.id,
            tag: observed.tag_name,
            target_commitish: observed.target_commitish,
        })
    }
}

#[cfg(test)]
#[path = "protected_calls_tests.rs"]
mod tests;
