//! Immutable identity and ordered input plan for one final compiler artifact.

use crate::{Directives, SpecAddress};

use super::{ArtifactTarget, DocumentAddress, DocumentProvider, DocumentSubject, SourceIr};

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
    pub fn normal(
        origin: impl Into<String>,
        path: impl Into<String>,
        seed: SpecAddress,
    ) -> Result<Self, ArtifactPlanError> {
        let meta = ContributionMeta::new(origin, path)?;
        validate_package_relation("normal", &meta.origin, &seed, false)?;
        Ok(Self::from_kind(ArtifactInputKind::Normal { meta, seed }))
    }

    pub fn simple(
        origin: impl Into<String>,
        path: impl Into<String>,
        canonical_markdown: impl Into<String>,
    ) -> Result<Self, ArtifactPlanError> {
        let meta = ContributionMeta::new(origin, path)?;
        let source = SourceIr::new(
            DocumentAddress::StaticEntry {
                origin: meta.origin.clone(),
                path: meta.path.clone(),
            },
            super::SourceFormatId::canonical_markdown(),
            declared_subject(&meta),
            canonical_markdown,
        );
        Ok(Self::from_kind(ArtifactInputKind::Simple { meta, source }))
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
    pub(crate) fn from_kind(kind: ArtifactInputKind) -> Self {
        let subject = declared_subject(kind_meta(&kind));
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
/// — not the address', which may legitimately differ. The provider is
/// [`DocumentProvider::Undetermined`], never `Unclaimed`: a row DID declare
/// this document, so an owner exists; `origin` is a display string that
/// PROP-054 keeps display/provenance, so no TYPED provider exists here to
/// carry, and the owner-view adapter is what will supply one. Saying
/// `Unclaimed` here would assert that nothing declared the document, which is
/// false of every input in this file.
fn declared_subject(meta: &ContributionMeta) -> DocumentSubject {
    DocumentSubject::declared(DocumentProvider::Undetermined, meta.path.clone())
}

/// Crate-private so the wire conversion can re-run the same origin/target law
/// on every normal/hoisted contribution a carrier carries, at every level.
pub(crate) fn validate_package_relation(
    kind: &'static str,
    origin: &str,
    target: &SpecAddress,
    whole_unversioned: bool,
) -> Result<(), ArtifactPlanError> {
    let coordinate = origin.split_whitespace().next().unwrap_or_default();
    let suffix = origin.strip_prefix(coordinate).unwrap_or_default();
    let suffix_valid =
        suffix.is_empty() || (suffix.starts_with(" [shared by ") && suffix.ends_with(']'));
    if !suffix_valid {
        return Err(identity_error(
            kind,
            origin,
            target,
            "origin carries an unsupported suffix",
        ));
    }
    let Some((origin_group, origin_name)) = coordinate.split_once('/') else {
        return Err(identity_error(
            kind,
            origin,
            target,
            "origin is not a package coordinate",
        ));
    };
    let crate::Authority::Package {
        group,
        name,
        version,
    } = &target.authority
    else {
        return Err(identity_error(
            kind,
            origin,
            target,
            "target uses host authority",
        ));
    };
    if group != origin_group || name != origin_name {
        return Err(identity_error(
            kind,
            origin,
            target,
            "origin and target package coordinates differ",
        ));
    }
    if whole_unversioned
        && (version.is_some() || !target.anchor.is_empty() || target.pinned_r.is_some())
    {
        return Err(identity_error(
            kind,
            origin,
            target,
            "hoisted target must be an unversioned whole document",
        ));
    }
    Ok(())
}

fn identity_error(
    kind: &'static str,
    origin: &str,
    target: &SpecAddress,
    reason: &'static str,
) -> ArtifactPlanError {
    ArtifactPlanError::InputIdentity {
        kind,
        origin: origin.to_string(),
        target: target.to_string(),
        reason,
    }
}

fn validate_text(field: &'static str, value: &str) -> Result<(), ArtifactPlanError> {
    if value.trim().is_empty() {
        return Err(ArtifactPlanError::Blank { field });
    }
    if value.contains(['\n', '\r', '\0']) {
        return Err(ArtifactPlanError::UnsafeText { field });
    }
    Ok(())
}

/// The separator law on a path that becomes a [`DocumentSubject`]'s
/// `declared_path`.
///
/// Applied to the contribution path specifically, never through
/// [`validate_text`]: `validate_text` also judges origins, artifact ids and
/// the static-lane roots, and widening it would hold values that are not
/// selector paths to a selector path's contract. The rule itself lives once,
/// on [`DocumentSubject`]; this is the artifact plan's vocabulary for it.
fn validate_declared_path(field: &'static str, value: &str) -> Result<(), ArtifactPlanError> {
    if DocumentSubject::path_is_forward_slashed(value) {
        return Ok(());
    }
    Err(ArtifactPlanError::BackslashedPath {
        field,
        value: value.to_string(),
    })
}

fn simple_alias_machinery(text: &str) -> bool {
    let directives = Directives::parse(text);
    !directives.aliases.is_empty()
        || directives
            .errors
            .iter()
            .any(|error| error.message.contains("undeclared alias"))
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactPlanError {
    #[error("compiler {field} must not be blank")]
    Blank { field: &'static str },
    #[error("compiler {field} must not contain a newline or NUL")]
    UnsafeText { field: &'static str },
    #[error(
        "compiler {field} `{value}` must be forward-slashed: a `paths` selector dimension compiles its globs with a literal separator, so a backslashed path matches nothing at all"
    )]
    BackslashedPath { field: &'static str, value: String },
    #[error("artifact input {index} is invalid: {source}")]
    Input {
        index: usize,
        #[source]
        source: Box<ArtifactPlanError>,
    },
    #[error("simple input identity must be {expected:?}, got {actual:?}")]
    SimpleIdentity { expected: String, actual: String },
    #[error("simple input subject must be {expected}, got {actual}")]
    SimpleSubject { expected: String, actual: String },
    #[error(
        "invalid artifact context tuple: id `{artifact}`, target {target:?}, frame {frame}, mode {mode}"
    )]
    InvalidContextTuple {
        artifact: String,
        target: ArtifactTarget,
        frame: String,
        mode: String,
    },
    #[error("simple artifact inputs must use canonical Markdown, got `{actual}`")]
    SimpleFormat { actual: String },
    #[error(
        "the simple package `{origin}` carries alias machinery (`#use … as` / `@!`) that is `normal`-format only (PROP-035 §7.2); convert the package to `format = \"normal\"` or drop the alias"
    )]
    SimpleAlias { index: usize, origin: String },
    #[error(
        "simple artifact inputs {first} and {second} claim `{origin}:{path}` with different source text"
    )]
    ConflictingSimpleIdentity {
        first: usize,
        second: usize,
        origin: String,
        path: String,
    },
    #[error("{kind} input `{origin}` contradicts target `{target}`: {reason}")]
    InputIdentity {
        kind: &'static str,
        origin: String,
        target: String,
        reason: &'static str,
    },
}
