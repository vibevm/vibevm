//! Resolution of one proxied command against the home's index and
//! `storage.toml`, mirroring `vibe-app`'s library availability and launcher
//! template rules without depending on that crate.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
#[path = "index.rs"]
mod index;

const LIBRARY_MARKER: &str = "vibe-library.toml";
const DEFAULT_LIBRARY: &str = "default";

/// A launcher ready to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Launch {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
}

#[derive(Deserialize)]
struct StorageToml {
    #[serde(default)]
    library: Vec<LibraryRow>,
}

#[derive(Deserialize)]
struct LibraryRow {
    id: String,
    path: PathBuf,
}

#[derive(Deserialize)]
struct Marker {
    id: String,
}

/// Resolve `command` for the proxy at `exe` under the session value `session`.
pub(crate) fn resolve(exe: &Path, command: &str, session: Option<&str>) -> Result<Launch, String> {
    let home = home_of(exe)?;
    let index = read_index(&home)?;
    let slot = index
        .installations
        .values()
        .find(|record| record.public_commands.iter().any(|own| own.name == command))
        .map(|record| record.slot.clone())
        .ok_or_else(|| {
            format!("command `{command}` is not reserved by any installed application")
        })?;
    let choices = parse_session(session.unwrap_or_default())?;
    let key = match choices.get(&slot) {
        Some(key) => key.clone(),
        None => index
            .slots
            .get(&slot)
            .and_then(|entry| entry.default.clone())
            .ok_or_else(|| {
                format!("slot `{slot}` has no default provider; run `vibe app use {slot} KEY` or set VIBE_APP_USE")
            })?,
    };
    let record = index
        .installations
        .get(&key)
        .filter(|record| record.slot == slot)
        .ok_or_else(|| format!("`{key}` is not an installed provider of slot `{slot}`"))?;
    let launcher_id = record
        .public_commands
        .iter()
        .find(|own| own.name == command)
        .map(|own| own.launcher.as_str())
        .ok_or_else(|| format!("provider `{key}` does not provide command `{command}`"))?;
    let launcher = record
        .launchers
        .iter()
        .find(|launcher| launcher.id == launcher_id)
        .ok_or_else(|| format!("provider `{key}` declares no launcher `{launcher_id}`"))?;
    let root = app_root(&home, &key, record)?;
    expand_launcher(launcher, &root, &home)
}

/// `slot=key;slot=key`: whitespace trimmed, empty entries ignored, a later
/// entry wins per slot (the same grammar as `vibe_app::SessionChoices`).
pub(crate) fn parse_session(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut choices = BTreeMap::new();
    for entry in text.split(';').filter(|entry| !entry.trim().is_empty()) {
        let malformed = || {
            format!(
                "malformed VIBE_APP_USE entry `{}`; expected SLOT=KEY",
                entry.trim()
            )
        };
        let (slot, key) = entry.split_once('=').ok_or_else(malformed)?;
        let (slot, key) = (slot.trim(), key.trim());
        if slot.is_empty() || key.is_empty() || key.contains('=') {
            return Err(malformed());
        }
        choices.insert(slot.to_owned(), key.to_owned());
    }
    Ok(choices)
}

/// The nearest ancestor of the proxy's own directory holding the index.
pub(crate) fn home_of(exe: &Path) -> Result<PathBuf, String> {
    let dir = exe
        .parent()
        .ok_or_else(|| format!("`{}` has no parent directory", exe.display()))?;
    dir.ancestors()
        .find(|candidate| candidate.join("applications").join("index.json").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "no application home holds applications/index.json above `{}`",
                dir.display()
            )
        })
}

fn read_index(home: &Path) -> Result<index::ApplicationIndex, String> {
    let path = home.join("applications").join("index.json");
    let bytes =
        fs::read(&path).map_err(|error| format!("reading `{}` failed: {error}", path.display()))?;
    index::read(&bytes).map_err(|error| {
        format!(
            "application index `{}` is unreadable: {error}",
            path.display()
        )
    })
}

/// An owned root from its library and directory, an external root as recorded.
fn app_root(home: &Path, key: &str, record: &index::Installation) -> Result<PathBuf, String> {
    match (&record.library, &record.directory, &record.app_root) {
        (Some(library), Some(directory), None) => {
            if directory
                .split('/')
                .any(|part| part.is_empty() || part == "..")
            {
                return Err(format!(
                    "installation `{key}` has a directory outside its library"
                ));
            }
            Ok(library_root(home, library)?.join(directory))
        }
        (None, None, Some(app_root)) if Path::new(app_root).is_absolute() => {
            Ok(PathBuf::from(app_root))
        }
        _ => Err(format!(
            "installation `{key}` has an inconsistent location in the index"
        )),
    }
}

/// A library is available when its path is a directory whose marker carries
/// the configured id; anything else is `unavailable: <reason>`.
fn library_root(home: &Path, id: &str) -> Result<PathBuf, String> {
    let path = if id == DEFAULT_LIBRARY {
        home.join("opt")
    } else {
        let storage = home.join("storage.toml");
        let text = match fs::read_to_string(&storage) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(format!("reading `{}` failed: {error}", storage.display())),
        };
        let config: StorageToml = toml::from_str(&text).map_err(|error| {
            format!(
                "storage configuration `{}` is invalid: {error}",
                storage.display()
            )
        })?;
        config
            .library
            .into_iter()
            .find(|row| row.id == id)
            .map(|row| row.path)
            .ok_or_else(|| format!("unavailable: library `{id}` is not configured"))?
    };
    if !path.is_dir() {
        return Err(format!(
            "unavailable: library directory `{}` is missing",
            path.display()
        ));
    }
    let marker = path.join(LIBRARY_MARKER);
    let named = fs::read_to_string(&marker)
        .map_err(|error| error.to_string())
        .and_then(|text| toml::from_str::<Marker>(&text).map_err(|error| error.to_string()))
        .map_err(|reason| {
            format!(
                "unavailable: library marker `{}` is invalid: {reason}",
                marker.display()
            )
        })?;
    if named.id != id {
        return Err(format!(
            "unavailable: library marker at `{}` names `{}`, not `{id}`",
            path.display(),
            named.id
        ));
    }
    Ok(path)
}

