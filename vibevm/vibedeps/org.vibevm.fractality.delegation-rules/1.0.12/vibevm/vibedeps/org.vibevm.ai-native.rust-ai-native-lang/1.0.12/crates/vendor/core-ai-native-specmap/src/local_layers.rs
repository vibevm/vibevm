//! Local boot layers are not shared-project specification authorities.
specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index");
use crate::config::Config;
use anyhow::{Context, Result, bail};
use std::path::{Component, Path, PathBuf};

/// Operation-local private locations, never an inventory of user contents.
pub(crate) struct LocalLayers {
    roots: Vec<(&'static str, PathBuf)>,
    project: PathBuf,
}
impl LocalLayers {
    pub(crate) fn current(project: &Path) -> Self {
        let settings = std::env::var_os("VIBE_SETTINGS")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(|home| PathBuf::from(home).join(".vibe"))
            })
            .or_else(|| {
                cfg!(windows)
                    .then(|| std::env::var_os("USERPROFILE"))
                    .flatten()
                    .filter(|value| !value.is_empty())
                    .map(|home| PathBuf::from(home).join(".vibe"))
            });
        Self::new(project, settings.as_deref())
    }
    fn new(project: &Path, settings: Option<&Path>) -> Self {
        let mut roots = vec![(
            "project boot/local",
            project.join("vibevm/vibespecs/boot/local"),
        )];
        if let Some(settings) = settings.filter(|path| path.is_absolute()) {
            roots.push(("global user project", settings.join("user-project")));
            roots.push(("project override", settings.join("project-overrides")));
        }
        Self {
            roots,
            project: project.into(),
        }
    }
    pub(crate) fn layer(&self, path: &Path) -> Result<Option<&'static str>> {
        let resolved = resolve(path)?;
        let project = resolve(&self.project)?;
        // An ordinary local layer may analyze its own specs, never siblings.
        // Reserved composed boot remains forbidden even within its own root.
        for candidate in [path, resolved.as_path()] {
            let parts: Vec<_> = candidate.components().collect();
            if parts.windows(4).any(|parts| {
                parts
                    .iter()
                    .zip(["vibevm", "vibespecs", "boot", "local"])
                    .all(|(part, name)| part.as_os_str() == name)
            }) {
                return Ok(Some("project boot/local"));
            }
        }
        for (layer, root) in &self.roots {
            let private = resolve(root)?;
            if path.starts_with(root) || resolved.starts_with(&private) {
                if *layer != "project boot/local"
                    && project.starts_with(&private)
                    && resolved.starts_with(&project)
                {
                    continue;
                }
                return Ok(Some(*layer));
            }
        }
        Ok(None)
    }
    pub(crate) fn validate(&self, project: &Path, cfg: &Config) -> Result<()> {
        for (namespace, path) in cfg
            .external_specs
            .iter()
            .map(|row| (row.namespace.as_str(), row.root.as_str()))
            .chain(
                cfg.spec_roots
                    .iter()
                    .chain(&cfg.root_spec_docs)
                    .map(|path| (cfg.namespace.as_str(), path.as_str())),
            )
        {
            if let Some(layer) = self.layer(&project.join(path))? {
                bail!(
                    "spec://{namespace}: local-layer reference targets {layer}; fix: cite the shared project's specifications or an ordinary declared dependency, not personal/override boot"
                );
            }
        }
        Ok(())
    }
}
/// Resolve existing aliases, retaining missing suffix components. Never read spec content.
fn resolve(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => {
                normalized.push(part.as_os_str());
                match std::fs::symlink_metadata(&normalized) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        normalized = normalized
                            .canonicalize()
                            .context("cannot resolve spec-root alias")?;
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error).context("cannot admit spec-root ancestor"),
                }
            }
        }
    }
    let mut existing = normalized.as_path();
    let mut tail = Vec::new();
    loop {
        match std::fs::symlink_metadata(existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tail.push(
                    existing
                        .file_name()
                        .context("unresolvable spec root")?
                        .to_os_string(),
                );
                existing = existing.parent().context("unresolvable spec root")?;
            }
            Err(error) => {
                return Err(error).context("cannot admit spec root without reading private layers");
            }
        }
    }
    let mut resolved = existing
        .canonicalize()
        .context("cannot resolve spec-root aliases; restore the declared source")?;
    for part in tail.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_and_outside_dependency_are_allowed_but_private_layers_are_not() {
        let scope = tempfile::tempdir().unwrap();
        let project = scope.path().join("project");
        let settings = scope.path().join("settings");
        std::fs::create_dir_all(&project).unwrap();
        let guard = LocalLayers::new(&project, Some(&settings));
        assert_eq!(guard.layer(&project.join("specs")).unwrap(), None);
        assert_eq!(
            guard
                .layer(&scope.path().join("outside-dependency/specs"))
                .unwrap(),
            None
        );
        assert_eq!(
            guard.layer(&settings.join("user-project/specs")).unwrap(),
            Some("global user project")
        );
        assert_eq!(
            guard
                .layer(&settings.join("project-overrides/id/specs"))
                .unwrap(),
            Some("project override")
        );
        assert_eq!(
            guard
                .layer(&project.join("vibevm/vibespecs/boot/local/private.md"))
                .unwrap(),
            Some("project boot/local")
        );
    }
    #[cfg(unix)]
    #[test]
    fn alias_into_private_layer_is_not_an_external_dependency() {
        let scope = tempfile::tempdir().unwrap();
        let project = scope.path().join("project");
        let settings = scope.path().join("settings");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(settings.join("user-project/specs")).unwrap();
        std::os::unix::fs::symlink(
            settings.join("user-project"),
            project.join("dependency-alias"),
        )
        .unwrap();
        let guard = LocalLayers::new(&project, Some(&settings));
        assert_eq!(
            guard
                .layer(&project.join("dependency-alias/specs"))
                .unwrap(),
            Some("global user project")
        );
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;
    use crate::config::ExternalSpec;
    #[test]
    fn local_boot_units_never_inflate_project_index_and_write_refuses_private_root() {
        let root = tempfile::tempdir().unwrap();
        let specs = root.path().join("vibevm/vibespecs");
        std::fs::create_dir_all(specs.join("boot/local")).unwrap();
        std::fs::write(specs.join("shared.md"), "# Shared {#SHARED}\n`req r1`\n").unwrap();
        std::fs::write(
            specs.join("boot/local/private.md"),
            "# Private {#PRIVATE}\n`req r1`\n",
        )
        .unwrap();
        let cfg = Config {
            namespace: "shared".into(),
            scan_roots: vec![],
            ..Config::default()
        };
        let map = crate::index::build(root.path(), &cfg);
        assert!(map.specUnits.iter().any(|unit| unit.uri.contains("SHARED")));
        assert!(
            !map.specUnits
                .iter()
                .any(|unit| unit.uri.contains("PRIVATE"))
        );
        let denied = Config {
            external_specs: vec![ExternalSpec {
                namespace: "private-user".into(),
                root: "vibevm/vibespecs/boot/local".into(),
            }],
            ..cfg
        };
        let error = crate::index::write(root.path(), &denied)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("spec://private-user")
                && error.contains("project boot/local")
                && error.contains("fix:"),
            "{error}"
        );
        assert!(!root.path().join("specmap.json").exists());
    }
    #[test]
    fn legitimate_external_dependency_stays_resolution_only() {
        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        std::fs::write(
            external.path().join("law.md"),
            "# Dependency {#DEP}\n`req r1`\n",
        )
        .unwrap();
        let cfg = Config {
            namespace: "shared".into(),
            scan_roots: vec![],
            external_specs: vec![ExternalSpec {
                namespace: "org.dependency/law".into(),
                root: external.path().to_string_lossy().into_owned(),
            }],
            ..Config::default()
        };
        let units = crate::mdspec::scan_external_units(root.path(), &cfg);
        assert!(
            units
                .iter()
                .any(|unit| unit.uri.contains("org.dependency/law") && unit.uri.contains("DEP"))
        );
        assert!(crate::index::build(root.path(), &cfg).specUnits.is_empty());
    }
}

