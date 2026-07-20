//! The M1.15 `--git` source-flag processing for `vibe install` — split from
//! `resolver.rs` (its sibling) to keep that file within the length budget.
//! A distinct responsibility from resolver construction: it translates the
//! `--git`/`--tag`/`--branch`/`--rev`/`--git-auth` flags into a typed
//! `GitPackageDep` and records it on the manifest before resolving.

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::PackageRef;
use vibe_core::manifest::Manifest;

use crate::cli::InstallArgs;

/// Process the M1.15 `--git`/`--tag`/`--branch`/`--rev`/`--git-auth`/
/// `--git-token-env` flags. Validates the flag combination, parses
/// the single positional pkgref, builds a `GitPackageDep`, merges it
/// into `manifest.requires.git_packages` (replacing any prior entry
/// for the same `(kind, name)`), and persists the manifest before
/// resolving so a panic mid-resolve cannot leave the on-disk
/// declaration out of sync. Removes any conflicting registry-resolved
/// entry for the same pkgref to keep `manifest.requires` in a valid
/// shape (no duplicate `(kind, name)` between `packages` and
/// `git_packages`).
pub(crate) fn apply_git_source_flag(
    args: &InstallArgs,
    manifest: &mut Manifest,
    project_root: &std::path::Path,
) -> Result<()> {
    use vibe_core::manifest::{AuthKind, GitPackageDep, GitRefKind};

    if args.exact {
        bail!(
            "--exact has no meaning with --git (constraint shape is registry-resolved); drop one of the two flags"
        );
    }
    if args.registry.is_some() {
        bail!("--git bypasses the registry layer; drop --registry or drop --git");
    }
    if args.packages.len() != 1 {
        bail!(
            "--git requires exactly one positional pkgref `<group>/<name>`; got {}",
            args.packages.len()
        );
    }
    // Allow user to type either `org.vibevm/internal` or
    // `org.vibevm/internal@*` — version is irrelevant for git-source (the
    // ref decides), but we accept both shapes for muscle-memory
    // compatibility.
    let pr = PackageRef::parse(&args.packages[0])
        .with_context(|| format!("parsing `{}`", args.packages[0]))?;
    let pr_group = pr.group.clone().ok_or_else(|| {
        anyhow!("package reference `{pr}` is not group-qualified — write `<group>/<name>`")
    })?;
    // The caller dispatches here only when `--git` is present; treat a
    // missing value as the internal invariant break it is, rather than
    // panic on it.
    let Some(url) = args.git.clone() else {
        bail!("--git is required for a git-source install (internal: dispatched without it)");
    };
    let ref_kind = match (
        args.tag.as_deref(),
        args.branch.as_deref(),
        args.rev.as_deref(),
    ) {
        (Some(t), None, None) => GitRefKind::Tag(t.to_string()),
        (None, Some(b), None) => GitRefKind::Branch(b.to_string()),
        (None, None, Some(r)) => GitRefKind::Rev(r.to_string()),
        (None, None, None) => bail!("--git requires exactly one of --tag, --branch, or --rev"),
        _ => bail!("--git accepts exactly one of --tag, --branch, --rev — drop the extras"),
    };
    let auth = match args.git_auth.as_deref() {
        None | Some("none") => AuthKind::None,
        Some("token-env") => AuthKind::TokenEnv,
        Some("credential-helper") => AuthKind::CredentialHelper,
        Some("ssh") => AuthKind::Ssh,
        Some(other) => bail!(
            "unknown --git-auth `{other}` — must be `none`, `token-env`, `credential-helper`, or `ssh`"
        ),
    };
    if args.git_token_env.is_some() && !matches!(auth, AuthKind::TokenEnv) {
        bail!(
            "--git-token-env is only meaningful with --git-auth token-env; got `{}`",
            args.git_auth.as_deref().unwrap_or("none")
        );
    }
    let dep = GitPackageDep {
        kind: pr.kind,
        group: pr_group.clone(),
        name: pr.name.to_string(),
        url,
        ref_kind,
        version: None,
        auth,
        token_env: args.git_token_env.clone(),
    };

    // The (group, name) dedup discipline across `requires.packages` /
    // `requires.git_packages` lives in the orchestrator now — the CLI
    // translates flags into the typed dep and hands it over, then
    // persists before resolving so a panic mid-resolve cannot strand the
    // declaration off disk.
    vibe_install::record_git_source(manifest, dep);
    manifest.write(project_root.join(Manifest::FILENAME))?;
    Ok(())
}
