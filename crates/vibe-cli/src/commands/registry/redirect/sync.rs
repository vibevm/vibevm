//! `vibe registry redirect-sync` — the CLI surface for target→stub tag
//! mirroring (PROP-002 §2.4.2). Argument parsing, registry resolution,
//! the stub-existence probe, and rendering live here; the tag-mirroring
//! domain lives in [`vibe_publish::redirect_sync`] (CONVERT-PLAN v0.1
//! §4.2). Hosts the shared CLI leg reused by `redirect --sync` and
//! `redirect-update --resync`.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#redirect");

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::{Manifest, RegistrySection};
use vibe_publish::redirect_sync::{self, RedirectSyncEvent, RedirectSyncObserver};
use vibe_publish::{
    creator_for_url, extract_host_segment, extract_org_segment, load_token_for_host,
};

use crate::cli::RegistryRedirectSyncArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::{RedirectSyncReport, require_group, resolve_target_registry};

/// Renders [`RedirectSyncEvent`]s as the progressive per-tag output the
/// tag-sync loop used to print inline.
struct CliRedirectSyncObserver<'a>(&'a output::Context);

impl RedirectSyncObserver for CliRedirectSyncObserver<'_> {
    fn on(&self, event: RedirectSyncEvent) {
        match event {
            RedirectSyncEvent::WouldPush { tag } => {
                self.0.step(&format!(
                    "Would push tag `{tag}` (target has it; stub does not)"
                ));
            }
            RedirectSyncEvent::Pushed { tag } => {
                self.0.step(&format!("Pushed tag `{tag}` into stub"));
            }
            RedirectSyncEvent::AlreadyPresent { tag } => {
                self.0
                    .skipped(&format!("tag `{tag}`"), "already present on stub");
            }
        }
    }
}

pub(in crate::commands::registry) fn run_redirect_sync(
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
    // Run the scope check once; the typed token gates every host call
    // below (PROP-002 §2.10 "never escalate scope", now a type).
    let validated_org = creator
        .validate_scope(&org_segment)
        .map_err(|e| anyhow!("{e}"))?;
    let push_url = creator.push_url(&validated_org, &stub_repo_name);

    // Probe stub existence so we fail fast with a clear message.
    let exists = creator
        .repo_exists(&validated_org, &stub_repo_name)
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

/// CLI-side leg shared by `redirect-sync`, `redirect --sync`, and
/// `redirect-update --resync`: drive [`vibe_publish::redirect_sync`]'s
/// tag-mirroring through a rendering observer and shape the result into
/// the JSON-envelope [`RedirectSyncReport`]. The domain (shallow-clone,
/// marker read, tag classification, push) lives in vibe-publish; this
/// wrapper owns only the ctx→observer and outcome→report translation.
pub(super) fn do_redirect_sync(
    ctx: &output::Context,
    registry_section: &RegistrySection,
    pkgref_qualified: &str,
    stub_url: &str,
    target_url_hint: &str,
    push_url: &str,
    dry_run: bool,
) -> Result<RedirectSyncReport> {
    let observer = CliRedirectSyncObserver(ctx);
    let outcome =
        redirect_sync::sync_redirect_tags(&observer, stub_url, target_url_hint, push_url, dry_run)
            .map_err(|e| anyhow!("{e}"))?;
    Ok(RedirectSyncReport {
        ok: true,
        command: "registry:redirect-sync",
        registry: registry_section.name.clone(),
        pkgref: pkgref_qualified.to_string(),
        stub_url: stub_url.to_string(),
        target_url: outcome.target_url,
        pushed_tags: outcome.pushed_tags,
        already_present: outcome.already_present,
        dry_run,
    })
}
