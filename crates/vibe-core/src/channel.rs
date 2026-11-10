//! Channel lowering — the one function that turns `@lts` into an exact
//! pin, and the seam that answers what a channel's members are
//! (PROP-075 `#CHANNEL-SYNTAX`).
//!
//! **A channel names a version set and lowers to the newest member of
//! that set as an exact pin, before the solve.** That is what
//! `nvm install --lts` means and it is what keeps the solver untouched:
//! nothing below this function ever sees a [`VersionSpec::Channel`], and
//! a path that forgot to call it fails closed, because
//! [`VersionSpec::matches`] of a channel is `false` and the solver's
//! version set for one is empty.
//!
//! The resolver of channel membership is a trait rather than a concrete
//! type because the answer comes from two different places: a template's
//! `[[template.channel]]` rules evaluated over its version records, and a
//! concrete package's publisher-assigned `[package] channels` labels read
//! out of the registry index. Both are in `vibe-registry`; neither
//! belongs here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX");

use crate::package_ref::{ChannelName, Group, PackageRef, VersionSpec};

/// What a channel's members are, for one package.
///
/// An implementor answers from the package's version domain: the
/// template's channel rules over its version records, or the concrete
/// versions' index-recorded labels, or both unioned. It is **not** asked
/// about the two reserved names — [`lower_channel`] answers `latest` and
/// `stable` from [`Self::domain`] itself, so every package has them
/// whether or not it declares anything.
/// ```
/// use vibe_core::channel::ChannelResolver;
/// use vibe_core::{ChannelName, Group};
///
/// struct Published;
/// impl ChannelResolver for Published {
///     fn domain(&self, _g: &Group, _n: &str) -> Result<Vec<semver::Version>, String> {
///         Ok(vec!["1.0.0".parse().unwrap(), "2.0.0".parse().unwrap()])
///     }
///     fn members(
///         &self,
///         _g: &Group,
///         _n: &str,
///         channel: &ChannelName,
///     ) -> Result<Option<Vec<semver::Version>>, String> {
///         Ok((channel.as_str() == "lts").then(|| vec!["1.0.0".parse().unwrap()]))
///     }
/// }
///
/// let group = Group::parse("org.example")?;
/// let resolver: &dyn ChannelResolver = &Published;
/// assert_eq!(resolver.domain(&group, "tool")?.len(), 2);
/// let lts = ChannelName::parse("lts").unwrap();
/// assert_eq!(resolver.members(&group, "tool", &lts)?.unwrap().len(), 1);
/// // An undeclared channel is `None`, which `lower_channel` turns into
/// // `ChannelError::UnknownChannel`.
/// let canary = ChannelName::parse("canary").unwrap();
/// assert!(resolver.members(&group, "tool", &canary)?.is_none());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait ChannelResolver {
    /// Every version this package's domain holds, in any order.
    fn domain(&self, group: &Group, name: &str) -> Result<Vec<semver::Version>, String>;

    /// The versions that belong to `channel`, in any order. An empty
    /// answer means the channel exists but has no member; `Ok(None)`
    /// means the package declares no such channel at all, which is the
    /// refusal [`lower_channel`] reports.
    fn members(
        &self,
        group: &Group,
        name: &str,
        channel: &ChannelName,
    ) -> Result<Option<Vec<semver::Version>>, String>;
}

/// Why a channel could not be lowered. Every variant names the package
/// and the channel, because a refusal a user cannot act on is a bug.
///
/// ```
/// use vibe_core::channel::ChannelError;
///
/// let refusal = ChannelError::UnknownChannel {
///     group: "org.vibevm.node".to_string(),
///     name: "node".to_string(),
///     channel: "canary".to_string(),
/// };
/// assert!(refusal.to_string().contains("canary"));
/// assert!(refusal.to_string().contains("fix:"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
pub enum ChannelError {
    /// The requirement carries no group, so no package can be asked.
    #[error(
        "`{name}@{channel}` names a channel but no group — qualify it as \
         `<group>/{name}@{channel}` (violates \
         spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX; \
         fix: write the package's group)"
    )]
    Unqualified { name: String, channel: String },
    /// The package declares no such channel.
    #[error(
        "`{group}/{name}` declares no channel `{channel}` (violates \
         spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX; \
         fix: run `vibe app versions {group}/{name}` to see the channels it has)"
    )]
    UnknownChannel {
        group: String,
        name: String,
        channel: String,
    },
    /// The channel exists and nothing is in it.
    #[error(
        "channel `{channel}` of `{group}/{name}` has no version (violates \
         spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX; \
         fix: pick another channel or an explicit version)"
    )]
    EmptyChannel {
        group: String,
        name: String,
        channel: String,
    },
    /// The resolver could not answer — a transport failure, a missing
    /// index, an unreadable cache. The inner text is the resolver's own.
    #[error(
        "channel `{channel}` of `{group}/{name}` could not be resolved: {reason} (violates \
         spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX; \
         fix: resolve the reported cause, or name an explicit version)"
    )]
    Unresolvable {
        group: String,
        name: String,
        channel: String,
        reason: String,
    },
}

