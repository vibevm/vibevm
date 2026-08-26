//! Immutable identity and ordered input plan for one final compiler artifact.

use std::collections::BTreeMap;

use crate::{Directives, SpecAddress};

use super::{ArtifactTarget, DocumentAddress, SourceIr};

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
        validate_text("contribution path", &self.path)
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
        let valid = match (target, &frame, mode, artifact.as_str()) {
            (
                ArtifactTarget::StaticMarkdown,
                ArtifactFrame::CompatibilityFragment,
                _,
                "static-fragment",
            ) => true,
            (
                ArtifactTarget::StaticMarkdown,
                ArtifactFrame::StaticLane { generated_path, .. },
                StaticCompileMode::QualifyPerNode,
                "static-md",
            ) => generated_path.ends_with(".md"),
            (
                ArtifactTarget::StaticXml,
                ArtifactFrame::StaticLane { generated_path, .. },
                StaticCompileMode::QualifyPerNode,
                "static-xml",
            ) => generated_path.ends_with(".xml"),
            (target, ArtifactFrame::CompatibilityFragment, _, artifact)
                if target.is_custom() && artifact == target.backend_id() =>
            {
                true
            }
            _ => false,
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
            artifact: ArtifactId("static-fragment".to_string()),
            target: ArtifactTarget::StaticMarkdown,
            frame: ArtifactFrame::CompatibilityFragment,
            mode,
        }
    }

    pub fn artifact(&self) -> &ArtifactId {
        &self.artifact
    }

    pub fn target(&self) -> ArtifactTarget {
        self.target
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

/// Immutable invocation input for compiling one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPlan {
    context: ArtifactContext,
    contributions: Vec<ArtifactInput>,
}

impl ArtifactPlan {
    pub(crate) fn new(
        context: ArtifactContext,
        contributions: Vec<ArtifactInput>,
    ) -> Result<Self, ArtifactPlanError> {
        // The identity is the typed (origin, path) pair, never a joined string:
        // a delimiter spelling cannot separate ("a", "b\0c") from ("a\0b", "c"),
        // so a plan holding both would drop one conflict check on the floor.
        let mut simple_identities: BTreeMap<(&str, &str), (usize, &SourceIr)> = BTreeMap::new();
        for (index, contribution) in contributions.iter().enumerate() {
            contribution
                .validate()
                .map_err(|source| ArtifactPlanError::Input {
                    index,
                    source: Box::new(source),
                })?;
            if let ArtifactInputKind::Simple { meta, source } = contribution.kind() {
                if simple_alias_machinery(source.text()) {
                    return Err(ArtifactPlanError::SimpleAlias {
                        index,
                        origin: meta.origin.clone(),
                    });
                }
                let key = (meta.origin.as_str(), meta.path.as_str());
                if let Some((first, prior)) = simple_identities.get(&key) {
                    if *prior != source {
                        return Err(ArtifactPlanError::ConflictingSimpleIdentity {
                            first: *first,
                            second: index,
                            origin: meta.origin.clone(),
                            path: meta.path.clone(),
                        });
                    }
                } else {
                    simple_identities.insert(key, (index, source));
                }
            }
        }
        Ok(Self {
            context,
            contributions,
        })
    }

    pub(crate) fn compatibility(seed: SpecAddress, mode: StaticCompileMode) -> Self {
        let meta = ContributionMeta {
            origin: crate::compiler::close::document_origin(&seed),
            path: seed.doc_path.clone(),
        };
        Self {
            context: ArtifactContext::compatibility(mode),
            contributions: vec![ArtifactInput::from_kind(ArtifactInputKind::Normal {
                meta,
                seed,
            })],
        }
    }

    /// Build one validated final STATIC artifact in exact input order.
    pub fn static_lane(
        target: ArtifactTarget,
        generated_path: impl Into<String>,
        source_root: impl Into<String>,
        contributions: Vec<ArtifactInput>,
    ) -> Result<Self, ArtifactPlanError> {
        let artifact = target.backend_id();
        if target.is_custom() {
            return Err(ArtifactPlanError::InvalidContextTuple {
                artifact: artifact.to_string(),
                target,
                frame: "StaticLane".to_string(),
                mode: format!("{:?}", StaticCompileMode::QualifyPerNode),
            });
        }
        let context = ArtifactContext::new(
            ArtifactId::new(artifact)?,
            target,
            ArtifactFrame::StaticLane {
                generated_path: generated_path.into(),
                source_root: source_root.into(),
            },
            StaticCompileMode::QualifyPerNode,
        )?;
        Self::new(context, contributions)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn custom_for_test(
        backend: &'static str,
        contributions: Vec<ArtifactInput>,
    ) -> Result<Self, ArtifactPlanError> {
        let target = ArtifactTarget::custom(backend);
        let context = ArtifactContext::new(
            ArtifactId::new(backend)?,
            target,
            ArtifactFrame::CompatibilityFragment,
            StaticCompileMode::Plain,
        )?;
        Self::new(context, contributions)
    }

    pub(crate) fn context(&self) -> &ArtifactContext {
        &self.context
    }

    pub(crate) fn contributions(&self) -> &[ArtifactInput] {
        &self.contributions
    }

    pub(crate) fn input_witness(&self, index: usize) -> Option<ArtifactInputWitness> {
        let input = self.contributions.get(index)?;
        Some(ArtifactInputWitness {
            index,
            origin: input.meta().origin.clone(),
            path: input.meta().path.clone(),
            kind: match input.kind() {
                ArtifactInputKind::Normal { .. } => ArtifactInputType::Normal,
                ArtifactInputKind::Simple { .. } => ArtifactInputType::Simple,
                ArtifactInputKind::Elided { .. } => ArtifactInputType::Elided,
                ArtifactInputKind::Hoisted { .. } => ArtifactInputType::Hoisted,
            },
        })
    }
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInput {
    kind: ArtifactInputKind,
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

    pub(crate) fn from_kind(kind: ArtifactInputKind) -> Self {
        Self { kind }
    }

    pub(crate) fn kind(&self) -> &ArtifactInputKind {
        &self.kind
    }

    pub(crate) fn meta(&self) -> &ContributionMeta {
        match self.kind() {
            ArtifactInputKind::Normal { meta, .. }
            | ArtifactInputKind::Simple { meta, .. }
            | ArtifactInputKind::Elided { meta }
            | ArtifactInputKind::Hoisted { meta, .. } => meta,
        }
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
            }
            ArtifactInputKind::Hoisted { meta, target } => {
                validate_package_relation("hoisted", &meta.origin, target, true)?;
            }
            ArtifactInputKind::Elided { .. } => {}
        }
        Ok(())
    }
}

fn validate_package_relation(
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
    #[error("artifact input {index} is invalid: {source}")]
    Input {
        index: usize,
        #[source]
        source: Box<ArtifactPlanError>,
    },
    #[error("simple input identity must be {expected:?}, got {actual:?}")]
    SimpleIdentity { expected: String, actual: String },
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
