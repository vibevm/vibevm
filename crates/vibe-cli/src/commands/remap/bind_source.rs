//! Explicit endpoint verification adds a new binding, never invented history.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use crate::{cli::RemapBindSourceArgs, output};
use anyhow::{Context, Result, bail};
use vibe_core::manifest::{LockedRemapSource, Manifest};
use vibe_install::InstallSource;

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
pub(crate) fn run(ctx: &output::Context, args: RemapBindSourceArgs, offline: bool) -> Result<()> {
    if offline {
        bail!(
            "bind-source requires selected-endpoint verification; --offline cache reuse cannot prove a new source binding"
        );
    }
    let selected = args
        .path
        .canonicalize()
        .context("locating selected project")?;
    let workspace =
        super::packages::owning_workspace(&selected).context("locating owning workspace")?;
    let publication = vibe_install::LockPublication::capture(&workspace.root)?;
    let old = publication.lockfile();
    let index = old
        .remaps
        .iter()
        .position(|row| row.package == args.package)
        .ok_or_else(|| anyhow::anyhow!("`{}` is not a held project remap", args.package))?;
    let source: LockedRemapSource = toml::from_str(
        &std::fs::read_to_string(&args.source).context("reading explicit source binding")?,
    )
    .map_err(|_| anyhow::anyhow!("invalid source binding TOML; source values omitted"))?;
    let mut bound = old.remaps[index].clone().bind_source(source)?;
    let held = bound.locked_package()?;
    // No read-store shortcut: a NEW binding must be witnessed at its supplied
    // endpoint. The operation owns all acquisition/cache output until it ends.
    let temporary = tempfile::Builder::new()
        .prefix("vibe-remap-bind-")
        .tempdir()?;
    let options = vibe_package_source::PackageSourceOptions {
        storage: Some(vibe_package_source::SourceStorage {
            acquisition_root: temporary.path().join("acquisition"),
            read_only_store: None,
        }),
        no_default_registry: true,
        no_prefer_local: true,
        ..Default::default()
    };
    let manifest = Manifest::read(selected.join(Manifest::FILENAME))?;
    let source = vibe_package_source::build_install_resolver_with_sources(
        vibe_package_source::InstallResolverRequest {
            options: &options,
            manifest: &manifest,
            embedded_root: None,
            project_root: &selected,
            offline: false,
            locked: &[held],
            progress: &ctx.progress(),
        },
        vibe_package_source::deferred_user_registry(crate::output::seeds_default_registry()),
    )?;
    let pkgref = vibe_core::PackageRef::parse(&format!("{}@={}", bound.package, bound.version))?;
    let acquired = source.resolve_and_fetch_with_progress(
        &pkgref,
        &temporary.path().join("store"),
        Some(&bound.content_hash),
        &ctx.progress(),
    )?;
    if acquired.package_meta().kind != vibe_core::PackageKind::Remap
        || acquired.resolved.version.to_string() != bound.version
        || format!("{}/{}", acquired.resolved.group, acquired.resolved.name) != bound.package
        || acquired.content_hash != bound.content_hash
    {
        bail!("explicit source does not reproduce the held remap identity; lock left unchanged");
    }
    if let Some(binding) = &mut bound.source {
        binding.resolved_commit = acquired.resolved_commit.clone();
    }
    bound.locked_package()?;
    let mut after = old.clone();
    after.remaps[index] = bound;
    let changed = publication.publish(&after)?;
    if ctx.is_json() {
        return ctx.emit_json(
            &serde_json::json!({"command":"remap:bind-source", "ok":true,
            "package":args.package, "binding":"newly_verified", "changed":changed}),
        );
    }
    ctx.summary(if changed {
        "verified source binding recorded; remap was not adopted or activated"
    } else {
        "verified source binding unchanged"
    });
    Ok(())
}