/// Lower a channel requirement in place to the exact pin of its newest
/// member. A requirement that names no channel is left untouched, so this
/// is safe to call on every requirement at an entry path.
///
/// ```
/// use vibe_core::channel::{ChannelResolver, lower_channel};
/// use vibe_core::{ChannelName, Group, PackageRef, VersionSpec};
///
/// struct Node;
/// impl ChannelResolver for Node {
///     fn domain(&self, _g: &Group, _n: &str) -> Result<Vec<semver::Version>, String> {
///         Ok(vec!["22.9.0".parse().unwrap(), "24.1.0".parse().unwrap()])
///     }
///     fn members(
///         &self,
///         _g: &Group,
///         _n: &str,
///         channel: &ChannelName,
///     ) -> Result<Option<Vec<semver::Version>>, String> {
///         Ok((channel.as_str() == "lts").then(|| vec!["22.9.0".parse().unwrap()]))
///     }
/// }
///
/// let mut pkgref = PackageRef::parse("org.vibevm.node/node@lts")?;
/// let via = lower_channel(&mut pkgref, &Node)?;
/// assert_eq!(via.unwrap().as_str(), "lts");
/// assert_eq!(pkgref.version, VersionSpec::parse("=22.9.0")?);
/// // `latest` is answered from the domain, declared or not.
/// let mut newest = PackageRef::parse("org.vibevm.node/node@latest")?;
/// lower_channel(&mut newest, &Node)?;
/// assert_eq!(newest.version, VersionSpec::parse("=24.1.0")?);
/// // A requirement that is not a channel is untouched.
/// let mut plain = PackageRef::parse("org.vibevm.node/node@^22")?;
/// assert!(lower_channel(&mut plain, &Node)?.is_none());
/// assert_eq!(plain.version, VersionSpec::parse("^22")?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn lower_channel(
    pkgref: &mut PackageRef,
    resolver: &dyn ChannelResolver,
) -> Result<Option<ChannelName>, ChannelError> {
    let Some(channel) = pkgref.version.channel().cloned() else {
        return Ok(None);
    };
    let Some(group) = pkgref.group.clone() else {
        return Err(ChannelError::Unqualified {
            name: pkgref.name.to_string(),
            channel: channel.as_str().to_string(),
        });
    };
    let name = pkgref.name.to_string();
    let unresolvable = |reason: String| ChannelError::Unresolvable {
        group: group.to_string(),
        name: name.clone(),
        channel: channel.as_str().to_string(),
        reason,
    };

    let members = if channel.is_reserved() {
        let domain = resolver.domain(&group, &name).map_err(&unresolvable)?;
        if channel.as_str() == ChannelName::STABLE {
            domain.into_iter().filter(|v| v.pre.is_empty()).collect()
        } else {
            domain
        }
    } else {
        resolver
            .members(&group, &name, &channel)
            .map_err(&unresolvable)?
            .ok_or_else(|| ChannelError::UnknownChannel {
                group: group.to_string(),
                name: name.clone(),
                channel: channel.as_str().to_string(),
            })?
    };
    let newest = members
        .into_iter()
        .max()
        .ok_or_else(|| ChannelError::EmptyChannel {
            group: group.to_string(),
            name: name.clone(),
            channel: channel.as_str().to_string(),
        })?;
    pkgref.version = VersionSpec::Req(exact(&newest));
    Ok(Some(channel))
}

/// The `=X.Y.Z` requirement that pins exactly `version` — the shape
/// [`VersionSpec::is_exact_pin`] recognises.
///
/// ```
/// use vibe_core::channel::exact;
///
/// let req = exact(&"24.22.2".parse().unwrap());
/// assert_eq!(req.to_string(), "=24.22.2");
/// ```
pub fn exact(version: &semver::Version) -> semver::VersionReq {
    semver::VersionReq {
        comparators: vec![semver::Comparator {
            op: semver::Op::Exact,
            major: version.major,
            minor: Some(version.minor),
            patch: Some(version.patch),
            pre: version.pre.clone(),
        }],
    }
}

#[cfg(test)]
#[path = "channel/tests.rs"]
mod tests;
