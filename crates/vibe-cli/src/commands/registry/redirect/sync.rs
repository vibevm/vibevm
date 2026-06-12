//! `vibe registry redirect-sync` — target→stub tag mirroring
//! (PROP-002 §2.4.2). Hosts the shared sync leg reused by
//! `redirect --sync` and `redirect-update --resync`.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::{Manifest, RegistrySection};
use vibe_publish::{
    creator_for_url, extract_host_segment, extract_org_segment, load_token_for_host,
};

use crate::cli::RegistryRedirectSyncArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::{RedirectSyncReport, require_group, resolve_target_registry};

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
/// Inner sync logic — shared by `vibe registry redirect --sync` and
/// `vibe registry redirect-sync`. Reads the stub's `vibe-redirect.toml`,
/// enumerates target tags, pushes the missing ones into the stub.
pub(super) fn do_redirect_sync(
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
pub(super) fn build_target_fetch_url(
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

pub(super) fn derive_target_token_env(target_url: &str) -> Option<String> {
    let host = extract_host_segment(target_url).ok()?;
    let upper = host.to_ascii_uppercase().replace(['.', '-'], "_");
    Some(format!("VIBEVM_TARGET_TOKEN_{upper}"))
}

pub(super) fn inject_token_into_url(url: &str, token: &str) -> String {
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
