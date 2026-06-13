//! Resolver construction for `vibe install` — the [`InstallResolver`]
//! local / multi-registry dispatch and the M1.15 `--git` source-flag
//! recording.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};
use vibe_install::InstallSource;
use vibe_registry::{CachedPackage, LocalRegistry, MultiRegistryResolver, RegistryError};

use crate::cli::InstallArgs;

/// Either a M0-shape local-directory registry (used by `--registry <path>`
/// and the in-tree fixture path) or a full PROP-002 multi-registry
/// resolver covering the `[[registry]]` / `[[mirror]]` / `[[override]]`
/// sections in `vibe.toml`. The orchestrator consumes it through the
/// [`InstallSource`] seam; construction stays here at the CLI's
/// composition root (R-001).
pub(crate) enum InstallResolver {
    Local(LocalRegistry),
    // Boxed: `MultiRegistryResolver` is by far the larger variant
    // (it carries the registry list plus the override / git-source /
    // path-source maps), so an unboxed enum would bloat every
    // `InstallResolver` value to the size of the multi-registry path.
    Multi(Box<MultiRegistryResolver>),
}

impl InstallSource for InstallResolver {
    /// Resolve `pkgref` and materialise its content into the
    /// per-project cache. `expected_hash` (typically the lockfile pin
    /// for `(pkgref.kind, pkgref.name, version)`) is forwarded to the
    /// multi-registry path's mirror-aware fetch so a source serving
    /// disagreeing bytes can be skipped in favour of a matching one.
    /// The local-directory path ignores the hint — there's only ever
    /// one source on that path, and integrity is checked against the
    /// lockfile pin at apply time.
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        cache_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        match self {
            InstallResolver::Local(r) => {
                let resolved = r.resolve(pkgref)?;
                r.fetch(&resolved, cache_root)
            }
            InstallResolver::Multi(m) => {
                let resolution = m.resolve(pkgref)?;
                m.fetch_with_expected_hash(&resolution, cache_root, expected_hash)
            }
        }
    }

    fn solve(
        &self,
        roots: &[PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        // Cell selection lives in the registry module (R-001); this
        // match only routes the resource the caller already owns.
        let flags = crate::registry::selection_flags(matches!(self, InstallResolver::Local(_)));
        let solver = match self {
            InstallResolver::Local(r) => {
                crate::registry::dep_solver(&flags, crate::registry::ProviderResource::Local(r))
            }
            InstallResolver::Multi(m) => {
                crate::registry::dep_solver(&flags, crate::registry::ProviderResource::Multi(m))
            }
        };
        solver.solve(roots)
    }
}

impl InstallResolver {
    /// Enumerate every `group` that publishes a package of the bare
    /// `name` — the candidate set short-name resolution (PROP-008
    /// §2.6) walks. The local-directory path scans the registry tree;
    /// the multi-registry path walks each registry's index. The result
    /// is de-duplicated and sorted; `len() > 1` is a collision. Not
    /// part of [`InstallSource`]: qualification is the CLI's input
    /// boundary, not the orchestrator's.
    pub(crate) fn candidate_groups(&self, name: &str) -> Result<Vec<Group>> {
        match self {
            InstallResolver::Local(r) => Ok(r.candidate_groups(name)?),
            InstallResolver::Multi(m) => Ok(m.resolve_name_candidates(name)),
        }
    }
}

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
pub(super) fn apply_git_source_flag(
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
    let url = args.git.clone().expect("caller checked args.git.is_some()");
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

/// Build the install resolver for this invocation.
///
/// Precedence (matches `VIBEVM-SPEC.md` §9.1):
/// 1. `--registry <path>` — explicit local-directory registry (M0 shape,
///    used by tests and offline workflows).
/// 2. `[[registry]]` array in `vibe.toml` → [`MultiRegistryResolver`]
///    covering priority order, mirrors, and overrides per
///    [PROP-002](../../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md).
pub(crate) fn build_install_resolver(
    args: &InstallArgs,
    manifest: &Manifest,
) -> Result<InstallResolver> {
    if let Some(explicit) = &args.registry {
        let p = explicit
            .canonicalize()
            .with_context(|| format!("registry path `{}`", explicit.display()))?;
        let p = crate::commands::init::strip_unc_public(p);
        let local = crate::registry::local_registry(p.clone())
            .map_err(|e| anyhow!("failed to open registry at `{}`: {e}", p.display()))?;
        return Ok(InstallResolver::Local(local));
    }

    if manifest.registries.is_empty() {
        bail!(
            "no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`."
        );
    }

    let mrr =
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
            .context("opening multi-registry resolver")?
            .with_strict_auth(args.auth_required)
            .with_git_packages(manifest.requires.git_packages.clone());
    Ok(InstallResolver::Multi(Box::new(mrr)))
}
