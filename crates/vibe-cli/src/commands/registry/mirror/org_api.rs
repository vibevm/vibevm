//! Listing a per-package registry's packages through its host's
//! repository API (PROP-002 `#registry-mirror`).
//!
//! A `github` / `gitverse` registry without an index can still be listed
//! whole, because the host knows which repositories its organisation
//! holds. This module asks that one question; the caller turns each
//! repository the host names into a package coordinate and reads its
//! versions from its `v<semver>` tags, exactly as `--package` does.
//!
//! **The manifest states the coordinate; the naming convention confirms
//! it.** An organisation holds repositories that are not packages, and
//! only the `fqdn` convention spells the whole coordinate in the
//! repository's name — `name` and the `kind-*` conventions drop the group
//! entirely, and the built-in `vibespecs-gitverse` registry is one of
//! them. A group is therefore never guessed from a repository name:
//! `[package] group`, `name` and `kind` of the `vibe.toml` at the
//! repository's newest `v<semver>` tag state the coordinate, and the
//! repository is kept only when [`NamingConvention::repo_name`] composes
//! that coordinate back into the repository's actual name — the same
//! function the mirror uses to address a package's repository, so what
//! survives is exactly what this registry would have published there. A
//! repository whose manifest names some other package (a fork, a
//! rename left behind) fails that check and is dropped.
//!
//! Under `fqdn` the name is *also* read on its own first
//! ([`name_claim`]), split at its last dot and composed back: a name the
//! convention could not have produced is dropped before it costs a single
//! request, and one that could is cross-checked against the manifest. The
//! group-less conventions have no such pre-filter, so under them every
//! repository of the organisation is asked for its tags — the price of a
//! convention that does not carry the group.
//!
//! **Transport.** The request goes through the one admitted,
//! header-carrying GET client in the CLI
//! ([`crate::commands::vvm::bundle::api`]), so the operation's source
//! policy admits the API address before a socket is opened:
//! `exclude_hosting = ["gitverse"]` covers `api.gitverse.ru` as it covers
//! `gitverse.ru`, because the two are one hosting's host keys
//! (`SourceHosting::host_keys`). The API base itself is not passed
//! through the remap Locator — PROP-066 has no artifact kind for a host's
//! REST API, and the release lane's `[self] api` sets the precedent that
//! an API endpoint is re-pointed by naming it. Here that is
//! `VIBEVM_REGISTRY_API_<REGISTRY>`, shaped like the existing
//! `VIBEVM_INDEX_URL_<REGISTRY>`.
//!
//! **Token.** `api.gitverse.ru` answers 401 to *every* anonymous request
//! — even for an organisation that does not exist — so a GitVerse
//! listing needs a read token. Two legs, highest first:
//!
//! 1. the registry's own declared token: under `auth = "token-env"`, the
//!    variable `token_env` names, else the host-derived
//!    `VIBEVM_REGISTRY_TOKEN_<HOST>` — resolved by
//!    `vibe_registry::resolve_registry_auth`, the one place that reads a
//!    row's credential declaration. A registry that declares where its
//!    credential lives outranks a machine-wide convention, and this is
//!    the same variable the registry's Git and index reads already use —
//!    one credential per registry, not a second one for listing;
//! 2. the per-host publish-token chain for the **registry's** host, not
//!    the API's ([`vibe_publish::token::load_token_for_host`]:
//!    `VIBEVM_PUBLISH_TOKEN_<HOST>`, `VIBEVM_PUBLISH_TOKEN`,
//!    `~/.vibe/<host-prefix>.publish.token`,
//!    `~/.vibe/git.publish.token`). `gitverse.ru` is the host an operator
//!    holds a token for; `api.gitverse.ru` would derive the nonsense
//!    prefix `api`. The release lane reads GitVerse the same way.
//!
//! Neither leg is required: without a token the listing is anonymous, and
//! a refusal says whether one was sent — never which, and never its
//! value. The token travels as a bearer to the API origin only (the
//! client pins it there across redirects) and is never printed, logged or
//! put in an error.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

