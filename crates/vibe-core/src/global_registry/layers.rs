//! Registry declaration layers and their precedence (PROP-002
//! `#registry-layers`).
//!
//! Highest first: invocation flags (`--registry <dir>`, handled by the
//! caller before any merge) → the project-local override
//! `<project>/.vibe/registry.local.toml` → the project `vibe.toml` → the
//! user's `~/.vibe/registry.toml` → the administrator's site profile → the
//! built-in defaults (seeded into the user file when it is missing). A
//! registry `name` declared by a higher layer hides the same name below it;
//! mirrors concatenate in layer order; overrides dedupe by `pkgref`.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-layers");

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{EffectiveRegistryConfig, GlobalRegistryConfig, GlobalRegistryError};
use crate::manifest::{Manifest, MirrorSection, OverrideSection, RegistrySection};

/// Basename of the project-local, never-committed registry override file.
pub const PROJECT_LOCAL_REGISTRY_FILE: &str = "registry.local.toml";

/// `<project_root>/.vibe/registry.local.toml`.
///
/// ```
/// use std::path::Path;
/// let path = vibe_core::project_local_registry_path(Path::new("/work/app"));
/// assert!(path.ends_with(".vibe/registry.local.toml"));
/// ```
pub fn project_local_registry_path(project_root: &Path) -> PathBuf {
    project_root.join(".vibe").join(PROJECT_LOCAL_REGISTRY_FILE)
}

impl GlobalRegistryConfig {
    /// Load the project-local override of `project_root`. Same grammar as
    /// `~/.vibe/registry.toml`; a missing file is the empty layer.
    pub fn load_project_local(project_root: &Path) -> Result<Self, GlobalRegistryError> {
        Self::load_from(&project_local_registry_path(project_root))
    }
}

/// Which declaration layer a registry row came from.
///
/// ```
/// use vibe_core::RegistryScope;
/// assert_eq!(RegistryScope::parse("LOCAL"), Some(RegistryScope::Local));
/// assert_eq!(RegistryScope::User.as_str(), "user");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryScope {
    /// `<project>/.vibe/registry.local.toml` — personal, never committed.
    Local,
    /// `<project>/vibe.toml` — shared with the team.
    Project,
    /// `~/.vibe/registry.toml` — this user on this machine.
    User,
    /// The administrator's site profile (applied by `vibe self apply`).
    Site,
}

impl RegistryScope {
    /// The spelling used by `--scope` and in reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            RegistryScope::Local => "local",
            RegistryScope::Project => "project",
            RegistryScope::User => "user",
            RegistryScope::Site => "site",
        }
    }

    /// Case-insensitive parse of a `--scope` value.
    pub fn parse(input: &str) -> Option<Self> {
        [
            RegistryScope::Local,
            RegistryScope::Project,
            RegistryScope::User,
            RegistryScope::Site,
        ]
        .into_iter()
        .find(|scope| scope.as_str().eq_ignore_ascii_case(input.trim()))
    }
}

/// The declaration layers of one invocation, each optional except the
/// user layer (an absent user file is the empty layer).
///
/// ```
/// use vibe_core::{GlobalRegistryConfig, RegistryLayers, RegistryScope};
/// use vibe_core::manifest::Manifest;
///
/// let project = Manifest::parse_str(
///     "[project]\nname='app'\nversion='0.1.0'\n\
///      [[registry]]\nname='corp'\nurl='https://github.com/corp'\n",
/// ).unwrap();
/// let local: GlobalRegistryConfig = toml::from_str(
///     "[[registry]]\nname='corp'\ntype='monorepo'\nurl='ssh://git@git.corp/vibe.git'\n",
/// ).unwrap();
/// let user = GlobalRegistryConfig::default();
/// let layers = RegistryLayers { local: Some(&local), project: Some(&project), user: &user, site: None };
/// let effective = layers.merge();
/// // The project-local override hides the project's row of the same name.
/// assert_eq!(effective.registries.len(), 1);
/// assert_eq!(effective.registries[0].url, "ssh://git@git.corp/vibe.git");
/// assert_eq!(layers.origins()[0].0, RegistryScope::Local);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RegistryLayers<'a> {
    pub local: Option<&'a GlobalRegistryConfig>,
    pub project: Option<&'a Manifest>,
    pub user: &'a GlobalRegistryConfig,
    /// The site-profile layer, below the user's own file. Reserved for the
    /// `vibe self apply` profile; `None` until one is applied.
    pub site: Option<&'a GlobalRegistryConfig>,
}

struct Layer<'a> {
    scope: RegistryScope,
    registries: &'a [RegistrySection],
    mirrors: &'a [MirrorSection],
    overrides: &'a [OverrideSection],
}

