//! `vibe registry redirect` — fresh-stub creation (PROP-002 §2.4.2).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::Manifest;
use vibe_publish::{
    creator_for_url, extract_host_segment, extract_org_segment, load_token_for_host,
};

use crate::cli::RegistryRedirectArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::sync::do_redirect_sync;
use super::{
    RedirectReport, build_redirect_readme, parse_target_auth, require_group,
    resolve_target_registry,
};

pub(in crate::commands::registry) fn run_redirect(
    ctx: &output::Context,
    args: RegistryRedirectArgs,
) -> Result<()> {
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
