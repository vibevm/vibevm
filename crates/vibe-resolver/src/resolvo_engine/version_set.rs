//! `SemverVersionSet` — vibevm's version constraints expressed as a
//! resolvo [`VersionSet`] over `semver::Version` (PROP-017 §2, §3).

use std::collections::BTreeSet;
use std::fmt;

use resolvo::utils::VersionSet;
use vibe_core::VersionSpec;

/// A set of `semver::Version`s — the unit resolvo interns and asks about
/// through `filter_candidates`. `Any` is the match-all set (the encoding
/// of `VersionSpec::Latest`); `Req` wraps a semver range; `None` is the
/// match-nothing set — the encoding of `[conflicts]` ("if present, the
/// version must be in ∅", so the rival package cannot be selected).
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum SemverVersionSet {
    Any,
    Req(semver::VersionReq),
    None,
    /// An explicit set of versions — used to pin a capability provider to
    /// exactly the versions of it that provide the required capability
    /// (PROP-017 §3).
    Explicit(BTreeSet<semver::Version>),
}

impl SemverVersionSet {
    /// Lift a vibevm `VersionSpec` into a version set.
    pub(crate) fn from_spec(spec: &VersionSpec) -> Self {
        match spec {
            VersionSpec::Latest => SemverVersionSet::Any,
            VersionSpec::Req(req) => SemverVersionSet::Req(req.clone()),
        }
    }

    /// `true` iff `version` is a member of the set.
    pub(crate) fn contains(&self, version: &semver::Version) -> bool {
        match self {
            SemverVersionSet::Any => true,
            SemverVersionSet::Req(req) => req.matches(version),
            SemverVersionSet::None => false,
            SemverVersionSet::Explicit(set) => set.contains(version),
        }
    }
}

impl VersionSet for SemverVersionSet {
    type V = semver::Version;
}

impl fmt::Display for SemverVersionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemverVersionSet::Any => f.write_str("*"),
            SemverVersionSet::Req(req) => write!(f, "{req}"),
            SemverVersionSet::None => f.write_str("(none)"),
            SemverVersionSet::Explicit(set) => {
                let joined = set
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" | ");
                write!(f, "{{{joined}}}")
            }
        }
    }
}
