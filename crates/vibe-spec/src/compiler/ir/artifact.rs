//! Immutable identity and ordered input plan for one final compiler artifact.

use crate::SpecAddress;

use super::{ArtifactTarget, DocumentAddress, DocumentProvider, DocumentSubject, SourceIr};

/// The validation laws and the refusal family they raise, out of line per
/// the file-length budget. Every name a caller used before the split is
/// re-exported here, so no path moved.
mod validate;

pub use validate::ArtifactPlanError;
pub(crate) use validate::validate_package_relation;
use validate::{simple_alias_machinery, validate_declared_path, validate_text};

/// Open identity of one final compiler artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, ArtifactPlanError> {
        let value = value.into();
        validate_text("artifact id", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Provenance of one top-level static contribution in effective-boot order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContributionMeta {
    pub(crate) origin: String,
    pub(crate) path: String,
}

impl ContributionMeta {
    pub(crate) fn new(
        origin: impl Into<String>,
        path: impl Into<String>,
    ) -> Result<Self, ArtifactPlanError> {
        let value = Self {
            origin: origin.into(),
            path: path.into(),
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), ArtifactPlanError> {
        validate_text("contribution origin", &self.origin)?;
        validate_text("contribution path", &self.path)?;
        validate_declared_path("contribution path", &self.path)
    }
}

/// The compatibility/static-lane policy carried through the whole artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticCompileMode {
    Plain,
    QualifyPerNode,
}

/// Frame policy surrounding all contributions of one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArtifactFrame {
    StaticLane {
        generated_path: String,
        source_root: String,
    },
    CompatibilityFragment,
}

/// The reserved artifact id of the BUILTIN compatibility row: a
/// `static-fragment` rendered by the `static-md` target/backend. It is the one
/// established row whose artifact id is not its backend id, so both the tuple
/// law and the wire's EMIT IDENTITY gate read it from here rather than
/// spelling the literal twice.
pub(crate) const COMPATIBILITY_ARTIFACT_ID: &str = "static-fragment";

/// Immutable identity and policy copied through every artifact-level IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactContext {
    artifact: ArtifactId,
    target: ArtifactTarget,
    frame: ArtifactFrame,
    mode: StaticCompileMode,
}

impl ArtifactContext {
    pub(crate) fn new(
        artifact: ArtifactId,
        target: ArtifactTarget,
        frame: ArtifactFrame,
        mode: StaticCompileMode,
    ) -> Result<Self, ArtifactPlanError> {
        if let ArtifactFrame::StaticLane {
            generated_path,
            source_root,
        } = &frame
        {
            validate_text("generated artifact path", generated_path)?;
            validate_text("spec source root", source_root)?;
        }
        let valid = if target == ArtifactTarget::StaticMarkdown {
            match (&frame, mode, artifact.as_str()) {
                (ArtifactFrame::CompatibilityFragment, _, COMPATIBILITY_ARTIFACT_ID) => true,
                (
                    ArtifactFrame::StaticLane { generated_path, .. },
                    StaticCompileMode::QualifyPerNode,
                    "static-md",
                ) => generated_path.ends_with(".md"),
                _ => false,
            }
        } else if target == ArtifactTarget::StaticXml {
            matches!(
                (&frame, mode, artifact.as_str()),
                (
                    ArtifactFrame::StaticLane { generated_path, .. },
                    StaticCompileMode::QualifyPerNode,
                    "static-xml",
                ) if generated_path.ends_with(".xml")
            )
        } else {
            // The one custom-target row: the artifact IS the backend id, under
            // the compatibility fragment. Identity never implies registration.
            target.is_custom()
                && frame == ArtifactFrame::CompatibilityFragment
                && artifact.as_str() == target.backend_id()
        };
        if !valid {
            return Err(ArtifactPlanError::InvalidContextTuple {
                artifact: artifact.as_str().to_string(),
                target,
                frame: format!("{frame:?}"),
                mode: format!("{mode:?}"),
            });
        }
        Ok(Self {
            artifact,
            target,
            frame,
            mode,
        })
    }

    pub(crate) fn compatibility(mode: StaticCompileMode) -> Self {
        Self {
            artifact: ArtifactId(COMPATIBILITY_ARTIFACT_ID.to_string()),
            target: ArtifactTarget::StaticMarkdown,
            frame: ArtifactFrame::CompatibilityFragment,
            mode,
        }
    }

    pub fn artifact(&self) -> &ArtifactId {
        &self.artifact
    }

    pub fn target(&self) -> ArtifactTarget {
        self.target.clone()
    }

    pub(crate) fn frame(&self) -> &ArtifactFrame {
        &self.frame
    }

    pub(crate) fn mode(&self) -> StaticCompileMode {
        self.mode
    }

