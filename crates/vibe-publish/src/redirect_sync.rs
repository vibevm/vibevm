//! Redirect-stub tag mirroring (PROP-002 §2.4.2) — the publish-side
//! domain behind `vibe registry redirect-sync`, `redirect --sync`, and
//! `redirect-update --resync`.
//!
//! Pushing the target's tag list into a registry stub is a maintainer-
//! side write — it shallow-clones the stub, lists tags on both sides, and
//! pushes the missing ones — so it lives in vibe-publish beside the
//! [`git_publish`](crate::git_publish) primitives it drives, not in
//! vibe-registry (which vibe-publish depends on; the reverse edge would
//! cycle). The CLI keeps argument parsing, registry resolution, the
//! stub-existence probe, and rendering (CONVERT-PLAN v0.1 §4.2).
//!
//! Per-tag progress crosses the seam as typed [`RedirectSyncEvent`]s
//! ([`RedirectSyncObserver`]); the structured [`RedirectSyncOutcome`]
//! carries the pushed / already-present breakdown back for the caller's
//! report.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::RedirectSection;

use crate::PublishError;
use crate::extract_host_segment;
use crate::git_publish;

/// Failure surface of redirect-sync — every refusal names the violated
/// spec unit and the fix surface (the product-error grammar).
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#redirect")]
pub enum RedirectSyncError {
    #[error(
        "stub at `{stub_url}` does not carry `vibe-redirect.toml` at HEAD — is this actually a \
         redirect stub? \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#redirect; \
          fix: point redirect-sync at a stub repo created by `vibe registry redirect`)"
    )]
    NotAStub { stub_url: String },

    #[error(
        "parsing `vibe-redirect.toml` from stub `{stub_url}` failed: {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#redirect; \
          fix: correct the marker file on the stub, or recreate it with `vibe registry redirect`)"
    )]
    MarkerParse { stub_url: String, reason: String },

    #[error(
        "stub `{stub_url}` uses `ref_policy = \"pinned\"` — every consumer resolves to \
         `pinned_ref = {pinned_ref:?}` regardless of stub tag, so there is nothing to sync \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#redirect; \
          fix: edit the marker to `ref_policy = \"pass-through-tag\"` to enable tag mirroring)"
    )]
    PinnedPolicy {
        stub_url: String,
        pinned_ref: String,
    },

    #[error(
        "redirect target `{target_url}` declares auth = \"token-env\" but {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#redirect; \
          fix: export the named env-var, or change the stub's `[redirect].auth`)"
    )]
    TargetAuth { target_url: String, reason: String },

    #[error(transparent)]
    Git(#[from] PublishError),
}

/// One observable step of [`sync_redirect_tags`]. Fields carry exactly
/// what a renderer needs; no pre-formatted prose crosses the seam
/// (R3-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectSyncEvent {
    /// Dry-run: this tag is on the target but missing from the stub, so a
    /// real run would push it.
    WouldPush { tag: String },
    /// A missing tag was pushed into the stub.
    Pushed { tag: String },
    /// This tag was already present on the stub — nothing to do.
    AlreadyPresent { tag: String },
}

/// The caller's view into a running redirect-sync. Implemented by the CLI
/// to render per-tag progress; tests and headless callers use
/// [`NullObserver`].
///
/// ```
/// use vibe_publish::redirect_sync::{RedirectSyncEvent, RedirectSyncObserver};
///
/// struct Collector(std::cell::RefCell<Vec<RedirectSyncEvent>>);
/// impl RedirectSyncObserver for Collector {
///     fn on(&self, event: RedirectSyncEvent) {
///         self.0.borrow_mut().push(event);
///     }
/// }
///
/// let observer = Collector(std::cell::RefCell::new(Vec::new()));
/// observer.on(RedirectSyncEvent::Pushed { tag: "v0.1.0".into() });
/// assert_eq!(observer.0.into_inner().len(), 1);
/// ```
pub trait RedirectSyncObserver {
    fn on(&self, event: RedirectSyncEvent);
}

/// Ignores every event — for headless callers and tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullObserver;

impl RedirectSyncObserver for NullObserver {
    fn on(&self, _event: RedirectSyncEvent) {}
}

/// Result of a redirect-sync pass — the target URL read from the stub's
/// marker plus the tags pushed and the tags already present.
#[derive(Debug, Clone)]
pub struct RedirectSyncOutcome {
    /// Target URL the stub's `vibe-redirect.toml` points at.
    pub target_url: String,
    /// Tags pushed into the stub on this run (empty on a no-op sync, or
    /// the would-be-pushed set on a dry-run).
    pub pushed_tags: Vec<String>,
    /// Tags already present on the stub before this run.
    pub already_present: Vec<String>,
}

