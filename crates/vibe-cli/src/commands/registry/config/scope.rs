//! `--scope project|local|user` — which declaration file a `vibe registry`
//! configuration command reads or edits (PROP-002 `#registry-layers`).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-layers");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::{Manifest, MirrorSection, RegistrySection, RegistryType};
use vibe_core::{GlobalRegistryConfig, RegistryScope};

use crate::commands::registry::resolve_project_root;

/// Parse a `--scope` value; only the three writable layers are accepted.
pub(super) fn parse_scope(raw: Option<&str>) -> Result<Option<RegistryScope>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    match RegistryScope::parse(raw) {
        Some(RegistryScope::Site) => bail!(
            "--scope site is not editable here: the site profile is applied as a whole by \
             `vibe self apply <file>` (spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-layers)"
        ),
        Some(scope) => Ok(Some(scope)),
        None => bail!("unknown --scope `{raw}` — must be `project`, `local` or `user`"),
    }
}

/// Parse a `--type` value (case-insensitive).
pub(super) fn parse_type(raw: Option<&str>) -> Result<Option<RegistryType>> {
    raw.map(|raw| {
        raw.parse::<RegistryType>()
            .map_err(|message| anyhow!(message))
    })
    .transpose()
}

/// One declaration file opened for reading or editing.
pub(super) enum ScopeFile {
    Project {
        path: PathBuf,
        manifest: Box<Manifest>,
    },
    Config {
        scope: RegistryScope,
        path: PathBuf,
        config: GlobalRegistryConfig,
    },
}

impl ScopeFile {
    /// Open the file behind `scope`. `project` and `local` need a project
    /// (a `vibe.toml` under `project_path`); `user` does not.
    pub(super) fn open(scope: RegistryScope, project_path: &Path) -> Result<Self> {
        match scope {
            RegistryScope::Project => {
                let path = project_manifest_path(project_path)?;
                let manifest = Manifest::read(&path)
                    .with_context(|| format!("reading `{}`", path.display()))?;
                Ok(ScopeFile::Project {
                    path,
                    manifest: Box::new(manifest),
                })
            }
            RegistryScope::Local => {
                let manifest = project_manifest_path(project_path)?;
                let root = manifest
                    .parent()
                    .ok_or_else(|| anyhow!("`{}` has no parent", manifest.display()))?;
                let path = vibe_core::project_local_registry_path(root);
                let config = GlobalRegistryConfig::load_from(&path)?;
                Ok(ScopeFile::Config {
                    scope,
                    path,
                    config,
                })
            }
            RegistryScope::User => {
                let path = vibe_core::settings::registry_config_path().ok_or_else(|| {
                    anyhow!("cannot locate the user settings directory for `registry.toml`")
                })?;
                let config = GlobalRegistryConfig::load_from(&path)?;
                Ok(ScopeFile::Config {
                    scope,
                    path,
                    config,
                })
            }
            RegistryScope::Site => bail!("the site profile is not editable by `vibe registry`"),
        }
    }

    pub(super) fn scope(&self) -> RegistryScope {
        match self {
            ScopeFile::Project { .. } => RegistryScope::Project,
            ScopeFile::Config { scope, .. } => *scope,
        }
    }

    pub(super) fn path(&self) -> &Path {
        match self {
            ScopeFile::Project { path, .. } | ScopeFile::Config { path, .. } => path,
        }
    }

    pub(super) fn registries(&self) -> &[RegistrySection] {
        match self {
            ScopeFile::Project { manifest, .. } => &manifest.registries,
            ScopeFile::Config { config, .. } => &config.registries,
        }
    }

    pub(super) fn registries_mut(&mut self) -> &mut Vec<RegistrySection> {
        match self {
            ScopeFile::Project { manifest, .. } => &mut manifest.registries,
            ScopeFile::Config { config, .. } => &mut config.registries,
        }
    }

    pub(super) fn mirrors(&self) -> &[MirrorSection] {
        match self {
            ScopeFile::Project { manifest, .. } => &manifest.mirrors,
            ScopeFile::Config { config, .. } => &config.mirrors,
        }
    }

    pub(super) fn mirrors_mut(&mut self) -> &mut Vec<MirrorSection> {
        match self {
            ScopeFile::Project { manifest, .. } => &mut manifest.mirrors,
            ScopeFile::Config { config, .. } => &mut config.mirrors,
        }
    }

    /// Mirrors that target `name` (or `*`), lowest priority first.
    pub(super) fn mirrors_for(&self, name: &str) -> Vec<&MirrorSection> {
        let mut mirrors: Vec<&MirrorSection> = self
            .mirrors()
            .iter()
            .filter(|mirror| mirror.of == name || mirror.of == "*")
            .collect();
        mirrors.sort_by_key(|mirror| mirror.priority);
        mirrors
    }

    pub(super) fn save(&self) -> Result<()> {
        match self {
            ScopeFile::Project { path, manifest } => manifest
                .write(path)
                .with_context(|| format!("writing `{}`", path.display())),
            ScopeFile::Config { path, config, .. } => config
                .write_to(path)
                .with_context(|| format!("writing `{}`", path.display())),
        }
    }
}

fn project_manifest_path(project_path: &Path) -> Result<PathBuf> {
    let root = resolve_project_root(project_path)?;
    let path = root.join(Manifest::FILENAME);
    if !path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first (or use `--scope user` for \
             `~/.vibe/registry.toml`)",
            root.display()
        );
    }
    Ok(path)
}
