//! `vibe registry redirect / redirect-sync / redirect-update` —
//! redirect-stub creation and maintenance (PROP-002 §2.4.2).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::Group;
use vibe_core::manifest::{Manifest, RegistrySection};
use vibe_publish::{
    creator_for_url, extract_host_segment, extract_org_segment, load_token_for_host,
};

use crate::cli::{RegistryRedirectArgs, RegistryRedirectSyncArgs, RegistryRedirectUpdateArgs};
use crate::output;

use super::resolve_project_root;

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

pub(super) fn run_redirect(ctx: &output::Context, args: RegistryRedirectArgs) -> Result<()> {
    use vibe_core::PackageRef;
    use vibe_core::manifest::{AuthKind, RedirectFile, RefPolicy};

    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let pkgref = PackageRef::parse(&args.pkgref)
        .with_context(|| format!("parsing pkgref `{}`", args.pkgref))?;
    let group = require_group(&pkgref)?;

    let registry_section =
        resolve_target_registry(&manifest, args.registry.as_deref(), &manifest_path)?;

    // Validate URL shape early — before any side-effecting work — so the
    // operator gets a fast actionable error instead of a network failure.
    let host = extract_host_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let org_segment = extract_org_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;

    // Validate target URL shape — must at least have a scheme git accepts.
    if args.to.trim().is_empty() {
        bail!("--to must be a non-empty git URL");
    }

    // Validate ref-policy + pinned-ref combination.
    let (ref_policy, pinned_ref) = match args.ref_policy.as_str() {
        "pass-through-tag" => {
            if args.pinned_ref.is_some() {
                bail!(
                    "--pinned-ref is only meaningful with --ref-policy pinned; drop it or \
                     change to --ref-policy pinned"
                );
            }
            (RefPolicy::PassThroughTag, None)
        }
        "pinned" => {
            let r = args.pinned_ref.as_deref().ok_or_else(|| {
                anyhow!("--ref-policy pinned requires --pinned-ref <tag/branch/rev>")
            })?;
            (RefPolicy::Pinned, Some(r.to_string()))
        }
        other => bail!(
            "unknown --ref-policy `{other}` — must be `pass-through-tag` (default) or `pinned`"
        ),
    };

    let target_auth = parse_target_auth(args.target_auth.as_deref())?;
    if matches!(target_auth, AuthKind::TokenEnv) && args.target_token_env.is_none() {
        tracing::debug!(
            target: "vibe_cli::registry::redirect",
            "target-auth=token-env without explicit --target-token-env; will derive from host on resolve"
        );
    }
    if args.target_token_env.is_some() && !matches!(target_auth, AuthKind::TokenEnv) {
        bail!(
            "--target-token-env is only meaningful with --target-auth token-env; got --target-auth {:?}",
            target_auth.as_str()
        );
    }

    // Compute the stub repo name from naming convention.
    let stub_repo_name = registry_section
        .naming
        .repo_name(pkgref.kind, group, &pkgref.name)
        .with_context(|| format!("deriving the stub repo name for `{group}/{}`", pkgref.name))?;
    // Stub URL surfaced in JSON / human output. Construction mirrors what
    // [`MultiRegistryResolver`] does at resolve time.
    let stub_url = format!(
        "{}/{}",
        registry_section.url.trim_end_matches('/'),
        stub_repo_name
    );

    // Build the stub source dir — `vibe-redirect.toml` + README.
    let stub_section = vibe_core::manifest::RedirectSection {
        target_url: args.to.clone(),
        ref_policy,
        pinned_ref: pinned_ref.clone(),
        auth: target_auth,
        token_env: args.target_token_env.clone(),
        description: args.description.clone(),
    };
    let stub_file = RedirectFile {
        redirect: stub_section,
    };

    let staging = tempfile::tempdir().context("creating stub staging dir")?;
    let stub_marker_path = staging.path().join(RedirectFile::FILENAME);
    stub_file
        .write(&stub_marker_path)
        .with_context(|| format!("writing `{}`", stub_marker_path.display()))?;

    // README — operator-friendly summary so a human visiting the stub
    // repo on the host's web UI understands what they're looking at
    // without needing to read the marker file.
    let readme = build_redirect_readme(
        &pkgref.qualified_name(),
        &args.to,
        args.description.as_deref(),
    );
    std::fs::write(staging.path().join("README.md"), readme).with_context(|| {
        format!(
            "writing README into stub staging dir `{}`",
            staging.path().display()
        )
    })?;

    ctx.heading(&format!(
        "Creating redirect stub: {} → {}{}",
        pkgref.qualified_name(),
        args.to,
        if args.dry_run { " [dry-run]" } else { "" }
    ));

    if args.dry_run {
        ctx.step(&format!(
            "Would create repository `{stub_repo_name}` on `{host}` (org `{org_segment}`)"
        ));
        ctx.step(&format!(
            "Would write `{}` and README; would push to `{stub_url}`",
            RedirectFile::FILENAME
        ));
        let report = RedirectReport {
            ok: true,
            command: "registry:redirect",
            registry: registry_section.name.clone(),
            pkgref: pkgref.qualified_name(),
            stub_url: stub_url.clone(),
            target_url: args.to.clone(),
            ref_policy: match ref_policy {
                RefPolicy::PassThroughTag => "pass-through-tag",
                RefPolicy::Pinned => "pinned",
            },
            pinned_ref,
            target_auth: target_auth.as_str(),
            created_repo: false,
            dry_run: true,
            sync: None,
        };
        if ctx.is_json() {
            ctx.emit_json(&report)?;
        } else {
            ctx.summary(
                "\nvibe registry redirect [dry-run]: re-run without `--dry-run` to create the stub.",
            );
        }
        return Ok(());
    }

    // GitVerse publish path is a stub today (PROP-002 §2.10 — GitVerse
    // does not expose org-scoped repo creation). Refuse early with the
    // same shape as `vibe registry publish`.
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru") {
        bail!(
            "GitVerse publish is not implemented yet — the GitVerse public API does not expose \
             org-scoped repository creation. Use a GitHub `[[registry]]` for redirect stubs, or \
             create the stub repo by hand and `vibe registry publish --repo-url` content into it."
        );
    }

    let token = load_token_for_host(&host).context("loading publish token")?;
    ctx.step(&format!(
        "Loaded publish token from {} (value redacted)",
        match token.source() {
            vibe_publish::TokenSource::Explicit => "explicit argument".to_string(),
            vibe_publish::TokenSource::EnvVar(name) => format!("$ {name}"),
            vibe_publish::TokenSource::File(p) => p.display().to_string(),
        }
    ));
    let creator = creator_for_url(&registry_section.url, org_segment.clone(), token)
        .map_err(|e| anyhow!("{e}"))?;

    // Refuse to clobber an existing stub — operators who want to update
    // a stub's marker file should hand-edit it (the M1.16 v0 surface).
    let exists = creator
        .repo_exists(&org_segment, &stub_repo_name)
        .map_err(|e| anyhow!("{e}"))?;
    if exists {
        bail!(
            "stub repository `{stub_repo_name}` already exists in `{org_segment}` on `{host}`. \
             Editing an existing redirect stub is a manual procedure for v0 — clone it, edit \
             `{}`, push back. `vibe registry redirect` only handles fresh-stub creation.",
            RedirectFile::FILENAME
        );
    }

    let opts = vibe_publish::CreateOpts {
        description: Some(format!(
            "vibevm registry stub for {} (delegated to {})",
            pkgref.qualified_name(),
            args.to
        )),
        default_branch: Some("main".to_string()),
        homepage: None,
    };
    let _info = creator
        .create_repo(&org_segment, &stub_repo_name, &opts)
        .map_err(|e| anyhow!("{e}"))?;
    ctx.step(&format!(
        "Created repository `{stub_repo_name}` on `{host}`"
    ));

    // Push the stub contents to `main`. Token embedded only at the
    // moment of git invocation; never in stdout / stderr / logs.
    let push_url = creator.push_url(&org_segment, &stub_repo_name);
    let commit_msg = format!("stub: delegate {} to {}", pkgref.qualified_name(), args.to);
    vibe_publish::git_publish::push_initial(staging.path(), &push_url, &commit_msg)
        .map_err(|e| anyhow!("{e}"))?;
    ctx.step(&format!(
        "Pushed stub `{}` to `main`",
        RedirectFile::FILENAME
    ));

    // Optional: sync target tags into the stub immediately.
    let sync_report = if args.sync && matches!(ref_policy, RefPolicy::PassThroughTag) {
        ctx.step("Synchronising target tags into the freshly-created stub");
        Some(do_redirect_sync(
            ctx,
            registry_section,
            &pkgref.qualified_name(),
            &stub_url,
            &args.to,
            &push_url,
            args.dry_run,
        )?)
    } else if args.sync && matches!(ref_policy, RefPolicy::Pinned) {
        ctx.step(
            "Skipping --sync: pinned-policy stubs do not pass through target tags (every \
             consumer resolves to --pinned-ref regardless of stub tag)",
        );
        None
    } else {
        None
    };

    let report = RedirectReport {
        ok: true,
        command: "registry:redirect",
        registry: registry_section.name.clone(),
        pkgref: pkgref.qualified_name(),
        stub_url: stub_url.clone(),
        target_url: args.to.clone(),
        ref_policy: match ref_policy {
            RefPolicy::PassThroughTag => "pass-through-tag",
            RefPolicy::Pinned => "pinned",
        },
        pinned_ref,
        target_auth: target_auth.as_str(),
        created_repo: true,
        dry_run: false,
        sync: sync_report,
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    ctx.summary(&format!(
        "\nvibe registry redirect: stub `{stub_url}` delegates `{}` → `{}`. Consumers \
         resolving `{}` will be redirected to the target transparently. Tag the stub with \
         `git tag vX.Y.Z && git push origin vX.Y.Z` to surface a target version, or run \
         `vibe registry redirect-sync {}` to mirror the target's tag list.",
        pkgref.qualified_name(),
        args.to,
        pkgref.qualified_name(),
        pkgref.qualified_name(),
    ));
    Ok(())
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

pub(super) fn run_redirect_sync(
    ctx: &output::Context,
    args: RegistryRedirectSyncArgs,
) -> Result<()> {
    use vibe_core::PackageRef;

    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let pkgref = PackageRef::parse(&args.pkgref)
        .with_context(|| format!("parsing pkgref `{}`", args.pkgref))?;
    let group = require_group(&pkgref)?;
    let registry_section =
        resolve_target_registry(&manifest, args.registry.as_deref(), &manifest_path)?;
    let host = extract_host_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let org_segment = extract_org_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let stub_repo_name = registry_section
        .naming
        .repo_name(pkgref.kind, group, &pkgref.name)
        .with_context(|| format!("deriving the stub repo name for `{group}/{}`", pkgref.name))?;
    let stub_url = format!(
        "{}/{}",
        registry_section.url.trim_end_matches('/'),
        stub_repo_name
    );

    ctx.heading(&format!(
        "Syncing target tags into stub: {}{}",
        pkgref.qualified_name(),
        if args.dry_run { " [dry-run]" } else { "" }
    ));

    // Load token + build push URL using the same path as `vibe registry
    // redirect`. Read access does not strictly require a token for
    // public registries, but using the credentialed URL when available
    // (e.g. when the registry is `auth = "token-env"`) lets us read
    // private stubs symmetrically.
    let token = load_token_for_host(&host).context("loading publish token")?;
    let creator = creator_for_url(&registry_section.url, org_segment.clone(), token)
        .map_err(|e| anyhow!("{e}"))?;
    let push_url = creator.push_url(&org_segment, &stub_repo_name);

    // Probe stub existence so we fail fast with a clear message.
    let exists = creator
        .repo_exists(&org_segment, &stub_repo_name)
        .map_err(|e| anyhow!("{e}"))?;
    if !exists {
        bail!(
            "stub repository `{stub_repo_name}` does not exist in `{org_segment}` on `{host}`. \
             Run `vibe registry redirect {} --to <target-url>` first to create it.",
            pkgref.qualified_name()
        );
    }

    let report = do_redirect_sync(
        ctx,
        registry_section,
        &pkgref.qualified_name(),
        &stub_url,
        "<read-from-stub>",
        &push_url,
        args.dry_run,
    )?;

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if report.pushed_tags.is_empty() {
        ctx.summary(&format!(
            "\nvibe registry redirect-sync: `{}` is in sync with target. {} tag{} already \
             present on stub.",
            pkgref.qualified_name(),
            report.already_present.len(),
            if report.already_present.len() == 1 {
                ""
            } else {
                "s"
            }
        ));
    } else {
        ctx.summary(&format!(
            "\nvibe registry redirect-sync: pushed {} tag{} into stub `{}`. {} tag{} were \
             already present.",
            report.pushed_tags.len(),
            if report.pushed_tags.len() == 1 {
                ""
            } else {
                "s"
            },
            pkgref.qualified_name(),
            report.already_present.len(),
            if report.already_present.len() == 1 {
                ""
            } else {
                "s"
            }
        ));
    }
    Ok(())
}

pub(super) fn run_redirect_update(
    ctx: &output::Context,
    args: RegistryRedirectUpdateArgs,
) -> Result<()> {
    use vibe_core::PackageRef;
    use vibe_core::manifest::{RedirectFile, RefPolicy};
    use vibe_publish::git_publish;

    // Validate args-level invariants FIRST, before touching the
    // filesystem or network. Operators expect "bad flag combo" to fail
    // with a clear message even when no project is in scope.
    if args.description.is_some() && args.clear_description {
        bail!("--description and --clear-description are mutually exclusive; pass exactly one");
    }

    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let pkgref = PackageRef::parse(&args.pkgref)
        .with_context(|| format!("parsing pkgref `{}`", args.pkgref))?;
    let group = require_group(&pkgref)?;
    let registry_section =
        resolve_target_registry(&manifest, args.registry.as_deref(), &manifest_path)?;
    let host = extract_host_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let org_segment = extract_org_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let stub_repo_name = registry_section
        .naming
        .repo_name(pkgref.kind, group, &pkgref.name)
        .with_context(|| format!("deriving the stub repo name for `{group}/{}`", pkgref.name))?;
    let stub_url = format!(
        "{}/{}",
        registry_section.url.trim_end_matches('/'),
        stub_repo_name
    );

    ctx.heading(&format!(
        "Updating redirect stub: {}{}",
        pkgref.qualified_name(),
        if args.dry_run { " [dry-run]" } else { "" }
    ));

    // GitVerse host refusal — symmetric to redirect-create. v0 only
    // creates / mutates GitHub stubs via the `RepoCreator` machinery.
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru") {
        bail!(
            "GitVerse publish is not implemented yet — the GitVerse public API does not expose \
             org-scoped repository creation. For redirect-update against a GitVerse stub, \
             clone the stub by hand, edit `{}`, push back.",
            RedirectFile::FILENAME
        );
    }

    let token = load_token_for_host(&host).context("loading publish token")?;
    ctx.step(&format!(
        "Loaded publish token from {} (value redacted)",
        match token.source() {
            vibe_publish::TokenSource::Explicit => "explicit argument".to_string(),
            vibe_publish::TokenSource::EnvVar(name) => format!("$ {name}"),
            vibe_publish::TokenSource::File(p) => p.display().to_string(),
        }
    ));
    let creator = creator_for_url(&registry_section.url, org_segment.clone(), token)
        .map_err(|e| anyhow!("{e}"))?;
    let push_url = creator.push_url(&org_segment, &stub_repo_name);

    let exists = creator
        .repo_exists(&org_segment, &stub_repo_name)
        .map_err(|e| anyhow!("{e}"))?;
    if !exists {
        bail!(
            "stub repository `{stub_repo_name}` does not exist in `{org_segment}` on `{host}`. \
             Run `vibe registry redirect {} --to <target-url>` first to create it.",
            pkgref.qualified_name()
        );
    }

    // Shallow-clone the stub so we have a working tree to write the
    // updated marker into and commit_and_push back onto `main`.
    let stub_clone = git_publish::shallow_clone(&push_url).map_err(|e| anyhow!("{e}"))?;
    let marker_path = stub_clone.path().join(RedirectFile::FILENAME);
    if !marker_path.exists() {
        bail!(
            "stub at `{stub_url}` does not carry `{}` at HEAD — is this actually a redirect \
             stub? `vibe registry redirect-update` only operates on stub repos.",
            RedirectFile::FILENAME
        );
    }
    let existing = RedirectFile::read(&marker_path).with_context(|| {
        format!(
            "parsing `{}` from stub `{stub_url}`",
            RedirectFile::FILENAME
        )
    })?;

    let (new_section, changes) = compute_updated_redirect_section(&existing.redirect, &args)?;

    if changes.is_empty() {
        bail!(
            "no changes requested — the computed `{}` is identical to the stub's current marker. \
             Pass at least one of --to / --ref-policy / --pinned-ref / --target-auth / \
             --target-token-env / --description / --clear-description.",
            RedirectFile::FILENAME
        );
    }

    let trust_required = changes.iter().any(|c| c.requires_trust());
    if trust_required && !args.trust_redirect {
        let fields: Vec<&str> = changes
            .iter()
            .filter(|c| c.requires_trust())
            .map(|c| c.field)
            .collect();
        bail!(
            "this update changes `{}` which alters resolution outcomes for every consumer of \
             `{}`. Pass `--trust-redirect` to confirm a deliberate switch (PROP-002 §2.4.2 trust \
             model — never silent, always operator-initiated).",
            fields.join("`, `"),
            pkgref.qualified_name()
        );
    }

    // Build the report shape (used for --json and the dry-run path
    // before we exit; for a real apply it's emitted at the end).
    let new_target_url = new_section.target_url.clone();
    let new_ref_policy_str: &'static str = match new_section.ref_policy {
        RefPolicy::PassThroughTag => "pass-through-tag",
        RefPolicy::Pinned => "pinned",
    };
    let new_target_auth_str: &'static str = new_section.auth.as_str();
    let new_pinned_ref = new_section.pinned_ref.clone();

    for c in &changes {
        ctx.step(&format!(
            "{}: {} → {}",
            c.field,
            c.before.as_deref().unwrap_or("<unset>"),
            c.after.as_deref().unwrap_or("<unset>"),
        ));
    }

    if args.dry_run {
        let report = RedirectUpdateReport {
            ok: true,
            command: "registry:redirect-update",
            registry: registry_section.name.clone(),
            pkgref: pkgref.qualified_name(),
            stub_url: stub_url.clone(),
            target_url: new_target_url,
            ref_policy: new_ref_policy_str,
            pinned_ref: new_pinned_ref,
            target_auth: new_target_auth_str,
            changes,
            trust_required,
            dry_run: true,
            sync: None,
        };
        if ctx.is_json() {
            ctx.emit_json(&report)?;
        } else {
            ctx.summary(
                "\nvibe registry redirect-update [dry-run]: re-run without `--dry-run` to push \
                 the rewritten marker.",
            );
        }
        return Ok(());
    }

    // Write new marker + regenerate README in the existing clone. Both
    // files are full rewrites; git status -s after `git add -A` tells
    // us whether anything actually changed on disk (commit_and_push
    // bails if not).
    let new_file = RedirectFile {
        redirect: new_section,
    };
    new_file
        .write(&marker_path)
        .with_context(|| format!("writing `{}`", marker_path.display()))?;
    let readme = build_redirect_readme(
        &pkgref.qualified_name(),
        &new_file.redirect.target_url,
        new_file.redirect.description.as_deref(),
    );
    std::fs::write(stub_clone.path().join("README.md"), readme).with_context(|| {
        format!(
            "writing README into stub clone `{}`",
            stub_clone.path().display()
        )
    })?;

    let commit_msg = build_redirect_update_commit_msg(&pkgref.qualified_name(), &changes);
    git_publish::commit_and_push(stub_clone.path(), &push_url, &commit_msg)
        .map_err(|e| anyhow!("{e}"))?;
    ctx.step(&format!(
        "Pushed updated `{}` to `main`",
        RedirectFile::FILENAME
    ));

    let sync_report =
        if args.resync && matches!(new_file.redirect.ref_policy, RefPolicy::PassThroughTag) {
            ctx.step("Re-syncing target tags into the updated stub");
            Some(do_redirect_sync(
                ctx,
                registry_section,
                &pkgref.qualified_name(),
                &stub_url,
                &new_file.redirect.target_url,
                &push_url,
                false,
            )?)
        } else if args.resync && matches!(new_file.redirect.ref_policy, RefPolicy::Pinned) {
            ctx.step("Skipping --resync: pinned-policy stubs do not pass through target tags");
            None
        } else {
            None
        };

    let report = RedirectUpdateReport {
        ok: true,
        command: "registry:redirect-update",
        registry: registry_section.name.clone(),
        pkgref: pkgref.qualified_name(),
        stub_url: stub_url.clone(),
        target_url: new_file.redirect.target_url.clone(),
        ref_policy: new_ref_policy_str,
        pinned_ref: new_file.redirect.pinned_ref.clone(),
        target_auth: new_target_auth_str,
        changes,
        trust_required,
        dry_run: false,
        sync: sync_report,
    };
    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    ctx.summary(&format!(
        "\nvibe registry redirect-update: rewrote `{}` on stub `{stub_url}`. Consumers \
         resolving `{}` now see the new marker.",
        RedirectFile::FILENAME,
        pkgref.qualified_name(),
    ));
    Ok(())
}

