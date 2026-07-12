//! The publish orchestrator — the host-agnostic flow from package
//! source directory to pushed, tagged release. [`Publisher`] reads the
//! manifest, coordinates with the [`RepoCreator`] seam for repo
//! presence and creation, and shells out to git (via
//! [`crate::git_publish`]) for the working-tree → push → tag flow.
//! Layering per
//! [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#publish");

use std::path::PathBuf;

use vibe_core::PackageKind;
use vibe_core::manifest::{Manifest, NamingConvention};

use crate::creator::{CreateOpts, RepoCreator, RepoInfo};
use crate::{PublishError, extract_org_segment, git_publish};

/// Inputs to a single publish run.
///
/// [`with_defaults`](PublishConfig::with_defaults) is the blessed
/// constructor — `v` tag prefix, registry-default naming, real run.
/// Flip `dry_run` to plan without touching the host:
///
/// ```
/// use std::path::PathBuf;
/// use vibe_publish::PublishConfig;
///
/// let mut config = PublishConfig::with_defaults(
///     PathBuf::from("fixtures/registry/org.vibevm.world/wal/v0.1.0"),
///     "https://github.com/vibespecs".to_string(),
/// );
/// config.dry_run = true; // describe the plan, change nothing
/// assert_eq!(config.tag_prefix, "v");
/// ```
#[derive(Debug, Clone)]
pub struct PublishConfig {
    /// Filesystem directory containing `vibe.toml` and the rest
    /// of the package content. The publisher copies this into a fresh
    /// staging clone before pushing.
    pub source_dir: PathBuf,
    /// Org URL (the `[[registry]].url` from `vibe.toml`). The publisher
    /// extracts the org segment for the host API and combines it with
    /// the package repo name for the `git push` URL.
    pub org_url: String,
    /// Convention for mapping `<kind>:<name>` to a repo name under the
    /// org. Defaults to the registry's recorded `naming` field.
    pub naming: NamingConvention,
    /// Tag prefix — `v` is the convention. Surfaces in the tag name as
    /// `<prefix><semver>` (e.g. `v0.1.0`). Customisable for hosts /
    /// registries that pick a different prefix.
    pub tag_prefix: String,
    /// `true` → describe what would happen but make no changes (no API
    /// calls, no pushes, no tags).
    pub dry_run: bool,
}

impl PublishConfig {
    pub fn with_defaults(source_dir: PathBuf, org_url: String) -> Self {
        PublishConfig {
            source_dir,
            org_url,
            naming: NamingConvention::KindName,
            tag_prefix: "v".to_string(),
            dry_run: false,
        }
    }
}

/// Outcome of a successful publish — what was done, on what host, with
/// what URLs. CLI renders this for the operator.
///
/// ```
/// use vibe_core::PackageKind;
/// use vibe_publish::PublishOutcome;
///
/// let outcome = PublishOutcome {
///     kind: PackageKind::Flow,
///     name: "wal".to_string(),
///     version: semver::Version::parse("0.1.0").unwrap(),
///     repo_name: "org.vibevm.wal".to_string(),
///     repo_url: "https://github.com/vibespecs/org.vibevm.wal.git".to_string(),
///     tag: "v0.1.0".to_string(),
///     created_repo: true,
///     host: "github.com".to_string(),
///     dry_run: false,
/// };
/// assert_eq!(outcome.tag, format!("v{}", outcome.version));
/// ```
#[derive(Debug, Clone)]
pub struct PublishOutcome {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,
    pub repo_name: String,
    pub repo_url: String,
    pub tag: String,
    pub created_repo: bool,
    pub host: String,
    pub dry_run: bool,
}

/// The publish orchestrator.
///
/// Canonical flow — pick a [`RepoCreator`], wrap it, call
/// [`publish`](Publisher::publish), render the outcome. Shown here on
/// the direct-git path against a stand-in local URL (`no_run`: the
/// real call reads the package from disk and shells out to `git`):
///
/// ```no_run
/// use std::path::PathBuf;
/// use vibe_publish::{DirectGitCreator, PublishConfig, Publisher};
///
/// // Operator-provisioned repo: push with local git credentials.
/// let creator = DirectGitCreator::new("file:///tmp/registry/org.vibevm.wal.git");
/// let publisher = Publisher::new(&creator);
///
/// let mut config = PublishConfig::with_defaults(
///     PathBuf::from("fixtures/registry/org.vibevm.world/wal/v0.1.0"),
///     "file:///tmp/registry".to_string(),
/// );
/// config.dry_run = true; // plan only — no push, no tag
///
/// let outcome = publisher.publish(&config).unwrap();
/// println!("would publish {} as {}", outcome.name, outcome.tag);
/// ```
pub struct Publisher<'c, C: RepoCreator + ?Sized> {
    creator: &'c C,
}

impl<'c, C: RepoCreator + ?Sized> Publisher<'c, C> {
    pub fn new(creator: &'c C) -> Self {
        Publisher { creator }
    }

