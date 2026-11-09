//! Declared input sizes (`PROP-065#sizes`): which inputs a plan will fetch,
//! and the bytes it will download. A verified cached input counts zero, so
//! does one an `[artifacts] packages` drop already holds
//! (`PROP-066#drops`); a package-relative input is not a download.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#sizes");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::path::Path;

use vibe_core::manifest::{SourceInputDecl, StepDecl, StepKind};
use vibe_core::remap::drop::{package_lookup, url_file_name};

use crate::installer::source_build::{digest_of, source_filename};
use crate::settings::SettingsRoot;

/// What a plan will download.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Downloads {
    pub bytes: u64,
    /// Inputs that will be fetched without a declared size.
    pub unknown: u32,
    /// Inputs an `[artifacts] packages` drop already holds, which the
    /// installer copies instead of fetching (`PROP-066#drops`).
    pub from_drop: u32,
}

/// Where a plan's inputs may already be, passed as data: the
/// `[artifacts] packages` drops of the invocation's one Locator and the
/// package coordinate the drop convention looks them up under
/// (`PROP-066#drops`). `vibe-app` reads no environment of its own, so the
/// caller resolves both.
#[derive(Clone, Copy, Default)]
pub(crate) struct DropLookup<'a> {
    /// Drop roots in winning order; empty when none is configured.
    pub drops: &'a [&'a Path],
    /// The `group/name@version` of the package the inputs belong to.
    pub package: &'a str,
}

impl DropLookup<'_> {
    /// Whether a configured drop already holds this input's exact bytes,
    /// so the installer will copy them and contact nothing.
    ///
    /// The prediction is the fetch's own rule, not a looser one: a drop
    /// serves the FIRST rung of the convention that holds a file, and that
    /// file is verified against the declared digest — a rung holding other
    /// bytes is a refusal the candidate list then falls through, which does
    /// reach the network. So only a matching digest counts as served.
    fn serves(self, sha256: &str, file_name: Option<&str>) -> bool {
        let digest = sha256.to_ascii_lowercase();
        self.drops.iter().any(|drop| {
            package_lookup(drop, Some(self.package), &digest, file_name)
                .iter()
                .find(|rung| rung.is_file())
                .is_some_and(|rung| digest_of(rung).is_ok_and(|actual| actual == digest))
        })
    }
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
/// package-relative, already verified in the work directory, or already
/// verified in the shared sources cache the step reads before it tries any
/// candidate (`PROP-066#locator`).
pub(crate) fn input_local(
    settings: &SettingsRoot,
    work: &Path,
    step: &StepDecl,
    source: &str,
    sha256: &str,
) -> bool {
    if !source.contains("://") {
        return true;
    }
    let digest = sha256.to_ascii_lowercase();
    let file = super::kinds::input_file(work, step);
    if digest_of(&file).is_ok_and(|actual| actual == digest) {
        return true;
    }
    let Some(name) = file.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let cached = settings.app_sources_cache().join(&digest).join(name);
    digest_of(&cached).is_ok_and(|actual| actual == digest)
}

/// Sum the declared sizes of what the running steps will fetch, and count
/// the inputs a drop will serve instead.
pub(crate) fn downloads<'a>(
    running: impl Iterator<Item = &'a StepDecl>,
    settings: &SettingsRoot,
    work: &Path,
    lookup: DropLookup<'_>,
) -> Downloads {
    let mut total = Downloads::default();
    for step in running {
        match &step.kind {
            StepKind::Sources { sources } => {
                for source in sources {
                    let Some(url) = source.url.as_deref() else {
                        continue;
                    };
                    if source_cached(settings, source) {
                        continue;
                    }
                    if lookup.serves(&source.sha256, url_file_name(url).as_deref()) {
                        total.from_drop += 1;
                        continue;
                    }
                    total.add(source.size);
                }
            }
            StepKind::Archive {
                source,
                sha256,
                size,
                ..
            } => {
                if input_local(settings, work, step, source, sha256) {
                    continue;
                }
                if lookup.serves(sha256, url_file_name(source).as_deref()) {
                    total.from_drop += 1;
                    continue;
                }
                total.add(*size);
            }
            StepKind::Native { source, sha256, .. } => {
                if input_local(settings, work, step, source, sha256) {
                    continue;
                }
                if lookup.serves(sha256, url_file_name(source).as_deref()) {
                    total.from_drop += 1;
                    continue;
                }
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
        let total = downloads(steps.iter(), &settings, &work, DropLookup::default());
        assert_eq!(
            total,
            Downloads {
                bytes: 1100,
                unknown: 1,
                from_drop: 0,
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
            downloads([remote].iter(), &settings, &work, DropLookup::default()),
            Downloads::default()
        );
    }

    /// An input an `[artifacts] packages` drop already holds is copied, not
    /// transferred: it counts zero bytes and is reported as the drop's
    /// (`PROP-066#drops`). Other bytes at the rung are a download again —
    /// that candidate is refused and the list falls through to the
    /// network. And an `archive` input already in the shared sources cache
    /// counts zero even with no drop at all (`PROP-066#locator`).
    #[test]
    fn a_drop_or_cached_input_counts_zero_and_is_reported_as_local() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let work = dir.path().join("work");
        let drop = dir.path().join("drop");
        let codkit = tests_digest(b"payload");
        let node = tests_digest(b"node");
        let steps = [
            archive(
                "codkit",
                "https://h/codkit-1.0.0.zip",
                &codkit,
                Some(16_644_318),
            ),
            StepDecl::new(
                "node-download",
                "Download the runtime",
                StepKind::Sources {
                    sources: vec![source(
                        "node",
                        Some("https://h/node-24.zip"),
                        &node,
                        Some(37_754_834),
                    )],
                },
            ),
        ];
        let drops = [drop.as_path()];
        let lookup = DropLookup {
            drops: &drops,
            package: "org.vibevm.codkit/codkit@1.0.0",
        };

        // An empty drop changes nothing: both inputs are downloads.
        assert_eq!(
            downloads(steps.iter(), &settings, &work, lookup),
            Downloads {
                bytes: 16_644_318 + 37_754_834,
                unknown: 0,
                from_drop: 0,
            }
        );

        // Both files at the first rung of the package convention.
        let rung = drop.join("org.vibevm.codkit").join("codkit").join("1.0.0");
        fs::create_dir_all(&rung).unwrap();
        fs::write(rung.join("codkit-1.0.0.zip"), b"payload").unwrap();
        fs::write(rung.join("node-24.zip"), b"node").unwrap();
        assert_eq!(
            downloads(steps.iter(), &settings, &work, lookup),
            Downloads {
                bytes: 0,
                unknown: 0,
                from_drop: 2,
            }
        );

        // Other bytes at the rung are no longer the drop's to serve.
        fs::write(rung.join("node-24.zip"), b"tampered").unwrap();
        assert_eq!(
            downloads(steps.iter(), &settings, &work, lookup),
            Downloads {
                bytes: 37_754_834,
                unknown: 0,
                from_drop: 1,
            }
        );

        // No drop configured, and the archive input already cached under
        // its digest: still not a download.
        let cached = settings.app_sources_cache().join(&codkit).join("codkit");
        fs::create_dir_all(cached.parent().unwrap()).unwrap();
        fs::write(&cached, b"payload").unwrap();
        assert_eq!(
            downloads(steps.iter(), &settings, &work, DropLookup::default()),
            Downloads {
                bytes: 37_754_834,
                unknown: 0,
                from_drop: 0,
            }
        );
    }
}