/// Merge a [`RegistryRedirectUpdateArgs`] flag set into an existing
/// `[redirect]` section. Returns the new section and a list of changed
/// fields. Validates flag combinations (mutual exclusion already
/// checked by the caller) and any cross-field invariants (pinned policy
/// requires pinned_ref; token_env only meaningful with token-env auth).
///
/// Switching `auth` away from `token-env` clears `token_env`
/// automatically — keeping it would be a parse error on the next read.
/// Switching `ref_policy` to `pass-through-tag` clears `pinned_ref` for
/// the same reason.
fn compute_updated_redirect_section(
    current: &vibe_core::manifest::RedirectSection,
    args: &RegistryRedirectUpdateArgs,
) -> Result<(
    vibe_core::manifest::RedirectSection,
    Vec<RedirectChangeEntry>,
)> {
    use vibe_core::manifest::{AuthKind, RedirectSection, RefPolicy};

    // target_url
    let new_target_url = match &args.to {
        Some(t) => {
            if t.trim().is_empty() {
                bail!("--to must be a non-empty git URL");
            }
            t.clone()
        }
        None => current.target_url.clone(),
    };

    // ref_policy
    let new_ref_policy = match args.ref_policy.as_deref() {
        None => current.ref_policy,
        Some("pass-through-tag") => RefPolicy::PassThroughTag,
        Some("pinned") => RefPolicy::Pinned,
        Some(other) => {
            bail!("unknown --ref-policy `{other}` — must be `pass-through-tag` or `pinned`")
        }
    };

    // pinned_ref — depends on new_ref_policy
    let new_pinned_ref = match new_ref_policy {
        RefPolicy::PassThroughTag => {
            if args.pinned_ref.is_some() {
                bail!(
                    "--pinned-ref is only meaningful with `--ref-policy pinned`; drop it or pass \
                     `--ref-policy pinned`"
                );
            }
            // Switching to (or staying at) pass-through clears any
            // previously-set pinned_ref — the marker would otherwise be
            // rejected at parse.
            None
        }
        RefPolicy::Pinned => {
            // Prefer the explicit flag; fall back to the existing
            // pinned_ref iff the policy already was pinned. Switching
            // from pass-through to pinned without --pinned-ref is a
            // hard error since there is nothing to preserve.
            match args.pinned_ref.as_deref() {
                Some(r) => Some(r.to_string()),
                None => match current.ref_policy {
                    RefPolicy::Pinned => current.pinned_ref.clone(),
                    RefPolicy::PassThroughTag => bail!(
                        "switching to `--ref-policy pinned` requires `--pinned-ref \
                         <tag/branch/rev>` (no existing pinned ref to preserve)"
                    ),
                },
            }
        }
    };

    // auth
    let new_auth = match args.target_auth.as_deref() {
        None => current.auth,
        Some(s) => parse_target_auth(Some(s))?,
    };

    // token_env — only meaningful with TokenEnv
    let new_token_env = match new_auth {
        AuthKind::TokenEnv => match &args.target_token_env {
            Some(name) if name.trim().is_empty() => {
                bail!("--target-token-env must be a non-empty env-var name")
            }
            Some(name) => Some(name.clone()),
            None => current.token_env.clone(),
        },
        _ => {
            if args.target_token_env.is_some() {
                bail!("--target-token-env is only meaningful with --target-auth token-env");
            }
            None
        }
    };

    // description
    let new_description = if args.clear_description {
        None
    } else if let Some(d) = &args.description {
        Some(d.clone())
    } else {
        current.description.clone()
    };

    let new_section = RedirectSection {
        target_url: new_target_url,
        ref_policy: new_ref_policy,
        pinned_ref: new_pinned_ref,
        auth: new_auth,
        token_env: new_token_env,
        description: new_description,
    };

    let changes = diff_redirect_sections(current, &new_section);
    Ok((new_section, changes))
}

