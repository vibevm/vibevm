//! Pure expected upload catalogue for one validated 1.1.0 subject.
//! Entries describe required bytes; they do not capture or retain asset bytes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PUBLICATION-EXACT-SUBJECT");

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use vibe_wire::generated::publication_subject::{
    PublicationAsset, PublicationInstallerKind, PublicationSubject,
};

use crate::publication_subject::{
    PublicationSubjectError, PublicationSubjectExpectations, publication_subject_digest,
    validate_publication_subject,
};

const DOMAIN: &[u8] = b"vibevm.publication.expected-upload-catalogue/e1\0";
const ENTRY_COUNT: usize = 19;

pub mod vault;

/// Where a later owner must obtain and independently check the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadSource {
    TargetBundle { target: String },
    TargetBootstrap { target: String },
    TargetFragment { target: String },
    Aggregate,
    Installer { kind: PublicationInstallerKind },
}

/// Expected identity of one remote upload. This carries no payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedUpload {
    pub name: String,
    pub media_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub source: UploadSource,
}

/// Fixed canonical upload order and a domain-framed digest. This is plan
/// input, never an observation or publication authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedUploadCatalogue {
    pub subject_sha256: String,
    pub entries: Vec<ExpectedUpload>,
    pub digest: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UploadCatalogueError {
    #[error("publication subject is invalid: {0}")]
    Subject(#[from] PublicationSubjectError),
    #[error("expected upload catalogue has a repeated name `{0}`")]
    DuplicateName(String),
    #[error("expected upload catalogue size is invalid for `{0}`")]
    InvalidSize(String),
    #[error("expected upload catalogue does not contain exactly 19 entries")]
    WrongCount,
}

fn entry(
    asset: &PublicationAsset,
    source: UploadSource,
) -> Result<ExpectedUpload, UploadCatalogueError> {
    Ok(ExpectedUpload {
        name: asset.name.clone(),
        media_type: asset.media_type.clone(),
        sha256: asset.sha256.clone(),
        size_bytes: asset
            .size_bytes
            .parse()
            .map_err(|_| UploadCatalogueError::InvalidSize(asset.name.clone()))?,
        source,
    })
}

fn frame(hasher: &mut Sha256, tag: &[u8], bytes: &[u8]) {
    hasher.update((tag.len() as u64).to_be_bytes());
    hasher.update(tag);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn source_frames(hasher: &mut Sha256, source: &UploadSource) {
    match source {
        UploadSource::TargetBundle { target } => {
            frame(hasher, b"source-kind", b"target-bundle");
            frame(hasher, b"source-target", target.as_bytes());
        }
        UploadSource::TargetBootstrap { target } => {
            frame(hasher, b"source-kind", b"target-bootstrap");
            frame(hasher, b"source-target", target.as_bytes());
        }
        UploadSource::TargetFragment { target } => {
            frame(hasher, b"source-kind", b"target-fragment");
            frame(hasher, b"source-target", target.as_bytes());
        }
        UploadSource::Aggregate => frame(hasher, b"source-kind", b"aggregate"),
        UploadSource::Installer { kind } => {
            frame(hasher, b"source-kind", b"installer");
            let spelling: &[u8] = match kind {
                PublicationInstallerKind::Bash => b"bash",
                PublicationInstallerKind::Powershell => b"powershell",
                PublicationInstallerKind::Cmd => b"cmd",
            };
            frame(hasher, b"source-installer", spelling);
        }
    }
}

/// Build the exact 5×3 platform + aggregate + three-installer upload set.
/// Subject validation owns target order, role names, media, size caps and
/// digest syntax. Source archive and Cargo.lock remain non-upload witnesses.
pub fn expected_upload_catalogue(
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
) -> Result<ExpectedUploadCatalogue, UploadCatalogueError> {
    validate_publication_subject(subject, expected)?;
    let subject_sha256 = publication_subject_digest(subject, expected)?;
    let mut entries = Vec::with_capacity(ENTRY_COUNT);
    for target in &subject.targets {
        entries.push(entry(
            &target.bundle,
            UploadSource::TargetBundle {
                target: target.target.clone(),
            },
        )?);
        entries.push(entry(
            &target.bootstrap,
            UploadSource::TargetBootstrap {
                target: target.target.clone(),
            },
        )?);
        entries.push(entry(
            &target.fragment,
            UploadSource::TargetFragment {
                target: target.target.clone(),
            },
        )?);
    }
    entries.push(entry(&subject.aggregate, UploadSource::Aggregate)?);
    for installer in &subject.installers {
        entries.push(entry(
            &installer.asset,
            UploadSource::Installer {
                kind: installer.kind.clone(),
            },
        )?);
    }
    if entries.len() != ENTRY_COUNT {
        return Err(UploadCatalogueError::WrongCount);
    }
    let mut names = BTreeSet::new();
    for row in &entries {
        if !names.insert(row.name.as_str()) {
            return Err(UploadCatalogueError::DuplicateName(row.name.clone()));
        }
    }
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    frame(&mut hasher, b"subject-sha256", subject_sha256.as_bytes());
    frame(
        &mut hasher,
        b"entry-count",
        &(entries.len() as u64).to_be_bytes(),
    );
    for (index, row) in entries.iter().enumerate() {
        frame(&mut hasher, b"entry-index", &(index as u64).to_be_bytes());
        source_frames(&mut hasher, &row.source);
        frame(&mut hasher, b"name", row.name.as_bytes());
        frame(&mut hasher, b"media-type", row.media_type.as_bytes());
        frame(&mut hasher, b"size-bytes", &row.size_bytes.to_be_bytes());
        frame(&mut hasher, b"sha256", row.sha256.as_bytes());
    }
    let digest = format!("sha256:{:x}", hasher.finalize());
    Ok(ExpectedUploadCatalogue {
        subject_sha256,
        entries,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release_manifest::{
        DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, SUPPORTED_DISTRIBUTION_TARGETS,
    };

    const SUBJECT: &str = include_str!(
        "../../../formats/corpora/publication_subject/e1/valid/complete-five-targets.json"
    );

    fn fixture() -> PublicationSubject {
        serde_json::from_str(SUBJECT).unwrap()
    }
    fn expected() -> PublicationSubjectExpectations<'static> {
        PublicationSubjectExpectations {
            version: "1.1.0",
            repository: "vibevm/vibevm",
            smoke_policy_id: "distribution-smoke/e1",
            smoke_policy_epoch: 1,
        }
    }

    #[test]
    fn five_canonical_targets_and_four_final_assets_make_nineteen_metadata_only_rows() {
        let subject = fixture();
        let catalogue = expected_upload_catalogue(&subject, &expected()).unwrap();
        let names = catalogue
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        let mut wanted = Vec::new();
        for target in SUPPORTED_DISTRIBUTION_TARGETS {
            wanted.push(format!("vibevm-1.1.0-{target}.zip"));
            wanted.push(format!(
                "vibe-bootstrap-{target}{}",
                if target == "x86_64-pc-windows-msvc" {
                    ".exe"
                } else {
                    ""
                }
            ));
            wanted.push(format!("vibevm-1.1.0-{target}.fragment.json"));
        }
        wanted.push(subject.aggregate.name.clone());
        wanted.extend(subject.installers.iter().map(|row| row.asset.name.clone()));
        assert_eq!(names, wanted.iter().map(String::as_str).collect::<Vec<_>>());
        assert_eq!(catalogue.entries.len(), 19);
        assert!(catalogue.digest.starts_with("sha256:"));
        assert!(!names.contains(&"Cargo.lock"));
        assert!(!names.contains(&DISTRIBUTION_SOURCE_ARCHIVE_FILENAME));
        assert!(matches!(
            catalogue.entries[0].source,
            UploadSource::TargetBundle { .. }
        ));
        assert!(matches!(
            catalogue.entries[15].source,
            UploadSource::Aggregate
        ));
        assert!(matches!(
            catalogue.entries[18].source,
            UploadSource::Installer {
                kind: PublicationInstallerKind::Cmd
            }
        ));
        let ExpectedUpload {
            name: _,
            media_type: _,
            sha256: _,
            size_bytes: _,
            source: _,
        } = &catalogue.entries[0];
    }

    #[test]
    fn changed_asset_moves_catalogue_digest_and_wrong_or_duplicate_subject_refuses() {
        let subject = fixture();
        let baseline = expected_upload_catalogue(&subject, &expected()).unwrap();
        let mut changed = subject.clone();
        changed.targets[0].fragment.sha256 = format!("sha256:{}", "f".repeat(64));
        let changed = expected_upload_catalogue(&changed, &expected()).unwrap();
        assert_ne!(changed.digest, baseline.digest);
        let mut wrong = subject.clone();
        wrong.targets.swap(0, 1);
        assert!(expected_upload_catalogue(&wrong, &expected()).is_err());
        let mut duplicate = subject.clone();
        duplicate.targets[1].target = duplicate.targets[0].target.clone();
        assert!(expected_upload_catalogue(&duplicate, &expected()).is_err());
        let mut foreign = subject;
        foreign.repository = "other/repo".into();
        assert!(expected_upload_catalogue(&foreign, &expected()).is_err());
    }
}