mod naming;

pub(super) use naming::{NameClaim, name_claim, produces};

use std::sync::Arc;

use anyhow::{Context, Result};
use vibe_core::manifest::{RegistrySection, RegistryType};
use vibe_publish::token::{Token, TokenSource};
use vibe_registry::HttpAdmission;

use crate::commands::vvm::bundle::api::{Answer, ApiClient, document};

/// Repositories asked for per page — the maximum both hosts accept.
const PER_PAGE: usize = 100;
/// Pages read at most, so a host that never shortens a page cannot spin
/// the listing forever.
const PAGE_LIMIT: usize = 100;

/// The GitVerse API carries its version in `Accept` (see
/// `vibe_publish::gitverse`); without the suffix it answers 400.
const GITVERSE_ACCEPT: &str = "application/vnd.gitverse.object+json;version=1";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";
/// GitVerse's public API root for `gitverse.ru`.
const GITVERSE_API: &str = "https://api.gitverse.ru";
/// GitHub's public API root for `github.com`.
const GITHUB_API: &str = "https://api.github.com";

/// The repository-listing API of one registry's host.
pub(super) struct OrgApi {
    shape: Shape,
    /// The API root, without a trailing slash.
    base: String,
    /// The organisation whose repositories are listed.
    org: String,
    /// The registry url's host — whose token conventions apply.
    host: String,
}

/// Which REST shape the host speaks. Both are Gitea-shaped for this one
/// question (`GET /orgs/{org}/repos` returning an array of objects with a
/// `name`), and differ only in the `Accept` header they require.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Github,
    Gitverse,
}

/// Why a registry's packages cannot be listed through its host, and what
/// to do instead. Every variant ends in the `--package` fix, because that
/// is the path that still works.
#[derive(Debug, thiserror::Error)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
pub(super) enum Unlistable {
    #[error(
        "registry `{registry}` has no index that answers, and the `--offline` posture may not \
         contact its host either, so its packages cannot be listed \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
         fix: name the packages to copy with `--package <group>/<name>`, or mirror without \
         `--offline`)"
    )]
    Offline { registry: String },
    #[error(
        "registry `{registry}` has no index that answers, and a `{kind}` registry has no \
         repository-listing API to ask instead, so its packages cannot be listed \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
         fix: name the packages to copy with `--package <group>/<name>` — their versions are \
         then read from their tags — declare the registry as `type = \"github\"` or \
         `type = \"gitverse\"` if its host serves that API, or publish an index for the \
         registry)"
    )]
    NoHostApi {
        registry: String,
        kind: &'static str,
    },
    #[error(
        "registry `{registry}` has no index that answers, and its url `{url}` does not name one \
         organisation on its host, so its repositories cannot be listed \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
         fix: give the registry its organisation root (`https://<host>/<organisation>`), or name \
         the packages to copy with `--package <group>/<name>`)"
    )]
    NoOrganisation { registry: String, url: String },
    #[error(
        "the repository listing of registry `{registry}` at `{address}` answered {status}: \
         {credential}, so its packages cannot be listed \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
         fix: {credential_fix}; or name the packages to copy with `--package <group>/<name>`)"
    )]
    Refused {
        registry: String,
        address: String,
        status: u16,
        /// Whether a read token was sent — never which, never its value.
        credential: String,
        /// Where a read token for this host may be configured. The
        /// `--package` half of the fix is in the message itself, so every
        /// variant of this error names it whatever the caller supplies.
        credential_fix: String,
    },
}

