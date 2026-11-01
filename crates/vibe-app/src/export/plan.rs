//! What one export writes, per platform (`PROP-059#launcher-runtime`):
//! for each launcher the entry file and its icon files, with the exact
//! paths, before anything is written — so `--dry-run` answers the same
//! list a real export records.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vibe_wire::generated::application_exported as wire;
use vibe_wire::generated::application_launchers_all as all_wire;

use super::entries::{self, EntryText};
use super::{ExportRequest, SystemPlatform, SystemTargets};
use crate::error::StateError;
use crate::settings::io;

/// The content of one written file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Body {
    Text {
        text: String,
        executable: bool,
    },
    Copy(PathBuf),
    /// A `.lnk`, saved by running this PowerShell script.
    Shortcut(String),
}

/// One launcher's files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Planned {
    key: String,
    name: String,
    /// What the record names: the entry file, or the `.app` directory.
    pub(super) record: PathBuf,
    /// Whether `record` is a wrapper directory rewritten whole.
    pub(super) bundle: bool,
    pub(super) files: Vec<(PathBuf, Body)>,
    /// Icon files recorded beside the entry (Linux theme copies).
    extra: Vec<PathBuf>,
}

impl Planned {
    pub(super) fn entry(&self) -> wire::Entry {
        let mut paths = vec![self.record.display().to_string()];
        paths.extend(self.extra.iter().map(|path| path.display().to_string()));
        wire::Entry {
            key: self.key.clone(),
            name: self.name.clone(),
            paths,
        }
    }
}

/// `group/name#id` of a launcher key: the spelling an entry starts, which
/// names the current version of the package rather than the one exported.
fn unversioned(key: &str) -> String {
    match crate::catalog::split_launcher_key(key) {
        Some((package, id)) => {
            let package = package.rsplit_once('@').map_or(package, |(name, _)| name);
            format!("{package}#{id}")
        }
        None => key.to_owned(),
    }
}

/// A file-name-safe slug of the unversioned key, stable across updates so a
/// re-export after one replaces the same files.
fn slug(key: &str) -> String {
    unversioned(key)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

/// A display name as a file name on every platform.
fn file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '-'
            } else {
                ch
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').to_owned();
    if trimmed.is_empty() {
        "launcher".into()
    } else {
        trimmed
    }
}

