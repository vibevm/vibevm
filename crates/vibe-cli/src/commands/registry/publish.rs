//! `vibe registry publish` — publish a package to its registry host, or
//! push directly to a known git URL via `--repo-url`.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::manifest::{
    DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_URL, Manifest, NamingConvention,
};
use vibe_publish::{
    DirectGitCreator, PublishConfig, Publisher, creator_for_url, extract_host_segment,
    extract_org_segment, load_token_for_host,
};

use crate::cli::RegistryPublishArgs;
use crate::output;

use super::resolve_project_root;

#[derive(Debug, Serialize)]
struct PublishReport {
    ok: bool,
    command: &'static str,
    host: String,
    org_url: String,
    repo_name: String,
    repo_url: String,
    tag: String,
    created_repo: bool,
    dry_run: bool,
    /// Status of the optional post-publish index hook. Always
    /// present; `fired = false` + `error = None` means the hook was
    /// dormant (no env config) and the operator wanted no index update.
    #[serde(skip_serializing_if = "Option::is_none")]
    index_hook: Option<vibe_publish::HookReport>,
}

/// Envelope emitted when the operator targets a host whose publish path
/// is intentionally a stub (today: GitVerse — see `run_publish`). Marked
/// `ok: false` so CI / scripting can distinguish stub-paths from a
/// successful publish without parsing the message.
#[derive(Debug, Serialize)]
struct PublishStubReport {
    ok: bool,
    command: &'static str,
    host: String,
    org_url: String,
    registry: String,
    stub: bool,
    reason: String,
}

/// Envelope emitted on the `--repo-url` no-API path. The shape mirrors
/// [`PublishReport`] minus fields that don't apply (no `org_url` because
/// the URL is repo-level; no `created_repo` because direct-push never
/// provisions). `mode = "direct-git"` lets consumers distinguish this
/// path from the registry path without parsing host strings.
#[derive(Debug, Serialize)]
struct DirectPublishReport {
    ok: bool,
    command: &'static str,
    mode: &'static str,
    host: String,
    repo_url: String,
    repo_name: String,
    tag: String,
    dry_run: bool,
}

