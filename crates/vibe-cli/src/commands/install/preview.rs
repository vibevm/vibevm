//! No-publication projection of the same source-selected install metadata phase.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS");

use crate::{cli::InstallArgs, output};
use anyhow::{Context as _, Result};
use std::path::PathBuf;
use vibe_core::{GlobalRegistryConfig, PackageRef, manifest::Lockfile, user_config::UserConfig};
use vibe_orchestrator::ports::InstallManifestMutation;
use vibe_wire::generated::install::e2::plan::{InstallPlan, PlanEntry};

pub(crate) fn run(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
    seed_default_registry: bool,
) -> Result<()> {
    let scratch = tempfile::Builder::new()
        .prefix("vibe-preview-")
        .tempdir()
        .context("creating owned preview metadata scratch")?;
    let result = prepare(
        ctx,
        &args,
        embedded_root,
        root_offline,
        seed_default_registry,
        scratch.path(),
    );
    // The metadata/source handles are dropped by prepare before cleanup. Never
    // report a successful preview while its owned temporary state remains.
    let scratch_path = scratch.path().to_path_buf();
    let cleanup = scratch.close();
    let report = match (result, cleanup) {
        (Ok(report), Ok(())) => report,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(error)) => {
            return Err(error).with_context(|| {
                format!(
                    "cleaning preview metadata scratch `{}`",
                    scratch_path.display()
                )
            });
        }
        (Err(error), Err(cleanup)) => {
            return Err(error.context(format!(
                "preview metadata scratch cleanup also failed at `{}`: {cleanup}",
                scratch_path.display()
            )));
        }
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    ctx.heading(&format!(
        "Preview: {} selected packages",
        report.packages.len()
    ));
    for row in report.packages {
        let registry = row
            .source_evidence
            .as_ref()
            .and_then(|source| source.registry.as_deref())
            .unwrap_or("unknown source");
        ctx.step(&format!("{}@{} — {registry}", row.package, row.version));
    }
    Ok(())
}