fn diff_redirect_sections(
    before: &vibe_core::manifest::RedirectSection,
    after: &vibe_core::manifest::RedirectSection,
) -> Vec<RedirectChangeEntry> {
    use vibe_core::manifest::RefPolicy;

    let mut out: Vec<RedirectChangeEntry> = Vec::new();
    if before.target_url != after.target_url {
        out.push(RedirectChangeEntry {
            field: "target_url",
            before: Some(before.target_url.clone()),
            after: Some(after.target_url.clone()),
        });
    }
    if before.ref_policy != after.ref_policy {
        let pol = |p: RefPolicy| -> &'static str {
            match p {
                RefPolicy::PassThroughTag => "pass-through-tag",
                RefPolicy::Pinned => "pinned",
            }
        };
        out.push(RedirectChangeEntry {
            field: "ref_policy",
            before: Some(pol(before.ref_policy).to_string()),
            after: Some(pol(after.ref_policy).to_string()),
        });
    }
    if before.pinned_ref != after.pinned_ref {
        out.push(RedirectChangeEntry {
            field: "pinned_ref",
            before: before.pinned_ref.clone(),
            after: after.pinned_ref.clone(),
        });
    }
    if before.auth != after.auth {
        out.push(RedirectChangeEntry {
            field: "auth",
            before: Some(before.auth.as_str().to_string()),
            after: Some(after.auth.as_str().to_string()),
        });
    }
    if before.token_env != after.token_env {
        out.push(RedirectChangeEntry {
            field: "token_env",
            before: before.token_env.clone(),
            after: after.token_env.clone(),
        });
    }
    if before.description != after.description {
        out.push(RedirectChangeEntry {
            field: "description",
            before: before.description.clone(),
            after: after.description.clone(),
        });
    }
    out
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

