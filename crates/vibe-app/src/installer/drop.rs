//! Application inputs read from an `[artifacts] packages` drop
//! (`PROP-066#drops`, `PROP-066#drops-author`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::fs;
use std::path::Path;

use vibe_core::remap::Artifact;
use vibe_core::remap::drop::shown_path;

use super::{io, source_build};
use crate::error::InstallerError;

/// Read an input from an `[artifacts] packages` drop by the package
/// convention (`PROP-066#drops`): the first rung that holds the file is
/// verified against the declared digest. A wrong file is a digest refusal;
/// a missing one names the expected file name and every path searched.
pub(crate) fn copy_from_drop(
    drop: &Path,
    artifact: &Artifact,
    sha256: &str,
    destination: &Path,
) -> Result<(), InstallerError> {
    let file = artifact
        .url
        .as_deref()
        .and_then(vibe_core::remap::drop::url_file_name);
    copy_from_drop_named(
        drop,
        artifact.package.as_deref(),
        file.as_deref(),
        sha256,
        destination,
    )
}

/// The first place the package convention looks for this input in `drop` —
/// the exact path a person creates to serve it (`PROP-066#drops`), in this
/// platform's one separator style. `None` when the input names no file the
/// drop could be searched by.
pub(crate) fn first_rung(drop: &Path, artifact: &Artifact, sha256: &str) -> Option<String> {
    let file = artifact
        .url
        .as_deref()
        .and_then(vibe_core::remap::drop::url_file_name);
    vibe_core::remap::drop::package_lookup(
        drop,
        artifact.package.as_deref(),
        sha256,
        file.as_deref(),
    )
    .first()
    .map(|rung| shown_path(rung))
}

/// The `declared origin: …` entry a candidate list ends with when the
/// active source policy refused the declared origin itself.
///
/// The admission refusal's own fix — "use an admitted HTTP endpoint or a
/// separately supported transport capability" — is right for a registry on
/// an open network and wrong for an application input on a closed one: the
/// endpoint is excluded on purpose, and the one thing the reader can do is
/// fill the drop that has just missed. So when a drop was searched for
/// this input, the entry keeps the refusal's own sentence and spec address
/// — never a second spelling of either — and replaces only its fix with
/// the exact rung to put the file at. With no drop in play the refusal is
/// rendered unchanged.
pub(crate) fn declared_origin_refused(
    refusal: &vibe_registry::ContactRefusal,
    rung: Option<&str>,
) -> String {
    let shown = refusal.to_string();
    let with_drop_fix = match (refusal, rung) {
        (vibe_registry::ContactRefusal::ExcludedHost { .. }, Some(rung)) => {
            shown.split_once("; fix: ").map(|(before_fix, _)| {
                format!(
                    "{before_fix}; fix: put the file at `{rung}` in the `[artifacts] packages` \
                     drop, or stop excluding this endpoint in `[source_policy] exclude_hosting` \
                     — `vibe artifacts where` prints every expected location)"
                )
            })
        }
        _ => None,
    };
    format!(
        "declared origin: {}",
        with_drop_fix.as_deref().unwrap_or(shown.as_str())
    )
}

