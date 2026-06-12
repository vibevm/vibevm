//! `vibe registry redirect / redirect-sync / redirect-update` —
//! redirect-stub creation and maintenance (PROP-002 §2.4.2).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

mod create;
mod sync;
mod update;

#[cfg(test)]
mod tests;

pub(super) use create::run_redirect;
pub(super) use sync::run_redirect_sync;
pub(super) use update::run_redirect_update;

use std::path::Path;

use anyhow::{Result, anyhow, bail};
use serde::Serialize;
use vibe_core::Group;
use vibe_core::manifest::{Manifest, RegistrySection};

// ---------------------------------------------------------------------------
// vibe registry redirect / redirect-sync (PROP-002 §2.4.2)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct RedirectReport {
    ok: bool,
    command: &'static str,
    registry: String,
    pkgref: String,
    stub_url: String,
    target_url: String,
    ref_policy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pinned_ref: Option<String>,
    target_auth: &'static str,
    created_repo: bool,
    dry_run: bool,
    /// `Some` when `--sync` is passed and the sync leg ran. `None` when
    /// the operator did not request sync, when the policy is `pinned`
    /// (sync is meaningless), or when this is a dry-run.
    #[serde(skip_serializing_if = "Option::is_none")]
    sync: Option<RedirectSyncReport>,
}

#[derive(Debug, Serialize)]
struct RedirectSyncReport {
    ok: bool,
    command: &'static str,
    registry: String,
    pkgref: String,
    stub_url: String,
    target_url: String,
    /// Tags pushed into the stub on this run. Empty on a no-op sync
    /// (target and stub already agree).
    pushed_tags: Vec<String>,
    /// Tags already present in the stub before this run (informational —
    /// helps a CI run that aggregates sync output across many stubs).
    already_present: Vec<String>,
    dry_run: bool,
}

#[derive(Debug, Serialize)]
struct RedirectUpdateReport {
    ok: bool,
    command: &'static str,
    registry: String,
    pkgref: String,
    stub_url: String,
    /// Target URL on the new marker. Mirrors the post-update marker
    /// content; equals `target_url` of the existing marker when `--to`
    /// was not passed.
    target_url: String,
    ref_policy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pinned_ref: Option<String>,
    target_auth: &'static str,
    /// Per-field before/after diff for the marker rewrite. Empty only on
    /// dry-runs of trivial edits — in real applies the handler bails
    /// before push when this would be empty.
    changes: Vec<RedirectChangeEntry>,
    /// `true` when the change set carries fields that require
    /// `--trust-redirect` per PROP-002 §2.4.2 (target_url, ref_policy,
    /// or pinned_ref under pinned policy).
    trust_required: bool,
    dry_run: bool,
    /// `Some` when `--resync` was passed and the sync leg ran. `None`
    /// when no resync was requested, when the new policy is `pinned`,
    /// or on dry-run.
    #[serde(skip_serializing_if = "Option::is_none")]
    sync: Option<RedirectSyncReport>,
}

/// Per-field before / after entry produced by
/// [`compute_updated_redirect_section`]. Field values are rendered as
/// canonical strings — `None` for absent optional fields, the kebab-case
/// `RefPolicy` / `AuthKind` discriminant otherwise. This keeps the JSON
/// envelope ergonomic for log aggregators without needing to know the
/// internal enum shapes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RedirectChangeEntry {
    field: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<String>,
}

fn parse_target_auth(s: Option<&str>) -> Result<vibe_core::manifest::AuthKind> {
    match s {
        None | Some("none") => Ok(vibe_core::manifest::AuthKind::None),
        Some("token-env") => Ok(vibe_core::manifest::AuthKind::TokenEnv),
        Some("credential-helper") => Ok(vibe_core::manifest::AuthKind::CredentialHelper),
        Some("ssh") => Ok(vibe_core::manifest::AuthKind::Ssh),
        Some(other) => bail!(
            "unknown --target-auth `{other}` — must be `none`, `token-env`, `credential-helper`, or `ssh`"
        ),
    }
}

/// Extract the `(group, …)` half of a pkgref's identity, rejecting an
/// unqualified registry-subcommand argument (PROP-008 §2.4). Registry
/// resolution and repo naming are group-keyed; a bare name has no group.
fn require_group(pkgref: &vibe_core::PackageRef) -> Result<&Group> {
    pkgref.group.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "package reference `{pkgref}` is not group-qualified — write `<group>/<name>`"
        )
    })
}

/// Resolve the registry to act on for a redirect / redirect-sync command.
fn resolve_target_registry<'m>(
    manifest: &'m Manifest,
    requested: Option<&str>,
    manifest_path: &Path,
) -> Result<&'m RegistrySection> {
    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` entries in `{}`. `vibe registry redirect` needs a registry org \
             where the stub will be created.",
            manifest_path.display()
        );
    }
    match requested {
        Some(name) => manifest.registry_by_name(name).ok_or_else(|| {
            anyhow!(
                "no `[[registry]]` named `{name}` in `{}`",
                manifest_path.display()
            )
        }),
        None => manifest
            .primary_registry()
            .ok_or_else(|| anyhow!("no `[[registry]]` configured")),
    }
}

fn build_redirect_readme(pkgref: &str, target_url: &str, description: Option<&str>) -> String {
    let desc_block = description
        .map(|d| format!("\n> {d}\n"))
        .unwrap_or_default();
    format!(
        "# {pkgref} — registry stub\n\n\
         This repository is a vibevm registry stub that redirects consumers to\n\
         the canonical home of `{pkgref}`:\n\n\
         > {target_url}\n\
         {desc_block}\n\
         Operators reach this package via `vibe install {pkgref}` through the\n\
         org's `[[registry]]` configuration; vibevm follows the\n\
         `vibe-redirect.toml` marker transparently. The actual package content\n\
         (`vibe.toml`, spec files, etc.) lives at the target URL above.\n\n\
         See [PROP-002 §2.4.2](https://example.invalid/spec) for the redirect\n\
         protocol and [`docs/registry-redirect.md`](https://example.invalid/docs)\n\
         for the operator reference.\n"
    )
}

impl RedirectChangeEntry {
    /// Per PROP-002 §2.4.2 trust model — these three fields change what
    /// content a consumer ends up materialising. The other fields
    /// (auth, token_env, description) are operator-side metadata and do
    /// not require `--trust-redirect`.
    fn requires_trust(&self) -> bool {
        matches!(self.field, "target_url" | "ref_policy" | "pinned_ref")
    }
}
