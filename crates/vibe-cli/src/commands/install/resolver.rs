//! Resolver construction for `vibe install` — the [`InstallResolver`]
//! local / multi-registry dispatch and the M1.15 `--git` source-flag
//! recording.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::Manifest;
use vibe_core::{
    EffectiveRegistryConfig, GlobalRegistryConfig, Group, PackageRef, merge_effective,
};
use vibe_install::InstallSource;
use vibe_registry::{CachedPackage, LocalRegistry, MultiRegistryResolver, RegistryError};
use vibe_resolver::EmbeddedPrecedence;

use crate::cli::InstallArgs;
use crate::registry::ProviderCell;

/// Either a M0-shape local-directory registry (used by `--registry <path>`
/// and the in-tree fixture path) or a full PROP-002 multi-registry
/// resolver covering the `[[registry]]` / `[[mirror]]` / `[[override]]`
/// sections in `vibe.toml`. The orchestrator consumes it through the
/// [`InstallSource`] seam; construction stays here at the CLI's
/// composition root (R-001).
pub(crate) enum InstallResolver {
    /// The local-directory registry plus the optional `--solver` cell
    /// name threaded through to the R-001 selection seam.
    Local(LocalRegistry, Option<&'static str>),
    // Boxed: `MultiRegistryResolver` is by far the larger variant
    // (it carries the registry list plus the override / git-source /
    // path-source maps), so an unboxed enum would bloat every
    // `InstallResolver` value to the size of the multi-registry path.
    Multi(Box<MultiRegistryResolver>, Option<&'static str>),
    /// PROP-030: the embedded local-directory registry (a source install's
    /// in-tree `packages/`) composed with an optional declared multi-registry
    /// walk at the origin-selected precedence. `declared = None` is the
    /// no-`[[registry]]` project where the local family stands alone. The
    /// Vec is the ordered local-registry family — project-local first (when
    /// `<project_root>/packages/` is discovered, PROP-030 §3.3), then
    /// vibe-embedded. The composite at the resolver layer (PROP-030 §3)
    /// honours this ordering: the first local wins a clash inside the family.
    Embedded {
        locals: Vec<LocalRegistry>,
        /// PROP-030 §3.3: how many leading entries of `locals` are the
        /// project-local registry (0 when only vibe-embedded is in the
        /// family, 1 when project-packages were discovered). The fetch path
        /// tags the resolved package `is_local` (portable) for an index < this
        /// count, else `is_embedded` (machine-local) — so the lock records the
        /// right `source_kind` and the reproducibility guard fires only for
        /// the vibe-embedded half.
        project_local_count: usize,
        declared: Option<Box<MultiRegistryResolver>>,
        precedence: EmbeddedPrecedence,
        /// PROP-030 §3.1: when set (`--embedded-short-circuit`), version
        /// enumeration stops at the embedded registry for any coordinate it
        /// serves, so the declared walk (and its network round-trip) is
        /// consulted only for packages the embedded registry lacks.
        short_circuit: bool,
        solver: Option<&'static str>,
    },
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
            InstallResolver::Local(r, _) => {
                let resolved = r.resolve(pkgref)?;
                r.fetch(&resolved, cache_root)
            }
            InstallResolver::Multi(m, _) => {
                let resolution = m.resolve(pkgref)?;
                m.fetch_with_expected_hash(&resolution, cache_root, expected_hash)
            }
            InstallResolver::Embedded {
                locals,
                project_local_count,
                declared,
                precedence,
                ..
            } => {
                let fetch_local = || -> Result<CachedPackage, RegistryError> {
                    // Walk the local family in order (project-local first,
                    // then vibe-embedded). The first local that serves the
                    // coordinate wins; an absence falls through to the next;
                    // any real failure halts. Provenance tagging:
                    //   index < project_local_count → is_local (portable,
                    //     per-project packages/ — PROP-030 §3.3)
                    //   else → is_embedded (machine-local, vibe's in-tree
                    //     packages — PROP-030 §2)
                    // so the lock records the right source_kind and the
                    // reproducibility guard fires only for the vibe-embedded
                    // half.
                    let mut last_absent: Option<RegistryError> = None;
                    for (idx, local) in locals.iter().enumerate() {
                        match local.resolve(pkgref) {
                            Ok(resolved) => {
                                let mut cached = local.fetch(&resolved, cache_root)?;
                                if idx < *project_local_count {
                                    cached.is_local = true;
                                } else {
                                    cached.is_embedded = true;
                                }
                                return Ok(cached);
                            }
                            Err(e) if is_registry_absent(&e) => {
                                last_absent = Some(e);
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    // `last_absent` is always `Some` when `locals` is
                    // non-empty (every local either Ok's or sets it). The
                    // empty-`locals` case is forbidden by the construction
                    // path (build_install_resolver returns Embedded only
                    // when !locals.is_empty()). Fall through to the
                    // declared walk with the typed absence; if somehow
                    // neither is set, propagate as a generic "not here".
                    match last_absent {
                        Some(e) => Err(e),
                        None => Err(RegistryError::UnqualifiedPkgref(pkgref.to_string())),
                    }
                };
                let fetch_declared = || -> Result<CachedPackage, RegistryError> {
                    match declared {
                        Some(m) => {
                            let resolution = m.resolve(pkgref)?;
                            m.fetch_with_expected_hash(&resolution, cache_root, expected_hash)
                        }
                        None => {
                            let group = pkgref.group.clone().ok_or_else(|| {
                                RegistryError::UnqualifiedPkgref(pkgref.to_string())
                            })?;
                            Err(RegistryError::UnknownPackage {
                                group,
                                name: pkgref.name.to_string(),
                            })
                        }
                    }
                };
                // Fetch in precedence order, falling through only a genuine
                // "not here" (a real failure halts).
                match precedence {
                    EmbeddedPrecedence::EmbeddedFirst => match fetch_local() {
                        Err(e) if is_registry_absent(&e) => fetch_declared(),
                        other => other,
                    },
                    EmbeddedPrecedence::EmbeddedLast => match fetch_declared() {
                        Err(e) if is_registry_absent(&e) => fetch_local(),
                        other => other,
                    },
                }
            }
        }
    }

    fn solve(
        &self,
        roots: &[PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        // Cell selection lives in the registry module (R-001); this
        // match only routes the resource the caller already owns.
        let (provider_cell, solver_override) = match self {
            InstallResolver::Local(_, s) => (ProviderCell::Local, *s),
            InstallResolver::Multi(_, s) => (ProviderCell::Multi, *s),
            InstallResolver::Embedded { solver, .. } => (ProviderCell::Embedded, *solver),
        };
        let flags = crate::registry::selection_flags(provider_cell, solver_override);
        let solver = match self {
            InstallResolver::Local(r, _) => {
                crate::registry::dep_solver(&flags, crate::registry::ProviderResource::Local(r))
            }
            InstallResolver::Multi(m, _) => {
                crate::registry::dep_solver(&flags, crate::registry::ProviderResource::Multi(m))
            }
            InstallResolver::Embedded {
                locals,
                declared,
                precedence,
                short_circuit,
                ..
            } => crate::registry::dep_solver(
                &flags,
                crate::registry::ProviderResource::Embedded {
                    locals: locals.iter().collect(),
                    declared: declared.as_deref(),
                    precedence: *precedence,
                    short_circuit: *short_circuit,
                },
            ),
        };
        solver.solve(roots)
    }

    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &std::path::Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        match self {
            // A local-directory registry has no git backend — in-place needs
            // a real git source to clone and incrementally update (PROP-022
            // §2.4).
            InstallResolver::Local(..) => {
                let group = pkgref
                    .group
                    .clone()
                    .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
                Err(RegistryError::InPlaceUnsupported {
                    group,
                    name: pkgref.name.to_string(),
                })
            }
            InstallResolver::Multi(m, _) => {
                let resolution = m.resolve(pkgref)?;
                m.materialise_in_place(&resolution, slot)
            }
            // In-place needs a git backend to clone and incrementally update;
            // the embedded local-directory registry has none. Serve it from
            // the declared walk when that carries the package, else refuse with
            // the same InPlaceUnsupported a `--registry <dir>` install gives.
            InstallResolver::Embedded { declared, .. } => match declared {
                Some(m) => match m.resolve(pkgref) {
                    Ok(resolution) => m.materialise_in_place(&resolution, slot),
                    Err(e) if is_registry_absent(&e) => {
                        let group = pkgref
                            .group
                            .clone()
                            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
                        Err(RegistryError::InPlaceUnsupported {
                            group,
                            name: pkgref.name.to_string(),
                        })
                    }
                    Err(e) => Err(e),
                },
                None => {
                    let group = pkgref
                        .group
                        .clone()
                        .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
                    Err(RegistryError::InPlaceUnsupported {
                        group,
                        name: pkgref.name.to_string(),
                    })
                }
            },
        }
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
            InstallResolver::Local(r, _) => Ok(r.candidate_groups(name)?),
            InstallResolver::Multi(m, _) => Ok(m.resolve_name_candidates(name)),
            InstallResolver::Embedded {
                locals, declared, ..
            } => {
                // The local family is a Vec: union candidate_groups across
                // every local (project-local + vibe-embedded), then layer in
                // the declared walk, then sort + dedup.
                let mut groups = Vec::new();
                for local in locals {
                    groups.extend(local.candidate_groups(name)?);
                }
                if let Some(m) = declared {
                    groups.extend(m.resolve_name_candidates(name));
                }
                groups.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                groups.dedup();
                Ok(groups)
            }
        }
    }
}

mod git_source_flag;
pub(super) use git_source_flag::apply_git_source_flag;

/// Validate the `--solver` flag into the cell name the R-001 selection
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

/// The registry errors that mean "this source does not serve the
/// coordinate" — the embedded/declared composition falls through these and
/// halts on anything else (PROP-002 §2.3.1 fall-through set).
fn is_registry_absent(err: &RegistryError) -> bool {
    matches!(
        err,
        RegistryError::UnknownPackage { .. }
            | RegistryError::NoMatchingVersion { .. }
            | RegistryError::PackageNotFoundEverywhere { .. }
    )
}

/// The effective declared-registry config for this invocation: the project
/// manifest merged with the machine-global `~/.vibe/registry.toml`
/// (project-first, PROP-002 §2.2.2), then narrowed to local-only sources
/// under `--offline` (§2.2.2.1). `global` is loaded once at the composition
/// root and passed in, so this stays a pure, testable transform.
fn effective_registry_config(
    manifest: &Manifest,
    args: &InstallArgs,
    global: &GlobalRegistryConfig,
) -> EffectiveRegistryConfig {
    let eff = merge_effective(manifest, global);
    if args.offline { eff.local_only() } else { eff }
}

/// Open the declared multi-registry walk from a precomputed effective config —
/// shared by the plain multi-registry path and the embedded composition.
fn open_multi_from(
    eff: &EffectiveRegistryConfig,
    manifest: &Manifest,
    args: &InstallArgs,
) -> Result<MultiRegistryResolver> {
    Ok(
        MultiRegistryResolver::open(&eff.registries, &eff.mirrors, &eff.overrides)
            .context("opening multi-registry resolver")?
            .with_strict_auth(args.auth_required)
            .with_git_packages(manifest.requires.git_packages.clone()),
    )
}

/// Build the install resolver for this invocation.
///
/// Precedence (matches `VIBEVM-SPEC.md` §9.1):
/// 1. `--registry <path>` — explicit local-directory registry (M0 shape,
///    used by tests and offline workflows).
/// 2. `[[registry]]` array in `vibe.toml`, merged with the machine-global
///    `~/.vibe/registry.toml` (project-first, PROP-002 §2.2.2) →
///    [`MultiRegistryResolver`] covering priority order, mirrors, and
///    overrides per
///    [PROP-002](../../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md).
///
/// `global` is the machine-global registry config, loaded once at the caller
/// (composition root) and threaded in so this function performs no filesystem
/// I/O of its own and stays test-hermetic.
pub(crate) fn build_install_resolver(
    args: &InstallArgs,
    manifest: &Manifest,
    embedded_root: Option<&Path>,
    project_root: &Path,
    global: &GlobalRegistryConfig,
) -> Result<InstallResolver> {
    let solver = validate_solver(args.solver.as_deref())?;
    if args.prefer_embedded && args.no_prefer_embedded {
        bail!("--prefer-embedded and --no-prefer-embedded are mutually exclusive");
    }
    if args.embedded_short_circuit && args.no_prefer_embedded {
        bail!(
            "--embedded-short-circuit and --no-prefer-embedded are mutually exclusive \
             (short-circuit only makes sense with embedded-first precedence)"
        );
    }
    if args.prefer_local && args.no_prefer_local {
        bail!("--prefer-local and --no-prefer-local are mutually exclusive");
    }
    if let Some(explicit) = &args.registry {
        let p = explicit
            .canonicalize()
            .with_context(|| format!("registry path `{}`", explicit.display()))?;
        let p = crate::commands::init::strip_unc_public(p);
        let local = crate::registry::local_registry(p.clone())
            .map_err(|e| anyhow!("failed to open registry at `{}`: {e}", p.display()))?;
        return Ok(InstallResolver::Local(local, solver));
    }

    // The declared walk: project `[[registry]]` merged with the machine-global
    // `~/.vibe/registry.toml` (project-first, PROP-002 §2.2.2), narrowed to
    // local-only sources under `--offline` (§2.2.2.1). Computed once, shared.
    let effective = effective_registry_config(manifest, args, global);

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
    if !args.no_prefer_local
        && let Some(root) = super::project_packages_root(project_root)
    {
        let root = crate::commands::init::strip_unc_public(root);
        locals.push(crate::registry::local_registry(root.clone()).map_err(|e| {
            anyhow!(
                "failed to open the project-local registry at `{}`: {e}",
                root.display()
            )
        })?);
        project_local_count = 1;
    }
    if let Some(root) = embedded_root.filter(|_: &&Path| !args.no_default_registry) {
        let root = crate::commands::init::strip_unc_public(root.to_path_buf());
        locals.push(crate::registry::local_registry(root.clone()).map_err(|e| {
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
        // PROP-002 §2.2.2.1: `--offline` has already filtered the effective set
        // to local sources, so a machine-local `file://` registry still
        // composes with the locals while a remote github/gitverse walk is
        // dropped — no host is contacted, no credential prompt is possible. The
        // declared walk is `None` only when no registry survives.
        let declared = if effective.registries.is_empty() {
            None
        } else {
            Some(Box::new(open_multi_from(&effective, manifest, args)?))
        };
        let precedence = if args.no_prefer_embedded {
            EmbeddedPrecedence::EmbeddedLast
        } else {
            EmbeddedPrecedence::EmbeddedFirst
        };
        return Ok(InstallResolver::Embedded {
            locals,
            project_local_count,
            declared,
            precedence,
            short_circuit: args.embedded_short_circuit,
            solver,
        });
    }

    // No local source (no project-local packages/, and no vibe-embedded or it
    // was suppressed) and no explicit `--registry`.
    if effective.registries.is_empty() {
        // PROP-002 §2.2.2.1: under `--offline` the remote walk is disabled and
        // no local registry survived, so there is nothing to resolve from —
        // fail with an actionable message rather than reach the network.
        if args.offline {
            bail!(
                "--offline: no local registry available to resolve from. \
                 Offline resolution needs a local (`file://`) `[[registry]]` — in the \
                 project `vibe.toml` or `~/.vibe/registry.toml` — a project-local \
                 `packages/` directory, the embedded registry of a source install \
                 (check `vibe self doctor`), or an explicit `--registry <dir>`; \
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
        Box::new(open_multi_from(&effective, manifest, args)?),
        solver,
    ))
}

#[cfg(test)]
#[path = "flag_tests.rs"]
mod flag_tests;
