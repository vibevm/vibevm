//! The registry declaration layers `vibe show config` reports, each row
//! with the layer it came from (PROP-002 `#registry-layers`), and the
//! applied site profile (`PROP-019#site-profile`).

use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use vibe_core::manifest::Manifest;

use super::forward_slash_display;

#[derive(Debug, Clone, Serialize)]
pub(super) struct ConfigSiteProfileSummary {
    /// Where an applied profile is stored. `null` without a settings dir.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) path: Option<String>,
    /// `true` when a profile is applied and parses.
    pub(super) loaded: bool,
    /// The profile's `name`, when it declares one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<String>,
    /// Why an existing profile could not be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

/// One layer's mirrors and overrides, with the layer they came from.
pub(super) type LayerSections<'a> = (
    vibe_core::RegistryScope,
    &'a [vibe_core::manifest::MirrorSection],
    &'a [vibe_core::manifest::OverrideSection],
);

/// The registry declaration files below the project manifest.
pub(super) struct DeclarationLayers {
    local: vibe_core::GlobalRegistryConfig,
    user: vibe_core::GlobalRegistryConfig,
    site: Option<vibe_core::GlobalRegistryConfig>,
    pub(super) site_summary: ConfigSiteProfileSummary,
}

impl DeclarationLayers {
    pub(super) fn load(project_root: &Path) -> Result<Self> {
        let local = vibe_core::GlobalRegistryConfig::load_project_local(project_root)?;
        let user = match vibe_core::settings::registry_config_path() {
            Some(path) => vibe_core::GlobalRegistryConfig::load_from(&path)?,
            None => vibe_core::GlobalRegistryConfig::default(),
        };
        let path = vibe_core::settings::site_profile_path();
        let shown = path.as_deref().map(forward_slash_display);
        // Like the user config, an unreadable profile is reported, not fatal:
        // inspection of the other layers must still work.
        let (site, site_summary) = match path.as_deref().map(vibe_core::site_profile::installed_at)
        {
            Some(Ok(Some(site))) => (
                Some(site.profile.registry_layer()),
                ConfigSiteProfileSummary {
                    path: shown,
                    loaded: true,
                    name: site.profile.name.clone(),
                    error: None,
                },
            ),
            Some(Err(error)) => (
                None,
                ConfigSiteProfileSummary {
                    path: shown,
                    loaded: false,
                    name: None,
                    error: Some(error.to_string()),
                },
            ),
            Some(Ok(None)) | None => (
                None,
                ConfigSiteProfileSummary {
                    path: shown,
                    loaded: false,
                    name: None,
                    error: None,
                },
            ),
        };
        Ok(Self {
            local,
            user,
            site,
            site_summary,
        })
    }

    pub(super) fn layers<'a>(&'a self, manifest: &'a Manifest) -> vibe_core::RegistryLayers<'a> {
        vibe_core::RegistryLayers {
            local: Some(&self.local),
            project: Some(manifest),
            user: &self.user,
            site: self.site.as_ref(),
        }
    }

    /// `(layer, mirrors, overrides)`, highest layer first.
    pub(super) fn sections<'a>(&'a self, manifest: &'a Manifest) -> Vec<LayerSections<'a>> {
        use vibe_core::RegistryScope;
        let mut out = vec![
            (
                RegistryScope::Local,
                &self.local.mirrors[..],
                &self.local.overrides[..],
            ),
            (
                RegistryScope::Project,
                &manifest.mirrors[..],
                &manifest.overrides[..],
            ),
            (
                RegistryScope::User,
                &self.user.mirrors[..],
                &self.user.overrides[..],
            ),
        ];
        if let Some(site) = &self.site {
            out.push((RegistryScope::Site, &site.mirrors[..], &site.overrides[..]));
        }
        out
    }
}

pub(super) fn provenance_of(scope: vibe_core::RegistryScope) -> &'static str {
    match scope {
        vibe_core::RegistryScope::Local => ".vibe/registry.local.toml",
        vibe_core::RegistryScope::Project => "vibe.toml",
        vibe_core::RegistryScope::User => "~/.vibe/registry.toml",
        vibe_core::RegistryScope::Site => "site-profile.toml",
    }
}
