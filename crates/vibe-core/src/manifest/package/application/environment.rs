//! The application recipe/shell environment projection preserves local identity.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-ENV-IDENTITY");

use super::{InstallerOptionDecl, StepKind, steps_of};
use crate::manifest::{FieldError, FieldPath};
use std::collections::BTreeMap;

/// Existing application spelling, without escaping or normalizing the source ID.
/// Validation is separate so compatibility callers retain this exact mapping.
///
/// ```
/// assert_eq!(vibe_core::manifest::step_env_name("VIBE_PARAM_", "install-dir"), "VIBE_PARAM_INSTALL_DIR");
/// ```
pub fn step_env_name(prefix: &str, id: &str) -> String {
    let tail: String = id
        .chars()
        .map(|ch| match ch {
            '-' | '.' => '_',
            other => other.to_ascii_uppercase(),
        })
        .collect();
    format!("{prefix}{tail}")
}

/// Independent recipe/shell export namespaces, never a package identity policy.
///
/// ```
/// use vibe_core::manifest::StepEnvironment;
/// assert!(StepEnvironment::Feature.validate(["a-b", "a-c"]).is_ok());
/// assert!(StepEnvironment::Parameter.validate(["a-b", "a.b"]).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepEnvironment {
    Feature,
    Parameter,
}

impl StepEnvironment {
    fn prefix(self) -> &'static str {
        match self {
            Self::Feature => "VIBE_FEATURE_",
            Self::Parameter => "VIBE_PARAM_",
        }
    }
    fn field(self) -> &'static str {
        match self {
            Self::Feature => "features",
            Self::Parameter => "parameters",
        }
    }

    /// Check the full declaration in its original order; only IDs are inspected.
    ///
    /// ```
    /// use vibe_core::manifest::{StepEnvironment, StepEnvironmentError};
    /// let error = StepEnvironment::Feature.validate(["a-b", "a.b"]).unwrap_err();
    /// assert!(matches!(error, StepEnvironmentError::Collision { index: 1, .. }));
    /// ```
    pub fn validate<'a>(
        self,
        ids: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), StepEnvironmentError> {
        let mut emitted = BTreeMap::new();
        for (index, id) in ids.into_iter().enumerate() {
            let key = step_env_name(self.prefix(), id);
            let mut chars = key.chars();
            let portable = chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !portable {
                return Err(StepEnvironmentError::Unrepresentable {
                    index,
                    id: id.into(),
                    key,
                });
            }
            if let Some(first_id) = emitted.insert(key.clone(), id) {
                return Err(StepEnvironmentError::Collision {
                    index,
                    id: id.into(),
                    first_id: first_id.into(),
                    key,
                });
            }
        }
        Ok(())
    }
}

/// A declaration row cannot be represented by the existing environment contract.
/// No answer values are carried or rendered here.
///
/// ```
/// use vibe_core::manifest::StepEnvironment;
/// let error = StepEnvironment::Feature.validate(["two words"]).unwrap_err();
/// assert!(error.to_string().contains("VIBE_FEATURE_TWO WORDS"));
/// assert_eq!(error.index(), 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-ENV-IDENTITY")]
pub enum StepEnvironmentError {
    #[error(
        "identifier `{id}` projects to nonportable environment key `{key}` (violates REQ spec://org.vibevm.core/vibevm/common/PROP-069#APP-ENV-IDENTITY; fix surface: rename this installer feature or parameter to a portable export spelling)"
    )]
    Unrepresentable {
        index: usize,
        id: String,
        key: String,
    },
    #[error(
        "identifiers `{first_id}` and `{id}` both project to environment key `{key}` (violates REQ spec://org.vibevm.core/vibevm/common/PROP-069#APP-ENV-IDENTITY; fix surface: rename one installer feature or parameter and its references)"
    )]
    Collision {
        index: usize,
        id: String,
        first_id: String,
        key: String,
    },
}

impl StepEnvironmentError {
    /// Zero-based offending row in the caller's original declaration sequence.
    ///
    /// ```
    /// use vibe_core::manifest::StepEnvironment;
    /// assert_eq!(StepEnvironment::Parameter.validate(["a-b", "a.b"]).unwrap_err().index(), 1);
    /// ```
    pub fn index(&self) -> usize {
        match self {
            Self::Unrepresentable { index, .. } | Self::Collision { index, .. } => *index,
        }
    }
}

impl StepKind {
    /// Only these child kinds receive the typed feature/parameter environment.
    ///
    /// ```
    /// use vibe_core::manifest::StepKind;
    /// let recipe = StepKind::Recipe { interpreter: None, script: "build.sh".into() };
    /// assert!(recipe.receives_step_environment());
    /// ```
    pub fn receives_step_environment(&self) -> bool {
        matches!(self, Self::Recipe { .. } | Self::Shell { .. })
    }
}

impl InstallerOptionDecl {
    /// Check only options with an actual expanded recipe/shell declaration,
    /// including hooks and method sugar. Different options remain independent.
    ///
    /// ```
    /// use vibe_core::manifest::{FieldPath, InstallerOptionDecl};
    /// let option: InstallerOptionDecl = toml::from_str("id='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'")?;
    /// option.validate_step_environment(&FieldPath::root())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn validate_step_environment(&self, at: &FieldPath) -> Result<(), FieldError> {
        if !steps_of(self)
            .iter()
            .any(|s| s.kind.receives_step_environment())
        {
            return Ok(());
        }
        let (features, parameters, scope): (Vec<&str>, Vec<&str>, FieldPath) =
            match &self.configuration {
                Some(c) => (
                    c.features.iter().map(|f| f.id.as_str()).collect(),
                    c.parameters.iter().map(|p| p.id.as_str()).collect(),
                    at.table("configuration"),
                ),
                None => (
                    self.features.iter().map(|f| f.id.as_str()).collect(),
                    self.parameters.iter().map(|p| p.id.as_str()).collect(),
                    at.clone(),
                ),
            };
        for (kind, ids) in [
            (StepEnvironment::Feature, features),
            (StepEnvironment::Parameter, parameters),
        ] {
            kind.validate(ids).map_err(|error| {
                let field = scope.table(kind.field()).index(error.index()).field("id");
                FieldError::new(
                    field.clone(),
                    format!("installer option `{}` field `{field}`: {error}", self.id),
                )
            })?;
        }
        Ok(())
    }
}
