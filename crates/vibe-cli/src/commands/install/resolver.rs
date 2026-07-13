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
    /// no-`[[registry]]` project where the embedded registry stands alone.
    Embedded {
        embedded: LocalRegistry,
        declared: Option<Box<MultiRegistryResolver>>,
        precedence: EmbeddedPrecedence,
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
                embedded,
                declared,
                precedence,
                ..
            } => {
                let fetch_embedded = || -> Result<CachedPackage, RegistryError> {
                    let resolved = embedded.resolve(pkgref)?;
                    let mut cached = embedded.fetch(&resolved, cache_root)?;
                    // Tag the provenance so `record.rs` writes source_kind =
                    // "embedded" and the reproducibility guard keys on it (§5).
                    cached.is_embedded = true;
                    Ok(cached)
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
                    EmbeddedPrecedence::EmbeddedFirst => match fetch_embedded() {
                        Err(e) if is_registry_absent(&e) => fetch_declared(),
                        other => other,
                    },
                    EmbeddedPrecedence::EmbeddedLast => match fetch_declared() {
                        Err(e) if is_registry_absent(&e) => fetch_embedded(),
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
                embedded,
                declared,
                precedence,
                ..
            } => crate::registry::dep_solver(
                &flags,
                crate::registry::ProviderResource::Embedded {
                    embedded,
                    declared: declared.as_deref(),
                    precedence: *precedence,
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
                embedded, declared, ..
            } => {
                let mut groups = embedded.candidate_groups(name)?;
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

/// Open the declared multi-registry walk from the manifest — shared by the
/// plain multi-registry path and the embedded composition.
fn open_multi(manifest: &Manifest, args: &InstallArgs) -> Result<MultiRegistryResolver> {
    Ok(
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
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
/// 2. `[[registry]]` array in `vibe.toml` → [`MultiRegistryResolver`]
///    covering priority order, mirrors, and overrides per
///    [PROP-002](../../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md).
pub(crate) fn build_install_resolver(
    args: &InstallArgs,
    manifest: &Manifest,
    embedded_root: Option<&Path>,
) -> Result<InstallResolver> {
    let solver = validate_solver(args.solver.as_deref())?;
    if args.prefer_embedded && args.no_prefer_embedded {
        bail!("--prefer-embedded and --no-prefer-embedded are mutually exclusive");
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

    // PROP-030: a source-installed `vibe` exposes its in-tree `packages/` as an
    // ambient embedded registry, composed with the declared walk. Precedence is
    // developer/embedded-first by default; `--no-prefer-embedded` flips it so a
    // published package wins a clash. `--no-default-registry` (and, at the
    // composition root, `VIBE_NO_DEFAULT_REGISTRY`) suppresses it entirely. When
    // the project declares no `[[registry]]`, the embedded registry stands in
    // alone, lifting the bail below.
    if let Some(root) = embedded_root.filter(|_| !args.no_default_registry) {
        let root = crate::commands::init::strip_unc_public(root.to_path_buf());
        let embedded = crate::registry::local_registry(root.clone()).map_err(|e| {
            anyhow!(
                "failed to open the embedded registry at `{}`: {e}",
                root.display()
            )
        })?;
        let declared = if manifest.registries.is_empty() {
            None
        } else {
            Some(Box::new(open_multi(manifest, args)?))
        };
        let precedence = if args.no_prefer_embedded {
            EmbeddedPrecedence::EmbeddedLast
        } else {
            EmbeddedPrecedence::EmbeddedFirst
        };
        return Ok(InstallResolver::Embedded {
            embedded,
            declared,
            precedence,
            solver,
        });
    }

    if manifest.registries.is_empty() {
        bail!(
            "no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`."
        );
    }

    Ok(InstallResolver::Multi(
        Box::new(open_multi(manifest, args)?),
        solver,
    ))
}
