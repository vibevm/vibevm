//! Read-only local capture and pure construction of a publication subject.
//!
//! The distribution root is supplied as a pinned `Project`. Large assets are
//! witnessed by bounded two-pass streaming observations; this module never
//! retains ZIP bytes, writes the output tree, or contacts a release host.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PUBLICATION-EXACT-SUBJECT");

use sha2::{Digest, Sha256};
use vibe_safefs::{Project, StableFileState};
use vibe_wire::generated::publication_subject::{
    PublicationAsset, PublicationComponent, PublicationComponentName, PublicationInstaller,
    PublicationInstallerKind, PublicationSmoke, PublicationSource, PublicationSourceArchive,
    PublicationSubject, PublicationTarget,
};

use crate::publication_subject::{
    PublicationSubjectError, PublicationSubjectExpectations, publication_subject_digest,
    validate_publication_subject,
};
use crate::release_manifest::{
    AggregateDistributionManifest, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
    DISTRIBUTION_BASH_INSTALLER_FILENAME, DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
    DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_CMD_INSTALLER_FILENAME,
    DISTRIBUTION_MANIFEST_MAX_BYTES, DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
    DISTRIBUTION_PRODUCT, DISTRIBUTION_REPOSITORY, DistributionComponentName,
    PlatformDistributionFragment, ReleaseManifestError, SUPPORTED_DISTRIBUTION_TARGETS,
};

const VERSION: &str = "1.1.0";
const SMOKE_POLICY: &str = "distribution-smoke/e1";

