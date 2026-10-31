//! The launcher views (`PROP-059#launcher-operations`, `#menu`): `vibe app
//! launchers PACKAGE` for one installation, `--all [--menu]` over every
//! ready, exposed installation of a home, and the `vibe app icon` choice.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{IconDecl, LauncherDecl, LauncherKind, LauncherSource};
use vibe_wire::generated::{
    application_icon as icon_wire, application_launchers as wire,
    application_launchers_all as all_wire,
};

use super::{PROJECT_REQUIRED, TemplateVars, app_root, locate_icon, resolve, unavailable_reason};
use crate::error::LaunchError;
use crate::icons::{ResolvedIcon, wire_shape};
use crate::index::{Index, Installation, Location};
use crate::storage::Storage;

/// The application icon of an installation: package-relative files kept at
/// the same relative path in an owned payload (`PROP-065#icon`).
pub(crate) fn application_icon(
    installation: &Installation,
    root: &Result<PathBuf, String>,
) -> Option<ResolvedIcon> {
    let icon = installation.icon.as_ref()?;
    let root = match (&installation.location, root) {
        (Location::Owned { .. }, Ok(root)) => Some(root.clone()),
        _ => None,
    };
    Some(ResolvedIcon::resolve(icon, |template| {
        root.as_ref()
            .map(|root| root.join(template))
            .ok_or_else(|| "the application icon is kept only in an owned payload".into())
    }))
}

fn launcher_icon(
    icon: &IconDecl,
    root: &Result<PathBuf, String>,
    vars: Option<&TemplateVars>,
) -> ResolvedIcon {
    ResolvedIcon::resolve(icon, |template| match (root, vars) {
        (Ok(_), Some(vars)) => locate_icon(template, vars),
        (Err(reason), _) => Err(reason.clone()),
        (Ok(_), None) => Err("app_root is unavailable".into()),
    })
}

fn kind(value: LauncherKind) -> wire::LauncherKind {
    match value {
        LauncherKind::Gui => wire::LauncherKind::Gui,
        LauncherKind::Tui => wire::LauncherKind::Tui,
        LauncherKind::Command => wire::LauncherKind::Command,
    }
}

fn or_default(own: &[String], fallback: &[String]) -> Vec<String> {
    if own.is_empty() { fallback } else { own }.to_vec()
}

/// The `vibe-app-launchers/1` listing of one installation.
pub fn listing(
    installation: &Installation,
    storage: &Storage,
    vibe_home: &Path,
    project_root: Option<&Path>,
) -> wire::ApplicationLaunchers {
    let root = app_root(installation, storage);
    let vars = root.as_ref().ok().map(|root| TemplateVars {
        app_root: root.clone(),
        vibe_home: vibe_home.to_path_buf(),
        project_root: project_root.map(Path::to_path_buf),
    });
    let launchers = installation
        .launchers
        .iter()
        .map(|launcher| row(installation, launcher, &root, vars.as_ref()))
        .collect();
    let menu = &installation.menu;
    wire::ApplicationLaunchers {
        protocol: "vibe-app-launchers/1".into(),
        application_id: installation.slot.clone(),
        package: wire::PackageIdentity {
            group: installation.package.group.clone(),
            name: installation.package.name.clone(),
            version: installation.package.version.clone(),
        },
        default_launcher_id: installation
            .launchers
            .iter()
            .find(|launcher| launcher.default)
            .map(|launcher| launcher.id.clone()),
        menu_group: menu.menu_group.clone(),
        categories: menu.categories.clone(),
        keywords: menu.keywords.clone(),
        icon_set: application_icon(installation, &root).and_then(|icon| wire_shape(&icon)),
        app_root: root.as_ref().ok().map(|path| path.display().to_string()),
        launchers,
    }
}

fn row(
    installation: &Installation,
    launcher: &LauncherDecl,
    root: &Result<PathBuf, String>,
    vars: Option<&TemplateVars>,
) -> wire::Launcher {
    let outcome = match (root, vars) {
        (Ok(_), Some(vars)) => resolve(launcher, vars),
        (Err(reason), _) => Err(reason.clone()),
        (Ok(_), None) => Err("app_root is unavailable".into()),
    };
    let (status, reason, resolved) = match outcome {
        Ok(resolved) => (wire::LauncherStatus::Available, None, Some(resolved)),
        Err(reason) if reason == PROJECT_REQUIRED => {
            (wire::LauncherStatus::RequiresProject, Some(reason), None)
        }
        Err(reason) => (wire::LauncherStatus::Unavailable, Some(reason), None),
    };
    let text = |path: &PathBuf| path.display().to_string();
    let short_form = match &launcher.icon {
        Some(IconDecl::Path(path)) => Some(path.clone()),
        _ => None,
    };
    let menu = &installation.menu;
    wire::Launcher {
        id: launcher.id.clone(),
        name: launcher.name.clone(),
        description: launcher.description.clone(),
        default: launcher.default,
        terminal: launcher.is_terminal(),
        kind: kind(launcher.effective_kind()),
        menu: launcher.shows_in_menu(),
        source: match launcher.source {
            LauncherSource::Authored => wire::LauncherSource::Authored,
            LauncherSource::DesktopEntry => wire::LauncherSource::DesktopEntry,
        },
        categories: or_default(&launcher.categories, &menu.categories),
        keywords: or_default(&launcher.keywords, &menu.keywords),
        generic_name: launcher.generic_name.clone(),
        names: launcher.names.clone(),
        descriptions: launcher.descriptions.clone(),
        window_class: launcher.window_class.clone(),
        mime_types: launcher.mime_types.clone(),
        actions: launcher
            .actions
            .iter()
            .map(|action| wire::LauncherAction {
                id: action.id.clone(),
                name: action.name.clone(),
                args: wire::ArgsValue {
                    template: action.args.clone(),
                    resolved: vars.and_then(|vars| {
                        action
                            .args
                            .iter()
                            .map(|arg| super::expand_template(arg, vars))
                            .collect::<Result<Vec<_>, _>>()
                            .ok()
                    }),
                },
                icon_set: action
                    .icon
                    .as_ref()
                    .and_then(|icon| wire_shape(&launcher_icon(icon, root, vars))),
            })
            .collect(),
        icon_set: launcher
            .icon
            .as_ref()
            .and_then(|icon| wire_shape(&launcher_icon(icon, root, vars))),
        executable: wire::PathValue {
            template: Some(launcher.executable.clone()),
            base: "app_root".into(),
            resolved: resolved.as_ref().map(|value| text(&value.executable)),
        },
        args: wire::ArgsValue {
            template: launcher.args.clone(),
            resolved: resolved.as_ref().map(|value| value.args.clone()),
        },
        working_directory: wire::PathValue {
            template: launcher.working_directory.clone(),
            base: "app_root".into(),
            resolved: resolved
                .as_ref()
                .and_then(|value| value.working_directory.as_ref().map(text)),
        },
        icon: wire::IconValue {
            template: short_form,
            resolved: resolved
                .as_ref()
                .and_then(|value| value.icon.as_ref().map(text)),
            diagnostic: resolved.and_then(|value| value.icon_diagnostic),
        },
        status,
        reason,
    }
}