    /// Publish the package at `config.source_dir` under the org named in
    /// `config.org_url`, returning a [`PublishOutcome`] describing what
    /// landed.
    ///
    /// Contract:
    /// - **Source.** `config.source_dir` MUST hold a `vibe.toml` with a
    ///   `[package]` table; a project- or workspace-only manifest is
    ///   refused as [`PublishError::SourceInvalid`].
    /// - **Branch.** When the creator declares a
    ///   [`direct_repo_url`](RepoCreator::direct_repo_url) the host-API
    ///   dance is skipped entirely (no org extraction, no token); else
    ///   the org is extracted from `config.org_url`, the repo is probed
    ///   and created if absent, and the credentialed push URL is built.
    /// - **Idempotence boundary.** A fresh commit + tag is always pushed;
    ///   a pre-existing tag is a [`PublishError::TagCollision`] (publish
    ///   never force-pushes tags). `config.dry_run` plans without any
    ///   network or git side effect.
    /// - **Errors.** Every failure is a typed [`PublishError`] naming the
    ///   violated expectation and a fix surface — scope / auth / host
    ///   problems before any push, git / IO problems during it; the
    ///   token never appears in any of them (PROP-000 §20).
    ///
    /// The canonical call shape is the [`Publisher`] type's doctest above.
    pub fn publish(&self, config: &PublishConfig) -> Result<PublishOutcome, PublishError> {
        // Step 1: read the package manifest. `publish` only operates on
        // a publishable `[package]` manifest; a `[project]`-only or
        // `[workspace]`-only `vibe.toml` is rejected as source-invalid.
        let manifest_path = config.source_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path).map_err(|e| PublishError::SourceInvalid {
            path: manifest_path.clone(),
            reason: format!("could not read or parse manifest: {e}"),
        })?;
        let meta = manifest
            .require_package()
            .map_err(|e| PublishError::SourceInvalid {
                path: manifest_path.clone(),
                reason: e.to_string(),
            })?;

        let kind = meta.kind;
        let group = meta.group.clone();
        let name = meta.name.clone();
        let version = meta.version.clone();
        let tag = format!("{}{}", config.tag_prefix, version);

        // Direct-git short-circuit: when the adapter declares a direct
        // repo URL, vibevm pushes straight to it using local-git creds.
        // No org extraction (the URL is repo-level, not org-level), no
        // repo_exists probe, no create_repo, no token. Repo presence is
        // the operator's responsibility — `git push` errors out cleanly
        // if the URL is wrong, and `git_publish::push_with_classification`
        // surfaces a structured `PublishError` with the URL inline
        // (credentials redacted per PROP-000 §20).
        if let Some(direct_url) = self.creator.direct_repo_url() {
            // No naming convention probe — the operator supplied the
            // URL, so the host's actual repo path is whatever they
            // chose. For the human-facing `PublishOutcome.repo_name`
            // we fall back to the configured naming convention's form
            // (matches what the operator typically picked when they
            // provisioned the repo); the truth-of-the-matter is the
            // URL itself, surfaced in `repo_url`.
            let repo_name = config
                .naming
                .repo_name(Some(kind), &group, &name)
                .map_err(|e| PublishError::SourceInvalid {
                    path: manifest_path.clone(),
                    reason: format!("could not derive a repo name: {e}"),
                })?;
            if !config.dry_run {
                git_publish::push_release(&config.source_dir, direct_url, &tag, &name, &version)?;
            }
            return Ok(PublishOutcome {
                kind,
                name,
                version,
                repo_name,
                repo_url: direct_url.to_string(),
                tag,
                created_repo: false,
                host: self.creator.host_name().to_string(),
                dry_run: config.dry_run,
            });
        }

        let repo_name = config
            .naming
            .repo_name(Some(kind), &group, &name)
            .map_err(|e| PublishError::SourceInvalid {
                path: manifest_path.clone(),
                reason: format!("could not derive a repo name: {e}"),
            })?;

        // Step 2: derive org segment from the configured org URL.
        let org_segment = extract_org_segment(&config.org_url)?;

        // Step 3: figure out repo presence.
        let exists = self.creator.repo_exists(&org_segment, &repo_name)?;
        let mut created_repo = false;

        let repo_info = if exists {
            tracing::info!(target: "vibe_publish", org = %org_segment, repo = %repo_name, "repo already exists; reusing");
            RepoInfo {
                html_url: format!("{}/{}", config.org_url.trim_end_matches('/'), repo_name),
                clone_url: format!("{}/{}.git", config.org_url.trim_end_matches('/'), repo_name),
            }
        } else if config.dry_run {
            // Synthesise a plausible RepoInfo for the rendered plan.
            // `created_repo = true` here advertises *what would happen* —
            // the repo does not exist, so a non-dry-run would create it.
            // The CLI renders this as "Would create repository …" which
            // is the correct user expectation for the dry-run case.
            tracing::info!(target: "vibe_publish", "dry-run: would create repo");
            created_repo = true;
            RepoInfo {
                html_url: format!("{}/{}", config.org_url.trim_end_matches('/'), repo_name),
                clone_url: format!("{}/{}.git", config.org_url.trim_end_matches('/'), repo_name),
            }
        } else {
            let opts = CreateOpts {
                description: meta.description.clone(),
                default_branch: Some("main".to_string()),
                homepage: meta.homepage.clone(),
            };
            let info = self.creator.create_repo(&org_segment, &repo_name, &opts)?;
            created_repo = true;
            info
        };

        // Step 4: push contents and tag (skipped on dry-run).
        // Push URL is constructed by the host adapter — SSH-auth hosts
        // return the SSH form; HTTPS-token-auth hosts inject credentials
        // for this single push only. Token never appears in stdout /
        // stderr / log lines; modern git redacts URL passwords in its
        // own diagnostics. The push URL MUST NOT appear in any
        // vibevm-produced output (the user-facing PublishOutcome.repo_url
        // carries the public clone URL for display).
        if !config.dry_run {
            let push_url = self.creator.push_url(&org_segment, &repo_name);
            git_publish::push_release(&config.source_dir, &push_url, &tag, &name, &version)?;
        }

        Ok(PublishOutcome {
            kind,
            name,
            version,
            repo_name,
            repo_url: repo_info.clone_url,
            tag,
            created_repo,
            host: self.creator.host_name().to_string(),
            dry_run: config.dry_run,
        })
    }
}
