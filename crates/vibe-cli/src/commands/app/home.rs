//! Which application home a command acts on (PROP-059 `#project-scope`):
//! `-g` selects the user settings root; otherwise a directory holding
//! `vibe.toml` selects its project home `vibevm/vibeopt`, and anything else
//! the user settings root. The CLI decides here and threads the chosen
//! [`SettingsRoot`] into `vibe-app`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#project-scope");

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use vibe_app::SettingsRoot;
use vibe_core::manifest::Manifest;

use super::global;

/// The home one command acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AppHome {
    User(SettingsRoot),
    Project {
        root: PathBuf,
        settings: SettingsRoot,
    },
}

impl AppHome {
    pub(crate) fn settings(&self) -> &SettingsRoot {
        match self {
            Self::User(settings) | Self::Project { settings, .. } => settings,
        }
    }
}

/// The project a directory names: the directory itself when it holds
/// `vibe.toml`, canonicalised the way every project command resolves it.
pub(crate) fn project_at(dir: &Path) -> Result<Option<PathBuf>> {
    if !dir.join(Manifest::FILENAME).is_file() {
        return Ok(None);
    }
    crate::commands::resolve_project_root(dir).map(Some)
}

/// The project named by `--path` (which must hold `vibe.toml`), else by the
/// working directory when it holds one.
pub(crate) fn selected_project(explicit: Option<&Path>) -> Result<Option<PathBuf>> {
    match explicit {
        Some(path) => crate::commands::resolve_project_root(path).map(Some),
        None => project_at(&std::env::current_dir()?),
    }
}

/// The home for `global` and the selected project.
pub(crate) fn select(global_scope: bool, project: Option<&Path>) -> Result<AppHome> {
    Ok(choose(global_scope, project, global::settings_root()?))
}

fn choose(global_scope: bool, project: Option<&Path>, user: SettingsRoot) -> AppHome {
    match project {
        Some(root) if !global_scope => AppHome::Project {
            settings: SettingsRoot::project(root, &user),
            root: root.to_path_buf(),
        },
        _ => AppHome::User(user),
    }
}

/// The `opt/bin` directories `vibe shell` puts first on PATH, in priority
/// order: the project home's when it exists and the scope is not `-g`, then
/// the user home's.
pub(crate) fn shell_bins(
    global_scope: bool,
    project: Option<&Path>,
    user: &SettingsRoot,
) -> Vec<PathBuf> {
    let mut bins = Vec::new();
    if let AppHome::Project { settings, .. } = choose(global_scope, project, user.clone())
        && settings.path().is_dir()
    {
        bins.push(settings.proxy_dir());
    }
    bins.push(user.proxy_dir());
    bins
}

/// `--library` names a storage library of the user home; a project home has
/// only its default library.
pub(crate) fn refuse_library(library: Option<&str>) -> Result<()> {
    if let Some(id) = library {
        bail!(
            "--library `{id}` selects a storage library of the user home, but this command acts \
             on the project home `vibevm/vibeopt`, which has only its default library \
             (violates spec://org.vibevm.core/vibevm/common/PROP-059#project-scope; fix: drop \
             --library, or add -g to install into the user home)"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_scope_or_no_project_selects_the_user_home() {
        let dir = tempfile::tempdir().unwrap();
        let user = SettingsRoot::new(&dir.path().join("settings"));
        let project = dir.path().join("project");
        assert_eq!(
            choose(true, Some(&project), user.clone()),
            AppHome::User(user.clone())
        );
        assert_eq!(
            choose(false, None, user.clone()),
            AppHome::User(user.clone())
        );
        let AppHome::Project { root, settings } = choose(false, Some(&project), user.clone())
        else {
            panic!("a project without -g selects its project home");
        };
        assert_eq!(root, project);
        assert_eq!(settings.path(), project.join("vibevm").join("vibeopt"));
        assert_eq!(settings.app_sources_cache(), user.app_sources_cache());
    }

    #[test]
    fn project_at_requires_a_manifest_in_the_directory_itself() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(project_at(dir.path()).unwrap(), None);
        std::fs::write(dir.path().join(Manifest::FILENAME), "").unwrap();
        let found = project_at(dir.path()).unwrap().unwrap();
        assert_eq!(found, dir.path().canonicalize().unwrap());
        let nested = dir.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        assert_eq!(project_at(&nested).unwrap(), None);
    }

    #[test]
    fn shell_puts_an_existing_project_home_before_the_user_home() {
        let dir = tempfile::tempdir().unwrap();
        let user = SettingsRoot::new(&dir.path().join("settings"));
        let project = dir.path().join("project");
        assert_eq!(
            shell_bins(false, Some(&project), &user),
            vec![user.proxy_dir()],
            "a project home that does not exist yet is skipped"
        );
        let home = SettingsRoot::project(&project, &user);
        std::fs::create_dir_all(home.path()).unwrap();
        assert_eq!(
            shell_bins(false, Some(&project), &user),
            vec![home.proxy_dir(), user.proxy_dir()]
        );
        assert_eq!(
            shell_bins(true, Some(&project), &user),
            vec![user.proxy_dir()]
        );
        assert_eq!(shell_bins(false, None, &user), vec![user.proxy_dir()]);
    }

    #[test]
    fn library_is_refused_for_a_project_home() {
        assert!(refuse_library(None).is_ok());
        let message = refuse_library(Some("usb")).unwrap_err().to_string();
        assert!(message.contains("#project-scope"), "{message}");
    }
}