/// Mirror the redirect target's tag list into the stub. Reads the stub's
/// `vibe-redirect.toml`, enumerates target and stub tags, and pushes the
/// tags the target has but the stub lacks (each as an annotated tag on
/// the stub's `main` commit — stub content is flat, so the commit is
/// identical regardless of which target tag the stub tag fronts).
///
/// `push_url` is the credentialed stub URL; `target_url_hint` is the
/// caller's expected target (`"<read-from-stub>"` when the caller has
/// none) — a mismatch with the stub-stored target is logged, not fatal,
/// since the stub is the source of truth. On `dry_run`, no tags are
/// pushed and the would-be-pushed set is reported.
///
/// Stubs declaring `ref_policy = "pinned"` cannot pass tags through, so
/// syncing one is a [`RedirectSyncError::PinnedPolicy`] refusal.
pub fn sync_redirect_tags(
    observer: &dyn RedirectSyncObserver,
    stub_url: &str,
    target_url_hint: &str,
    push_url: &str,
    dry_run: bool,
) -> Result<RedirectSyncOutcome, RedirectSyncError> {
    use vibe_core::manifest::{RedirectFile, RefPolicy};

    // Step 1: shallow-clone the stub so we can read the marker file and
    // have a working tree to anchor new tags onto.
    let stub_clone = git_publish::shallow_clone(push_url)?;
    let marker_path = stub_clone.path().join(RedirectFile::FILENAME);
    if !marker_path.exists() {
        return Err(RedirectSyncError::NotAStub {
            stub_url: stub_url.to_string(),
        });
    }
    let stub_file =
        RedirectFile::read(&marker_path).map_err(|e| RedirectSyncError::MarkerParse {
            stub_url: stub_url.to_string(),
            reason: e.to_string(),
        })?;

    // Pinned policy — stub tags don't pass through, so syncing is a
    // semantic mistake.
    if matches!(stub_file.redirect.ref_policy, RefPolicy::Pinned) {
        return Err(RedirectSyncError::PinnedPolicy {
            stub_url: stub_url.to_string(),
            pinned_ref: stub_file.redirect.pinned_ref.clone().unwrap_or_default(),
        });
    }

    let target_url = stub_file.redirect.target_url.clone();
    if target_url_hint != "<read-from-stub>" && target_url_hint != target_url {
        // The CLI surface (`--to`) only matches the stub on `redirect`;
        // `redirect-sync` reads from the stub itself. The hint disagreeing
        // is a sanity check, not a hard error — log it.
        tracing::debug!(
            target: "vibe_publish::redirect_sync",
            "target_url hint `{target_url_hint}` disagrees with stub-stored `{target_url}`; using stub"
        );
    }

    // Step 2: build a target-side fetch URL with credentials if the stub
    // declares `auth = "token-env"`. Public targets need no token.
    let target_fetch_url = build_target_fetch_url(&target_url, &stub_file.redirect)?;

    // Step 3: list tags on both sides. ls-remote is the source of truth
    // for the stub side too (the shallow clone has all refs by virtue of
    // `--single-branch`, but ls-remote does not depend on that).
    let target_tags = git_publish::ls_remote_tags(&target_fetch_url)?;
    let stub_tags = git_publish::ls_remote_tags(push_url)?;

    // Step 4: classify.
    let mut to_push: Vec<String> = Vec::new();
    let mut already: Vec<String> = Vec::new();
    for t in &target_tags {
        if stub_tags.iter().any(|s| s == t) {
            already.push(t.clone());
        } else {
            to_push.push(t.clone());
        }
    }
    to_push.sort();
    already.sort();

    if dry_run {
        for t in &to_push {
            observer.on(RedirectSyncEvent::WouldPush { tag: t.clone() });
        }
        for t in &already {
            observer.on(RedirectSyncEvent::AlreadyPresent { tag: t.clone() });
        }
        return Ok(RedirectSyncOutcome {
            target_url,
            pushed_tags: to_push,
            already_present: already,
        });
    }

    // Step 5: push the missing tags. Each tag is annotated, anchored at
    // the stub's `main` commit.
    for t in &to_push {
        git_publish::push_tag_only(stub_clone.path(), push_url, t)?;
        observer.on(RedirectSyncEvent::Pushed { tag: t.clone() });
    }
    for t in &already {
        observer.on(RedirectSyncEvent::AlreadyPresent { tag: t.clone() });
    }

    Ok(RedirectSyncOutcome {
        target_url,
        pushed_tags: to_push,
        already_present: already,
    })
}

