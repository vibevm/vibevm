//! Which version of an application package a requirement lands on when
//! only part of the version domain can actually be installed here
//! (PROP-075 `#VERSIONS-SELECT`, PROP-066 `#drops`).
//!
//! In an open network the answer is nvm's: the newest version that
//! satisfies the requirement. In a closed one the version domain comes
//! from a mirrored template snapshot and the payloads come from a
//! directory a person filled by hand, and the two do not have to agree —
//! the snapshot lists `22.23.5` because that is what upstream published,
//! while the drop holds `22.23.4` because that is the archive somebody
//! carried in. `vibe install -g node@22` then resolves to a version whose
//! bytes are nowhere on this machine and fails with no network to fall
//! back on.
//!
//! So the rule is a setting, not a law. `newest` is the default and is
//! exactly nvm's semantics; `prefer-drop` keeps the requirement but
//! prefers an installable version inside it; `drop-only` refuses rather
//! than name a version this machine cannot install. Nothing here reads a
//! drop, an index or a registry: the caller says which candidates are
//! **available** and this module decides, so the whole policy is one pure
//! function over a list.
//!
//! An **exact** requirement (`=X.Y.Z`) is never redirected by any policy.
//! A person who named one version meant that version, and a policy that
//! silently installed another would be answering a question nobody asked;
//! under `drop-only` such a pin is simply resolved as before and refuses
//! at the fetch, where the digest is known and the refusal can name the
//! file. [`VersionSpec::is_exact_pin`] is the predicate callers check.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT");

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::VersionSpec;

/// `[versions] select` — how a requirement over an application package
/// chooses among the versions that satisfy it (PROP-075
/// `#VERSIONS-SELECT`).
///
/// ```
/// use vibe_core::version_select::VersionSelect;
///
/// assert_eq!(VersionSelect::default(), VersionSelect::Newest);
/// assert_eq!("prefer-drop".parse::<VersionSelect>().unwrap(), VersionSelect::PreferDrop);
/// assert_eq!(VersionSelect::DropOnly.as_str(), "drop-only");
/// assert!("preferdrop".parse::<VersionSelect>().is_err());
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT")]
pub enum VersionSelect {
    /// The newest version that satisfies the requirement, whatever this
    /// machine holds — nvm's own semantics, and the **default**: the
    /// owner's decision is that an open network must not change
    /// behaviour because a closed one needs another rule.
    #[default]
    Newest,
    /// The newest **available** version that satisfies the requirement —
    /// available meaning the drop already holds every input of the
    /// installer option this platform would select, or the version is
    /// installed here already. With none available the requirement falls
    /// back to [`VersionSelect::Newest`], because a closed network that
    /// has not been filled yet should still get the ordinary refusal
    /// about the missing file rather than a second refusal about policy.
    PreferDrop,
    /// Only available versions are candidates at all. With none the
    /// requirement refuses, naming the version `newest` would have picked
    /// and where its payload belongs — the posture for a machine that is
    /// never going to reach the network and would rather be told now.
    DropOnly,
}

impl VersionSelect {
    /// Every value, in the order help text and documentation list them.
    pub const ALL: [VersionSelect; 3] = [
        VersionSelect::Newest,
        VersionSelect::PreferDrop,
        VersionSelect::DropOnly,
    ];

    /// The stored, kebab-case spelling — the one written in
    /// `config.toml`, read from `VIBE_VERSIONS_SELECT` and printed by
    /// `vibe show config`.
    pub fn as_str(self) -> &'static str {
        match self {
            VersionSelect::Newest => "newest",
            VersionSelect::PreferDrop => "prefer-drop",
            VersionSelect::DropOnly => "drop-only",
        }
    }

    /// `true` for the policy that changes nothing — the fast path every
    /// caller takes before it pays for a drop probe.
    pub fn is_newest(self) -> bool {
        matches!(self, VersionSelect::Newest)
    }

    /// The comma-separated spellings, for the `fix:` of a refusal.
    pub fn spellings() -> String {
        VersionSelect::ALL
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl std::fmt::Display for VersionSelect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A `[versions] select` spelling no policy answers to.
///
/// ```
/// use vibe_core::version_select::VersionSelect;
///
/// let error = "oldest".parse::<VersionSelect>().unwrap_err();
/// assert!(error.to_string().contains("PROP-075#VERSIONS-SELECT"));
/// assert!(error.to_string().contains("newest, prefer-drop, drop-only"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown version-selection policy `{0}` \
     (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT; \
      fix: write one of newest, prefer-drop, drop-only)"
)]
pub struct UnknownVersionSelect(pub String);