#[cfg(test)]
mod own_layer_tests {
    use super::*;
    #[test]
    fn personal_and_override_projects_can_check_their_own_specs_but_not_siblings() {
        let scope = tempfile::tempdir().unwrap();
        let settings = scope.path().join("settings");
        for project in [
            settings.join("user-project"),
            settings.join("project-overrides/own"),
        ] {
            std::fs::create_dir_all(&project).unwrap();
            let guard = LocalLayers::new(&project, Some(&settings));
            assert_eq!(
                guard
                    .layer(&project.join("vibevm/vibespecs/law.md"))
                    .unwrap(),
                None
            );
            assert_eq!(
                guard
                    .layer(&settings.join("project-overrides/sibling/law.md"))
                    .unwrap(),
                Some("project override")
            );
            assert_eq!(
                guard
                    .layer(&project.join("vibevm/vibespecs/boot/local/composed.md"))
                    .unwrap(),
                Some("project boot/local")
            );
        }
    }
    #[cfg(unix)]
    #[test]
    fn alias_to_nested_nodes_local_boot_is_rejected_after_resolution() {
        let scope = tempfile::tempdir().unwrap();
        let project = scope.path().join("shared");
        let private = scope.path().join("other/vibevm/vibespecs/boot/local");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&private).unwrap();
        std::os::unix::fs::symlink(&private, project.join("external")).unwrap();
        assert_eq!(
            LocalLayers::new(&project, None)
                .layer(&project.join("external/law.md"))
                .unwrap(),
            Some("project boot/local")
        );
    }
}
