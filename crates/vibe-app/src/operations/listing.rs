//! The installed-record lookup and the `app list` inventory with hidden
//! installations, marks, requirements and environment entries
//! (`PROP-059#hidden-installations`, `PROP-065#uninstall-tree`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#hidden-installations");

use std::path::Path;

use vibe_core::manifest::{IconDecl, LauncherKind, LauncherSource, RequirementScope};
use vibe_wire::generated::application_list as wire;

use crate::catalog::LauncherCatalog;
use crate::error::AppError;
use crate::icons::wire_shape;
use crate::index::{EnvOp, Index, Installation, Location, Mark};
use crate::launch;
use crate::resolve::qualified_ref;
use crate::settings::SettingsRoot;
use crate::storage::Storage;

/// Find the one installed record a coordinate names in the index of `home`:
/// an exact match, or the slot default among several matching versions; a
/// miss names `home` (`PROP-059#project-scope`).
pub fn installed<'a>(
    home: &SettingsRoot,
    index: &'a Index,
    spelling: &str,
) -> Result<(&'a str, &'a Installation), AppError> {
    let requested = qualified_ref(spelling)?;
    let group = requested
        .group
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let candidates: Vec<(&String, &Installation)> = index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.package.group == group
                && record.package.name == requested.name.as_str()
                && semver::Version::parse(&record.package.version)
                    .is_ok_and(|version| requested.version.matches(&version))
        })
        .collect();
    // `pkg@1.0.0` is a caret requirement to the package grammar, but an
    // installed version spelled out literally is what the user means.
    let literal = spelling
        .rsplit_once('@')
        .map(|(_, version)| version.trim_start_matches('='));
    if let Some((key, record)) = candidates
        .iter()
        .find(|(_, record)| Some(record.package.version.as_str()) == literal)
    {
        return Ok((key.as_str(), record));
    }
    let defaults: Vec<&(&String, &Installation)> = candidates
        .iter()
        .filter(|(key, record)| index.default_of(&record.slot) == Some(key.as_str()))
        .collect();
    // A kept previous installation never answers an unversioned name when
    // its successor is installed (`PROP-065#update-and-rollback`).
    let current: Vec<&(&String, &Installation)> = candidates
        .iter()
        .filter(|(_, record)| record.lifecycle.superseded_by.is_none())
        .collect();
    match (
        candidates.as_slice(),
        defaults.as_slice(),
        current.as_slice(),
    ) {
        ([], _, _) => Err(AppError::not_installed(home, spelling)),
        ([(key, record)], _, _) | (_, [(key, record)], _) | (_, _, [(key, record)]) => {
            Ok((key.as_str(), record))
        }
        _ => Err(AppError::AmbiguousInstallation {
            package: spelling.into(),
        }),
    }
}

/// Every installation with its status, whether it is its slot's default,
/// whether it is hidden, its mark, what it requires and what requires it,
/// and the environment entries it applied.
pub fn list(settings: &SettingsRoot) -> Result<wire::ApplicationList, AppError> {
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let catalog = LauncherCatalog::load(settings)?;
    let installations = index
        .installations
        .iter()
        .map(|(key, record)| row(&index, &storage, &catalog, key, record))
        .collect();
    Ok(wire::ApplicationList {
        protocol: "vibe-app-list/1".into(),
        installations,
    })
}