#[derive(Debug, thiserror::Error)]
pub enum PublicationCaptureError {
    #[error("unsupported mutable distribution version `{0}`; expected 1.1.0")]
    Version(String),
    #[error("distribution {role} `{name}` is absent")]
    Missing { role: &'static str, name: String },
    #[error("distribution {role} `{name}` cannot be observed safely: {reason}")]
    Observation {
        role: &'static str,
        name: String,
        reason: String,
    },
    #[error("distribution {role} `{name}` differs from its fragment evidence")]
    Mismatch { role: &'static str, name: String },
    #[error("distribution fragment `{name}` is invalid: {source}")]
    Fragment {
        name: String,
        #[source]
        source: ReleaseManifestError,
    },
    #[error("distribution aggregate is invalid: {0}")]
    Aggregate(#[from] ReleaseManifestError),
    #[error("publication subject is invalid: {0}")]
    Subject(#[from] PublicationSubjectError),
}

#[derive(Debug, Clone)]
struct CapturedTarget {
    fragment: PlatformDistributionFragment,
    fragment_state: StableFileState,
    bundle_state: StableFileState,
    bootstrap_state: StableFileState,
}

/// Owned observations of the five local distribution outputs. No source or
/// installer authority is inferred from the live tree, and no ZIP bytes are
/// held after the streaming witness completes.
#[derive(Debug, Clone)]
pub struct CapturedDistribution {
    version: String,
    targets: Vec<CapturedTarget>,
}

/// Claimed bytes and source IDs supplied by a caller. This type does not
/// authenticate provenance: the xtask caller must prove that Cargo.lock and
/// all three installer byte strings came from the selected commit through
/// its bounded commit-file reader. Neither this value nor the resulting
/// inspectable subject grants a publication effect.
pub struct CommittedInputBytes<'a> {
    pub commit: &'a str,
    pub tree: &'a str,
    pub cargo_lock: &'a [u8],
    pub bash_installer: &'a [u8],
    pub powershell_installer: &'a [u8],
    pub cmd_installer: &'a [u8],
    pub smoke: PublicationSmoke,
}

/// Inspectable subject and its deterministic aggregate bytes. This is data,
/// not an apply-ready verdict or an effect grant.
pub struct CapturedPublicationSubject {
    pub subject: PublicationSubject,
    pub subject_digest: String,
    pub aggregate_bytes: Vec<u8>,
}

fn bundle_name(target: &str) -> String {
    format!("vibevm-{VERSION}-{target}.zip")
}

fn fragment_name(target: &str) -> String {
    format!("vibevm-{VERSION}-{target}.fragment.json")
}

fn bootstrap_name(target: &str) -> String {
    let suffix = if target == "x86_64-pc-windows-msvc" {
        ".exe"
    } else {
        ""
    };
    format!("vibe-bootstrap-{target}{suffix}")
}

fn observe(
    project: &Project,
    role: &'static str,
    name: &str,
    cap: u64,
) -> Result<StableFileState, PublicationCaptureError> {
    project
        .stable_file_state_bounded(name, cap)
        .map_err(|error| PublicationCaptureError::Observation {
            role,
            name: name.into(),
            reason: format!("{error:#}"),
        })?
        .ok_or_else(|| PublicationCaptureError::Missing {
            role,
            name: name.into(),
        })
}

fn matches_asset(
    state: &StableFileState,
    asset: &crate::release_manifest::DistributionAsset,
    name: &str,
) -> bool {
    asset.name == name
        && asset.size == state.bytes
        && asset.digest == format!("sha256:{}", state.sha256)
}

/// Capture exact named output files through one already-pinned local root.
/// The fragment parser owns relational distribution validation; this reader
/// additionally joins its declared bundle/bootstrap asset bytes to current
/// stable file witnesses before the result may be used by the pure builder.
pub fn capture_distribution(
    project: &Project,
    version: &str,
) -> Result<CapturedDistribution, PublicationCaptureError> {
    if version != VERSION {
        return Err(PublicationCaptureError::Version(version.into()));
    }
    let mut targets = Vec::with_capacity(SUPPORTED_DISTRIBUTION_TARGETS.len());
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let fragment_name = fragment_name(target);
        let fragment_state = observe(
            project,
            "fragment",
            &fragment_name,
            DISTRIBUTION_MANIFEST_MAX_BYTES,
        )?;
        let fragment_bytes = project
            .read_file_bounded(&fragment_name, DISTRIBUTION_MANIFEST_MAX_BYTES as usize)
            .map_err(|error| PublicationCaptureError::Observation {
                role: "fragment",
                name: fragment_name.clone(),
                reason: format!("{error:#}"),
            })?
            .ok_or_else(|| PublicationCaptureError::Missing {
                role: "fragment",
                name: fragment_name.clone(),
            })?;
        if fragment_state.bytes != fragment_bytes.len() as u64
            || fragment_state.sha256 != format!("{:x}", Sha256::digest(&fragment_bytes))
        {
            return Err(PublicationCaptureError::Mismatch {
                role: "fragment",
                name: fragment_name,
            });
        }
        let fragment =
            PlatformDistributionFragment::from_json_slice(&fragment_bytes).map_err(|source| {
                PublicationCaptureError::Fragment {
                    name: fragment_name.clone(),
                    source,
                }
            })?;
        if fragment.target != target || fragment.version != VERSION {
            return Err(PublicationCaptureError::Mismatch {
                role: "fragment identity",
                name: fragment_name,
            });
        }
        let bundle_name = bundle_name(target);
        let bundle_state = observe(
            project,
            "bundle",
            &bundle_name,
            DISTRIBUTION_BUNDLE_MAX_BYTES,
        )?;
        if !matches_asset(&bundle_state, &fragment.asset, &bundle_name) {
            return Err(PublicationCaptureError::Mismatch {
                role: "bundle",
                name: bundle_name,
            });
        }
        let bootstrap_name = bootstrap_name(target);
        let bootstrap_state = observe(
            project,
            "bootstrap",
            &bootstrap_name,
            DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        )?;
        if !matches_asset(&bootstrap_state, &fragment.bootstrap, &bootstrap_name) {
            return Err(PublicationCaptureError::Mismatch {
                role: "bootstrap",
                name: bootstrap_name,
            });
        }
        targets.push(CapturedTarget {
            fragment,
            fragment_state,
            bundle_state,
            bootstrap_state,
        });
    }
    Ok(CapturedDistribution {
        version: version.into(),
        targets,
    })
}

fn asset(name: &str, media_type: &str, size: u64, sha256: &str) -> PublicationAsset {
    PublicationAsset {
        name: name.into(),
        media_type: media_type.into(),
        size_bytes: size.to_string(),
        sha256: format!("sha256:{sha256}"),
    }
}

fn committed_asset(name: &str, media_type: &str, bytes: &[u8]) -> PublicationAsset {
    asset(
        name,
        media_type,
        bytes.len() as u64,
        &format!("{:x}", Sha256::digest(bytes)),
    )
}

/// Construct the complete subject without opening a path or invoking a
/// publication host. Smoke status is retained exactly; this function does
/// not convert a skip/unavailable probe into an apply-ready verdict.
pub fn subject_from_captured(
    capture: &CapturedDistribution,
    committed: CommittedInputBytes<'_>,
) -> Result<CapturedPublicationSubject, PublicationCaptureError> {
    for captured in &capture.targets {
        if captured.fragment.source_commit != committed.commit {
            return Err(PublicationCaptureError::Mismatch {
                role: "committed source",
                name: fragment_name(&captured.fragment.target),
            });
        }
    }
    let fragments = capture
        .targets
        .iter()
        .map(|target| target.fragment.clone())
        .collect::<Vec<_>>();
    let aggregate_bytes =
        AggregateDistributionManifest::from_platforms(fragments)?.to_json_bytes()?;
    let mut targets = Vec::with_capacity(capture.targets.len());
    for captured in &capture.targets {
        let fragment = &captured.fragment;
        let components = fragment
            .bundle
            .components
            .iter()
            .map(|component| PublicationComponent {
                name: match component.name {
                    DistributionComponentName::Vibe => PublicationComponentName::Vibe,
                    DistributionComponentName::VibeIndex => PublicationComponentName::VibeIndex,
                },
                path: component.path.clone(),
                sha256: component.digest.clone(),
                size_bytes: component.size.to_string(),
            })
            .collect();
        targets.push(PublicationTarget {
            target: fragment.target.clone(),
            bundle: asset(
                &fragment.asset.name,
                "application/zip",
                captured.bundle_state.bytes,
                &captured.bundle_state.sha256,
            ),
            bootstrap: asset(
                &fragment.bootstrap.name,
                "application/octet-stream",
                captured.bootstrap_state.bytes,
                &captured.bootstrap_state.sha256,
            ),
            fragment: asset(
                &fragment_name(&fragment.target),
                "application/json",
                captured.fragment_state.bytes,
                &captured.fragment_state.sha256,
            ),
            components,
            source_archive: PublicationSourceArchive {
                path: fragment.bundle.source_archive.path.clone(),
                sha256: fragment.bundle.source_archive.digest.clone(),
                size_bytes: fragment.bundle.source_archive.size.to_string(),
                tree_oid: fragment.bundle.source_archive.tree_oid.clone(),
            },
        });
    }
    let installers = [
        (
            PublicationInstallerKind::Bash,
            DISTRIBUTION_BASH_INSTALLER_FILENAME,
            "text/x-shellscript",
            committed.bash_installer,
        ),
        (
            PublicationInstallerKind::Powershell,
            DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
            "text/plain",
            committed.powershell_installer,
        ),
        (
            PublicationInstallerKind::Cmd,
            DISTRIBUTION_CMD_INSTALLER_FILENAME,
            "text/plain",
            committed.cmd_installer,
        ),
    ]
    .into_iter()
    .map(|(kind, name, media_type, bytes)| PublicationInstaller {
        kind,
        asset: committed_asset(name, media_type, bytes),
    })
    .collect();
    let subject = PublicationSubject {
        schema: 1,
        product: DISTRIBUTION_PRODUCT.into(),
        repository: DISTRIBUTION_REPOSITORY.into(),
        version: capture.version.clone(),
        mutable: true,
        source: PublicationSource {
            commit: committed.commit.into(),
            tree: committed.tree.into(),
            cargo_lock: committed_asset("Cargo.lock", "text/plain", committed.cargo_lock),
        },
        targets,
        aggregate: committed_asset(
            DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
            "application/json",
            &aggregate_bytes,
        ),
        installers,
        smoke: committed.smoke,
    };
    let expected = PublicationSubjectExpectations {
        version: VERSION,
        repository: DISTRIBUTION_REPOSITORY,
        smoke_policy_id: SMOKE_POLICY,
        smoke_policy_epoch: 1,
    };
    validate_publication_subject(&subject, &expected)?;
    let subject_digest = publication_subject_digest(&subject, &expected)?;
    Ok(CapturedPublicationSubject {
        subject,
        subject_digest,
        aggregate_bytes,
    })
}

#[cfg(test)]
#[path = "publication_capture/tests.rs"]
mod tests;