impl<'a> RegistryLayers<'a> {
    fn ordered(&self) -> Vec<Layer<'a>> {
        let config = |scope, config: &'a GlobalRegistryConfig| Layer {
            scope,
            registries: &config.registries,
            mirrors: &config.mirrors,
            overrides: &config.overrides,
        };
        let mut layers = Vec::with_capacity(4);
        if let Some(local) = self.local {
            layers.push(config(RegistryScope::Local, local));
        }
        if let Some(project) = self.project {
            layers.push(Layer {
                scope: RegistryScope::Project,
                registries: &project.registries,
                mirrors: &project.mirrors,
                overrides: &project.overrides,
            });
        }
        layers.push(config(RegistryScope::User, self.user));
        if let Some(site) = self.site {
            layers.push(config(RegistryScope::Site, site));
        }
        layers
    }

    /// Every visible registry row with the layer that declared it, in
    /// effective (priority) order.
    pub fn origins(&self) -> Vec<(RegistryScope, RegistrySection)> {
        let mut seen = HashSet::new();
        let mut rows = Vec::new();
        for layer in self.ordered() {
            for row in layer.registries {
                if seen.insert(row.name.clone()) {
                    rows.push((layer.scope, row.clone()));
                }
            }
        }
        rows
    }

    /// The resolution-ready merge of every layer.
    pub fn merge(&self) -> EffectiveRegistryConfig {
        let registries = self.origins().into_iter().map(|(_, row)| row).collect();
        let mut mirrors = Vec::new();
        let mut overrides: Vec<OverrideSection> = Vec::new();
        let mut seen_overrides = HashSet::new();
        for layer in self.ordered() {
            let publication = match layer.scope {
                RegistryScope::Project => crate::RoutePublication::Project,
                RegistryScope::Local | RegistryScope::User | RegistryScope::Site => {
                    crate::RoutePublication::Local
                }
            };
            mirrors.extend(layer.mirrors.iter().cloned().map(|mut mirror| {
                mirror.publication = publication;
                mirror
            }));
            for row in layer.overrides {
                if seen_overrides.insert(row.pkgref.clone()) {
                    overrides.push(row.clone());
                }
            }
        }
        EffectiveRegistryConfig {
            registries,
            mirrors,
            overrides,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(body: &str) -> GlobalRegistryConfig {
        toml::from_str(body).unwrap()
    }

    fn project(body: &str) -> Manifest {
        Manifest::parse_str(&format!("[project]\nname='app'\nversion='0.1.0'\n{body}")).unwrap()
    }

    /// Every layer pair: the higher layer's row wins a name collision and
    /// the lower layer's distinct rows follow in layer order.
    #[test]
    fn precedence_matrix() {
        let local = config(
            "[[registry]]\nname='shared'\nurl='file:///local'\n[[registry]]\nname='l'\nurl='file:///l'\n",
        );
        let project = project(
            "[[registry]]\nname='shared'\nurl='file:///project'\n[[registry]]\nname='p'\nurl='file:///p'\n",
        );
        let user = config(
            "[[registry]]\nname='shared'\nurl='file:///user'\n[[registry]]\nname='u'\nurl='file:///u'\n",
        );
        let site = config(
            "[[registry]]\nname='shared'\nurl='file:///site'\n[[registry]]\nname='s'\nurl='file:///s'\n",
        );
        type Case<'a> = (
            Option<&'a GlobalRegistryConfig>,
            Option<&'a Manifest>,
            Option<&'a GlobalRegistryConfig>,
            &'a str,
            &'a [&'a str],
        );
        let cases: [Case<'_>; 4] = [
            (
                Some(&local),
                Some(&project),
                Some(&site),
                "file:///local",
                &["shared", "l", "p", "u", "s"],
            ),
            (
                None,
                Some(&project),
                Some(&site),
                "file:///project",
                &["shared", "p", "u", "s"],
            ),
            (
                None,
                None,
                Some(&site),
                "file:///user",
                &["shared", "u", "s"],
            ),
            (
                Some(&local),
                None,
                None,
                "file:///local",
                &["shared", "l", "u"],
            ),
        ];
        for (local, project, site, winner, names) in cases {
            let layers = RegistryLayers {
                local,
                project,
                user: &user,
                site,
            };
            let merged = layers.merge();
            assert_eq!(merged.registries[0].url, winner);
            let got: Vec<&str> = merged.registries.iter().map(|r| r.name.as_str()).collect();
            assert_eq!(got, names);
        }
        // Site sits below user even with no other layer.
        let empty_user = GlobalRegistryConfig::default();
        let layers = RegistryLayers {
            local: None,
            project: None,
            user: &empty_user,
            site: Some(&site),
        };
        assert_eq!(layers.origins()[0].0, RegistryScope::Site);
    }

    #[test]
    fn every_type_parses_in_every_layer() {
        for kind in crate::manifest::RegistryType::ALL {
            let row = format!(
                "[[registry]]\nname='r'\ntype='{}'\nurl='file:///x'\n",
                kind.as_str().to_ascii_uppercase()
            );
            assert_eq!(config(&row).registries[0].r#type, Some(kind));
            assert_eq!(project(&row).registries[0].r#type, Some(kind));
        }
    }

    #[test]
    fn missing_local_file_is_the_empty_layer() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            GlobalRegistryConfig::load_project_local(dir.path()).unwrap(),
            GlobalRegistryConfig::default()
        );
        std::fs::create_dir_all(dir.path().join(".vibe")).unwrap();
        std::fs::write(
            project_local_registry_path(dir.path()),
            "[[registry]]\nname='corp'\ntype='Monorepo'\nurl='ssh://git@h/r.git'\n",
        )
        .unwrap();
        let loaded = GlobalRegistryConfig::load_project_local(dir.path()).unwrap();
        assert_eq!(
            loaded.registries[0].r#type,
            Some(crate::manifest::RegistryType::Monorepo)
        );
    }

    #[test]
    fn local_mirrors_are_personal_routes() {
        let local = config("[[mirror]]\nof='corp'\nurl='file:///m'\n");
        let user = GlobalRegistryConfig::default();
        let merged = RegistryLayers {
            local: Some(&local),
            project: None,
            user: &user,
            site: None,
        }
        .merge();
        assert_eq!(
            merged.mirrors[0].publication,
            crate::RoutePublication::Local
        );
    }
}
