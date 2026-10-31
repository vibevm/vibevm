//! Launchers derived from desktop entries (`PROP-059#desktop-entries`):
//! after the payload is in place, every `launchers_from` match that is a
//! shown `Application` entry becomes one launcher; an authored row with the
//! same id overrides it field by field. `vibe app import-desktop` prints the
//! same rows as manifest TOML.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#desktop-entries");

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{
    IconDecl, IconSet, LauncherActionDecl, LauncherDecl, LauncherKind, LauncherSource,
    MENU_CATEGORIES, png_dimensions,
};

use crate::desktop::{DesktopFile, exec_words};

/// Derive one launcher per shown `Application` entry matching `patterns`
/// (globs relative to `app_root`), in path order. A file that does not
/// parse, is no `Application`, is `Hidden` or has no `Exec` derives none.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let apps = dir.path().join("share/applications");
/// std::fs::create_dir_all(&apps).unwrap();
/// std::fs::write(apps.join("edit.desktop"), "[Desktop Entry]\nType=Application\nName=Edit\nExec=edit %F\n").unwrap();
/// let launchers =
///     vibe_app::derive_launchers(dir.path(), &["share/applications/*.desktop".into()], None);
/// assert_eq!(launchers[0].id, "edit");
/// assert_eq!(launchers[0].executable, "$app_bin/edit");
/// assert_eq!(launchers[0].args, ["{files}"]);
/// ```
pub fn derive_launchers(
    app_root: &Path,
    patterns: &[String],
    search_path: Option<&OsStr>,
) -> Vec<LauncherDecl> {
    let mut files = BTreeSet::new();
    let base = glob::Pattern::escape(&app_root.to_string_lossy());
    for pattern in patterns {
        let Ok(paths) = glob::glob(&format!("{base}/{pattern}")) else {
            continue;
        };
        files.extend(paths.filter_map(Result::ok).filter(|path| path.is_file()));
    }
    files
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let file = DesktopFile::parse(&text).ok()?;
            entry_launcher(&stem(&path), &file, Some(app_root), search_path)
        })
        .collect()
}

fn stem(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.strip_suffix(".desktop").unwrap_or(&name).to_owned()
}

/// The launcher one parsed entry describes, or `None` when it is not a
/// shown `Application` with a `Name` and an `Exec`.
pub(crate) fn entry_launcher(
    id: &str,
    file: &DesktopFile,
    app_root: Option<&Path>,
    search_path: Option<&OsStr>,
) -> Option<LauncherDecl> {
    let main = DesktopFile::MAIN;
    if file.string(main, "Type").as_deref() != Some("Application")
        || file.boolean(main, "Hidden") == Some(true)
    {
        return None;
    }
    let name = file.string(main, "Name")?;
    let words = exec_words(&file.string(main, "Exec")?).ok()?;
    let kind = if file.boolean(main, "Terminal") == Some(true) {
        LauncherKind::Tui
    } else {
        LauncherKind::Gui
    };
    let mut categories = Vec::new();
    for category in file.list(main, "Categories") {
        if MENU_CATEGORIES.contains(&category.as_str()) && !categories.contains(&category) {
            categories.push(category);
        }
    }
    let actions = file
        .list(main, "Actions")
        .into_iter()
        .filter_map(|action| {
            let group = format!("Desktop Action {action}");
            let words = exec_words(&file.string(&group, "Exec")?).ok()?;
            Some(LauncherActionDecl {
                name: file.string(&group, "Name")?,
                args: words.iter().skip(1).map(|word| literal(word)).collect(),
                icon: file
                    .string(&group, "Icon")
                    .and_then(|icon| icon_of(&icon, app_root)),
                id: action,
            })
        })
        .collect();
    Some(LauncherDecl {
        id: id.to_owned(),
        name,
        description: file.string(main, "Comment"),
        executable: executable(&words[0], app_root, search_path),
        kind: Some(kind),
        args: words.iter().skip(1).map(|word| literal(word)).collect(),
        working_directory: file
            .string(main, "Path")
            .and_then(|path| inside_root(Path::new(&path), app_root)),
        terminal: Some(kind.terminal()),
        default: false,
        menu: Some(file.boolean(main, "NoDisplay") != Some(true)),
        categories,
        keywords: file.list(main, "Keywords"),
        generic_name: file.string(main, "GenericName"),
        window_class: file.string(main, "StartupWMClass"),
        mime_types: file.list(main, "MimeType"),
        names: file.localised(main, "Name"),
        descriptions: file.localised(main, "Comment"),
        icon: file
            .string(main, "Icon")
            .and_then(|icon| icon_of(&icon, app_root)),
        actions,
        source: LauncherSource::DesktopEntry,
    })
}

/// A word of `Exec` as a launcher template: a literal `$` doubled.
fn literal(word: &str) -> String {
    word.replace('$', "$$")
}

