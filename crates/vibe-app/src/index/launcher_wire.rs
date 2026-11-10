//! Launcher rows and icon sets between the index domain model and the
//! `vibe-user-applications/3` wire form (`PROP-059#menu`,
//! `#desktop-entries`). A record written before kinds existed has no
//! `kind`: it reads as `tui` when `terminal`, otherwise `gui`, with
//! `menu = false`.

use vibe_core::manifest::{
    IconDecl, IconSet, LauncherActionDecl, LauncherDecl, LauncherKind, LauncherSource,
};
use vibe_wire::generated::application::e6::index as wire;

pub(super) fn launcher_to_wire(launcher: &LauncherDecl) -> wire::Launcher {
    let (icon, icon_set) = icon_to_wire(launcher.icon.as_ref());
    wire::Launcher {
        id: launcher.id.clone(),
        name: launcher.name.clone(),
        description: launcher.description.clone(),
        executable: launcher.executable.clone(),
        args: launcher.args.clone(),
        working_directory: launcher.working_directory.clone(),
        icon,
        icon_set,
        terminal: launcher.is_terminal(),
        default: launcher.default,
        kind: Some(match launcher.effective_kind() {
            LauncherKind::Gui => wire::LauncherKind::Gui,
            LauncherKind::Tui => wire::LauncherKind::Tui,
            LauncherKind::Command => wire::LauncherKind::Command,
        }),
        menu: launcher.shows_in_menu(),
        categories: launcher.categories.clone(),
        keywords: launcher.keywords.clone(),
        generic_name: launcher.generic_name.clone(),
        window_class: launcher.window_class.clone(),
        mime_types: launcher.mime_types.clone(),
        names: launcher.names.clone(),
        descriptions: launcher.descriptions.clone(),
        actions: launcher
            .actions
            .iter()
            .map(|action| {
                let (icon, icon_set) = icon_to_wire(action.icon.as_ref());
                wire::LauncherAction {
                    id: action.id.clone(),
                    name: action.name.clone(),
                    args: action.args.clone(),
                    icon,
                    icon_set,
                }
            })
            .collect(),
        source: (launcher.source == LauncherSource::DesktopEntry)
            .then_some(wire::LauncherSource::DesktopEntry),
    }
}

pub(super) fn launcher_from_wire(launcher: wire::Launcher) -> LauncherDecl {
    let kind = match launcher.kind {
        Some(wire::LauncherKind::Gui) => LauncherKind::Gui,
        Some(wire::LauncherKind::Tui) => LauncherKind::Tui,
        Some(wire::LauncherKind::Command) => LauncherKind::Command,
        None => LauncherKind::from_terminal(launcher.terminal),
    };
    LauncherDecl {
        id: launcher.id,
        name: launcher.name,
        description: launcher.description,
        executable: launcher.executable,
        kind: Some(kind),
        args: launcher.args,
        working_directory: launcher.working_directory,
        terminal: Some(launcher.terminal),
        default: launcher.default,
        menu: Some(launcher.menu),
        categories: launcher.categories,
        keywords: launcher.keywords,
        generic_name: launcher.generic_name,
        window_class: launcher.window_class,
        mime_types: launcher.mime_types,
        names: launcher.names,
        descriptions: launcher.descriptions,
        icon: icon_from_wire(launcher.icon, launcher.icon_set),
        actions: launcher
            .actions
            .into_iter()
            .map(|action| LauncherActionDecl {
                id: action.id,
                name: action.name,
                args: action.args,
                icon: icon_from_wire(action.icon, action.icon_set),
            })
            .collect(),
        source: match launcher.source {
            Some(wire::LauncherSource::DesktopEntry) => LauncherSource::DesktopEntry,
            _ => LauncherSource::Authored,
        },
    }
}

/// The short form as `icon`, a set as `iconSet`.
pub(super) fn icon_to_wire(icon: Option<&IconDecl>) -> (Option<String>, Option<wire::IconSet>) {
    match icon {
        None => (None, None),
        Some(IconDecl::Path(path)) => (Some(path.clone()), None),
        Some(IconDecl::Set(set)) => (
            None,
            Some(wire::IconSet {
                svg: set.svg.clone(),
                png: set.png.clone(),
                ico: set.ico.clone(),
                icns: set.icns.clone(),
                symbolic: set.symbolic.clone(),
                maskable: set.maskable,
            }),
        ),
    }
}

pub(super) fn icon_from_wire(icon: Option<String>, set: Option<wire::IconSet>) -> Option<IconDecl> {
    match (icon, set) {
        (_, Some(set)) => Some(IconDecl::Set(IconSet {
            svg: set.svg,
            png: set.png,
            ico: set.ico,
            icns: set.icns,
            symbolic: set.symbolic,
            maskable: set.maskable,
        })),
        (Some(path), None) => Some(IconDecl::Path(path)),
        (None, None) => None,
    }
}
