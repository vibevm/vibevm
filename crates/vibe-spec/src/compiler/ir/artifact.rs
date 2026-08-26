//! Immutable identity and ordered input plan for one final compiler artifact.

use std::collections::BTreeMap;

use crate::{Directives, SpecAddress};

use super::{DocumentAddress, SourceIr};

/// Open identity of one final compiler artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactId(String);

impl ArtifactId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, ArtifactPlanError> {
        let value = value.into();
        validate_text("artifact id", &value)?;
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
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

/// The currently shipping final STATIC target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactTarget {
    StaticMarkdown,
    StaticXml,
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
pub(crate) struct ArtifactContext {
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
        let valid = match (&target, &frame, mode, artifact.as_str()) {
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
            _ => false,
        };
        if !valid {
            return Err(ArtifactPlanError::InvalidContextTuple {
                artifact: artifact.as_str().to_string(),
                target,
                frame,
                mode,
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

    pub(crate) fn artifact(&self) -> &ArtifactId {
        &self.artifact
    }

    pub(crate) fn target(&self) -> ArtifactTarget {
        self.target
    }

    pub(crate) fn frame(&self) -> &ArtifactFrame {
        &self.frame
    }

    pub(crate) fn mode(&self) -> StaticCompileMode {
        self.mode
    }
}

/// Immutable invocation input for compiling one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactPlan {
    context: ArtifactContext,
    contributions: Vec<ArtifactInput>,
}

impl ArtifactPlan {
    pub(crate) fn new(
        context: ArtifactContext,
        contributions: Vec<ArtifactInput>,
    ) -> Result<Self, ArtifactPlanError> {
        let mut simple_identities: BTreeMap<String, (usize, &SourceIr)> = BTreeMap::new();
        for (index, contribution) in contributions.iter().enumerate() {
            contribution
                .validate()
                .map_err(|source| ArtifactPlanError::Input {
                    index,
                    source: Box::new(source),
                })?;
            if let ArtifactInput::Simple { meta, source } = contribution {
                if simple_alias_machinery(source.text()) {
                    return Err(ArtifactPlanError::SimpleAlias {
                        index,
                        origin: meta.origin.clone(),
                    });
                }
                let key = format!("{}\0{}", meta.origin, meta.path);
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
            contributions: vec![ArtifactInput::Normal { meta, seed }],
        }
    }

    pub(crate) fn context(&self) -> &ArtifactContext {
        &self.context
    }

    pub(crate) fn contributions(&self) -> &[ArtifactInput] {
        &self.contributions
    }
}

/// One heterogeneous artifact input in effective-boot order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArtifactInput {
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
    pub(crate) fn meta(&self) -> &ContributionMeta {
        match self {
            Self::Normal { meta, .. }
            | Self::Simple { meta, .. }
            | Self::Elided { meta }
            | Self::Hoisted { meta, .. } => meta,
        }
    }

    fn validate(&self) -> Result<(), ArtifactPlanError> {
        self.meta().validate()?;
        if let Self::Simple { meta, source } = self {
            let expected = DocumentAddress::StaticEntry {
                origin: meta.origin.clone(),
                path: meta.path.clone(),
            };
            if source.address() != &expected {
                return Err(ArtifactPlanError::SimpleIdentity {
                    expected: Box::new(expected),
                    actual: Box::new(source.address().clone()),
                });
            }
            if source.format().as_str() != "markdown" {
                return Err(ArtifactPlanError::SimpleFormat {
                    actual: source.format().as_str().to_string(),
                });
            }
        }
        Ok(())
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
pub(crate) enum ArtifactPlanError {
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
    SimpleIdentity {
        expected: Box<DocumentAddress>,
        actual: Box<DocumentAddress>,
    },
    #[error(
        "invalid artifact context tuple: id `{artifact}`, target {target:?}, frame {frame:?}, mode {mode:?}"
    )]
    InvalidContextTuple {
        artifact: String,
        target: ArtifactTarget,
        frame: ArtifactFrame,
        mode: StaticCompileMode,
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
}