/// The first word of `Exec` as an executable template: a file in the
/// payload's `bin`, else in a PATH directory inside the application root,
/// else a path inside the root; anything else is looked for in `bin` at
/// launch.
fn executable(program: &str, app_root: Option<&Path>, search_path: Option<&OsStr>) -> String {
    let path = Path::new(program);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| program.to_owned());
    if let Some(root) = app_root {
        if path.is_absolute() || program.contains('/') {
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            if let Some(template) = inside_root(&absolute, Some(root))
                && absolute.is_file()
            {
                return template;
            }
        }
        if root.join("bin").join(&name).is_file() {
            return format!("$app_bin/{}", literal(&name));
        }
        let candidates = search_path
            .map(|value| std::env::split_paths(value).collect::<Vec<_>>())
            .unwrap_or_default();
        for directory in candidates {
            let candidate = directory.join(&name);
            if candidate.is_file()
                && let Some(template) = inside_root(&candidate, Some(root))
            {
                return template;
            }
        }
    }
    format!("$app_bin/{}", literal(&name))
}

/// `$app_root/<relative>` for a path inside the root; a relative path is
/// taken relative to the root.
fn inside_root(path: &Path, app_root: Option<&Path>) -> Option<String> {
    let relative = if path.is_absolute() {
        let root = app_root?;
        let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        path.strip_prefix(&root).ok()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    let text = relative.to_string_lossy().replace('\\', "/");
    if text.split('/').any(|part| part == "..") {
        return None;
    }
    Some(if text.is_empty() {
        "$app_root".into()
    } else {
        format!("$app_root/{}", literal(&text))
    })
}

/// The icon set of an `Icon` value: a path is taken as is (inside the
/// root), a theme name is collected from the hicolor theme under the root.
fn icon_of(value: &str, app_root: Option<&Path>) -> Option<IconDecl> {
    if value.contains('/') {
        let root = app_root?;
        let absolute = if Path::new(value).is_absolute() {
            PathBuf::from(value)
        } else {
            root.join(value)
        };
        let template = inside_root(&absolute, Some(root))?;
        let lower = value.to_ascii_lowercase();
        let mut set = IconSet::default();
        if lower.ends_with(".svg") {
            set.svg = Some(template);
        } else if lower.ends_with(".png") {
            match fs::read(&absolute)
                .ok()
                .and_then(|bytes| png_dimensions(&bytes))
            {
                Some((width, height)) if width == height => {
                    set.png.insert(width.to_string(), template);
                }
                _ => return Some(IconDecl::Path(template)),
            }
        } else if lower.ends_with(".ico") {
            set.ico = Some(template);
        } else {
            return None;
        }
        return Some(IconDecl::Set(set));
    }
    hicolor(value, app_root?).map(IconDecl::Set)
}

/// `share/icons/hicolor/<N>x<N>/apps/<name>.png`, `scalable/apps/<name>.svg`
/// and `symbolic/apps/<name>-symbolic.svg` under the application root.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let theme = dir.path().join("share/icons/hicolor");
/// std::fs::create_dir_all(theme.join("48x48/apps")).unwrap();
/// std::fs::create_dir_all(theme.join("scalable/apps")).unwrap();
/// std::fs::write(theme.join("48x48/apps/edit.png"), b"png").unwrap();
/// std::fs::write(theme.join("scalable/apps/edit.svg"), b"<svg/>").unwrap();
/// let set = vibe_app::hicolor_icon_set("edit", dir.path()).unwrap();
/// assert_eq!(set.png["48"], "$app_root/share/icons/hicolor/48x48/apps/edit.png");
/// assert_eq!(set.svg.as_deref(), Some("$app_root/share/icons/hicolor/scalable/apps/edit.svg"));
/// assert!(vibe_app::hicolor_icon_set("other", dir.path()).is_none());
/// ```
pub fn hicolor(name: &str, app_root: &Path) -> Option<IconSet> {
    let theme = "share/icons/hicolor";
    let mut set = IconSet::default();
    if let Ok(entries) = fs::read_dir(app_root.join(theme)) {
        for entry in entries.filter_map(Result::ok) {
            let directory = entry.file_name().to_string_lossy().into_owned();
            let Some((width, height)) = directory.split_once('x') else {
                continue;
            };
            let (Ok(width), Ok(_)) = (width.parse::<u32>(), height.parse::<u32>()) else {
                continue;
            };
            if width.to_string() != height {
                continue;
            }
            let relative = format!("{theme}/{directory}/apps/{name}.png");
            if app_root.join(&relative).is_file() {
                set.png.insert(
                    width.to_string(),
                    format!("$app_root/{}", literal(&relative)),
                );
            }
        }
    }
    let scalable = format!("{theme}/scalable/apps/{name}.svg");
    if app_root.join(&scalable).is_file() {
        set.svg = Some(format!("$app_root/{}", literal(&scalable)));
    }
    let symbolic = format!("{theme}/symbolic/apps/{name}-symbolic.svg");
    if app_root.join(&symbolic).is_file() {
        set.symbolic = Some(format!("$app_root/{}", literal(&symbolic)));
    }
    (!set.is_empty()).then_some(set)
}

/// The installed launchers of an option: authored rows in their order,
/// each overriding the derived launcher of the same id field by field, then
/// the derived launchers no row names. Every row must end up complete; a
/// spelled `terminal` must agree with the merged kind.
pub(crate) fn merge(
    authored: &[LauncherDecl],
    derived: Vec<LauncherDecl>,
) -> Result<Vec<LauncherDecl>, String> {
    let mut derived = derived;
    let mut out = Vec::new();
    for row in authored {
        let merged = match derived.iter().position(|candidate| candidate.id == row.id) {
            Some(at) => overlay(derived.remove(at), row),
            None => row.clone(),
        };
        out.push(merged);
    }
    out.extend(derived);
    out.into_iter().map(normalise).collect()
}

fn overlay(mut base: LauncherDecl, row: &LauncherDecl) -> LauncherDecl {
    fn take<T: Clone>(slot: &mut Option<T>, value: &Option<T>) {
        if value.is_some() {
            slot.clone_from(value);
        }
    }
    fn take_list<T: Clone>(slot: &mut T, value: &T, empty: bool) {
        if !empty {
            slot.clone_from(value);
        }
    }
    if !row.name.trim().is_empty() {
        base.name.clone_from(&row.name);
    }
    if !row.executable.is_empty() {
        base.executable.clone_from(&row.executable);
    }
    if row.kind.is_some() {
        base.kind = row.kind;
        base.terminal = None;
    }
    take(&mut base.terminal, &row.terminal);
    take(&mut base.description, &row.description);
    take(&mut base.working_directory, &row.working_directory);
    take(&mut base.menu, &row.menu);
    take(&mut base.generic_name, &row.generic_name);
    take(&mut base.window_class, &row.window_class);
    take(&mut base.icon, &row.icon);
    take_list(&mut base.args, &row.args, row.args.is_empty());
    take_list(
        &mut base.categories,
        &row.categories,
        row.categories.is_empty(),
    );
    take_list(&mut base.keywords, &row.keywords, row.keywords.is_empty());
    take_list(
        &mut base.mime_types,
        &row.mime_types,
        row.mime_types.is_empty(),
    );
    take_list(&mut base.names, &row.names, row.names.is_empty());
    take_list(
        &mut base.descriptions,
        &row.descriptions,
        row.descriptions.is_empty(),
    );
    take_list(&mut base.actions, &row.actions, row.actions.is_empty());
    base.default |= row.default;
    base
}

/// A complete installed row: `kind`, `terminal` and `menu` spelled out.
fn normalise(mut launcher: LauncherDecl) -> Result<LauncherDecl, String> {
    if !launcher.is_complete() {
        return Err(format!(
            "launcher `{}` names no complete row and no desktop entry derived it",
            launcher.id
        ));
    }
    let kind = launcher.effective_kind();
    if launcher
        .terminal
        .is_some_and(|terminal| terminal != kind.terminal())
    {
        return Err(format!(
            "launcher `{}` sets terminal, which disagrees with kind `{}`",
            launcher.id,
            kind.as_str()
        ));
    }
    launcher.kind = Some(kind);
    launcher.terminal = Some(kind.terminal());
    launcher.menu = Some(launcher.shows_in_menu());
    Ok(launcher)
}

/// `vibe app import-desktop`: the rows the given desktop entries derive,
/// as `[[installer_options.launchers]]` manifest TOML. `app_root`, when
/// given, resolves `Exec` and theme icons against an unpacked payload.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let file = dir.path().join("edit.desktop");
/// std::fs::write(&file, "[Desktop Entry]\nType=Application\nName=Edit\nExec=edit\nTerminal=true\n").unwrap();
/// let text = vibe_app::import_desktop(&[file], None).unwrap();
/// assert!(text.contains("[[installer_options.launchers]]"));
/// assert!(text.contains("kind = \"tui\""));
/// ```
pub fn import_desktop(files: &[PathBuf], app_root: Option<&Path>) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct Rows {
        launchers: Vec<LauncherDecl>,
    }
    let mut launchers = Vec::new();
    let mut skipped = Vec::new();
    for path in files {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("reading `{}` failed: {error}", path.display()))?;
        let file = DesktopFile::parse(&text)
            .map_err(|reason| format!("`{}` is not a desktop entry: {reason}", path.display()))?;
        match entry_launcher(&stem(path), &file, app_root, None) {
            Some(mut launcher) => {
                // `terminal` follows from `kind` in authored text.
                launcher.terminal = None;
                launchers.push(launcher);
            }
            None => skipped.push(path.display().to_string()),
        }
    }
    let rendered = toml::to_string(&Rows { launchers })
        .map_err(|error| format!("rendering the launcher rows failed: {error}"))?;
    let mut out = String::new();
    for path in skipped {
        out.push_str(&format!(
            "# {path}: skipped, not a shown Application entry with Name and Exec\n"
        ));
    }
    for line in rendered.lines() {
        let header = line
            .strip_prefix("[[launchers")
            .map(|rest| format!("[[installer_options.launchers{rest}"))
            .or_else(|| {
                line.strip_prefix("[launchers")
                    .map(|rest| format!("[installer_options.launchers{rest}"))
            });
        out.push_str(header.as_deref().unwrap_or(line));
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
#[path = "derive/tests.rs"]
mod tests;