/// The host API that lists `reg`'s repositories, or why there is none.
pub(super) fn resolve(reg: &RegistrySection, offline: bool) -> Result<OrgApi, Unlistable> {
    if offline {
        return Err(Unlistable::Offline {
            registry: reg.name.clone(),
        });
    }
    let kind = reg.effective_type();
    let shape = match kind {
        RegistryType::Github => Shape::Github,
        RegistryType::Gitverse => Shape::Gitverse,
        RegistryType::Git | RegistryType::Monorepo | RegistryType::Localdir => {
            return Err(Unlistable::NoHostApi {
                registry: reg.name.clone(),
                kind: kind.as_str(),
            });
        }
    };
    let (host, org) = host_and_org(&reg.url).ok_or_else(|| Unlistable::NoOrganisation {
        registry: reg.name.clone(),
        url: vibe_publish::git_publish::redact_credentials(&reg.url),
    })?;
    Ok(OrgApi {
        shape,
        base: api_base(shape, &reg.name, &host),
        org,
        host,
    })
}

impl OrgApi {
    /// Every repository the organisation holds, as the host names them,
    /// sorted and deduplicated. Which of them are packages is not decided
    /// here — the caller asks each one's manifest (see [`produces`]).
    pub(super) fn repositories(
        &self,
        reg: &RegistrySection,
        admission: Option<Arc<HttpAdmission>>,
    ) -> Result<Vec<String>> {
        let client = ApiClient::new(admission)?;
        let token = read_token(reg, &self.host)?;
        let mut repositories: Vec<String> = Vec::new();
        for page in 1..=PAGE_LIMIT {
            let address = format!(
                "{}/orgs/{}/repos?page={page}&per_page={PER_PAGE}",
                self.base, self.org
            );
            let answer = client.request(
                &address,
                &[("Accept", self.accept())],
                token.as_ref().map(|held| &held.token),
            )?;
            let (response, at) = match answer {
                Answer::Ok(response, at) => (response, at),
                Answer::Status(status, at) => return Err(self.refused(reg, status, &at, &token)),
            };
            let page: Vec<ApiRepo> = serde_json::from_slice(&document(response, &at)?)
                .with_context(|| format!("reading the repository listing at `{at}`"))?;
            let count = page.len();
            repositories.extend(
                page.into_iter()
                    .map(|repo| repo.name)
                    .filter(|name| !name.is_empty()),
            );
            if count < PER_PAGE {
                break;
            }
        }
        repositories.sort();
        repositories.dedup();
        Ok(repositories)
    }

    fn accept(&self) -> &'static str {
        match self.shape {
            Shape::Github => GITHUB_ACCEPT,
            Shape::Gitverse => GITVERSE_ACCEPT,
        }
    }

    /// The host would not answer the listing: say with what status, whether
    /// a token was sent, and every way to configure one.
    fn refused(
        &self,
        reg: &RegistrySection,
        status: reqwest::StatusCode,
        at: &str,
        token: &Option<ReadToken>,
    ) -> anyhow::Error {
        let declared = reg
            .resolve_token_env_name()
            .map(|name| format!("`auth = \"token-env\"` with `${name}`"))
            .unwrap_or_else(|| "`auth = \"token-env\"`".to_string());
        let per_host = vibe_publish::token::host_env_var(&self.host)
            .map(|name| format!(", `${name}`"))
            .unwrap_or_default();
        let file = self
            .host
            .split('.')
            .next()
            .filter(|label| !label.is_empty())
            .map(|label| format!(" or `~/.vibe/{}.publish.token`", label.to_ascii_lowercase()))
            .unwrap_or_default();
        Unlistable::Refused {
            registry: reg.name.clone(),
            address: at.to_string(),
            status: status.as_u16(),
            credential: match token {
                Some(held) => format!("a read token from {} was sent and rejected", held.origin),
                None => "no read token was configured for this registry".to_string(),
            },
            credential_fix: format!(
                "configure a read token for `{}`: {declared}{per_host}{file}",
                self.host
            ),
        }
        .into()
    }
}

/// One repository as both hosts describe it. Everything but the name is
/// the host's business.
#[derive(Debug, serde::Deserialize)]
struct ApiRepo {
    #[serde(default)]
    name: String,
}

