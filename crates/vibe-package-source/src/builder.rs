//! Resolver construction — the surface-neutral
//! [`build_install_resolver`] entry and its pure helpers, moved verbatim
//! from `vibe-cli/src/commands/install/resolver.rs` (R7.4 A15a). The
//! surface's argument grammar is already projected away: everything here
//! reads [`PackageSourceOptions`].

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::Manifest;
use vibe_core::{EffectiveRegistryConfig, GlobalRegistryConfig};
use vibe_registry::LocalRegistry;
use vibe_resolver::EmbeddedPrecedence;

use crate::cells::local_registry;
use crate::factory::{open_multi_from, source_operation};
use crate::project_local::project_packages_root;
use crate::source::InstallResolver;
use crate::{PackageSourceOptions, SourceOperation};
use vibe_core::GlobalRegistrySource;

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
pub(super) fn effective_registry_config(
    manifest: &Manifest,
    global: &GlobalRegistryConfig,
    local: &GlobalRegistryConfig,
    offline: bool,
) -> EffectiveRegistryConfig {
    // PROP-002 `#registry-layers`: the project-local override sits above
    // the project manifest, the machine file below it.
    let eff = vibe_core::RegistryLayers {
        local: Some(local),
        project: Some(manifest),
        user: global,
        site: None,
    }
    .merge();
    if offline { eff.local_only() } else { eff }
}