fn build_redirect_update_commit_msg(pkgref: &str, changes: &[RedirectChangeEntry]) -> String {
    if let Some(c) = changes.iter().find(|c| c.field == "target_url")
        && let Some(after) = &c.after
    {
        return format!("stub: retarget {pkgref} to {after}");
    }
    let fields: Vec<&str> = changes.iter().map(|c| c.field).collect();
    format!("stub: update marker for {pkgref} ({})", fields.join(", "))
}

/// Inner sync logic — shared by `vibe registry redirect --sync` and
/// `vibe registry redirect-sync`. Reads the stub's `vibe-redirect.toml`,
/// enumerates target tags, pushes the missing ones into the stub.
fn do_redirect_sync(
    ctx: &output::Context,
    registry_section: &RegistrySection,
    pkgref_qualified: &str,
    stub_url: &str,
    target_url_hint: &str,
    push_url: &str,
    dry_run: bool,
) -> Result<RedirectSyncReport> {
    use vibe_core::manifest::{RedirectFile, RefPolicy};
    use vibe_publish::git_publish;

    // Step 1: shallow-clone the stub so we can read the marker file
    // and have a working tree to anchor new tags onto.
    let stub_clone = git_publish::shallow_clone(push_url).map_err(|e| anyhow!("{e}"))?;
    let marker_path = stub_clone.path().join(RedirectFile::FILENAME);
    if !marker_path.exists() {
        bail!(
            "stub at `{stub_url}` does not carry `{}` at HEAD — is this actually a redirect \
             stub? `vibe registry redirect-sync` only operates on stub repos.",
            RedirectFile::FILENAME
        );
    }
    let stub_file = RedirectFile::read(&marker_path).with_context(|| {
        format!(
            "parsing `{}` from stub `{stub_url}`",
            RedirectFile::FILENAME
        )
    })?;

    // Pinned policy — stub tags don't pass through, so syncing is a
    // semantic mistake.
    if matches!(stub_file.redirect.ref_policy, RefPolicy::Pinned) {
        bail!(
            "stub `{stub_url}` uses `ref_policy = \"pinned\"` — every consumer resolves to \
             `pinned_ref = {:?}` regardless of stub tag, so there is nothing to sync. Edit \
             `{}` to change the policy if you want pass-through behaviour.",
            stub_file.redirect.pinned_ref.as_deref().unwrap_or(""),
            RedirectFile::FILENAME
        );
    }

    let target_url = stub_file.redirect.target_url.clone();
    if target_url_hint != "<read-from-stub>" && target_url_hint != target_url {
        // The CLI surface (`--to`) only matches the stub on `redirect`
        // since `redirect-sync` reads from the stub itself. The hint
        // disagreeing is a sanity check, not a hard error — log it.
        tracing::debug!(
            target: "vibe_cli::registry::redirect_sync",
            "target_url hint `{target_url_hint}` disagrees with stub-stored `{target_url}`; using stub"
        );
    }

    // Step 2: build a target-side fetch URL with credentials if the
    // stub declares `auth = "token-env"`. Public targets need no token.
    let target_fetch_url = build_target_fetch_url(&target_url, &stub_file.redirect)?;

    // Step 3: list tags on both sides.
    let target_tags = git_publish::ls_remote_tags(&target_fetch_url).map_err(|e| anyhow!("{e}"))?;
    // For listing stub tags we use `git ls-remote` directly so we do
    // not depend on the shallow clone having all refs (it does, by
    // virtue of `--single-branch`, but ls-remote is the source of truth).
    let stub_tags = git_publish::ls_remote_tags(push_url).map_err(|e| anyhow!("{e}"))?;

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
            ctx.step(&format!(
                "Would push tag `{t}` (target has it; stub does not)"
            ));
        }
        for t in &already {
            ctx.skipped(&format!("tag `{t}`"), "already present on stub");
        }
        return Ok(RedirectSyncReport {
            ok: true,
            command: "registry:redirect-sync",
            registry: registry_section.name.clone(),
            pkgref: pkgref_qualified.to_string(),
            stub_url: stub_url.to_string(),
            target_url,
            pushed_tags: to_push,
            already_present: already,
            dry_run: true,
        });
    }

    // Step 5: push the missing tags. Each tag is annotated, anchored
    // at the stub's `main` commit. Stubs are flat — tag → marker file
    // — so the commit is identical regardless of which target tag the
    // stub tag fronts.
    for t in &to_push {
        git_publish::push_tag_only(stub_clone.path(), push_url, t).map_err(|e| anyhow!("{e}"))?;
        ctx.step(&format!("Pushed tag `{t}` into stub"));
    }
    for t in &already {
        ctx.skipped(&format!("tag `{t}`"), "already present on stub");
    }

    Ok(RedirectSyncReport {
        ok: true,
        command: "registry:redirect-sync",
        registry: registry_section.name.clone(),
        pkgref: pkgref_qualified.to_string(),
        stub_url: stub_url.to_string(),
        target_url,
        pushed_tags: to_push,
        already_present: already,
        dry_run: false,
    })
}

