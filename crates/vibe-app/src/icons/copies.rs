//! The home's copies of package-relative icon files (`PROP-059#menu`). A
//! package-relative icon names a file of the package tree, which is not part
//! of a payload: an installation keeps its files under
//! `<home>/applications/icons/<dir>/` at the same relative path, refreshed
//! by every install, update and modify of its key and removed with it. A
//! storage move leaves them, because they belong to the home.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::StateError;
use crate::index::{Installation, PackageCoordinate};
use crate::resolve::strip_verbatim;
use crate::settings::{SettingsRoot, io};

/// Whether an icon template names a file of the package tree: relative,
/// without a template variable and without `..`.
pub(crate) fn is_package_relative(template: &str) -> bool {
    !template.contains('$') && join_relative(Path::new(""), template).is_some()
}

/// `base` joined with a relative template one normal component at a time;
/// `None` for an empty, absolute or `..`-traversing path.
pub(crate) fn join_relative(base: &Path, template: &str) -> Option<PathBuf> {
    let mut path = base.to_path_buf();
    let mut named = false;
    for part in Path::new(template).components() {
        match part {
            Component::Normal(name) => {
                path.push(name);
                named = true;
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    named.then_some(path)
}

/// `<home>/applications/icons/<dir>`, the copies of one installation.
pub(crate) fn copies_dir(home: &SettingsRoot, package: &PackageCoordinate) -> PathBuf {
    home.app_icons_dir().join(package.directory_name())
}

/// Every package-relative icon file of the application and of the recorded
/// launchers and their actions, each once.
fn package_relative_files(installation: &Installation) -> BTreeSet<&str> {
    let launchers = installation.launchers.iter().flat_map(|launcher| {
        launcher.icon.iter().chain(
            launcher
                .actions
                .iter()
                .filter_map(|action| action.icon.as_ref()),
        )
    });
    installation
        .icon
        .iter()
        .chain(launchers)
        .flat_map(|icon| icon.files())
        .map(|(_, template)| template)
        .filter(|template| is_package_relative(template))
        .collect()
}

/// Replace the copies of `installation` in `home` with its package-relative
/// icon files found in the package tree at `package_root`. A file that is
/// missing, or that a link resolves outside the package tree, is not copied
/// (the listings report it missing); the copies are written beside the old
/// ones and renamed over them.
pub(crate) fn copy_package_icons(
    home: &SettingsRoot,
    installation: &Installation,
    package_root: &Path,
) -> Result<(), StateError> {
    let target = copies_dir(home, &installation.package);
    let staging = home.app_icons_dir().join(format!(
        ".staging-{}-{}",
        installation.package.directory_name(),
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&staging);
    let copied = stage(installation, package_root, &staging).inspect_err(|_| {
        let _ = fs::remove_dir_all(&staging);
    })?;
    match fs::remove_dir_all(&target) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(io("removing", &target, source));
        }
    }
    if copied {
        fs::rename(&staging, &target).map_err(|source| io("placing", &target, source))?;
    }
    Ok(())
}

/// Copy the files into `staging`; whether any file was copied.
fn stage(
    installation: &Installation,
    package_root: &Path,
    staging: &Path,
) -> Result<bool, StateError> {
    let Ok(root) = fs::canonicalize(package_root).map(strip_verbatim) else {
        return Ok(false);
    };
    let mut copied = false;
    for template in package_relative_files(installation) {
        let (Some(source), Some(copy)) = (
            join_relative(&root, template),
            join_relative(staging, template),
        ) else {
            continue;
        };
        let Ok(real) = fs::canonicalize(&source).map(strip_verbatim) else {
            continue;
        };
        if !real.starts_with(&root) || !real.is_file() {
            continue;
        }
        if let Some(parent) = copy.parent() {
            fs::create_dir_all(parent).map_err(|source| io("creating", parent, source))?;
        }
        fs::copy(&real, &copy).map_err(|source| io("copying", &copy, source))?;
        copied = true;
    }
    Ok(copied)
}

/// Remove the copies of `package` from `home`; nothing to remove is fine.
pub(crate) fn remove_package_icons(home: &SettingsRoot, package: &PackageCoordinate) {
    let _ = fs::remove_dir_all(copies_dir(home, package));
}

#[cfg(test)]
mod tests {
    use vibe_core::manifest::{IconDecl, LauncherDecl};

    use super::*;
    use crate::index::{ApplicationMenu, AuthoredCommands, Lifecycle, Location, Method};

    fn installation(icon: Option<IconDecl>, launcher_icon: Option<IconDecl>) -> Installation {
        Installation {
            secret_origins: Default::default(),
            configuration: None,
            pending: None,
            slot: "demo".into(),
            package: PackageCoordinate {
                group: "org.example".into(),
                name: "demo".into(),
                version: "1.0.0".into(),
            },
            option: "main".into(),
            platform: "linux".into(),
            arch: "x86_64".into(),
            method: Method::Steps,
            location: Location::External {
                app_root: PathBuf::from("/nowhere"),
            },
            portable: false,
            launchers: vec![LauncherDecl {
                id: "main".into(),
                icon: launcher_icon,
                ..LauncherDecl::default()
            }],
            public_commands: Vec::new(),
            commands: AuthoredCommands::default(),
            features: Vec::new(),
            parameters: Default::default(),
            license: None,
            icon,
            menu: ApplicationMenu::default(),
            steps: Vec::new(),
            lifecycle: Lifecycle::default(),
        }
    }

    #[test]
    fn package_relative_means_no_variable_and_no_escape() {
        assert!(is_package_relative("icons/app.svg"));
        assert!(is_package_relative("./app.png"));
        assert!(!is_package_relative("$app_root/share/app.svg"));
        assert!(!is_package_relative("../app.svg"));
        assert!(!is_package_relative("/abs/app.svg"));
        assert!(!is_package_relative(""));
    }

    /// The application's and the launcher's package-relative files are
    /// copied keeping their paths; a payload template and a missing file are
    /// not; a refresh replaces the set and removal deletes it.
    #[test]
    fn copies_keep_relative_paths_refresh_and_go() {
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        fs::create_dir_all(package.join("icons")).unwrap();
        fs::write(package.join("icons/app.svg"), "<svg/>").unwrap();
        fs::write(package.join("tool.svg"), "<svg/>").unwrap();
        let home = SettingsRoot::new(&dir.path().join("home"));
        let record = installation(
            Some(IconDecl::Path("icons/app.svg".into())),
            Some(IconDecl::Path("tool.svg".into())),
        );
        copy_package_icons(&home, &record, &package).unwrap();
        let copies = copies_dir(&home, &record.package);
        assert!(copies.ends_with("applications/icons/org.example.demo@1.0.0"));
        assert!(copies.join("icons/app.svg").is_file());
        assert!(copies.join("tool.svg").is_file());

        let refreshed = installation(
            Some(IconDecl::Path("icons/app.svg".into())),
            Some(IconDecl::Path("$app_root/tool.svg".into())),
        );
        copy_package_icons(&home, &refreshed, &package).unwrap();
        assert!(copies.join("icons/app.svg").is_file());
        assert!(!copies.join("tool.svg").exists(), "a refresh drops it");

        let missing = installation(Some(IconDecl::Path("absent.svg".into())), None);
        copy_package_icons(&home, &missing, &package).unwrap();
        assert!(!copies.exists(), "nothing to copy leaves no directory");

        copy_package_icons(&home, &record, &package).unwrap();
        remove_package_icons(&home, &record.package);
        assert!(!copies.exists());
        let leftovers: Vec<_> = fs::read_dir(home.app_icons_dir()).unwrap().collect();
        assert!(leftovers.is_empty(), "no staging directory stays");
    }
}