/// The argument array of one launch, mirroring `vibe_app::launch_arguments`
/// (PROP-059 `#desktop-entries`): `{file}` / `{url}` take the first caller
/// argument, `{files}` / `{urls}` all of them, an unfilled placeholder
/// argument is dropped, and caller arguments no placeholder took follow.
pub(crate) fn operands(args: &[String], caller: &[OsString]) -> Vec<OsString> {
    let mut consumed = false;
    let mut out = Vec::new();
    for arg in args {
        let single = ["{file}", "{url}"]
            .into_iter()
            .find(|placeholder| arg.contains(placeholder));
        if arg == "{files}" || arg == "{urls}" {
            out.extend(caller.iter().cloned());
            consumed = true;
        } else if let Some(placeholder) = single {
            consumed = true;
            match caller.first() {
                Some(first) => out.push(OsString::from(
                    arg.replace(placeholder, &first.to_string_lossy()),
                )),
                None if arg == placeholder => {}
                None => out.push(OsString::from(arg.replace(placeholder, ""))),
            }
        } else {
            out.push(OsString::from(arg));
        }
    }
    if !consumed {
        out.extend(caller.iter().cloned());
    }
    out
}

pub(crate) struct Vars<'a> {
    pub app_root: &'a Path,
    pub vibe_home: &'a Path,
}

fn expand_launcher(
    launcher: &index::Launcher,
    app_root: &Path,
    home: &Path,
) -> Result<Launch, String> {
    let root = fs::canonicalize(app_root)
        .map(strip_verbatim)
        .map_err(|_| format!("unavailable: app_root `{}` is missing", app_root.display()))?;
    let vars = Vars {
        app_root: &root,
        vibe_home: home,
    };
    let executable = contained(&vars, &launcher.executable, "executable")?;
    if !executable.is_file() {
        return Err(format!(
            "executable `{}` is not a regular file",
            executable.display()
        ));
    }
    let working_directory = match &launcher.working_directory {
        Some(template) => {
            let directory = contained(&vars, template, "working_directory")?;
            if !directory.is_dir() {
                return Err(format!(
                    "working_directory `{}` is not a directory",
                    directory.display()
                ));
            }
            Some(directory)
        }
        None => None,
    };
    let args = launcher
        .args
        .iter()
        .map(|arg| expand(arg, &vars, &launcher.id))
        .collect::<Result<_, _>>()?;
    Ok(Launch {
        executable,
        args,
        working_directory,
    })
}

/// A template path resolved under `app_root`: no absolute literal, no `..`,
/// and the canonical target stays inside the root.
fn contained(vars: &Vars<'_>, template: &str, field: &str) -> Result<PathBuf, String> {
    if Path::new(template).is_absolute() {
        return Err(format!("{field} `{template}` is an absolute literal path"));
    }
    let path = PathBuf::from(expand(template, vars, field)?);
    if path.components().any(|part| part == Component::ParentDir) {
        return Err(format!("{field} `{template}` traverses with `..`"));
    }
    let path = if path.is_absolute() {
        path
    } else {
        vars.app_root.join(path)
    };
    let resolved = fs::canonicalize(&path)
        .map(strip_verbatim)
        .map_err(|_| format!("{field} `{}` is unavailable", path.display()))?;
    if !resolved.starts_with(vars.app_root) {
        return Err(format!("{field} `{template}` escapes app_root"));
    }
    Ok(resolved)
}

/// One-pass expansion of `$app_root`, `$app_bin`, `$vibe_home` (braced or
/// not) and `$$`; `$project_root` needs a project, which a proxy never has.
pub(crate) fn expand(template: &str, vars: &Vars<'_>, what: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            result.push(ch);
            continue;
        }
        if chars.next_if_eq(&'$').is_some() {
            result.push('$');
            continue;
        }
        let mut name = String::new();
        if chars.next_if_eq(&'{').is_some() {
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some(next) => name.push(next),
                    None => return Err(format!("{what}: unterminated template variable")),
                }
            }
        } else {
            while let Some(next) =
                chars.next_if(|next| next.is_ascii_alphanumeric() || *next == '_')
            {
                name.push(next);
            }
        }
        let value = match name.as_str() {
            "app_root" => vars.app_root.to_path_buf(),
            "app_bin" => vars.app_root.join("bin"),
            "vibe_home" => vars.vibe_home.to_path_buf(),
            "project_root" => {
                return Err(format!(
                    "{what} requires a project; run it with `vibe app run --path`"
                ));
            }
            other => return Err(format!("{what}: unknown template variable `${other}`")),
        };
        result.push_str(&value.to_string_lossy());
    }
    Ok(result)
}

/// Drop the Windows `\\?\` prefix `canonicalize` adds to local paths.
pub(crate) fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest.to_string()),
        _ => path,
    }
}