/// A read token with where it came from — the origin, never the value.
struct ReadToken {
    token: Token,
    origin: String,
}

/// The read token for `reg`'s host, by the two-leg precedence this
/// module's header documents. No token is an anonymous listing.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: the variable is the one the registry row itself names (`token_env`), resolved by `resolve_registry_auth` like the registry's Git and index reads, which read it at contact time too; threading every row's token through the composition root would put secrets in long-lived state (spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth)."
)]
fn read_token(reg: &RegistrySection, host: &str) -> Result<Option<ReadToken>> {
    // Leg 1: what the registry itself declares. `resolve_registry_auth`
    // is the one place that decides whether a row carries a `token-env`
    // credential and which variable holds it, so the listing reads the
    // same credential the registry's Git and index reads do.
    let declared = vibe_registry::resolve_registry_auth(
        vibe_registry::RegistryAuthInput::SelectedRegistry(reg),
    )?;
    if let Some(name) = declared.token_env_name
        && let Some(value) = std::env::var(&name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    {
        return Ok(Some(ReadToken {
            token: Token::from_explicit(value),
            origin: format!("${name}"),
        }));
    }
    // Leg 2: the per-host publish-token chain. No token configured is an
    // anonymous listing; a token file that cannot be read is a problem
    // the operator must hear about, not a silent downgrade.
    let token = match vibe_publish::token::load_token_for_host(host) {
        Ok(token) => token,
        Err(vibe_publish::PublishError::AuthMissing { .. }) => return Ok(None),
        Err(error) => {
            return Err(
                anyhow::Error::from(error).context(format!("reading the read token for `{host}`"))
            );
        }
    };
    let origin = match token.source() {
        TokenSource::EnvVar(name) => format!("${name}"),
        TokenSource::File(path) => format!("`{}`", path.display()),
        TokenSource::Explicit => "an explicit token".to_string(),
    };
    Ok(Some(ReadToken { token, origin }))
}

/// The API root for `host`: the operator's override, else the host's
/// public API. `VIBEVM_REGISTRY_API_<REGISTRY>` is shaped exactly like the
/// existing `VIBEVM_INDEX_URL_<REGISTRY>` — an invocation-layer re-point
/// of one registry's endpoint, which is also how a test aims the listing
/// at a local server.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: `VIBEVM_REGISTRY_API_<REGISTRY>` is keyed by a registry name known only after the layers are read, exactly like `VIBEVM_INDEX_URL_<REGISTRY>` (vibe_registry::index_client::locate), an invocation-layer re-point (spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror)."
)]
fn api_base(shape: Shape, registry: &str, host: &str) -> String {
    if let Some(override_base) =
        std::env::var(format!("VIBEVM_REGISTRY_API_{}", env_suffix(registry)))
            .ok()
            .map(|base| base.trim().to_string())
            .filter(|base| !base.is_empty())
    {
        return override_base.trim_end_matches('/').to_string();
    }
    match shape {
        // GitHub Enterprise serves the same API under `/api/v3`; the
        // public host serves it on its own `api.` name.
        Shape::Github if host == "github.com" => GITHUB_API.to_string(),
        Shape::Github => format!("https://{host}/api/v3"),
        Shape::Gitverse if host == "gitverse.ru" => GITVERSE_API.to_string(),
        // An on-premises GitVerse answers on `api.<host>`, as the release
        // lane's `[self] api` default does.
        Shape::Gitverse => format!("https://api.{host}"),
    }
}