impl std::str::FromStr for VersionSelect {
    type Err = UnknownVersionSelect;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let lower = raw.trim().to_ascii_lowercase();
        VersionSelect::ALL
            .into_iter()
            .find(|value| value.as_str() == lower)
            .ok_or_else(|| UnknownVersionSelect(raw.to_owned()))
    }
}

/// One version of the requirement's admitted set, with the one fact the
/// policy decides on: can this machine install it without the network?
///
/// `available` is the caller's verdict and nothing is inferred from it
/// here. Its two grounds are deliberately the same fact seen twice: an
/// `[artifacts] packages` drop already holds every input the installer
/// option this platform would select downloads (PROP-066 `#drops` — the
/// `in drop` mark of `vibe app versions`, computed by exactly that
/// probe), **or** the version is installed in this home already, which
/// downloads nothing at all. A policy that preferred the drop but
/// ignored an installed version would re-download what is already on
/// disk, which is the opposite of what either policy is for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT")]
pub struct Candidate {
    pub version: semver::Version,
    pub available: bool,
}

impl Candidate {
    /// A candidate this machine can install with no network.
    pub fn available(version: semver::Version) -> Self {
        Candidate {
            version,
            available: true,
        }
    }

    /// A candidate whose payload would have to be downloaded.
    pub fn unavailable(version: semver::Version) -> Self {
        Candidate {
            version,
            available: false,
        }
    }
}

/// What the policy decided over one requirement's admitted set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT")]
pub enum Selection {
    /// Resolve as before. `newest` always answers this, and so does
    /// `prefer-drop` when the version it would pick is the one the
    /// ordinary pick lands on anyway — so a caller never rewrites a
    /// requirement into the pin it already meant.
    Unchanged,
    /// Pin the requirement to `version`, which is **not** what the
    /// ordinary pick would have chosen: `instead_of` is that version, and
    /// it is what the one-line note names.
    Redirect {
        version: semver::Version,
        instead_of: semver::Version,
    },
    /// `drop-only`, and nothing in the admitted set is available. The
    /// refusal names `newest` — the version the ordinary pick would have
    /// landed on — so the operator knows which payload to carry in.
    /// `None` means the requirement matched nothing at all, which is the
    /// registry's own `NoMatchingVersion` and not this policy's business.
    Refuse { newest: Option<semver::Version> },
}

/// The version a requirement lands on out of `candidates` — the pick rule
/// the registries themselves walk, in one place.
///
/// Newest non-prerelease match first, and only then the newest match of
/// any prerelease-ness: the same two-step
/// `LocalRegistry::resolve` and the per-package registry's `pick_version`
/// apply, so a policy's answer can never be a version the registry would
/// have refused to pick.
///
/// ```
/// use vibe_core::version_select::pick;
///
/// let versions: Vec<semver::Version> = ["1.0.0", "2.0.0-rc.1", "1.9.0"]
///     .iter()
///     .map(|v| v.parse().unwrap())
///     .collect();
/// assert_eq!(pick(&versions).unwrap().to_string(), "1.9.0");
/// let pre: Vec<semver::Version> = vec!["2.0.0-rc.1".parse().unwrap()];
/// assert_eq!(pick(&pre).unwrap().to_string(), "2.0.0-rc.1");
/// assert!(pick(&[]).is_none());
/// ```
pub fn pick(candidates: &[semver::Version]) -> Option<&semver::Version> {
    candidates
        .iter()
        .filter(|version| version.pre.is_empty())
        .max()
        .or_else(|| candidates.iter().max())
}