fn icon_files(
    installation: &all_wire::Installation,
    launcher: &all_wire::Launcher,
) -> Vec<all_wire::IconFile> {
    launcher
        .icon_set
        .as_ref()
        .or(installation.icon_set.as_ref())
        .map(|set| {
            set.files
                .iter()
                .filter(|file| file.path.is_some())
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn icon_of(files: &[all_wire::IconFile], slot: all_wire::IconSlot) -> Option<PathBuf> {
    files
        .iter()
        .filter(|file| file.slot == slot)
        .max_by_key(|file| file.size)
        .and_then(|file| file.path.as_ref().map(PathBuf::from))
}

/// The user's own icon, when they set one: it replaces the package's set.
fn user_icon(launcher: &all_wire::Launcher) -> Option<String> {
    launcher
        .display_icon
        .clone()
        .filter(|icon| Some(icon) != launcher.icon.resolved.as_ref())
}

fn text_of(
    request: &ExportRequest<'_>,
    launcher: &all_wire::Launcher,
    icon: Option<String>,
) -> EntryText {
    let mut command = request.targets.start.clone();
    let terminal = !matches!(launcher.kind, all_wire::LauncherKind::Gui);
    if !terminal {
        // A window of its own starts detached and recorded, so `running`
        // and a client's taskbar see what the system menu started; a
        // terminal program stays attached to the terminal opened for it.
        command.push("--json".into());
    }
    command.push(unversioned(&launcher.key));
    EntryText {
        key: launcher.key.clone(),
        home: request.settings.path().display().to_string(),
        name: launcher.display_name.clone(),
        names: launcher.names.clone(),
        generic_name: launcher.generic_name.clone(),
        comment: launcher.description.clone(),
        comments: launcher.descriptions.clone(),
        command,
        icon,
        terminal,
        categories: launcher.categories.clone(),
        keywords: launcher.keywords.clone(),
        mime_types: launcher.mime_types.clone(),
        window_class: launcher.window_class.clone(),
    }
}

/// Every launcher's files for the request's platform.
pub(super) fn plan(
    listing: &all_wire::ApplicationLaunchersAll,
    request: &ExportRequest<'_>,
) -> Vec<Planned> {
    let rows: Vec<_> = super::exported_rows(listing, request.collection).collect();
    // Two launchers shown under one name would share a shortcut or a
    // wrapper; the second and later ones carry their launcher id.
    let mut taken = BTreeSet::new();
    rows.into_iter()
        .map(|(installation, launcher)| {
            let mut name = file_name(&launcher.display_name);
            if !taken.insert(name.to_lowercase()) {
                name = format!("{name} ({})", file_name(&launcher.id));
                taken.insert(name.to_lowercase());
            }
            let targets = request.targets;
            match targets.platform {
                SystemPlatform::Linux => linux(request, installation, launcher),
                SystemPlatform::Windows => windows(request, installation, launcher, &name),
                SystemPlatform::Macos => macos(request, installation, launcher, &name),
            }
        })
        .collect()
}

fn linux(
    request: &ExportRequest<'_>,
    installation: &all_wire::Installation,
    launcher: &all_wire::Launcher,
) -> Planned {
    let root = &request.targets.root;
    let slug = slug(&launcher.key);
    let theme_name = format!("vibe-{slug}");
    let mut copies: Vec<(PathBuf, Body)> = Vec::new();
    let custom = user_icon(launcher);
    if custom.is_none() {
        let hicolor = root.join("icons").join("hicolor");
        for file in icon_files(installation, launcher) {
            let Some(source) = file.path.as_ref().map(PathBuf::from) else {
                continue;
            };
            let target = match (&file.slot, file.size) {
                (all_wire::IconSlot::Png, Some(size)) => hicolor
                    .join(format!("{size}x{size}"))
                    .join("apps")
                    .join(format!("{theme_name}.png")),
                (all_wire::IconSlot::Svg, _) => hicolor
                    .join("scalable")
                    .join("apps")
                    .join(format!("{theme_name}.svg")),
                _ => continue,
            };
            if copies.iter().all(|(path, _)| *path != target) {
                copies.push((target, Body::Copy(source)));
            }
        }
    }
    let icon = match (&custom, copies.is_empty()) {
        (Some(path), _) => Some(path.clone()),
        (None, false) => Some(theme_name.clone()),
        (None, true) => launcher.display_icon.clone(),
    };
    let desktop = root
        .join("applications")
        .join(format!("vibe-{slug}.desktop"));
    let text = entries::desktop_entry(&text_of(request, launcher, icon));
    let extra = copies.iter().map(|(path, _)| path.clone()).collect();
    let mut files = vec![(
        desktop.clone(),
        Body::Text {
            text,
            executable: false,
        },
    )];
    files.extend(copies);
    Planned {
        key: launcher.key.clone(),
        name: launcher.display_name.clone(),
        record: desktop,
        bundle: false,
        files,
        extra,
    }
}

fn windows(
    request: &ExportRequest<'_>,
    installation: &all_wire::Installation,
    launcher: &all_wire::Launcher,
    name: &str,
) -> Planned {
    let targets: &SystemTargets = request.targets;
    let folder = request
        .collection
        .map(str::to_owned)
        .or_else(|| installation.menu_group.clone())
        .map(|group| file_name(&group));
    let directory = match folder {
        Some(folder) => targets.root.join(folder),
        None => targets.root.clone(),
    };
    let lnk = directory.join(format!("{name}.lnk"));
    let icon = user_icon(launcher)
        .filter(|icon| icon.to_lowercase().ends_with(".ico"))
        .map(PathBuf::from)
        .or_else(|| icon_of(&icon_files(installation, launcher), all_wire::IconSlot::Ico));
    let text = text_of(request, launcher, None);
    let script = entries::shortcut_script(
        &text,
        &lnk.display().to_string(),
        None,
        icon.as_deref()
            .map(|path| path.display().to_string())
            .as_deref(),
    );
    Planned {
        key: launcher.key.clone(),
        name: launcher.display_name.clone(),
        record: lnk.clone(),
        bundle: false,
        files: vec![(lnk, Body::Shortcut(script))],
        extra: Vec::new(),
    }
}

fn macos(
    request: &ExportRequest<'_>,
    installation: &all_wire::Installation,
    launcher: &all_wire::Launcher,
    name: &str,
) -> Planned {
    const EXECUTABLE: &str = "vibe-launcher";
    const IN_TERMINAL: &str = "start.command";
    let bundle = request.targets.root.join(format!("{name}.app"));
    let contents = bundle.join("Contents");
    let icns = user_icon(launcher)
        .filter(|icon| icon.to_lowercase().ends_with(".icns"))
        .map(PathBuf::from)
        .or_else(|| {
            icon_of(
                &icon_files(installation, launcher),
                all_wire::IconSlot::Icns,
            )
        });
    let text = text_of(request, launcher, None);
    let identifier = format!("org.vibevm.launcher.{}", slug(&launcher.key));
    let mut files = vec![(
        contents.join("Info.plist"),
        Body::Text {
            text: entries::info_plist(
                &text,
                &identifier,
                EXECUTABLE,
                icns.as_ref().map(|_| "icon.icns"),
            ),
            executable: false,
        },
    )];
    let start = entries::shell_shim(&text.command);
    let macos = contents.join("MacOS");
    if text.terminal {
        files.push((
            macos.join(EXECUTABLE),
            Body::Text {
                text: entries::terminal_shim(IN_TERMINAL),
                executable: true,
            },
        ));
        files.push((
            macos.join(IN_TERMINAL),
            Body::Text {
                text: start,
                executable: true,
            },
        ));
    } else {
        files.push((
            macos.join(EXECUTABLE),
            Body::Text {
                text: start,
                executable: true,
            },
        ));
    }
    if let Some(icns) = icns {
        files.push((
            contents.join("Resources").join("icon.icns"),
            Body::Copy(icns),
        ));
    }
    Planned {
        key: launcher.key.clone(),
        name: launcher.display_name.clone(),
        record: bundle,
        bundle: true,
        files,
        extra: Vec::new(),
    }
}

/// Save one `.lnk` by running its script in Windows PowerShell.
pub(super) fn save_shortcut(lnk: &Path, script: &str) -> Result<(), StateError> {
    let output = std::process::Command::new(crate::installer::programs::windows_powershell())
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|source| io("saving the shortcut", lnk, source))?;
    if output.status.success() {
        return Ok(());
    }
    Err(io(
        "saving the shortcut",
        lnk,
        std::io::Error::other(String::from_utf8_lossy(&output.stderr).trim().to_owned()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slug_and_the_started_spelling_drop_the_version() {
        let key = "org.example/editor@1.0.0#edit";
        assert_eq!(unversioned(key), "org.example/editor#edit");
        assert_eq!(slug(key), "org.example-editor-edit");
        assert_eq!(file_name("A/B: \"c\"."), "A-B- -c-");
        assert_eq!(file_name("  "), "launcher");
    }
}