    #[cfg(test)]
    pub(crate) fn testing(
        artifact: ArtifactId,
        target: ArtifactTarget,
        frame: ArtifactFrame,
        mode: StaticCompileMode,
    ) -> Self {
        Self {
            artifact,
            target,
            frame,
            mode,
        }
    }
}

mod plan;

pub use plan::ArtifactPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactInputType {
    Normal,
    Simple,
    Elided,
    Hoisted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInputWitness {
    pub index: usize,
    pub origin: String,
    pub path: String,
    pub kind: ArtifactInputType,
}

/// One validated heterogeneous artifact input in effective-boot order.
///
/// The [`DocumentSubject`] sits beside the kind rather than inside it, so all
/// four kinds answer the subject question the same way and none can be added
/// later without one. `Normal` and `Simple` hand it on to the document they
/// produce; `Elided` and `Hoisted` produce no document at all, so their
/// subject is carried and never reaches a [`SourceIr`] — the honest answer,
/// since no source/document transform is ever invoked for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInput {
    kind: ArtifactInputKind,
    subject: DocumentSubject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArtifactInputKind {
    Normal {
        meta: ContributionMeta,
        seed: SpecAddress,
    },
    Simple {
        meta: ContributionMeta,
        source: SourceIr,
    },
    Elided {
        meta: ContributionMeta,
    },
    Hoisted {
        meta: ContributionMeta,
        target: SpecAddress,
    },
}

impl ArtifactInput {
    /// Add one normal root. Its root package must match provenance; nodes
    /// reached from that root may intentionally remain cross-origin.
    ///
    /// The subject's provider is [`DocumentProvider::Undetermined`]: this
    /// form takes provenance as a display string only, so it cannot say
    /// which typed provider declared the row. A caller that HAS the typed
    /// components uses [`ArtifactInput::normal_declared_by`] instead.
    pub fn normal(
        origin: impl Into<String>,
        path: impl Into<String>,
        seed: SpecAddress,
    ) -> Result<Self, ArtifactPlanError> {
        Self::normal_declared_by(origin, path, seed, DocumentProvider::Undetermined)
    }

    /// One normal root whose declaring provider is TYPED.
    ///
    /// Same arguments and same laws as [`ArtifactInput::normal`], plus the
    /// provider the declaring row names. The provider is supplied rather
    /// than derived: `origin` stays display/provenance (PROP-054 keeps it
    /// so, and it may carry a `[shared by …]` suffix), and parsing a typed
    /// identity back out of it is exactly what the carried subject exists to
    /// avoid.
    pub fn normal_declared_by(
        origin: impl Into<String>,
        path: impl Into<String>,
        seed: SpecAddress,
        provider: DocumentProvider,
    ) -> Result<Self, ArtifactPlanError> {
        let meta = ContributionMeta::new(origin, path)?;
        validate_package_relation("normal", &meta.origin, &seed, false)?;
        Ok(Self::from_kind_declared_by(
            ArtifactInputKind::Normal { meta, seed },
            provider,
        ))
    }

    /// One simple contribution, carried verbatim.
    ///
    /// Its subject's provider is [`DocumentProvider::Undetermined`], for the
    /// same reason [`ArtifactInput::normal`]'s is;
    /// [`ArtifactInput::simple_declared_by`] is the typed form.
    pub fn simple(
        origin: impl Into<String>,
        path: impl Into<String>,
        canonical_markdown: impl Into<String>,
    ) -> Result<Self, ArtifactPlanError> {
        Self::simple_declared_by(
            origin,
            path,
            canonical_markdown,
            DocumentProvider::Undetermined,
        )
    }

    /// One simple contribution whose declaring provider is TYPED.
    ///
    /// The subject is minted once and handed to the [`SourceIr`] this input
    /// carries, so the document that already exists and the input that owns
    /// it cannot disagree — `validate` re-checks exactly that.
    pub fn simple_declared_by(
        origin: impl Into<String>,
        path: impl Into<String>,
        canonical_markdown: impl Into<String>,
        provider: DocumentProvider,
    ) -> Result<Self, ArtifactPlanError> {
        let meta = ContributionMeta::new(origin, path)?;
        let source = SourceIr::new(
            DocumentAddress::StaticEntry {
                origin: meta.origin.clone(),
                path: meta.path.clone(),
            },
            super::SourceFormatId::canonical_markdown(),
            declared_subject(&meta, provider.clone()),
            canonical_markdown,
        );
        Ok(Self::from_kind_declared_by(
            ArtifactInputKind::Simple { meta, source },
            provider,
        ))
    }

    pub fn elided(
        origin: impl Into<String>,
        path: impl Into<String>,
    ) -> Result<Self, ArtifactPlanError> {
        let meta = ContributionMeta::new(origin, path)?;
        Ok(Self::from_kind(ArtifactInputKind::Elided { meta }))
    }

    pub fn hoisted(
        origin: impl Into<String>,
        path: impl Into<String>,
        target: SpecAddress,
    ) -> Result<Self, ArtifactPlanError> {
        let meta = ContributionMeta::new(origin, path)?;
        validate_package_relation("hoisted", &meta.origin, &target, true)?;
        Ok(Self::from_kind(ArtifactInputKind::Hoisted { meta, target }))
    }

    /// Rebuild one input from its kind, minting the subject the kind's own
    /// provenance declares — so a crate-internal caller cannot author an input
    /// whose subject disagrees with its contribution row.
    ///
    /// The provider is [`DocumentProvider::Undetermined`]: a caller that
    /// reaches this entry has a kind and nothing else. The wire conversion
    /// and the compatibility wrapper are exactly that caller.
    pub(crate) fn from_kind(kind: ArtifactInputKind) -> Self {
        Self::from_kind_declared_by(kind, DocumentProvider::Undetermined)
    }

    /// [`ArtifactInput::from_kind`] with the declaring provider named.
    ///
    /// The one place a subject is minted for an input: the path always comes
    /// from the kind's own contribution row, never from the caller, so a
    /// typed provider can be supplied without any caller ever authoring a
    /// whole subject.
    fn from_kind_declared_by(kind: ArtifactInputKind, provider: DocumentProvider) -> Self {
        let subject = declared_subject(kind_meta(&kind), provider);
        Self { kind, subject }
    }

    pub(crate) fn kind(&self) -> &ArtifactInputKind {
        &self.kind
    }

    pub(crate) fn meta(&self) -> &ContributionMeta {
        kind_meta(self.kind())
    }

    /// The immutable subject this contribution hands to the document it
    /// produces.
    pub(crate) fn subject(&self) -> &DocumentSubject {
        &self.subject
    }

    fn validate(&self) -> Result<(), ArtifactPlanError> {
        self.meta().validate()?;
        match self.kind() {
            ArtifactInputKind::Normal { meta, seed } => {
                validate_package_relation("normal", &meta.origin, seed, false)?;
            }
            ArtifactInputKind::Simple { meta, source } => {
                let expected = DocumentAddress::StaticEntry {
                    origin: meta.origin.clone(),
                    path: meta.path.clone(),
                };
                if source.address() != &expected {
                    return Err(ArtifactPlanError::SimpleIdentity {
                        expected: format!("{expected:?}"),
                        actual: format!("{:?}", source.address()),
                    });
                }
                if source.format().as_str() != "markdown" {
                    return Err(ArtifactPlanError::SimpleFormat {
                        actual: source.format().as_str().to_string(),
                    });
                }
                // The one kind whose document already exists, so the one place
                // the carried subject can be observed disagreeing with the row
                // that declared it: a crate-internal caller may assemble the
                // meta/source pair by hand, and a subject that does not match
                // would silently rescope the document's transforms.
                if source.subject() != &self.subject {
                    return Err(ArtifactPlanError::SimpleSubject {
                        expected: self.subject.to_string(),
                        actual: source.subject().to_string(),
                    });
                }
            }
            ArtifactInputKind::Hoisted { meta, target } => {
                validate_package_relation("hoisted", &meta.origin, target, true)?;
            }
            ArtifactInputKind::Elided { .. } => {}
        }
        Ok(())
    }
}

fn kind_meta(kind: &ArtifactInputKind) -> &ContributionMeta {
    match kind {
        ArtifactInputKind::Normal { meta, .. }
        | ArtifactInputKind::Simple { meta, .. }
        | ArtifactInputKind::Elided { meta }
        | ArtifactInputKind::Hoisted { meta, .. } => meta,
    }
}

/// The subject one contribution row declares.
///
/// The path is the row's own already-validated non-blank, forward-slashed path
/// — not the address', which may legitimately differ.
///
/// The provider is the caller's, and it is never `Unclaimed`: a row DID
/// declare this document, so an owner exists, and saying `Unclaimed` would
/// assert that nothing declared it — false of every input in this file. Until
/// T10B the only sayable answer was [`DocumentProvider::Undetermined`],
/// because `origin` is a display string PROP-054 keeps display/provenance and
/// no typed provider reached this seat. The boot adapter now carries the typed
/// pair beside `origin`, so a workspace-built input names its real provider
/// and `Undetermined` survives only for the compatibility forms and the wire
/// rebuild.
fn declared_subject(meta: &ContributionMeta, provider: DocumentProvider) -> DocumentSubject {
    DocumentSubject::declared(provider, meta.path.clone())
}
