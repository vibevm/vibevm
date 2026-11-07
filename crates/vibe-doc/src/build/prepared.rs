//! Finite selected observations retained until the existing output writer.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-CENSUS",
    r = 1
);
use super::*;
use crate::citations::sources::CapturedPages;
use std::sync::Arc;

/// A rendered build retaining selected sources until publication.
/// Plain [`Built`] values are checked snapshots, not source observation guards.
#[derive(Debug)]
pub struct PreparedBuild {
    pub(super) built: Built,
    pages: Arc<CapturedPages>,
    root: std::path::PathBuf,
    borrowed: Option<Arc<CapturedPages>>,
}
impl PreparedBuild {
    pub fn built(&self) -> &Built {
        &self.built
    }
    pub fn recheck(&self) -> Result<()> {
        self.pages.recheck().map_err(|message| DocError::Page {
            path: self.root.clone(),
            message,
        })?;
        if let Some(borrowed) = &self.borrowed {
            borrowed.recheck().map_err(|message| DocError::Page {
                path: self.root.clone(),
                message,
            })?;
        }
        Ok(())
    }
    /// Recheck the retained observation immediately before the existing writer.
    pub fn write(&self, out: &Path) -> Result<()> {
        self.recheck()?;
        write(&self.built, out)
    }
}
/// Prepare a build whose selected page observations survive rendering and writing.
/// ```no_run
/// use vibe_doc::{build, citations::SpecSources};
/// # fn publish(options: &build::Options) -> vibe_doc::Result<()> {
/// let prepared = build::prepare_observed(std::path::Path::new("manual"), &SpecSources::new(), options, &vibe_core::progress::Progress::default())?;
/// prepared.write(std::path::Path::new("output"))?;
/// # Ok(()) }
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-CENSUS",
    r = 1
)]
pub fn prepare_observed(
    package_dir: &Path,
    sources: &SpecSources,
    options: &Options,
    progress: &Progress,
) -> Result<PreparedBuild> {
    let captured =
        Arc::new(
            sources
                .capture_pages(package_dir)
                .map_err(|message| DocError::Page {
                    path: package_dir.into(),
                    message,
                })?,
        );
    let set = captured.set();
    let borrowed = translations::capture_borrowed(package_dir, sources)?.map(Arc::new);
    let examples = borrowed
        .as_ref()
        .map(|pages| translations::examples_from(pages))
        .unwrap_or_default();
    let content = content_for_set(
        package_dir,
        sources,
        &options.base,
        options.derived.clone(),
        set,
        Some(examples),
    )?;
    let built_manifest = observed(progress, "Building documentation manifest", || {
        manifest::build_for_set(package_dir, sources, &options.manifest, set)
    })?;
    let built = finish(
        package_dir,
        set,
        &content,
        built_manifest,
        options,
        progress,
        declared_kind(package_dir),
    )?;
    let prepared = PreparedBuild {
        built,
        pages: captured,
        root: package_dir.into(),
        borrowed,
    };
    prepared.recheck()?;
    Ok(prepared)
}
