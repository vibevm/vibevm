//! The launcher views (`PROP-059#launcher-operations`, `#menu`,
//! `#launcher-catalog`): `vibe app launchers PACKAGE` for one installation,
//! `--all [--menu]` — the catalog a graphical client draws a desktop from —
//! over every ready, exposed installation of a home, and the `vibe app icon`
//! choice.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{IconDecl, LauncherDecl, LauncherKind, LauncherSource, launcher_command};
use vibe_wire::generated::{
    application_icon as icon_wire, application_launchers as wire,
    application_launchers_all as all_wire,
};

use super::{PROJECT_REQUIRED, TemplateVars, app_root, locate_icon, resolve, unavailable_reason};
use crate::catalog::LauncherCatalog;
use crate::error::LaunchError;
use crate::icons::{ResolvedIcon, copies_dir, is_package_relative, join_relative, wire_shape};
use crate::index::{Index, Installation};
use crate::settings::SettingsRoot;
use crate::storage::Storage;

/// Where the icon files of one installation are found (`PROP-059#menu`):
/// a package-relative file among the home's copies, a `$`-templated one in
/// the payload under `app_root`.
pub(crate) struct IconPlaces {
    copies: PathBuf,
    vars: Result<TemplateVars, String>,
}

impl IconPlaces {
    /// The places of `installation` in the home `vibe_home` with its
    /// derived `root`.
    pub(crate) fn new(
        vibe_home: &Path,
        project_root: Option<&Path>,
        installation: &Installation,
        root: &Result<PathBuf, String>,
    ) -> Self {
        Self {
            copies: copies_dir(&SettingsRoot::new(vibe_home), &installation.package),
            vars: root.clone().map(|root| TemplateVars {
                app_root: root,
                vibe_home: vibe_home.to_path_buf(),
                project_root: project_root.map(Path::to_path_buf),
            }),
        }
    }

    fn in_payload(&self, template: &str) -> Result<PathBuf, String> {
        match &self.vars {
            Ok(vars) => locate_icon(template, vars),
            Err(reason) => Err(reason.clone()),
        }
    }

    /// One icon file as an existing absolute path, or why there is none.
    pub(crate) fn locate(&self, template: &str) -> Result<PathBuf, String> {
        if !is_package_relative(template) {
            return self.in_payload(template);
        }
        let copy = join_relative(&self.copies, template).unwrap_or_else(|| self.copies.clone());
        if copy.is_file() {
            return Ok(copy);
        }
        // A record installed before the copies existed may keep the file at
        // the same relative path in its payload.
        self.in_payload(template)
            .map_err(|_| format!("icon `{}` does not exist", copy.display()))
    }

    /// Every file of an icon set located.
    pub(crate) fn resolve(&self, icon: &IconDecl) -> ResolvedIcon {
        ResolvedIcon::resolve(icon, |template| self.locate(template))
    }
}