/// Build a fetch URL for the target side of a redirect, applying
/// `[redirect].auth` if it asks for token-based auth. For `auth = "none"`
/// this returns the URL verbatim; for `auth = "token-env"` it injects
/// the resolved token using the same shape M1.14 plumbing applies
/// (`https://x-access-token:<TOKEN>@host/...`). Other auth regimes
/// (`credential-helper`, `ssh`) trust the local git's auth path.
fn build_target_fetch_url(
    target_url: &str,
    redirect: &vibe_core::manifest::RedirectSection,
) -> Result<String> {
    use vibe_core::manifest::AuthKind;
    match redirect.auth {
        AuthKind::None | AuthKind::CredentialHelper | AuthKind::Ssh => Ok(target_url.to_string()),
        AuthKind::TokenEnv => {
            let env_name = redirect
                .token_env
                .clone()
                .or_else(|| derive_target_token_env(target_url))
                .ok_or_else(|| {
                    anyhow!(
                        "target URL `{target_url}` declares auth = \"token-env\" but no \
                         `token_env` is set and the host cannot be derived for a default \
                         env-var name"
                    )
                })?;
            let value = std::env::var(&env_name).map_err(|_| {
                anyhow!(
                    "target URL `{target_url}` declares auth = \"token-env\" with env-var \
                     `{env_name}` but the variable is unset or empty in this shell"
                )
            })?;
            Ok(inject_token_into_url(target_url, &value))
        }
    }
}

