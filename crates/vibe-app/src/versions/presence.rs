//! Explicit package presence from the same observation that supplies its domain.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT");

use super::{Domain, Domains, Heading, VersionsError, VersionsRequest, split_target};

/// What a domain source established about package existence, never inferred
/// from an empty version list (PROP-075 `#VERSIONS-LIST-OUTPUT`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
pub enum PackagePresence {
    /// A source knows the package, even when it publishes no versions.
    Present,
    /// Every consulted source confirmed the package is absent.
    Absent,
    /// No definitive observation, including legacy and cache-only answers.
    #[default]
    Indeterminate,
}

/// A domain and its presence verdict from one source observation.
/// Existing [`Domain`] literals remain source compatible (PROP-075
/// `#VERSIONS-LIST-OUTPUT`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
pub struct DomainObservation {
    /// The unfiltered domain returned by this observation.
    pub domain: Domain,
    /// Explicit existence evidence, not the number of versions returned.
    pub presence: PackagePresence,
}

pub(super) fn observed_domain(
    heading: &Heading,
    request: &VersionsRequest<'_>,
    has_installations: bool,
    domains: &dyn Domains,
) -> Result<Domain, VersionsError> {
    if request.installed {
        return Ok(Domain::default());
    }
    let coordinate = heading.coordinate();
    let observation = domains
        .observe_domain(&heading.group, &heading.name)
        .map_err(|reason| VersionsError::Domain {
            package: coordinate.clone(),
            reason,
        })?;
    // Concrete rows and a known template are affirmative evidence even if a
    // source's absence verdict was stale. Never probe a second time to decide.
    if observation.presence == PackagePresence::Absent
        && observation.domain.template.is_none()
        && observation.domain.versions.is_empty()
        && !has_installations
        && split_target(request.target).0.contains('/')
    {
        return Err(VersionsError::UnknownPackage {
            package: coordinate,
            name: heading.name.clone(),
        });
    }
    Ok(observation.domain)
}

#[cfg(test)]
pub(super) mod fixtures {
    use super::*;
    use std::cell::Cell;

    pub struct FakeDomains {
        pub answer: Result<DomainObservation, String>,
        pub observations: Cell<usize>,
        pub legacy_calls: Cell<usize>,
    }

    impl Domains for FakeDomains {
        fn domain(&self, _: &str, _: &str) -> Result<Domain, String> {
            self.legacy_calls.set(self.legacy_calls.get() + 1);
            panic!("an explicit observation must not probe domain again")
        }

        fn observe_domain(&self, _: &str, _: &str) -> Result<DomainObservation, String> {
            self.observations.set(self.observations.get() + 1);
            self.answer.clone()
        }
    }

    pub struct Legacy(pub Cell<usize>);

    impl Domains for Legacy {
        fn domain(&self, _: &str, _: &str) -> Result<Domain, String> {
            self.0.set(self.0.get() + 1);
            Ok(Domain::default())
        }
    }
}