fn row(
    index: &Index,
    storage: &Storage,
    catalog: &LauncherCatalog,
    key: &str,
    record: &Installation,
) -> wire::Installation {
    let root = launch::app_root(record, storage);
    let reason = launch::unavailable_reason(&root);
    let places = launch::IconPlaces::new(catalog.home(), None, record, &root);
    let lifecycle = &record.lifecycle;
    wire::Installation {
        source_hosting: vibe_registry::source_hosting(record.source_evidence.as_ref()),
        source_evidence: record.source_evidence.clone(),
        key: key.to_owned(),
        slot: record.slot.clone(),
        package: wire::Package {
            group: record.package.group.clone(),
            name: record.package.name.clone(),
            version: record.package.version.clone(),
        },
        method: record.method.as_str().into(),
        status: if reason.is_none() {
            wire::Status::Ready
        } else {
            wire::Status::Unavailable
        },
        reason,
        library: match &record.location {
            Location::Owned { library, .. } => Some(library.clone()),
            Location::External { .. } => None,
        },
        icon: match &record.icon {
            Some(IconDecl::Path(icon)) if !icon.is_empty() => places.locate(icon).ok(),
            _ => None,
        }
        .map(|path| path_text(&path)),
        icon_set: launch::application_icon(record, &places).and_then(|icon| wire_shape(&icon)),
        menu_group: record.menu.menu_group.clone(),
        categories: record.menu.categories.clone(),
        keywords: record.menu.keywords.clone(),
        launchers: launcher_rows(record, storage, catalog),
        app_root: root.ok().map(|root| path_text(&root)),
        portable: record.portable,
        default: index.default_of(&record.slot) == Some(key),
        slot_off: index.is_off(&record.slot),
        path_dirs: record.path_dirs.clone(),
        hidden: lifecycle.hidden,
        mark: (lifecycle.mark == Mark::Auto).then_some(wire::Mark::Auto),
        held: lifecycle.held,
        superseded_by: lifecycle.superseded_by.clone(),
        requires: lifecycle
            .requires
            .iter()
            .map(|requirement| wire::Requirement {
                package: requirement.package.clone(),
                key: requirement.key.clone(),
                scope: scope(requirement.scope),
                step: requirement.step.clone(),
            })
            .collect(),
        required_by: index
            .required_by(key)
            .into_iter()
            .map(|edge| edge.owner.to_owned())
            .collect(),
        env: lifecycle
            .env
            .iter()
            .map(|entry| wire::EnvEntry {
                step: entry.step.clone(),
                op: match entry.op {
                    EnvOp::Set => wire::EnvOp::Set,
                    EnvOp::Unset => wire::EnvOp::Unset,
                    EnvOp::PathPrepend => wire::EnvOp::PathPrepend,
                },
                name: entry.name.clone(),
                value: entry.value.clone(),
            })
            .collect(),
    }
}

/// A summary of each launch point with its effective categories and
/// keywords and its icon set resolved under the payload (`PROP-059#menu`).
fn launcher_rows(
    record: &Installation,
    storage: &Storage,
    catalog: &LauncherCatalog,
) -> Vec<wire::Launcher> {
    let full = launch::listing(record, storage, catalog, None);
    record
        .launchers
        .iter()
        .zip(full.launchers)
        .map(|(launcher, row)| wire::Launcher {
            id: launcher.id.clone(),
            name: launcher.name.clone(),
            kind: match launcher.effective_kind() {
                LauncherKind::Gui => wire::LauncherKind::Gui,
                LauncherKind::Tui => wire::LauncherKind::Tui,
                LauncherKind::Command => wire::LauncherKind::Command,
            },
            menu: launcher.shows_in_menu(),
            default: launcher.default,
            source: match launcher.source {
                LauncherSource::Authored => wire::LauncherSource::Authored,
                LauncherSource::DesktopEntry => wire::LauncherSource::DesktopEntry,
            },
            categories: row.categories,
            keywords: row.keywords,
            generic_name: row.generic_name,
            names: row.names,
            descriptions: row.descriptions,
            window_class: row.window_class,
            mime_types: row.mime_types,
            actions: row
                .actions
                .into_iter()
                .map(|action| wire::LauncherAction {
                    id: action.id,
                    name: action.name,
                    args: action.args.template,
                    icon_set: action.icon_set.and_then(|icon| wire_shape(&icon)),
                })
                .collect(),
            icon_set: row.icon_set.and_then(|icon| wire_shape(&icon)),
        })
        .collect()
}

fn scope(value: RequirementScope) -> wire::Scope {
    match value {
        RequirementScope::Build => wire::Scope::Build,
        RequirementScope::Runtime => wire::Scope::Runtime,
    }
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