/// The versions of `domain` a requirement admits, in ascending order.
///
/// A channel is not a version set this function can compute — only a
/// template knows who belongs to `lts/jod` — so its members arrive as
/// `channel_members`, and a channel with no members supplied admits
/// nothing. That is the same fail-closed reading
/// [`VersionSpec::matches`] gives an unlowered channel, so a caller that
/// forgot to supply them gets a refusal rather than a plausible answer.
///
/// ```
/// use vibe_core::VersionSpec;
/// use vibe_core::version_select::admitted;
///
/// let domain: Vec<semver::Version> = ["22.23.4", "22.23.5", "24.22.3"]
///     .iter()
///     .map(|v| v.parse().unwrap())
///     .collect();
/// let req = VersionSpec::parse("22").unwrap();
/// assert_eq!(admitted(&req, &domain, None).len(), 2);
/// assert_eq!(admitted(&VersionSpec::Latest, &domain, None).len(), 3);
/// ```
pub fn admitted(
    spec: &VersionSpec,
    domain: &[semver::Version],
    channel_members: Option<&[semver::Version]>,
) -> Vec<semver::Version> {
    let mut found: Vec<semver::Version> = match (spec, channel_members) {
        // A lowered channel: its members are the admitted set, narrowed
        // to what the domain actually still carries.
        (VersionSpec::Channel(_), Some(members)) => domain
            .iter()
            .filter(|version| members.contains(version))
            .cloned()
            .collect(),
        (VersionSpec::Channel(_), None) => Vec::new(),
        (spec, _) => domain
            .iter()
            .filter(|version| spec.matches(version))
            .cloned()
            .collect(),
    };
    found.sort();
    found.dedup();
    found
}

/// The policy's verdict over one requirement's admitted set.
///
/// ```
/// use vibe_core::version_select::{Candidate, Selection, VersionSelect, select};
///
/// let candidates = vec![
///     Candidate::available("22.23.4".parse().unwrap()),
///     Candidate::unavailable("22.23.5".parse().unwrap()),
/// ];
/// assert_eq!(select(VersionSelect::Newest, &candidates), Selection::Unchanged);
/// assert_eq!(
///     select(VersionSelect::PreferDrop, &candidates),
///     Selection::Redirect {
///         version: "22.23.4".parse().unwrap(),
///         instead_of: "22.23.5".parse().unwrap(),
///     }
/// );
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT")]
pub fn select(policy: VersionSelect, candidates: &[Candidate]) -> Selection {
    if policy.is_newest() {
        return Selection::Unchanged;
    }
    let all: Vec<semver::Version> = candidates
        .iter()
        .map(|candidate| candidate.version.clone())
        .collect();
    let newest = pick(&all).cloned();
    let available: Vec<semver::Version> = candidates
        .iter()
        .filter(|candidate| candidate.available)
        .map(|candidate| candidate.version.clone())
        .collect();
    let picked = pick(&available).cloned();
    match (policy, picked, newest) {
        // Nothing available, and the requirement is free to look
        // elsewhere: the ordinary resolution runs and refuses, if it must,
        // about the file rather than about the policy.
        (VersionSelect::PreferDrop, None, _) => Selection::Unchanged,
        (VersionSelect::DropOnly, None, newest) => Selection::Refuse { newest },
        // The policy's pick IS the ordinary pick: say nothing and rewrite
        // nothing, so an operator sees a note only when the answer moved.
        (_, Some(picked), Some(newest)) if picked == newest => Selection::Unchanged,
        (_, Some(version), Some(instead_of)) => Selection::Redirect {
            version,
            instead_of,
        },
        // `picked` came out of `candidates`, so `newest` is `Some`
        // whenever `picked` is; this arm is unreachable and resolves as
        // before rather than asserting in a user's install.
        (_, Some(_), None) => Selection::Unchanged,
        (VersionSelect::Newest, _, _) => Selection::Unchanged,
    }
}

#[cfg(test)]
#[path = "version_select/tests.rs"]
mod tests;
