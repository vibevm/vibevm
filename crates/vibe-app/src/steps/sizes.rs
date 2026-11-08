//! Declared input sizes (`PROP-065#sizes`): which inputs a plan will fetch,
//! and the bytes it will download. A verified cached input counts zero; a
//! package-relative input is not a download.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#sizes");

use std::path::Path;

use vibe_core::manifest::{SourceInputDecl, StepDecl, StepKind};

use crate::installer::source_build::{digest_of, source_filename};
use crate::settings::SettingsRoot;

/// What a plan will download.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Downloads {
    pub bytes: u64,
    /// Inputs that will be fetched without a declared size.
    pub unknown: u32,
}

impl Downloads {
    fn add(&mut self, size: Option<u64>) {
        match size {
            Some(size) => self.bytes += size,
            None => self.unknown += 1,
        }
    }
}

/// Whether a URL source of a `sources` step is verified in the cache.
pub(crate) fn source_cached(settings: &SettingsRoot, source: &SourceInputDecl) -> bool {
    let Ok(name) = source_filename(source) else {
        return false;
    };
    let sha = source.sha256.to_ascii_lowercase();
    let cached = settings.app_sources_cache().join(&sha).join(name);
    digest_of(&cached).is_ok_and(|actual| actual == sha)
}

/// Whether the input of an `archive` or `native` step needs no download:
/// package-relative, or already verified in the work directory.
pub(crate) fn input_local(work: &Path, step: &StepDecl, source: &str, sha256: &str) -> bool {
    !source.contains("://")
        || digest_of(&super::kinds::input_file(work, step))
            .is_ok_and(|actual| actual == sha256.to_ascii_lowercase())
}

/// Sum the declared sizes of what the running steps will fetch.
pub(crate) fn downloads<'a>(
    running: impl Iterator<Item = &'a StepDecl>,
    settings: &SettingsRoot,
    work: &Path,
) -> Downloads {
    let mut total = Downloads::default();
    for step in running {
        match &step.kind {
            StepKind::Sources { sources } => {
                for source in sources {
                    if source.url.is_some() && !source_cached(settings, source) {
                        total.add(source.size);
                    }
                }
            }
            StepKind::Archive {
                source,
                sha256,
                size,
                ..
            } if !input_local(work, step, source, sha256) => total.add(*size),
            StepKind::Native { source, sha256, .. } if !input_local(work, step, source, sha256) => {
                total.add(None);
            }
            _ => {}
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use std::fs;

    use vibe_core::manifest::{ArchiveFormatDecl, StepDecl, StepKind};

    use super::*;
    use crate::installer::source_build::tests_digest;

    fn source(name: &str, url: Option<&str>, sha256: &str, size: Option<u64>) -> SourceInputDecl {
        SourceInputDecl {
            name: name.into(),
            url: url.map(Into::into),
            path: url.is_none().then(|| name.into()),
            drop: None,
            file: None,
            sha256: sha256.into(),
            size,
            extract: None,
        }
    }

    fn archive(id: &str, source: &str, sha256: &str, size: Option<u64>) -> StepDecl {
        StepDecl::new(
            id,
            id,
            StepKind::Archive {
                format: ArchiveFormatDecl::TarGz,
                source: source.into(),
                sha256: sha256.into(),
                size,
                payload_root: None,
            },
        )
    }

    #[test]
    fn only_fetched_inputs_count_and_a_cache_hit_counts_zero() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let work = dir.path().join("work");
        let cached_sha = tests_digest(b"cached");
        let cached_file = settings
            .app_sources_cache()
            .join(&cached_sha)
            .join("cached.tar.gz");
        fs::create_dir_all(cached_file.parent().unwrap()).unwrap();
        fs::write(&cached_file, b"cached").unwrap();
        let sources = StepDecl::new(
            "fetch",
            "Fetch",
            StepKind::Sources {
                sources: vec![
                    source(
                        "up",
                        Some("https://h/up.tar.gz"),
                        &"1".repeat(64),
                        Some(100),
                    ),
                    source("hit", Some("https://h/cached.tar.gz"), &cached_sha, Some(7)),
                    source("local", None, &"2".repeat(64), Some(5)),
                    source("odd", Some("https://h/odd.zip"), &"3".repeat(64), None),
                ],
            },
        );
        let steps = [
            sources,
            archive("remote", "https://h/a.tar.gz", &"4".repeat(64), Some(1000)),
            archive("packaged", "a.tar.gz", &"5".repeat(64), Some(9)),
        ];
        let total = downloads(steps.iter(), &settings, &work);
        assert_eq!(
            total,
            Downloads {
                bytes: 1100,
                unknown: 1
            }
        );
        let kept = work.join("inputs/remote");
        fs::create_dir_all(kept.parent().unwrap()).unwrap();
        fs::write(&kept, b"x").unwrap();
        let remote = archive(
            "remote",
            "https://h/a.tar.gz",
            &tests_digest(b"x"),
            Some(1000),
        );
        assert_eq!(
            downloads([remote].iter(), &settings, &work),
            Downloads::default()
        );
    }
}