fn derive_target_token_env(target_url: &str) -> Option<String> {
    let host = extract_host_segment(target_url).ok()?;
    let upper = host.to_ascii_uppercase().replace(['.', '-'], "_");
    Some(format!("VIBEVM_TARGET_TOKEN_{upper}"))
}

fn inject_token_into_url(url: &str, token: &str) -> String {
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
    use super::{
        build_redirect_readme, build_redirect_update_commit_msg, build_target_fetch_url,
        compute_updated_redirect_section, derive_target_token_env, diff_redirect_sections,
        inject_token_into_url, parse_target_auth,
    };
    use crate::cli::RegistryRedirectUpdateArgs;
    use std::path::PathBuf;
    use vibe_core::manifest::{AuthKind, RedirectSection, RefPolicy};

    // -----------------------------------------------------------------
    // redirect / redirect-sync helpers (PROP-002 §2.4.2)
    // -----------------------------------------------------------------

    #[test]
    fn parse_target_auth_canonical_spellings() {
        assert!(matches!(parse_target_auth(None).unwrap(), AuthKind::None));
        assert!(matches!(
            parse_target_auth(Some("none")).unwrap(),
            AuthKind::None
        ));
        assert!(matches!(
            parse_target_auth(Some("token-env")).unwrap(),
            AuthKind::TokenEnv
        ));
        assert!(matches!(
            parse_target_auth(Some("credential-helper")).unwrap(),
            AuthKind::CredentialHelper
        ));
        assert!(matches!(
            parse_target_auth(Some("ssh")).unwrap(),
            AuthKind::Ssh
        ));
    }

    #[test]
    fn parse_target_auth_rejects_unknown() {
        let err = parse_target_auth(Some("oauth")).unwrap_err();
        assert!(err.to_string().contains("unknown --target-auth"));
    }

    #[test]
    fn build_redirect_readme_includes_pkgref_and_target() {
        let r = build_redirect_readme(
            "flow:internal-helper",
            "https://gitlab.acme.example/flows/internal-helper",
            None,
        );
        assert!(r.contains("flow:internal-helper"));
        assert!(r.contains("https://gitlab.acme.example/flows/internal-helper"));
        assert!(r.contains("vibe-redirect.toml"));
    }

    #[test]
    fn build_redirect_readme_includes_description_when_present() {
        let r = build_redirect_readme(
            "flow:x",
            "https://example.invalid/x",
            Some("delegated to acme-corp"),
        );
        assert!(r.contains("delegated to acme-corp"));
    }

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

    // -----------------------------------------------------------------
    // compute_updated_redirect_section + helpers — partial update for
    // `vibe registry redirect-update` (PROP-002 §2.4.2)
    // -----------------------------------------------------------------

    fn baseline_pass_through() -> RedirectSection {
        RedirectSection {
            target_url: "https://github.com/old/flow-wal".into(),
            ref_policy: RefPolicy::PassThroughTag,
            pinned_ref: None,
            auth: AuthKind::None,
            token_env: None,
            description: Some("old description".into()),
        }
    }

    fn baseline_pinned() -> RedirectSection {
        RedirectSection {
            target_url: "https://github.com/old/flow-wal".into(),
            ref_policy: RefPolicy::Pinned,
            pinned_ref: Some("v0.3.0".into()),
            auth: AuthKind::None,
            token_env: None,
            description: None,
        }
    }

    fn empty_update_args() -> RegistryRedirectUpdateArgs {
        RegistryRedirectUpdateArgs {
            pkgref: "flow:wal".into(),
            to: None,
            registry: None,
            ref_policy: None,
            pinned_ref: None,
            target_auth: None,
            target_token_env: None,
            description: None,
            clear_description: false,
            trust_redirect: false,
            resync: false,
            path: PathBuf::from("."),
            dry_run: false,
        }
    }

    #[test]
    fn compute_update_only_description_change_detected() {
        let mut args = empty_update_args();
        args.description = Some("new description".into());
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert_eq!(new.description.as_deref(), Some("new description"));
        assert_eq!(new.target_url, "https://github.com/old/flow-wal");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "description");
        assert!(!changes.iter().any(|c| c.requires_trust()));
    }

    #[test]
    fn compute_update_clear_description_drops_field() {
        let mut args = empty_update_args();
        args.clear_description = true;
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert_eq!(new.description, None);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "description");
        assert_eq!(changes[0].before.as_deref(), Some("old description"));
        assert_eq!(changes[0].after, None);
    }

    #[test]
    fn compute_update_target_url_change_flags_trust() {
        let mut args = empty_update_args();
        args.to = Some("https://forgejo.example/x/y".into());
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert_eq!(new.target_url, "https://forgejo.example/x/y");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "target_url");
        assert!(changes[0].requires_trust());
    }

    #[test]
    fn compute_update_switch_to_pinned_requires_pinned_ref() {
        let mut args = empty_update_args();
        args.ref_policy = Some("pinned".into());
        // No --pinned-ref, no current pinned_ref → reject.
        let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
        assert!(
            err.to_string().contains("requires `--pinned-ref"),
            "expected pinned-ref-required hint, got: {err}"
        );
    }

    #[test]
    fn compute_update_switch_to_pinned_uses_explicit_ref() {
        let mut args = empty_update_args();
        args.ref_policy = Some("pinned".into());
        args.pinned_ref = Some("v1.2.3".into());
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert!(matches!(new.ref_policy, RefPolicy::Pinned));
        assert_eq!(new.pinned_ref.as_deref(), Some("v1.2.3"));
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.requires_trust()));
    }

    #[test]
    fn compute_update_switch_to_pass_through_clears_pinned_ref() {
        let mut args = empty_update_args();
        args.ref_policy = Some("pass-through-tag".into());
        let (new, changes) = compute_updated_redirect_section(&baseline_pinned(), &args).unwrap();
        assert!(matches!(new.ref_policy, RefPolicy::PassThroughTag));
        assert_eq!(new.pinned_ref, None);
        // Two changes: ref_policy + pinned_ref (was Some, now None).
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.requires_trust()));
    }

    #[test]
    fn compute_update_pinned_ref_alone_on_pinned_stub() {
        let mut args = empty_update_args();
        args.pinned_ref = Some("v0.4.0".into());
        // Current is pinned with v0.3.0 — flag bumps to v0.4.0 without
        // touching policy.
        let (new, changes) = compute_updated_redirect_section(&baseline_pinned(), &args).unwrap();
        assert!(matches!(new.ref_policy, RefPolicy::Pinned));
        assert_eq!(new.pinned_ref.as_deref(), Some("v0.4.0"));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "pinned_ref");
        assert!(changes[0].requires_trust());
    }

    #[test]
    fn compute_update_rejects_pinned_ref_on_pass_through() {
        let mut args = empty_update_args();
        args.pinned_ref = Some("v1.0.0".into());
        let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
        assert!(err.to_string().contains("--pinned-ref is only meaningful"));
    }

    #[test]
    fn compute_update_auth_flip_clears_token_env() {
        let with_token = RedirectSection {
            target_url: "https://x/y".into(),
            ref_policy: RefPolicy::PassThroughTag,
            pinned_ref: None,
            auth: AuthKind::TokenEnv,
            token_env: Some("VIBEVM_TARGET_TOKEN_X".into()),
            description: None,
        };
        let mut args = empty_update_args();
        args.target_auth = Some("none".into());
        let (new, changes) = compute_updated_redirect_section(&with_token, &args).unwrap();
        assert!(matches!(new.auth, AuthKind::None));
        assert_eq!(new.token_env, None);
        // Both auth and token_env appear in the diff — operator-side
        // metadata, not trust-required.
        assert!(changes.iter().any(|c| c.field == "auth"));
        assert!(changes.iter().any(|c| c.field == "token_env"));
        assert!(!changes.iter().any(|c| c.requires_trust()));
    }

    #[test]
    fn compute_update_rejects_token_env_without_matching_auth() {
        let mut args = empty_update_args();
        args.target_token_env = Some("WHATEVER".into());
        // Current auth is None; flag not provided → token_env not
        // meaningful.
        let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
        assert!(
            err.to_string()
                .contains("--target-token-env is only meaningful")
        );
    }

    #[test]
    fn compute_update_rejects_empty_to() {
        let mut args = empty_update_args();
        args.to = Some("   ".into());
        let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
        assert!(err.to_string().contains("--to must be a non-empty"));
    }

    #[test]
    fn compute_update_no_op_returns_empty_changes() {
        let args = empty_update_args();
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert_eq!(new, baseline_pass_through());
        assert!(changes.is_empty());
    }

    #[test]
    fn diff_redirect_sections_emits_field_names_in_canonical_order() {
        let before = baseline_pass_through();
        let mut after = baseline_pass_through();
        after.target_url = "new".into();
        after.description = Some("new".into());
        let changes = diff_redirect_sections(&before, &after);
        // Canonical iteration order: target_url, ref_policy, pinned_ref,
        // auth, token_env, description. With two fields touched the
        // order must be target_url first, description last.
        assert_eq!(changes[0].field, "target_url");
        assert_eq!(changes[1].field, "description");
    }

    #[test]
    fn redirect_update_commit_msg_highlights_target_url_change() {
        let changes = vec![
            super::RedirectChangeEntry {
                field: "target_url",
                before: Some("https://old/x".into()),
                after: Some("https://new/x".into()),
            },
            super::RedirectChangeEntry {
                field: "description",
                before: None,
                after: Some("delegated".into()),
            },
        ];
        let msg = build_redirect_update_commit_msg("flow:wal", &changes);
        assert!(msg.contains("retarget flow:wal"));
        assert!(msg.contains("https://new/x"));
    }

    #[test]
    fn redirect_update_commit_msg_lists_fields_when_no_target_change() {
        let changes = vec![
            super::RedirectChangeEntry {
                field: "auth",
                before: Some("none".into()),
                after: Some("token-env".into()),
            },
            super::RedirectChangeEntry {
                field: "token_env",
                before: None,
                after: Some("VAR".into()),
            },
        ];
        let msg = build_redirect_update_commit_msg("flow:wal", &changes);
        assert!(msg.contains("update marker for flow:wal"));
        assert!(msg.contains("auth, token_env"));
    }
}