fn prepare(
    ctx: &output::Context,
    args: &InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
    seed_default_registry: bool,
    scratch: &std::path::Path,
) -> Result<InstallPlan> {
    ctx.progress().checkpoint()?;
    crate::commands::app::home::refuse_library(args.library.as_deref())?;
    let root = super::resolve_project_root(&args.path)?;
    let config = UserConfig::load().context("loading user configuration for preview")?;
    let policy = args.policy(root_offline, ctx.no_recommends(), &config);
    let selected = super::SelectedManifest::read(&root).prepare().prove()?;
    let mut manifest = selected.manifest().clone();
    let mut workspace = selected.workspace().clone();
    let before = manifest.clone();
    super::CliGitSourceMutation { args }.apply(&mut manifest, &mut workspace, &root)?;
    workspace.advance_authored_selected(&root, &before, &manifest)?;
    let format = super::resolve_spec_format(&manifest, policy.spec_format_default);
    // Missing global configuration gets the same ordinary default catalog in
    // memory. It never seeds registry.toml, and an explicit empty file stays empty.
    let global = vibe_core::GlobalRegistrySource::deferred_with_site(
        vibe_core::settings::registry_config_path(),
        vibe_core::settings::site_profile_path(),
        if seed_default_registry {
            GlobalRegistryConfig {
                registries: vibe_core::default_registries(),
                ..Default::default()
            }
        } else {
            GlobalRegistryConfig::default()
        },
    );
    let mut options = super::resolver::package_source_options(args);
    options.storage = Some(vibe_package_source::SourceStorage {
        acquisition_root: scratch.to_path_buf(),
        read_only_store: preview_store_root()?,
    });
    // One pure operation context serves packages and remap metadata. Creating
    // it reads no global catalog, SSH file or Git executable.
    let operation = vibe_package_source::source_operation(&options, &manifest, &root, global)?;
    let lock_path = workspace.root.join(Lockfile::FILENAME);
    let locked = match std::fs::symlink_metadata(&lock_path) {
        Ok(metadata) if metadata.file_type().is_file() => Lockfile::read(&lock_path)?,
        Ok(_) => anyhow::bail!(
            "preview lock is not a regular file: {}",
            lock_path.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Lockfile::empty(vibe_orchestrator::generated_by(), "preview")
        }
        Err(error) => return Err(error).context("observing preview lock"),
    };
    let packages = if vibe_install::preview_has_requirements(
        &workspace,
        &manifest,
        !args.packages.is_empty(),
        &locked,
    ) {
        let source = vibe_package_source::build_install_resolver_with_operation(
            vibe_package_source::InstallResolverRequest {
                options: &options,
                manifest: &manifest,
                embedded_root: embedded_root.as_deref(),
                project_root: &root,
                offline: policy.offline,
                locked: &locked.packages,
                progress: &ctx.progress(),
            },
            &operation,
        )?
        .with_locked_checkouts(vibe_install::locked_checkout_paths(
            &workspace.root,
            &locked.packages,
        ))
        // The plan shows exactly what the install would do, including an
        // explicit version move at a held source.
        .with_locked_updates_for(&super::resolver::explicit_version_moves(
            args,
            &locked.packages,
        ));
        let roots = args
            .packages
            .iter()
            .map(|raw| {
                let parsed = PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))?;
                crate::commands::short_name::qualify(&source, &parsed, &locked)
            })
            .collect::<Result<Vec<_>>>()?;
        let request = vibe_install::InstallRequest {
            roots,
            features: vibe_resolver::FeatureRequest {
                explicit: args.features.clone(),
                no_defaults: args.no_default_features,
                all: args.all_features,
            },
            language: args.language.clone(),
            exact: args.exact,
            no_recommends: policy.no_recommends,
            generated_by: vibe_orchestrator::generated_by(),
            offline: policy.offline,
        };
        let observer = super::events::CtxObserver::new(ctx).with_progress(ctx.progress());
        let storage = options
            .storage
            .as_ref()
            .ok_or(vibe_install::Error::PreviewStorageRequired)?;
        let retained_store = storage.acquisition_root.join("retained-configuration");
        let preview = vibe_install::preview_prepared(
            &source,
            &root,
            &mut manifest,
            &mut workspace,
            &locked,
            vibe_install::PreviewSourceStorage {
                read_only_store: storage.read_only_store.as_deref(),
                acquisition_store: &retained_store,
            },
            request,
            format,
            &observer,
        )?;
        preview
            .package_sources()
            .map(|(group, name, version, evidence)| PlanEntry {
                package: format!("{group}/{name}"),
                version: version.to_string(),
                source_hosting: vibe_registry::source_hosting(Some(&evidence)),
                source_evidence: Some(evidence),
                source_candidates: preview
                    .package_candidates(group, name)
                    .map(super::report::candidate_rows),
            })
            .collect()
    } else {
        Vec::new()
    };
    let remaps = crate::commands::remap::packages::preview_project_requirements(
        &manifest,
        &locked,
        &operation,
        &root,
        &options,
        policy.offline,
        &ctx.progress(),
    )?;
    let complete = remaps.iter().all(|row| {
        matches!(
            row.outcome,
            vibe_wire::generated::install::e2::plan::RemapPlanOutcome::OfferOnly
        )
    });
    ctx.progress().checkpoint()?;
    Ok(super::report::hostings::with_hostings(InstallPlan {
        command: "install:plan".into(),
        packages,
        preview_complete: Some(complete),
        remaps: Some(remaps),
        hostings: None,
    }))
}

/// An unavailable settings home means there is no old store to read, not a
/// reason to create one or refuse metadata available from local sources.
pub(crate) fn preview_store_root() -> Result<Option<PathBuf>> {
    match vibe_registry::store::store_root() {
        Ok(path) => Ok(Some(path)),
        Err(vibe_registry::RegistryError::NoHomeDir) => Ok(None),
        Err(error) => Err(error.into()),
    }
}