/// `vibe app launchers --all [--menu]`: every ready, exposed installation
/// of the home in one answer, with only the `menu = true` launchers (and
/// only installations that have one) when `menu_only`.
pub fn listing_all(
    index: &Index,
    storage: &Storage,
    vibe_home: &Path,
    project_root: Option<&Path>,
    menu_only: bool,
) -> all_wire::ApplicationLaunchersAll {
    let installations = index
        .installations
        .iter()
        .filter(|(_, record)| !record.lifecycle.hidden)
        .filter(|(_, record)| unavailable_reason(&app_root(record, storage)).is_none())
        .filter_map(|(key, record)| {
            let mut document = listing(record, storage, vibe_home, project_root);
            if menu_only {
                document.launchers.retain(|launcher| launcher.menu);
                if document.launchers.is_empty() {
                    return None;
                }
            }
            let mut value = serde_json::to_value(&document).ok()?;
            let object = value.as_object_mut()?;
            object.remove("protocol");
            object.insert("key".into(), key.clone().into());
            object.insert(
                "default".into(),
                (index.default_of(&record.slot) == Some(key.as_str())).into(),
            );
            serde_json::from_value(value).ok()
        })
        .collect();
    all_wire::ApplicationLaunchersAll {
        protocol: "vibe-app-launchers-all/1".into(),
        menu_only,
        installations,
    }
}

/// `vibe app icon`: the best existing file of the launcher's icon set, or of
/// the application's when the launcher has none; without a launcher the
/// application's, then the default launcher's (`PROP-059#menu`).
pub fn icon_choice(
    installation: &Installation,
    storage: &Storage,
    vibe_home: &Path,
    launcher_id: Option<&str>,
    size: u32,
    prefer: Option<&str>,
) -> Result<icon_wire::ApplicationIcon, LaunchError> {
    let root = app_root(installation, storage);
    let vars = root.as_ref().ok().map(|root| TemplateVars {
        app_root: root.clone(),
        vibe_home: vibe_home.to_path_buf(),
        project_root: None,
    });
    let launcher = match launcher_id {
        Some(id) => Some(
            installation
                .launchers
                .iter()
                .find(|launcher| launcher.id == id)
                .ok_or_else(|| LaunchError::UnknownLauncher { id: id.into() })?,
        ),
        None => None,
    };
    let own = launcher
        .and_then(|launcher| launcher.icon.as_ref())
        .map(|icon| launcher_icon(icon, &root, vars.as_ref()));
    let application = application_icon(installation, &root);
    let default = installation
        .launchers
        .iter()
        .find(|launcher| launcher.default)
        .and_then(|launcher| launcher.icon.as_ref())
        .map(|icon| launcher_icon(icon, &root, vars.as_ref()));
    let candidates: Vec<ResolvedIcon> = match launcher {
        Some(_) => own.into_iter().chain(application).collect(),
        None => application.into_iter().chain(default).collect(),
    };
    let chosen = candidates
        .iter()
        .find_map(|icon| icon.choose(size, prefer))
        .ok_or_else(|| LaunchError::NoIcon {
            reason: format!(
                "no icon file of {} exists for `{}`",
                launcher_id.map_or("the application".to_owned(), |id| format!(
                    "launcher `{id}`"
                )),
                installation.package.key()
            ),
        })?;
    let slot = |name: &str| match name {
        "svg" => Some(icon_wire::IconSlot::Svg),
        "png" => Some(icon_wire::IconSlot::Png),
        "ico" => Some(icon_wire::IconSlot::Ico),
        "icns" => Some(icon_wire::IconSlot::Icns),
        "symbolic" => Some(icon_wire::IconSlot::Symbolic),
        _ => None,
    };
    Ok(icon_wire::ApplicationIcon {
        protocol: "vibe-app-icon/1".into(),
        package: icon_wire::PackageIdentity {
            group: installation.package.group.clone(),
            name: installation.package.name.clone(),
            version: installation.package.version.clone(),
        },
        launcher: launcher_id.map(str::to_owned),
        requested_size: size,
        prefer: prefer.and_then(slot),
        slot: slot(chosen.slot).unwrap_or(icon_wire::IconSlot::Png),
        size: chosen.size,
        path: chosen.path.clone().unwrap_or_default(),
    })
}
