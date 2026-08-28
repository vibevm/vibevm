//! Resolver construction — the surface-neutral
//! [`build_install_resolver`] entry and its pure helpers, moved verbatim
//! from `vibe-cli/src/commands/install/resolver.rs` (R7.4 A15a). The
//! surface's argument grammar is already projected away: everything here
//! reads [`PackageSourceOptions`].

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::Manifest;
use vibe_core::{EffectiveRegistryConfig, GlobalRegistryConfig, merge_effective};
use vibe_registry::{LocalRegistry, MultiRegistryResolver};
use vibe_resolver::EmbeddedPrecedence;

use crate::PackageSourceOptions;
use crate::cells::local_registry;
use crate::project_local::project_packages_root;
use crate::source::InstallResolver;

/// Validate the solver flag into the cell name the R-001 selection
/// seam accepts; `None` keeps the built-in default (resolvo).
fn validate_solver(flag: Option<&str>) -> Result<Option<&'static str>> {
    match flag {
        None => Ok(None),
        Some("resolvo") => Ok(Some("resolvo")),
        Some("naive") => Ok(Some("naive")),
        Some("sat") => Ok(Some("sat")),
        Some(other) => {
            bail!("unknown --solver `{other}` — must be `resolvo` (default), `naive`, or `sat`")
        }
    }
}

/// The effective declared-registry config for this invocation: the project
/// manifest merged with the machine-global `~/.vibe/registry.toml`
/// (project-first, PROP-002 §2.2.2), then narrowed to local-only sources
/// under the offline posture (§2.2.2.1; PROP-010 §2.5 — the resolved
/// ladder, computed by the SURFACE before it got here, so the flag > env >
/// config order lives in one place up there). `global` is loaded once at
/// the composition root and passed in, so this stays a pure, testable
/// transform.
fn effective_registry_config(
    manifest: &Manifest,
    global: &GlobalRegistryConfig,
    offline: bool,
) -> EffectiveRegistryConfig {
    let eff = merge_effective(manifest, global);
    if offline { eff.local_only() } else { eff }
}

/// Canonicalise and strip the Windows UNC (`\\?\`) prefix where present —
/// the same normalisation the CLI's init helper performs, kept local so the
/// composition below never depends on a surface utility.
fn strip_unc(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.as_os_str().to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    p
}

/// Open the declared multi-registry walk from a precomputed effective config —
/// shared by the plain multi-registry path and the embedded composition.
fn open_multi_from(
    eff: &EffectiveRegistryConfig,
    manifest: &Manifest,
    options: &PackageSourceOptions,
    offline: bool,
    locked: &[vibe_core::manifest::LockedPackage],
) -> Result<MultiRegistryResolver> {
    // PROP-010 §2.6 — the store is threaded in as a builder parameter
    // (never resolved per call) so the resolver's store reads stay
    // isolated under `$VIBE_SETTINGS` in tests.
    let store_root =
        vibe_registry::store::store_root().context("resolving the machine package store root")?;
    Ok(
        MultiRegistryResolver::open(&eff.registries, &eff.mirrors, &eff.overrides)
            .context("opening multi-registry resolver")?
            .with_strict_auth(options.auth_required)
            .with_git_packages(manifest.requires.git_packages.clone())
            .with_offline(offline)
            .with_store_root(store_root)
            .with_locked_packages(locked.to_vec()),
    )
}