/// [`copy_from_drop`] for an input whose file name is already known — the
/// author-side `drop = "packages"` input names it directly.
pub(crate) fn copy_from_drop_named(
    drop: &Path,
    package: Option<&str>,
    file: Option<&str>,
    sha256: &str,
    destination: &Path,
) -> Result<(), InstallerError> {
    let rungs = vibe_core::remap::drop::package_lookup(drop, package, sha256, file);
    let shown = shown_path(drop);
    let Some(found) = rungs.iter().find(|rung| rung.is_file()) else {
        let searched = rungs
            .iter()
            .map(|rung| format!("`{}`", shown_path(rung)))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(InstallerError::Fetch {
            source_ref: shown,
            reason: match file {
                Some(file) => format!(
                    "the `[artifacts] packages` drop holds no `{file}` (sha256 {sha256}); \
                     searched {searched} — put the file at the first of these paths \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-066#drops; \
                     `vibe artifacts where` prints the expected locations)"
                ),
                None => "the input declares no file name to look for in the drop".to_owned(),
            },
        });
    };
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| io("creating", parent, error))?;
    }
    fs::copy(found, destination).map_err(|error| io("copying", destination, error))?;
    let actual =
        source_build::digest_of(destination).map_err(|error| io("reading", destination, error))?;
    if actual != sha256.to_ascii_lowercase() {
        let _ = fs::remove_file(destination);
        return Err(InstallerError::Digest {
            source_ref: shown_path(found),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use sha2::{Digest, Sha256};
    use vibe_core::remap::{Artifact, ByteLocation, Locator};

    use super::super::{FetchWatch, Located, ServedBy, fetch_located, hex};
    use super::{declared_origin_refused, first_rung, shown_path};
    use crate::error::InstallerError;

    fn drop_locator(drop: &Path) -> Locator {
        Locator::from_sources(
            vec![vibe_core::remap::LoadedSource {
                layer: vibe_core::remap::Layer::Local,
                source: "config.toml".into(),
                rows: vec![vibe_core::remap::drop::package_drop_row(
                    drop,
                    vibe_core::remap::Layer::Local,
                    "config.toml",
                )],
                policy: Default::default(),
            }],
            Vec::new(),
        )
    }

    fn drop_input(digest: &str) -> Artifact {
        Artifact::new(vibe_core::remap::ArtifactKind::AppInput)
            .with_url("https://dist.example/releases/tool-1.0.tgz")
            .with_digest(digest)
            .with_package("org.example/tool@1.0.0")
    }

    fn fetch_from(
        locator: &Locator,
        artifact: &Artifact,
        digest: &str,
        out: &Path,
    ) -> Result<ServedBy, InstallerError> {
        fetch_located(
            &Located {
                locator,
                artifact,
                declared: "https://dist.example/releases/tool-1.0.tgz",
                sha256: digest,
                filename: Some("install"),
            },
            out.parent().unwrap(),
            out,
            true,
            &FetchWatch::default(),
        )
    }

    /// Each rung of the package convention serves the input, in order,
    /// offline, with the digest verified (`PROP-066#drops`).
    #[test]
    fn the_package_drop_serves_every_rung_of_its_convention() {
        let dir = tempfile::tempdir().unwrap();
        let drop = dir.path().join("drop");
        let digest = hex(&Sha256::digest(b"payload"));
        let rungs = [
            drop.join("org.example/tool/1.0.0/tool-1.0.tgz"),
            drop.join("org.example/tool/tool-1.0.tgz"),
            drop.join(&digest).join("tool-1.0.tgz"),
            drop.join("tool-1.0.tgz"),
        ];
        let locator = drop_locator(&drop);
        let artifact = drop_input(&digest);
        for rung in rungs.iter().rev() {
            fs::create_dir_all(rung.parent().unwrap()).unwrap();
            fs::write(rung, b"payload").unwrap();
            let out = dir.path().join("out").join("install");
            let _ = fs::remove_file(&out);
            let served = fetch_from(&locator, &artifact, &digest, &out).unwrap();
            assert_eq!(fs::read(&out).unwrap(), b"payload");
            assert_eq!(served.target, drop.display().to_string());
            assert_eq!(
                served.row.unwrap().source,
                vibe_core::remap::drop::PACKAGES_DROP_SOURCE
            );
        }
    }

    /// A wrong file in the drop is a digest refusal, and a missing one
    /// names the expected file and every searched path.
    #[test]
    fn the_package_drop_refuses_wrong_bytes_and_explains_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let drop = dir.path().join("drop");
        let digest = hex(&Sha256::digest(b"payload"));
        let locator = drop_locator(&drop);
        let artifact = drop_input(&digest);
        let out = dir.path().join("out").join("install");
        let missing = fetch_from(&locator, &artifact, &digest, &out)
            .unwrap_err()
            .to_string();
        assert!(missing.contains("holds no `tool-1.0.tgz`"), "{missing}");
        assert!(
            missing.contains("org.example/tool/1.0.0/tool-1.0.tgz"),
            "{missing}"
        );
        // Every searched path is shown in one separator style, whatever
        // style the configured drop was written in.
        assert!(
            missing.contains(&shown_path(
                &drop.join("org.example/tool/1.0.0/tool-1.0.tgz")
            )),
            "{missing}"
        );
        assert!(missing.contains("PROP-066#drops"), "{missing}");

        fs::create_dir_all(&drop).unwrap();
        fs::write(drop.join("tool-1.0.tgz"), b"tampered").unwrap();
        let wrong = fetch_from(&locator, &artifact, &digest, &out)
            .unwrap_err()
            .to_string();
        assert!(
            wrong.contains("does not match its declared sha256"),
            "{wrong}"
        );
        assert!(!out.exists(), "a refused file is not kept");
    }

    /// A declared origin the source policy excludes points at the drop
    /// that has just missed, not at another endpoint (`PROP-066#drops`):
    /// the entry keeps the admission refusal's own sentence and spec
    /// address and replaces only its fix with the exact first rung. With no
    /// drop in play the refusal is rendered unchanged.
    #[test]
    fn an_excluded_declared_origin_names_the_drop_that_missed() {
        let refusal = vibe_registry::ContactRefusal::ExcludedHost {
            endpoint: "https://github.com".into(),
        };
        let without = declared_origin_refused(&refusal, None);
        assert_eq!(without, format!("declared origin: {refusal}"));
        assert!(
            without.contains("use an admitted HTTP endpoint"),
            "{without}"
        );

        let dir = tempfile::tempdir().unwrap();
        let drop = dir.path().join("drop");
        let digest = hex(&Sha256::digest(b"payload"));
        let rung = first_rung(&drop, &drop_input(&digest), &digest)
            .expect("a named input has a first rung");
        assert_eq!(
            rung,
            shown_path(&drop.join("org.example/tool/1.0.0/tool-1.0.tgz"))
        );

        let with_drop = declared_origin_refused(&refusal, Some(&rung));
        assert!(
            with_drop.starts_with(
                "declared origin: excluded HTTP endpoint https://github.com (violates "
            ),
            "{with_drop}"
        );
        assert!(
            with_drop.contains("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW"),
            "{with_drop}"
        );
        assert!(
            with_drop.contains(&format!("fix: put the file at `{rung}` in the")),
            "{with_drop}"
        );
        assert!(
            !with_drop.contains("use an admitted HTTP endpoint"),
            "the fix no longer points away from the drop: {with_drop}"
        );
    }

    /// A `file://` remap target is read locally, exactly like a `path` one.
    #[test]
    fn a_file_url_target_is_read_like_a_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mirror").join("tool-1.0.tgz");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"payload").unwrap();
        let digest = hex(&Sha256::digest(b"payload"));
        let url = ByteLocation::Local(file.clone()).to_url();
        let row: vibe_core::remap::RowDecl =
            toml::from_str(&format!("sha256 = \"{digest}\"\nurl = \"{url}\"")).unwrap();
        let locator = Locator::from_sources(
            vec![vibe_core::remap::LoadedSource {
                layer: vibe_core::remap::Layer::Local,
                source: "settings.toml".into(),
                rows: vec![
                    row.into_row(vibe_core::remap::Provenance::new(
                        vibe_core::remap::Layer::Local,
                        "settings.toml",
                        0,
                    ))
                    .unwrap(),
                ],
                policy: Default::default(),
            }],
            Vec::new(),
        );
        let out = dir.path().join("out").join("install");
        fetch_from(&locator, &drop_input(&digest), &digest, &out).unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"payload");
    }
}
