//! The tree of a directory-layout registry (`monorepo`, `localdir`) for a
//! command that reads its index (PROP-002 `#registry-index`): a localdir is
//! its directory; a monorepo is cloned through the operation's controlled
//! transport, source policy and remap Locator — the same clone resolution
//! makes.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-index");

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};
use vibe_core::manifest::{MirrorSection, RegistrySection, RegistryType};
use vibe_index::layout::LayoutIdentity;
use vibe_registry::RegistryError;
use vibe_registry::multi_registry_resolver::RegistrySource;

/// An opened layout registry; holds the clone alive while it is read.
pub(crate) struct LayoutRoot {
    root: PathBuf,
    identity: LayoutIdentity,
    _source: Option<Arc<RegistrySource>>,
}

impl LayoutRoot {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn identity(&self) -> &LayoutIdentity {
        &self.identity
    }
}

/// Whether `reg` is a `monorepo` or `localdir` registry.
pub(crate) fn is_layout(reg: &RegistrySection) -> bool {
    matches!(
        reg.checked_type(),
        Ok(RegistryType::Monorepo | RegistryType::Localdir)
    )
}

/// Open the tree of the layout registry `reg`.
pub(crate) fn open(
    reg: &RegistrySection,
    mirrors: &[MirrorSection],
    operation: &vibe_package_source::SourceOperation,
    offline: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<LayoutRoot> {
    match reg.checked_type()? {
        RegistryType::Localdir => {
            let root = vibe_registry::localdir_path(&reg.url)?;
            if !root.is_dir() {
                bail!(
                    "registry `{}` names the directory `{}`, which does not exist \
                     (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type; \
                     fix: mount or create it, or correct the registry url)",
                    reg.name,
                    root.display()
                );
            }
            Ok(LayoutRoot {
                root,
                identity: LayoutIdentity {
                    registry: reg.name.clone(),
                    registry_url: reg.url.clone(),
                    branch: None,
                },
                _source: None,
            })
        }
        RegistryType::Monorepo => {
            let resolver = vibe_package_source::registry_resolver(
                operation,
                vec![reg.clone()],
                mirrors.to_vec(),
                offline,
                progress,
            )?;
            let source = resolver
                .selected_registry_source(&reg.name)
                .map_err(refusal_first)?;
            let root = match source.as_ref() {
                RegistrySource::Monorepo(mono) => {
                    mono.local().map_err(refusal_first)?.root().to_path_buf()
                }
                _ => bail!(
                    "internal: registry `{}` did not open as a monorepo",
                    reg.name
                ),
            };
            Ok(LayoutRoot {
                root,
                identity: LayoutIdentity {
                    registry: reg.name.clone(),
                    registry_url: reg.url.clone(),
                    branch: Some(reg.r#ref.clone()),
                },
                _source: Some(source),
            })
        }
        other => bail!(
            "registry `{}` is a `{}` registry, not a monorepo or localdir",
            reg.name,
            other.as_str()
        ),
    }
}

/// A contact refusal travels as itself, so a caller can tell a policy
/// skip from a broken registry.
pub(crate) fn refusal_first(error: RegistryError) -> anyhow::Error {
    match error {
        RegistryError::ContactRefused(refusal) => refusal.into(),
        other => other.into(),
    }
}