/// The application icon of an installation (`PROP-065#icon`).
pub(crate) fn application_icon(
    installation: &Installation,
    places: &IconPlaces,
) -> Option<ResolvedIcon> {
    installation.icon.as_ref().map(|icon| places.resolve(icon))
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

/// The `vibe-app-launchers/2` listing of one installation. `catalog` carries
/// the application home — `$vibe_home` of the templates, and the home whose
/// `applications/icons` keeps the package-relative icon files — together
/// with the user layer and the usage facts every row quotes
/// (`PROP-059#launcher-catalog`).
pub fn listing(
    installation: &Installation,
    storage: &Storage,
    catalog: &LauncherCatalog,
    project_root: Option<&Path>,
) -> wire::ApplicationLaunchers {
    let vibe_home = catalog.home();
    let root = app_root(installation, storage);
    let vars = root.as_ref().ok().map(|root| TemplateVars {
        app_root: root.clone(),
        vibe_home: vibe_home.to_path_buf(),
        project_root: project_root.map(Path::to_path_buf),
    });
    let places = IconPlaces::new(vibe_home, project_root, installation, &root);
    let launchers = installation
        .launchers
        .iter()
        .map(|launcher| Row {
            installation,
            launcher,
            root: &root,
            vars: vars.as_ref(),
            places: &places,
            catalog,
        })
        .map(row)
        .collect();
    let menu = &installation.menu;
    wire::ApplicationLaunchers {
        protocol: "vibe-app-launchers/2".into(),
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
        icon_set: application_icon(installation, &places).and_then(|icon| wire_shape(&icon)),
        app_root: root.as_ref().ok().map(|path| path.display().to_string()),
        state: LauncherCatalog::state(installation),
        launchers,
    }
}

/// One row's inputs: the installation it belongs to, its declaration, the
/// derived `app_root`, the template values, the icon places and the catalog
/// state the user layer and the usage facts come from.
struct Row<'a> {
    installation: &'a Installation,
    launcher: &'a LauncherDecl,
    root: &'a Result<PathBuf, String>,
    vars: Option<&'a TemplateVars>,
    places: &'a IconPlaces,
    catalog: &'a LauncherCatalog,
}

fn row(
    Row {
        installation,
        launcher,
        root,
        vars,
        places,
        catalog,
    }: Row<'_>,
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
    let located = short_form
        .as_deref()
        .filter(|template| !template.is_empty())
        .map(|template| places.locate(template));
    let resolved_icon = located
        .as_ref()
        .and_then(|value| value.as_ref().ok().map(text));
    let fields = catalog.fields(
        &installation.package.key(),
        launcher,
        resolved_icon.as_deref(),
    );
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
                    .and_then(|icon| wire_shape(&places.resolve(icon))),
            })
            .collect(),
        icon_set: launcher
            .icon
            .as_ref()
            .and_then(|icon| wire_shape(&places.resolve(icon))),
        executable: wire::PathValue {
            template: Some(launcher.executable.clone()),
            // A `command:NAME` executable is resolved on PATH, not under the
            // application root (`PROP-065#launcher-command`).
            base: if launcher_command(&launcher.executable).is_some() {
                "path".into()
            } else {
                "app_root".into()
            },
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
            resolved: resolved_icon.clone(),
            diagnostic: located.and_then(Result::err),
        },
        status,
        reason,
        key: fields.key,
        collections: fields.collections,
        pinned: fields.pinned,
        hidden_by: fields.hidden_by,
        display_name: fields.display_name,
        display_icon: fields.display_icon,
        usage: fields.usage,
    }
}

/// `vibe app launchers --all [--menu]`: the catalog — every ready, exposed
/// installation of the home in one answer, with only the `menu = true`
/// launchers (and only installations that have one) when `menu_only`, and
/// the home and the revision a client compares instead of refetching it
/// (`PROP-059#launcher-catalog`).
pub fn listing_all(
    index: &Index,
    storage: &Storage,
    catalog: &LauncherCatalog,
    project_root: Option<&Path>,
    menu_only: bool,
) -> all_wire::ApplicationLaunchersAll {
    let installations = index
        .installations
        .iter()
        .filter(|(_, record)| !record.lifecycle.hidden)
        .filter(|(_, record)| unavailable_reason(&app_root(record, storage)).is_none())
        .filter_map(|(key, record)| {
            let mut document = listing(record, storage, catalog, project_root);
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
        protocol: "vibe-app-launchers-all/2".into(),
        menu_only,
        home: catalog.home().display().to_string(),
        revision: catalog.revision(),
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
    let places = IconPlaces::new(vibe_home, None, installation, &root);
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
        .map(|icon| places.resolve(icon));
    let application = application_icon(installation, &places);
    let default = installation
        .launchers
        .iter()
        .find(|launcher| launcher.default)
        .and_then(|launcher| launcher.icon.as_ref())
        .map(|icon| places.resolve(icon));
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