/// `<registry>` as an environment-variable suffix: the shape
/// `vibe_registry::index_url_for` uses for `VIBEVM_INDEX_URL_<REGISTRY>`.
fn env_suffix(registry: &str) -> String {
    registry
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// The `(host, organisation)` a registry url names: its host and the one
/// path segment under it. A url with no path, or a nested one, names no
/// organisation.
fn host_and_org(url: &str) -> Option<(String, String)> {
    let host = super::git_source::url_host(url)?;
    let trimmed = url.trim().trim_end_matches('/');
    let path = match trimmed.split_once("://") {
        Some((_, rest)) => rest.split_once('/').map(|(_, path)| path)?,
        // scp form `git@host:org`.
        None => trimmed.split_once(':').map(|(_, path)| path)?,
    };
    let org = path.trim_matches('/').trim_end_matches(".git");
    if org.is_empty() || org.contains('/') {
        return None;
    }
    Some((host, org.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry(body: &str) -> RegistrySection {
        toml::from_str(body).expect("a registry row")
    }

    #[test]
    fn organisation_roots_are_read_from_every_url_form() {
        assert_eq!(
            host_and_org("https://github.com/vibespecs"),
            Some(("github.com".into(), "vibespecs".into()))
        );
        assert_eq!(
            host_and_org("https://gitverse.ru/vibespecs/"),
            Some(("gitverse.ru".into(), "vibespecs".into()))
        );
        assert_eq!(
            host_and_org("git@gitverse.ru:vibespecs"),
            Some(("gitverse.ru".into(), "vibespecs".into()))
        );
        // A repository, not an organisation root; and a host alone.
        assert!(host_and_org("https://github.com/vibespecs/wal.git").is_none());
        assert!(host_and_org("https://github.com").is_none());
    }

    #[test]
    fn only_a_host_with_a_repository_listing_api_resolves() {
        let github = registry("name='vibespecs'\nurl='https://github.com/vibespecs'\n");
        let api = resolve(&github, false).expect("github lists");
        assert_eq!(api.base, GITHUB_API);
        assert_eq!(api.org, "vibespecs");
        assert_eq!(api.host, "github.com");

        let gitverse = registry("name='gv'\nurl='https://gitverse.ru/vibespecs'\n");
        assert_eq!(
            resolve(&gitverse, false).expect("gitverse lists").base,
            GITVERSE_API
        );

        let plain = registry("name='corp'\ntype='git'\nurl='https://git.corp.example/vibe'\n");
        assert!(matches!(
            resolve(&plain, false),
            Err(Unlistable::NoHostApi { .. })
        ));
        // A convention that drops the group is no longer a refusal: the
        // manifest states the coordinate (the built-in
        // `vibespecs-gitverse` is `naming = "name"`).
        let bare_named =
            registry("name='gv'\nurl='https://gitverse.ru/vibespecs'\nnaming='name'\n");
        assert_eq!(
            resolve(&bare_named, false)
                .expect("a `name` registry lists")
                .org,
            "vibespecs"
        );
        assert!(matches!(
            resolve(&github, true),
            Err(Unlistable::Offline { .. })
        ));
    }

    #[test]
    fn every_refusal_names_the_package_flag() {
        for error in [
            Unlistable::Offline {
                registry: "gv".into(),
            },
            Unlistable::NoHostApi {
                registry: "gv".into(),
                kind: "git",
            },
            Unlistable::NoOrganisation {
                registry: "gv".into(),
                url: "https://gitverse.ru".into(),
            },
            Unlistable::Refused {
                registry: "gv".into(),
                address: "https://api.gitverse.ru/orgs/vibespecs/repos".into(),
                status: 401,
                credential: "no read token was configured for this registry".into(),
                credential_fix: "configure a read token".into(),
            },
        ] {
            assert!(error.to_string().contains("--package"), "{error}");
        }
    }

    #[test]
    fn an_on_premises_host_gets_its_own_api_and_an_override_wins() {
        assert_eq!(
            api_base(Shape::Gitverse, "corp", "git.corp.example"),
            "https://api.git.corp.example"
        );
        assert_eq!(
            api_base(Shape::Github, "corp", "github.corp.example"),
            "https://github.corp.example/api/v3"
        );
        assert_eq!(env_suffix("vibespecs-gitverse"), "VIBESPECS_GITVERSE");
    }
}
