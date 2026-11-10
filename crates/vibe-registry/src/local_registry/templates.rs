//! What a **directory-shaped** registry asks the template layer, and what
//! it answers about channels (PROP-075 `#TEMPLATE-DOMAIN-SEAM`'s fourth
//! call site, `#CHANNEL-SYNTAX`).
//!
//! `LocalRegistry::{list_versions, resolve}` themselves stay in the file
//! above, where the pre-PROP-075 behaviour lives and where each template
//! branch is one `if let`; everything those branches need is here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DOMAIN-SEAM");

use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef, VersionSpec};

use super::LocalRegistry;
use crate::RegistryError;

impl LocalRegistry {
    /// `pkgref` with its channel lowered to an exact pin, or `None` when
    /// it named no channel. The channel is noted on the template layer so
    /// the lock row carries `via_channel`.
    pub(super) fn lower(&self, pkgref: &PackageRef) -> Result<Option<PackageRef>, RegistryError> {
        if pkgref.version.channel().is_none() {
            return Ok(None);
        }
        let mut lowered = pkgref.clone();
        let channel = vibe_core::channel::lower_channel(&mut lowered, self).map_err(|error| {
            RegistryError::MalformedMeta {
                path: self.root().join("<channel>"),
                reason: error.to_string(),
            }
        })?;
        if let (Some(layer), Some(channel), Some(group)) =
            (self.template_layer(), &channel, lowered.group.as_ref())
            && let VersionSpec::Req(req) = &lowered.version
            && let Some(comparator) = req.comparators.first()
        {
            let version = semver::Version::new(
                comparator.major,
                comparator.minor.unwrap_or(0),
                comparator.patch.unwrap_or(0),
            );
            layer.record_channel(&format!("{group}/{}", lowered.name), &version, channel);
        }
        Ok(Some(lowered))
    }

    /// The **skeleton** manifest of a version this registry does not
    /// publish but its template produces — **call site 2** of
    /// `#TEMPLATE-DOMAIN-SEAM`, which costs no digest request.
    pub(crate) fn template_skeleton(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Option<Manifest>, RegistryError> {
        let Some(layer) = self.template_layer() else {
            return Ok(None);
        };
        layer
            .directory_skeleton(self.root(), group, name, version)
            .map_err(RegistryError::from)
    }
}

/// A directory registry answers channel membership from its own
/// template's `[[template.channel]]` rules (PROP-075 `#CHANNEL-SYNTAX`).
/// `latest` and `stable` are answered by `lower_channel` itself, from
/// [`vibe_core::channel::ChannelResolver::domain`], so every package has
/// them declared or not.
impl vibe_core::channel::ChannelResolver for LocalRegistry {
    fn domain(&self, group: &Group, name: &str) -> Result<Vec<semver::Version>, String> {
        LocalRegistry::list_versions(self, group, name).map_err(|error| error.to_string())
    }

    fn members(
        &self,
        group: &Group,
        name: &str,
        channel: &vibe_core::ChannelName,
    ) -> Result<Option<Vec<semver::Version>>, String> {
        let Some(layer) = self.template_layer() else {
            return Ok(None);
        };
        let Some(found) = layer
            .for_directory(self.root(), group, name)
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        Ok(found
            .domain
            .channels()
            .get(channel.as_str())
            .map(|members| members.iter().cloned().collect()))
    }
}
