//! Read-only source-owned kind preflight.
use super::*;
pub(crate) struct Selection {
    pub(super) resolver: InstallResolver,
    pub(super) candidates: Vec<vibe_registry::source_candidates::RetainedSourceCandidate>,
    _scratch: Option<tempfile::TempDir>,
}

pub(crate) fn claims_with_progress(
    args: &InstallArgs,
    offline: bool,
    progress: &vibe_core::progress::Progress,
) -> Option<Selection> {
    if args.packages.is_empty() {
        return None;
    }
    progress.checkpoint().ok()?;
    let root = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());
    let manifest = if root.join(Manifest::FILENAME).exists() {
        Manifest::read(root.join(Manifest::FILENAME)).ok()?
    } else {
        Manifest::default()
    };
    let scratch = if args.plan_only {
        Some(
            tempfile::Builder::new()
                .prefix("vibe-remap-routing-preview-")
                .tempdir()
                .ok()?,
        )
    } else {
        None
    };
    let mut options = crate::commands::install::resolver::package_source_options(args);
    if let Some(scratch) = &scratch {
        options.storage = Some(vibe_package_source::SourceStorage {
            acquisition_root: scratch.path().into(),
            read_only_store: None,
        });
    }
    let resolver = vibe_package_source::build_install_resolver_with_sources(
        vibe_package_source::InstallResolverRequest {
            options: &options,
            manifest: &manifest,
            embedded_root: None,
            project_root: &root,
            offline: offline || args.offline,
            locked: &[],
            progress,
        },
        vibe_package_source::deferred_user_registry(false),
    )
    .ok()?;
    let mut candidates = Vec::with_capacity(args.packages.len());
    for raw in &args.packages {
        progress.checkpoint().ok()?;
        let parsed = PackageRef::parse(raw).ok()?;
        let pkgref = crate::commands::short_name::qualify(
            &resolver,
            &parsed,
            &Lockfile::empty("remap-preflight", "0"),
        )
        .ok()?;
        let candidate = resolver.select_top_level(&pkgref).ok()?;
        progress.checkpoint().ok()?;
        if candidate.manifest().package.as_ref().map(|meta| meta.kind) != Some(PackageKind::Remap) {
            return None;
        }
        candidates.push(candidate);
    }
    Some(Selection {
        resolver,
        candidates,
        _scratch: scratch,
    })
}
