//! `vibe registry redirect-update` — partial marker rewrite on an
//! existing stub (PROP-002 §2.4.2 trust model).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::Manifest;
use vibe_publish::{
    creator_for_url, extract_host_segment, extract_org_segment, load_token_for_host,
};

use crate::cli::RegistryRedirectUpdateArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::sync::do_redirect_sync;
use super::{
    RedirectChangeEntry, RedirectUpdateReport, build_redirect_readme, parse_target_auth,
    require_group, resolve_target_registry,
};

pub(in crate::commands::registry) fn run_redirect_update(
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
pub(super) fn compute_updated_redirect_section(
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

pub(super) fn diff_redirect_sections(
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
pub(super) fn build_redirect_update_commit_msg(
    pkgref: &str,
    changes: &[RedirectChangeEntry],
) -> String {
    if let Some(c) = changes.iter().find(|c| c.field == "target_url")
        && let Some(after) = &c.after
    {
        return format!("stub: retarget {pkgref} to {after}");
    }
    let fields: Vec<&str> = changes.iter().map(|c| c.field).collect();
    format!("stub: update marker for {pkgref} ({})", fields.join(", "))
}
