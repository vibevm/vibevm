//! Prevalidated positive selector matching over caller-supplied subjects.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR");

use glob::{MatchOptions, Pattern};
use specmark::spec;
use vibe_core::manifest::ExtensionAppliesTo;

use super::model::{DependencyProviderId, HostIdentity};

/// The typed provider coordinate a selector subject carries.
///
/// Dependency and host spellings reach matching only through the provider's
/// existing one codec — the typed identities' render — and nothing parses a
/// rendered spelling back into identity.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorProvider<'subject> {
    /// An installed dependency provider, rendered versionless `group/name`.
    Dependency(&'subject DependencyProviderId),
    /// The selected host, rendered through the host-owner codec.
    Host(&'subject HostIdentity),
}

impl SelectorProvider<'_> {
    /// Render the provider through its one existing match-time codec.
    fn rendered(self) -> String {
        match self {
            Self::Dependency(id) => id.to_string(),
            Self::Host(identity) => identity.to_string(),
        }
    }
}

/// Provider/path subject presented to a selector-aware registry query.
///
/// Provider identity is typed and renders only at match time: versionless
/// `group/name` for a dependency, the host-owner spelling for a host. Paths
/// are an adapter contract: forward-slashed and case-sensitive.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectorSubject<'subject> {
    provider: Option<SelectorProvider<'subject>>,
    path: Option<&'subject str>,
}

impl<'subject> SelectorSubject<'subject> {
    /// Construct a dependency subject with either, both, or neither selector
    /// dimension.
    #[must_use]
    pub const fn new(
        package: Option<&'subject DependencyProviderId>,
        path: Option<&'subject str>,
    ) -> Self {
        Self {
            provider: match package {
                Some(id) => Some(SelectorProvider::Dependency(id)),
                None => None,
            },
            path,
        }
    }

    /// A point invocation with no provider or path subject.
    #[must_use]
    pub const fn unscoped() -> Self {
        Self::new(None, None)
    }

    /// A dependency-provider subject.
    #[must_use]
    pub const fn package(package: &'subject DependencyProviderId) -> Self {
        Self::new(Some(package), None)
    }

    /// A subject scoped to an explicit typed provider and/or path: the
    /// general constructor both selector dimensions compose through.
    ///
    /// A host-owned document with a path — unspellable through the
    /// dependency-shaped compatibility constructors — is exactly a
    /// `scoped(Some(SelectorProvider::Host(..)), Some(..))` subject, so a
    /// source/document row authoring both `packages` and `paths` can match a
    /// host subject the same way it matches a dependency subject.
    #[must_use]
    pub const fn scoped(
        provider: Option<SelectorProvider<'subject>>,
        path: Option<&'subject str>,
    ) -> Self {
        Self { provider, path }
    }

    /// A dependency-provider subject, the explicit typed spelling.
    #[must_use]
    pub const fn dependency(id: &'subject DependencyProviderId) -> Self {
        Self::scoped(Some(SelectorProvider::Dependency(id)), None)
    }

    /// A host subject carrying the selected host's typed identity.
    #[must_use]
    pub const fn host(identity: &'subject HostIdentity) -> Self {
        Self::scoped(Some(SelectorProvider::Host(identity)), None)
    }

    /// A forward-slashed path-only subject.
    #[must_use]
    pub const fn path(path: &'subject str) -> Self {
        Self::new(None, Some(path))
    }

    /// The typed provider coordinate this subject carries, if any.
    #[must_use]
    pub const fn provider(&self) -> Option<SelectorProvider<'subject>> {
        self.provider
    }
}

/// One authored selector dimension: the exact authored glob spellings in
/// authored order, beside their compiled patterns.
///
/// Equality is the canonical OR-set (PROP-054
/// `#TRANSFORM-PLAN-IDENTITY`): the byte-sorted, deduplicated authored
/// member set, so reordering or duplicating members never changes plan
/// identity. The projection is computed on demand — the compiled patterns
/// and raw accessors keep the authored members exactly — and identity
/// still never depends on the pattern library's internal representation.
#[derive(Debug, Clone)]
struct CompiledDimension {
    authored: Vec<String>,
    compiled: Vec<Pattern>,
}

impl PartialEq for CompiledDimension {
    fn eq(&self, other: &Self) -> bool {
        canonical_members(&self.authored) == canonical_members(&other.authored)
    }
}

impl Eq for CompiledDimension {}

/// The canonical OR-set projection of one dimension's authored members:
/// byte-sorted, deduplicated. Equality alone consumes it; matching still
/// evaluates the authored compiled members and never sorts execution.
fn canonical_members(authored: &[String]) -> Vec<&str> {
    let mut members: Vec<&str> = authored.iter().map(String::as_str).collect();
    members.sort_unstable();
    members.dedup();
    members
}

/// A prevalidated positive selector retained beside one declaration.
///
/// Compilation stays crate-private; the public surface evaluates subjects and
/// exposes the authored pattern members read-only, so a future canonical plan
/// digest can project them without matching order becoming semantic. A
/// selector with no authored `applies_to` matches every subject.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledSelector {
    packages: Option<CompiledDimension>,
    paths: Option<CompiledDimension>,
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

    /// Whether the subject satisfies every authored dimension.
    ///
    /// Within one dimension the authored globs are OR-ed; across dimensions
    /// they are AND-ed. An authored dimension with no subject value for it
    /// fails that dimension; an unauthored dimension passes.
    #[must_use]
    pub fn matches(&self, subject: SelectorSubject<'_>) -> bool {
        let provider = subject.provider.map(SelectorProvider::rendered);
        dimension_matches(self.packages.as_ref(), provider.as_deref())
            && dimension_matches(self.paths.as_ref(), subject.path)
    }

    /// The authored package-dimension globs in authored order, duplicated
    /// entries retained; absent when the dimension was not authored.
    #[must_use]
    pub fn package_patterns(&self) -> Option<&[String]> {
        self.packages
            .as_ref()
            .map(|dimension| dimension.authored.as_slice())
    }

    /// The authored path-dimension globs in authored order, duplicated
    /// entries retained; absent when the dimension was not authored.
    #[must_use]
    pub fn path_patterns(&self) -> Option<&[String]> {
        self.paths
            .as_ref()
            .map(|dimension| dimension.authored.as_slice())
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
) -> Result<Option<CompiledDimension>, SelectorCompileError> {
    patterns
        .map(|patterns| {
            let mut dimension = CompiledDimension {
                authored: Vec::with_capacity(patterns.len()),
                compiled: Vec::with_capacity(patterns.len()),
            };
            for pattern in patterns {
                dimension
                    .compiled
                    .push(Pattern::new(pattern).map_err(|error| SelectorCompileError {
                        field,
                        pattern: pattern.clone(),
                        reason: error.to_string(),
                    })?);
                dimension.authored.push(pattern.clone());
            }
            Ok(dimension)
        })
        .transpose()
}

fn dimension_matches(patterns: Option<&CompiledDimension>, value: Option<&str>) -> bool {
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
        .compiled
        .iter()
        .any(|pattern| pattern.matches_with(value, options))
}
