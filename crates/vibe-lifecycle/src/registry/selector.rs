//! Prevalidated positive selector matching over caller-supplied subjects.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR");

use glob::{MatchOptions, Pattern};
use specmark::spec;
use vibe_core::manifest::ExtensionAppliesTo;

use super::model::DependencyProviderId;

/// Package/path subject presented to a selector-aware registry query.
///
/// Package identity is typed and renders only as versionless `group/name`.
/// Paths are an adapter contract: forward-slashed and case-sensitive.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectorSubject<'subject> {
    package: Option<&'subject DependencyProviderId>,
    path: Option<&'subject str>,
}

impl<'subject> SelectorSubject<'subject> {
    /// Construct a subject with either, both, or neither selector dimension.
    #[must_use]
    pub const fn new(
        package: Option<&'subject DependencyProviderId>,
        path: Option<&'subject str>,
    ) -> Self {
        Self { package, path }
    }

    /// A point invocation with no package or path subject.
    #[must_use]
    pub const fn unscoped() -> Self {
        Self::new(None, None)
    }

    /// A package-only subject.
    #[must_use]
    pub const fn package(package: &'subject DependencyProviderId) -> Self {
        Self::new(Some(package), None)
    }

    /// A forward-slashed path-only subject.
    #[must_use]
    pub const fn path(path: &'subject str) -> Self {
        Self::new(None, Some(path))
    }
}

#[derive(Debug, Clone)]
pub(super) struct CompiledSelector {
    packages: Option<Vec<Pattern>>,
    paths: Option<Vec<Pattern>>,
}

impl CompiledSelector {
    pub(super) fn compile(
        selector: Option<&ExtensionAppliesTo>,
    ) -> Result<Self, SelectorCompileError> {
        let Some(selector) = selector else {
            return Ok(Self {
                packages: None,
                paths: None,
            });
        };

        Ok(Self {
            packages: compile_dimension("applies_to.packages", selector.packages.as_deref())?,
            paths: compile_dimension("applies_to.paths", selector.paths.as_deref())?,
        })
    }

    pub(super) fn matches(&self, subject: SelectorSubject<'_>) -> bool {
        let package = subject.package.map(ToString::to_string);
        dimension_matches(self.packages.as_deref(), package.as_deref())
            && dimension_matches(self.paths.as_deref(), subject.path)
    }
}

#[derive(Debug, Clone)]
pub(super) struct SelectorCompileError {
    pub(super) field: &'static str,
    pub(super) pattern: String,
    pub(super) reason: String,
}

fn compile_dimension(
    field: &'static str,
    patterns: Option<&[String]>,
) -> Result<Option<Vec<Pattern>>, SelectorCompileError> {
    patterns
        .map(|patterns| {
            patterns
                .iter()
                .map(|pattern| {
                    Pattern::new(pattern).map_err(|error| SelectorCompileError {
                        field,
                        pattern: pattern.clone(),
                        reason: error.to_string(),
                    })
                })
                .collect()
        })
        .transpose()
}

fn dimension_matches(patterns: Option<&[Pattern]>, value: Option<&str>) -> bool {
    let Some(patterns) = patterns else {
        return true;
    };
    let Some(value) = value else {
        return false;
    };
    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    patterns
        .iter()
        .any(|pattern| pattern.matches_with(value, options))
}