/// Build a fetch URL for the target side of a redirect, applying
/// `[redirect].auth` if it asks for token-based auth. For `auth = "none"`
/// this returns the URL verbatim; for `auth = "token-env"` it injects the
/// resolved token using the same shape M1.14 plumbing applies
/// (`https://x-access-token:<TOKEN>@host/...`). Other auth regimes
/// (`credential-helper`, `ssh`) trust the local git's auth path.
#[spec(
    deviates = "spec://discipline-core/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: the target token env-var name is discovered at runtime from \
              the stub's `vibe-redirect.toml` (`token_env`, or a host-derived default), \
              so the read cannot move to a composition root that has not yet read the marker"
)]
pub fn build_target_fetch_url(
    target_url: &str,
    redirect: &RedirectSection,
) -> Result<String, RedirectSyncError> {
    use vibe_core::manifest::AuthKind;
    match redirect.auth {
        AuthKind::None | AuthKind::CredentialHelper | AuthKind::Ssh => Ok(target_url.to_string()),
        AuthKind::TokenEnv => {
            let env_name = redirect
                .token_env
                .clone()
                .or_else(|| derive_target_token_env(target_url))
                .ok_or_else(|| RedirectSyncError::TargetAuth {
                    target_url: target_url.to_string(),
                    reason: "no `token_env` is set and the host cannot be derived for a default \
                             env-var name"
                        .to_string(),
                })?;
            let value = std::env::var(&env_name).map_err(|_| RedirectSyncError::TargetAuth {
                target_url: target_url.to_string(),
                reason: format!("env-var `{env_name}` is unset or empty in this shell"),
            })?;
            Ok(inject_token_into_url(target_url, &value))
        }
    }
}

/// Derive the default target-token env-var name from a URL's host —
/// `https://gitverse.ru/y` → `VIBEVM_TARGET_TOKEN_GITVERSE_RU`. Returns
/// `None` when the host cannot be extracted.
pub fn derive_target_token_env(target_url: &str) -> Option<String> {
    let host = extract_host_segment(target_url).ok()?;
    let upper = host.to_ascii_uppercase().replace(['.', '-'], "_");
    Some(format!("VIBEVM_TARGET_TOKEN_{upper}"))
}

/// Inject a token into an `https://` URL as
/// `https://x-access-token:<TOKEN>@host/...`. SSH-form and already-
/// credentialed URLs pass through unchanged.
///
/// ```
/// use vibe_publish::redirect_sync::inject_token_into_url;
///
/// assert_eq!(
///     inject_token_into_url("https://github.com/org/flow-wal.git", "abc123"),
///     "https://x-access-token:abc123@github.com/org/flow-wal.git",
/// );
/// // SSH-form has nowhere to land a token — passes through.
/// assert_eq!(
///     inject_token_into_url("git@github.com:org/flow-wal.git", "abc123"),
///     "git@github.com:org/flow-wal.git",
/// );
/// ```
pub fn inject_token_into_url(url: &str, token: &str) -> String {
    if !url.starts_with("https://") {
        // SSH-form / file:// — token has nowhere to land; pass through.
        return url.to_string();
    }
    let rest = &url[8..]; // past "https://"
    if rest.contains('@') {
        // Already credentialed — caller's choice; do not double-inject.
        return url.to_string();
    }
    format!("https://x-access-token:{token}@{rest}")
}

#[cfg(test)]
mod tests {
    use super::{build_target_fetch_url, derive_target_token_env, inject_token_into_url};
    use vibe_core::manifest::{AuthKind, RedirectSection, RefPolicy};

    #[test]
    fn derive_target_token_env_uppercase_and_underscore() {
        assert_eq!(
            derive_target_token_env("https://gitlab.acme.example/x").as_deref(),
            Some("VIBEVM_TARGET_TOKEN_GITLAB_ACME_EXAMPLE")
        );
        assert_eq!(
            derive_target_token_env("https://gitverse.ru/y").as_deref(),
            Some("VIBEVM_TARGET_TOKEN_GITVERSE_RU")
        );
    }

    #[test]
    fn inject_token_passes_through_ssh_form() {
        let url = "git@github.com:vibespecs/flow-wal.git";
        assert_eq!(inject_token_into_url(url, "secret"), url);
    }

    #[test]
    fn inject_token_skips_already_credentialed_https() {
        let url = "https://existing:cred@github.com/x/y";
        assert_eq!(inject_token_into_url(url, "newtoken"), url);
    }

    #[test]
    fn inject_token_embeds_into_https() {
        let url = "https://github.com/vibespecs/flow-wal.git";
        let out = inject_token_into_url(url, "abc123");
        assert_eq!(
            out,
            "https://x-access-token:abc123@github.com/vibespecs/flow-wal.git"
        );
    }

    #[test]
    fn build_target_fetch_url_none_passes_through() {
        let section = RedirectSection {
            target_url: "https://example.invalid/x".into(),
            ref_policy: RefPolicy::PassThroughTag,
            pinned_ref: None,
            auth: AuthKind::None,
            token_env: None,
            description: None,
        };
        let out = build_target_fetch_url("https://example.invalid/x", &section).unwrap();
        assert_eq!(out, "https://example.invalid/x");
    }

    #[test]
    fn build_target_fetch_url_token_env_demands_var_set() {
        let section = RedirectSection {
            target_url: "https://example.invalid/x".into(),
            ref_policy: RefPolicy::PassThroughTag,
            pinned_ref: None,
            auth: AuthKind::TokenEnv,
            token_env: Some("VIBEVM_TEST_DEFINITELY_UNSET_TOKEN_VAR".into()),
            description: None,
        };
        let err = build_target_fetch_url("https://example.invalid/x", &section).unwrap_err();
        assert!(
            err.to_string()
                .contains("VIBEVM_TEST_DEFINITELY_UNSET_TOKEN_VAR")
        );
    }
}