pub(super) fn run_publish(ctx: &output::Context, args: RegistryPublishArgs) -> Result<()> {
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

    // `--repo-url <url>`: bypass registries, host adapters, tokens, and
    // every host API. Operator supplied an SSH/HTTPS URL pointing at an
    // already-provisioned repo; we just push the package contents +
    // tag. Local git resolves credentials however it normally does
    // (SSH agent / credential.helper / netrc). No `[[registry]]` entry
    // is consulted; `vibe.toml` need only exist (asserted above so the
    // command behaves consistently w.r.t. project-root lookup).
    if let Some(direct_url) = args.repo_url.as_deref() {
        return run_publish_direct(ctx, &args, direct_url);
    }

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` entries in `{}`. `vibe registry publish` needs a target registry, \
             or pass `--repo-url <git-url>` to push directly to a known repo without an API call.",
            manifest_path.display()
        );
    }

    let registry_section = match &args.registry {
        Some(name) => manifest.registry_by_name(name).ok_or_else(|| {
            anyhow!(
                "no `[[registry]]` named `{name}` in `{}`",
                manifest_path.display()
            )
        })?,
        None => manifest
            .primary_registry()
            .ok_or_else(|| anyhow!("no `[[registry]]` configured"))?,
    };

    // Canonicalise the source dir.
    let source_dir = args
        .source
        .canonicalize()
        .with_context(|| format!("source path `{}`", args.source.display()))?;
    let source_dir = crate::commands::init::strip_unc_public(source_dir);

    // Pick the host adapter from the registry URL's host segment per
    // PROP-002 §2.10. `creator_for_url` returns a boxed `RepoCreator`
    // already scoped to the configured org; that's the boundary that
    // enforces "never operate outside the configured organization"
    // per PROP-000 §20. Each adapter additionally validates the org
    // at every method call as defence in depth.
    let host = extract_host_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let org_segment = extract_org_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;

    // GitVerse publish is currently a stub. The GitVerse public REST API
    // does not yet expose org-scoped repo creation (`POST /orgs/<org>/repos`
    // returns no parity for the GitHub flow `vibe registry publish`
    // depends on). Short-circuit here with a clear console message and
    // a JSON envelope marked `stub: true`, so consumers learn the
    // limitation before any token is loaded or any network call is made.
    // GitHub stays the canonical publish target; resolve-time reads
    // against GitVerse continue to work via `MultiRegistryResolver`.
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru") {
        let reason = format!(
            "GitVerse publishing is not implemented yet — the GitVerse public API does not \
             expose org-scoped repository creation, so `vibe registry publish` cannot drive \
             the create-repo + push-tag flow end to end. Publish to a GitHub `[[registry]]` \
             instead (default: `{}` → `{}`), or run with `--registry <name>` to pick a \
             different target. Resolve-time reads against `{}` are unaffected.",
            DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_URL, registry_section.name
        );
        if ctx.is_json() {
            ctx.emit_json(&PublishStubReport {
                ok: false,
                command: "registry:publish",
                host: host.clone(),
                org_url: registry_section.url.clone(),
                registry: registry_section.name.clone(),
                stub: true,
                reason,
            })?;
        } else {
            ctx.heading(&format!(
                "Publishing {} → registry `{}` (`{}`)",
                source_dir.display(),
                registry_section.name,
                registry_section.url,
            ));
            ctx.summary(&format!("\nvibe registry publish: {reason}"));
        }
        return Ok(());
    }

    ctx.heading(&format!(
        "Publishing {} → registry `{}` (`{}`){}",
        source_dir.display(),
        registry_section.name,
        registry_section.url,
        if args.dry_run { " [dry-run]" } else { "" },
    ));

    let token = load_token_for_host(&host).context("loading publish token")?;
    // The CLI surfaces the *source* of the token (env var, file path),
    // never the value. Token::Display redacts to `***` defensively in
    // case any future code path reaches for it.
    ctx.step(&format!(
        "Loaded publish token from {} (value redacted)",
        match token.source() {
            vibe_publish::TokenSource::Explicit => "explicit argument".to_string(),
            vibe_publish::TokenSource::EnvVar(name) => format!("$ {name}"),
            vibe_publish::TokenSource::File(p) => p.display().to_string(),
        }
    ));
    let creator =
        creator_for_url(&registry_section.url, org_segment, token).map_err(|e| anyhow!("{e}"))?;

    let config = PublishConfig {
        source_dir: source_dir.clone(),
        org_url: registry_section.url.clone(),
        naming: registry_section.naming,
        tag_prefix: "v".to_string(),
        dry_run: args.dry_run,
    };

    let outcome = Publisher::new(creator.as_ref())
        .publish(&config)
        .map_err(|e| anyhow!("{e}"))?;

    // Optional post-publish hook — POST the freshly-built entry to a
    // configured vibevm-index server. Activation is per-registry via
    // env vars; the hook stays dormant when either VIBEVM_INDEX_URL_<R>
    // or VIBEVM_INDEX_TOKEN_<R> is unset. Hook failures are warnings,
    // never fail the publish itself (PROP-005 §2.14).
    let hook_report = if outcome.dry_run {
        // Dry-runs do not push real bytes; suppress the hook.
        vibe_publish::HookReport::dormant()
    } else {
        vibe_publish::fire_index_hook(&outcome, &source_dir, &registry_section.name)
    };

    if ctx.is_json() {
        ctx.emit_json(&PublishReport {
            ok: true,
            command: "registry:publish",
            host: outcome.host.clone(),
            org_url: registry_section.url.clone(),
            repo_name: outcome.repo_name.clone(),
            repo_url: outcome.repo_url.clone(),
            tag: outcome.tag.clone(),
            created_repo: outcome.created_repo,
            dry_run: outcome.dry_run,
            index_hook: Some(hook_report),
        })?;
        return Ok(());
    }
    if hook_report.fired {
        ctx.step(&format!(
            "Index hook posted to {} (status {})",
            hook_report.url_endpoint.as_deref().unwrap_or("(unknown)"),
            hook_report.status.unwrap_or(0)
        ));
    } else if let Some(err) = &hook_report.error {
        tracing::warn!(target: "vibe_cli::registry::publish", "index hook skipped: {err}");
    }

    let action_verb = if outcome.dry_run {
        if outcome.created_repo {
            "Would create"
        } else {
            "Would reuse existing"
        }
    } else if outcome.created_repo {
        "Created"
    } else {
        "Reusing existing"
    };
    ctx.step(&format!(
        "{} repository `{}` on `{}`",
        action_verb, outcome.repo_name, outcome.host
    ));
    if outcome.dry_run {
        ctx.summary(&format!(
            "\nvibe registry publish [dry-run]: would push to `{}` and tag `{}`. \
             Re-run without `--dry-run` to apply.",
            outcome.repo_url, outcome.tag
        ));
    } else {
        ctx.summary(&format!(
            "\nvibe registry publish: pushed `{}:{}` @ {} → `{}` (tag `{}`).",
            outcome.kind, outcome.name, outcome.version, outcome.repo_url, outcome.tag
        ));
    }
    Ok(())
}

/// Execute the no-API publish path. Builds a [`DirectGitCreator`] for
/// the supplied URL, threads it through the regular [`Publisher`] flow
/// — which short-circuits at `direct_repo_url` — and renders the
/// outcome. No token loading, no host-API call.
fn run_publish_direct(
    ctx: &output::Context,
    args: &RegistryPublishArgs,
    direct_url: &str,
) -> Result<()> {
    let url = direct_url.trim();
    if url.is_empty() {
        bail!("--repo-url must be a non-empty git URL");
    }

    let source_dir = args
        .source
        .canonicalize()
        .with_context(|| format!("source path `{}`", args.source.display()))?;
    let source_dir = crate::commands::init::strip_unc_public(source_dir);

    ctx.heading(&format!(
        "Publishing {} → direct git URL `{}`{}",
        source_dir.display(),
        url,
        if args.dry_run { " [dry-run]" } else { "" },
    ));
    ctx.step("No host API in play — pushing with local git credentials.");

    let creator = DirectGitCreator::new(url.to_string());
    // `org_url` and `naming` are irrelevant on the direct path —
    // [`Publisher::publish`] short-circuits before consulting them.
    // Pass through harmless placeholders so the config validates.
    let config = PublishConfig {
        source_dir: source_dir.clone(),
        org_url: url.to_string(),
        naming: NamingConvention::default(),
        tag_prefix: "v".to_string(),
        dry_run: args.dry_run,
    };

    let outcome = Publisher::new(&creator)
        .publish(&config)
        .map_err(|e| anyhow!("{e}"))?;

    if ctx.is_json() {
        ctx.emit_json(&DirectPublishReport {
            ok: true,
            command: "registry:publish",
            mode: "direct-git",
            host: outcome.host.clone(),
            repo_url: outcome.repo_url.clone(),
            repo_name: outcome.repo_name.clone(),
            tag: outcome.tag.clone(),
            dry_run: outcome.dry_run,
        })?;
        return Ok(());
    }

    if outcome.dry_run {
        ctx.summary(&format!(
            "\nvibe registry publish [dry-run]: would push `{}:{}` @ {} → `{}` (tag `{}`). \
             Re-run without `--dry-run` to apply.",
            outcome.kind, outcome.name, outcome.version, outcome.repo_url, outcome.tag,
        ));
    } else {
        ctx.summary(&format!(
            "\nvibe registry publish: pushed `{}:{}` @ {} → `{}` (tag `{}`).",
            outcome.kind, outcome.name, outcome.version, outcome.repo_url, outcome.tag,
        ));
    }
    Ok(())
}