/// The project-local override layer `<project_root>/.vibe/registry.local.toml`
/// (PROP-002 `#registry-layers`); a missing file is the empty layer.
pub(super) fn project_local_layer(project_root: &Path) -> Result<GlobalRegistryConfig> {
    GlobalRegistryConfig::load_project_local(project_root).with_context(|| {
        format!(
            "loading `{}`",
            vibe_core::project_local_registry_path(project_root).display()
        )
    })
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

/// The roots of the local-registry family for this invocation, ordered
/// project-local first (PROP-030 §3.3): `<project_root>/vibevm/vibepacks/`
/// when that directory exists and `--no-prefer-local` did not disable the
/// lane, then a source build's embedded registry unless
/// `--no-default-registry` (or the composition root's CI gate) suppressed it.
///
/// The second value is how many LEADING roots are project-local (0 or 1) —
/// the tag the fetch path needs to call a resolved package portable
/// (`is_local`) rather than machine-local (`is_embedded`).
///
/// Discovery only: nothing here opens a registry, so
/// [`any_package_source`] can ask which sources exist without paying for
/// one — and cannot drift from what [`build_install_resolver`] opens,
/// because that function reads this same list.
fn local_family_roots(
    options: &PackageSourceOptions,
    embedded_root: Option<&Path>,
    project_root: &Path,
) -> (Vec<PathBuf>, usize) {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut project_local_count = 0usize;
    if !options.no_prefer_local
        && let Some(root) = project_packages_root(project_root)
    {
        roots.push(strip_unc(root));
        project_local_count = 1;
    }
    if let Some(root) = embedded_root.filter(|_: &&Path| !options.no_default_registry) {
        roots.push(strip_unc(root.to_path_buf()));
    }
    (roots, project_local_count)
}

/// Can anything OTHER than the local-registry family serve this invocation?
///
/// Three sources answer yes: a declared registry in the merged effective set
/// (the project's `[[registry]]` entries followed by the machine-global
/// `~/.vibe/registry.toml`, PROP-002 §2.2.2, already narrowed to local
/// sources when the posture is offline); a git source, which IS the resolver
/// and needs no registry; and — under the offline posture — a warm machine
/// store, a resolution source in its own right (PROP-010 §2.6).
///
/// The negation of this predicate is exactly the "nothing can serve" bail in
/// [`build_install_resolver_with_progress`], which is why both read it.
fn declared_source_available(
    effective: &EffectiveRegistryConfig,
    manifest: &Manifest,
    options: &PackageSourceOptions,
    offline: bool,
) -> bool {
    !effective.registries.is_empty()
        || options.has_git_source_flag
        || !manifest.requires.git_packages.is_empty()
        || (offline && !vibe_registry::store::list_all().is_empty())
}

/// Can ANY source serve this invocation — the one question
/// [`build_install_resolver`]'s "no registry configured" bail answers, asked
/// without opening a registry and without touching the network.
///
/// A command that refuses up front — `vibe update` will not re-fetch a
/// package nothing can resolve — must refuse on THIS answer rather than on
/// the project manifest alone. A project that declares no `[[registry]]` of
/// its own still resolves from the machine-global `~/.vibe/registry.toml`,
/// merged project-first (PROP-002 `##GLOBAL-REGISTRY-FILE` /
/// `##MERGE-PROJECT-FIRST`), from a project-local `vibevm/vibepacks/` or a
/// source build's embedded registry (PROP-030 §3.3), from a declared git
/// source, or — under the offline posture — from the warm machine store
/// (PROP-010 §2.5–§2.6).
///
/// `false` means [`build_install_resolver`] would bail "no registry
/// configured"; `true` means it would build a resolver, subject only to the
/// flag-contradiction guards, which are the caller's own argument errors and
/// not a missing source.
pub fn any_package_source(
    options: &PackageSourceOptions,
    manifest: &Manifest,
    embedded_root: Option<&Path>,
    project_root: &Path,
    global: &GlobalRegistryConfig,
    offline: bool,
) -> bool {
    // An explicit `--registry <dir>` short-circuits the whole composition.
    if options.registry.is_some() {
        return true;
    }
    if !local_family_roots(options, embedded_root, project_root)
        .0
        .is_empty()
    {
        return true;
    }
    // A malformed local override still counts as a declared source: the
    // build path reports the file with its own diagnostic.
    let Ok(local) = project_local_layer(project_root) else {
        return true;
    };
    declared_source_available(
        &effective_registry_config(manifest, global, &local, offline),
        manifest,
        options,
        offline,
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
    build_install_resolver_with_progress(
        options,
        manifest,
        embedded_root,
        project_root,
        global,
        offline,
        locked,
        &vibe_core::progress::Progress::default(),
    )
}

/// Observed sibling of [`build_install_resolver`]. The handle is retained by
/// this invocation's resolver and used only for registry/cache work.
#[allow(clippy::too_many_arguments)]
pub fn build_install_resolver_with_progress(
    options: &PackageSourceOptions,
    manifest: &Manifest,
    embedded_root: Option<&Path>,
    project_root: &Path,
    global: &GlobalRegistryConfig,
    offline: bool,
    locked: &[vibe_core::manifest::LockedPackage],
    progress: &vibe_core::progress::Progress,
) -> Result<InstallResolver> {
    let operation = source_operation(
        options,
        manifest,
        project_root,
        GlobalRegistrySource::ready(global.clone()),
    )?;
    build_install_resolver_with_operation(
        InstallResolverRequest {
            options,
            manifest,
            embedded_root,
            project_root,
            offline,
            locked,
            progress,
        },
        &operation,
    )
}

/// The invocation-scoped inputs every install-resolver construction reads:
/// the projected options, the selected manifest and roots, and the
/// resolved offline posture, held lock and progress handle.
///
/// ```no_run
/// use std::path::Path;
/// use vibe_core::{manifest::Manifest, progress::Progress};
/// use vibe_package_source::{
///     InstallResolverRequest, PackageSourceOptions, build_install_resolver_with_sources,
///     deferred_user_registry,
/// };
///
/// let options = PackageSourceOptions::default();
/// let manifest = Manifest::default();
/// let progress = Progress::default();
/// let request = InstallResolverRequest {
///     options: &options,
///     manifest: &manifest,
///     embedded_root: None,
///     project_root: Path::new("/work/project"),
///     offline: true,
///     locked: &[],
///     progress: &progress,
/// };
/// // `Copy`: one request feeds both the operation and the resolver build.
/// let _resolver = build_install_resolver_with_sources(request, deferred_user_registry(true))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Clone, Copy)]
pub struct InstallResolverRequest<'a> {
    pub options: &'a PackageSourceOptions,
    pub manifest: &'a Manifest,
    pub embedded_root: Option<&'a Path>,
    pub project_root: &'a Path,
    pub offline: bool,
    pub locked: &'a [vibe_core::manifest::LockedPackage],
    pub progress: &'a vibe_core::progress::Progress,
}

/// Deferred sibling used by project/global operations and preview.
pub fn build_install_resolver_with_sources(
    request: InstallResolverRequest<'_>,
    global: GlobalRegistrySource,
) -> Result<InstallResolver> {
    let operation = source_operation(
        request.options,
        request.manifest,
        request.project_root,
        global,
    )?;
    build_install_resolver_with_operation(request, &operation)
}

pub fn build_install_resolver_with_operation(
    request: InstallResolverRequest<'_>,
    operation: &SourceOperation,
) -> Result<InstallResolver> {
    let InstallResolverRequest {
        options,
        manifest,
        embedded_root,
        project_root,
        offline,
        locked,
        progress,
    } = request;
    // This is only a pure availability hint. Deferred global data is NOT empty.
    let known_global = operation.policy.global.known().cloned().unwrap_or_default();
    let global = &known_global;
    let deferred_global = operation.policy.global.known().is_none();
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
        if locked.is_empty()
            && operation.source_choice_mode()
                == vibe_registry::source_candidates::SourceChoiceMode::LegacyKnown
        {
            return Ok(InstallResolver::Local(local, solver));
        }
        // Explicit discovery controls new names only. Held endpoints are opened
        // without importing the current catalog into their source identity.
        return Ok(InstallResolver::LockedLocal {
            local,
            solver,
            held: Box::new(open_multi_from(
                manifest,
                options,
                offline,
                locked,
                project_root,
                progress,
                operation,
            )?),
        });
    }

    // The declared walk: project `[[registry]]` merged with the machine-global
    // `~/.vibe/registry.toml` (project-first, PROP-002 §2.2.2), narrowed to
    // local-only sources under the offline posture (§2.2.2.1). Computed once,
    // shared.
    let local_layer = project_local_layer(project_root)?;
    let effective = effective_registry_config(manifest, global, &local_layer, offline);

    // PROP-030 §3.3: build the local-registry family. Project-local
    // (`<project_root>/vibevm/vibepacks/`) is discovered from the current project —
    // not gated on the running vibe being source-installed, not CI-suppressed
    // (it is per-project and portable). Vibe-embedded (§2) derives from a
    // source install's `source_path`, suppressed by `--no-default-registry`
    // and the composition-root `CI` / `VIBE_NO_DEFAULT_REGISTRY` gate.
    // The family is ordered project-local first (a developer's own in-tree
    // packages win a clash), then vibe-embedded. WHICH roots those are is
    // `local_family_roots` — one answer, shared with `any_package_source`, so
    // a caller asking "is there a source at all" cannot disagree with what is
    // opened here. `project_local_count` is the number of leading locals that
    // are project-local (0 or 1), tracked so the fetch path can tag the
    // resolved package is_local (portable) vs is_embedded (machine-local).
    let (local_roots, project_local_count) =
        local_family_roots(options, embedded_root, project_root);
    let mut locals: Vec<LocalRegistry> = Vec::with_capacity(local_roots.len());
    for (index, root) in local_roots.into_iter().enumerate() {
        let lane = if index < project_local_count {
            "project-local"
        } else {
            "embedded"
        };
        locals.push(local_registry(root.clone()).map_err(|e| {
            anyhow!(
                "failed to open the {lane} registry at `{}`: {e}",
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
        let declared = if effective.registries.is_empty() && locked.is_empty() && !deferred_global {
            None
        } else {
            Some(Box::new(open_multi_from(
                manifest,
                options,
                offline,
                locked,
                project_root,
                progress,
                operation,
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

    // No local source (no project-local packages root, and no vibe-embedded or
    // it was suppressed) and no explicit registry path. Whatever the DECLARED
    // side can still serve lifts the bail and falls through to the Multi walk
    // below: a merged `[[registry]]`, a git-source install (or a re-install
    // whose manifest already carries a git-source entry — the git-source IS
    // the resolver, and the Multi path handles an empty declared set for it),
    // or, under the offline posture, the warm machine store, which is a
    // resolution source in its own right (PROP-010 §2.6).
    if !deferred_global
        && locked.is_empty()
        && !declared_source_available(&effective, manifest, options, offline)
    {
        // The project-local registry, named as the live layout spells it —
        // `vibevm/vibepacks/` today (`vibe_core::layout`), so the recipe
        // cannot go stale against a directory the tool no longer reads.
        let packages_root =
            vibe_core::machine_json_path(&vibe_core::layout::current_packages_root());
        // PROP-002 §2.2.2.1: under the offline posture (the resolved ladder,
        // PROP-010 §2.5) the remote walk is disabled and no local registry
        // survived, so there is nothing to resolve from — fail with an
        // actionable message rather than reach the network.
        if offline {
            bail!(
                "--offline: no local registry available to resolve from. \
                 Offline resolution needs a local (`file://`) `[[registry]]` — in the \
                 project `vibe.toml` or `~/.vibe/registry.toml` — a project-local \
                 `{packages_root}/` directory, the embedded registry of a source install \
                 (check `vibe self doctor`), an explicit `--registry <dir>`, or a \
                 warmed machine store (`vibe cache add <pkgref>`); \
                 remote registries are disabled under --offline."
            );
        }
        bail!(
            "no registry configured. Pass `--registry <path>`, add a `[[registry]]` \
             entry to `vibe.toml` (or `~/.vibe/registry.toml`), or place the package \
             in a project-local `{packages_root}/` directory."
        );
    }

    Ok(InstallResolver::Multi(
        Box::new(open_multi_from(
            manifest,
            options,
            offline,
            locked,
            project_root,
            progress,
            operation,
        )?),
        solver,
    ))
}

#[cfg(test)]
#[path = "flag_tests.rs"]
mod flag_tests;

#[cfg(test)]
#[path = "source_tests.rs"]
mod source_tests;