/// Build the install resolver for this invocation — the ONE construction
/// entry every surface shares (the CLI projects its grammar onto
/// [`PackageSourceOptions`]; a hosted surface passes the default).
///
/// Precedence (matches `VIBEVM-SPEC.md` §9.1):
/// 1. An explicit registry path — the local-directory registry (M0 shape,
///    used by tests and offline workflows).
/// 2. `[[registry]]` array in `vibe.toml`, merged with the machine-global
///    `~/.vibe/registry.toml` (project-first, PROP-002 §2.2.2) →
///    [`MultiRegistryResolver`] covering priority order, mirrors, and
///    overrides per
///    [PROP-002](../../../../vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml).
///
/// `global` is the machine-global registry config, loaded once at the caller
/// (composition root) and threaded in so this function performs no filesystem
/// I/O of its own beyond the registry paths it is handed and stays
/// test-hermetic.
///
/// `offline` is the resolved offline posture (PROP-010 §2.5), NOT a raw
/// flag: the caller resolves the full ladder (surface flags, then
/// `VIBE_OFFLINE`, then `[net].offline`) and hands the result down, so
/// every rung reaches this narrowing point through the same one boolean.
///
/// ```
/// use std::path::Path;
/// use vibe_core::GlobalRegistryConfig;
/// use vibe_core::manifest::Manifest;
/// use vibe_package_source::{PackageSourceOptions, build_install_resolver};
///
/// fn build(manifest: &Manifest, root: &Path, global: &GlobalRegistryConfig)
///     -> anyhow::Result<vibe_package_source::InstallResolver> {
///     // The hosted posture: default options, online, nothing locked.
///     build_install_resolver(
///         &PackageSourceOptions::default(),
///         manifest,
///         None,
///         root,
///         global,
///         false,
///         &[],
///     )
/// }
/// # let _ = build;
/// ```
pub fn build_install_resolver(
    options: &PackageSourceOptions,
    manifest: &Manifest,
    embedded_root: Option<&Path>,
    project_root: &Path,
    global: &GlobalRegistryConfig,
    offline: bool,
    locked: &[vibe_core::manifest::LockedPackage],
) -> Result<InstallResolver> {
    let solver = validate_solver(options.solver.as_deref())?;
    if options.prefer_embedded && options.no_prefer_embedded {
        bail!("--prefer-embedded and --no-prefer-embedded are mutually exclusive");
    }
    if options.embedded_short_circuit && options.no_prefer_embedded {
        bail!(
            "--embedded-short-circuit and --no-prefer-embedded are mutually exclusive \
             (short-circuit only makes sense with embedded-first precedence)"
        );
    }
    if options.prefer_local && options.no_prefer_local {
        bail!("--prefer-local and --no-prefer-local are mutually exclusive");
    }
    if let Some(explicit) = &options.registry {
        let p = explicit
            .canonicalize()
            .with_context(|| format!("registry path `{}`", explicit.display()))?;
        let p = strip_unc(p);
        let local = local_registry(p.clone())
            .map_err(|e| anyhow!("failed to open registry at `{}`: {e}", p.display()))?;
        return Ok(InstallResolver::Local(local, solver));
    }

    // The declared walk: project `[[registry]]` merged with the machine-global
    // `~/.vibe/registry.toml` (project-first, PROP-002 §2.2.2), narrowed to
    // local-only sources under the offline posture (§2.2.2.1). Computed once,
    // shared.
    let effective = effective_registry_config(manifest, global, offline);

    // PROP-030 §3.3: build the local-registry family. Project-local
    // (`<project_root>/packages/`) is discovered from the current project —
    // not gated on the running vibe being source-installed, not CI-suppressed
    // (it is per-project and portable). Vibe-embedded (§2) derives from a
    // source install's `source_path`, suppressed by `--no-default-registry`
    // and the composition-root `CI` / `VIBE_NO_DEFAULT_REGISTRY` gate.
    // The family is ordered project-local first (a developer's own in-tree
    // packages win a clash), then vibe-embedded.
    let mut locals: Vec<LocalRegistry> = Vec::new();
    // project_local_count is the number of leading locals that are
    // project-local (0 or 1). Tracked so the fetch path can tag the
    // resolved package is_local (portable) vs is_embedded (machine-local).
    let mut project_local_count: usize = 0;
    if !options.no_prefer_local
        && let Some(root) = project_packages_root(project_root)
    {
        let root = strip_unc(root);
        locals.push(local_registry(root.clone()).map_err(|e| {
            anyhow!(
                "failed to open the project-local registry at `{}`: {e}",
                root.display()
            )
        })?);
        project_local_count = 1;
    }
    if let Some(root) = embedded_root.filter(|_: &&Path| !options.no_default_registry) {
        let root = strip_unc(root.to_path_buf());
        locals.push(local_registry(root.clone()).map_err(|e| {
            anyhow!(
                "failed to open the embedded registry at `{}`: {e}",
                root.display()
            )
        })?);
    }

    // If any local source is present, compose it with the declared walk at the
    // origin-selected precedence. This lifts PROP-002's "no registry
    // configured" bail when either local is present (even without a declared
    // `[[registry]]`).
    if !locals.is_empty() {
        // PROP-002 §2.2.2.1: the offline posture has already filtered the
        // effective set to local sources, so a machine-local `file://`
        // registry still composes with the locals while a remote
        // github/gitverse walk is dropped — no host is contacted, no
        // credential prompt is possible. The declared walk is `None` only
        // when no registry survives.
        let declared = if effective.registries.is_empty() {
            None
        } else {
            Some(Box::new(open_multi_from(
                &effective, manifest, options, offline, locked,
            )?))
        };
        let precedence = if options.no_prefer_embedded {
            EmbeddedPrecedence::EmbeddedLast
        } else {
            EmbeddedPrecedence::EmbeddedFirst
        };
        return Ok(InstallResolver::Embedded {
            locals,
            project_local_count,
            declared,
            precedence,
            short_circuit: options.embedded_short_circuit,
            solver,
        });
    }

    // No local source (no project-local packages/, and no vibe-embedded or it
    // was suppressed) and no explicit registry path. A git-source install (or
    // a re-install whose manifest already carries a git-source entry) does not
    // need a registry — the git-source is the resolver, so skip the bail and
    // fall through to the Multi path (which handles an empty declared set
    // for a git-source-only resolution).
    let has_git_source = options.has_git_source_flag || !manifest.requires.git_packages.is_empty();
    if effective.registries.is_empty() && !has_git_source {
        // PROP-010 §2.6: under the offline posture the machine store is
        // a resolution source in its own right — a warm store serves
        // with zero registries. The old "no local registry" bail fires
        // only when nothing local can serve: no local registry AND an
        // empty store.
        if offline && !vibe_registry::store::list_all().is_empty() {
            return Ok(InstallResolver::Multi(
                Box::new(open_multi_from(
                    &effective, manifest, options, offline, locked,
                )?),
                solver,
            ));
        }
        // PROP-002 §2.2.2.1: under the offline posture (the resolved ladder,
        // PROP-010 §2.5) the remote walk is disabled and no local registry
        // survived, so there is nothing to resolve from — fail with an
        // actionable message rather than reach the network.
        if offline {
            bail!(
                "--offline: no local registry available to resolve from. \
                 Offline resolution needs a local (`file://`) `[[registry]]` — in the \
                 project `vibe.toml` or `~/.vibe/registry.toml` — a project-local \
                 `packages/` directory, the embedded registry of a source install \
                 (check `vibe self doctor`), an explicit `--registry <dir>`, or a \
                 warmed machine store (`vibe cache add <pkgref>`); \
                 remote registries are disabled under --offline."
            );
        }
        bail!(
            "no registry configured. Pass `--registry <path>`, add a `[[registry]]` \
             entry to `vibe.toml` (or `~/.vibe/registry.toml`), or place the package \
             in a project-local `packages/` directory."
        );
    }

    Ok(InstallResolver::Multi(
        Box::new(open_multi_from(
            &effective, manifest, options, offline, locked,
        )?),
        solver,
    ))
}

#[cfg(test)]
#[path = "flag_tests.rs"]
mod flag_tests;
