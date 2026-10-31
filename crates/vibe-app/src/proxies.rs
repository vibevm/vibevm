//! PATH proxies: `<settings>/opt/bin/<command>` is a byte copy of the native
//! `vibe-app-proxy` image for every reserved command. The set is synchronised
//! after each index write; only names the previous index reserved are ever
//! removed, so anything else in `opt/bin` stays untouched.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#proxy");

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use vibe_core::manifest::is_command_name;

use crate::error::AppError;
use crate::settings::{SettingsRoot, io};

/// The proxy file name of one command (`<command>.exe` on Windows).
///
/// ```
/// let name = vibe_app::proxy_file_name("tool");
/// assert_eq!(name, if cfg!(windows) { "tool.exe" } else { "tool" });
/// ```
pub fn proxy_file_name(command: &str) -> String {
    if cfg!(windows) {
        format!("{command}.exe")
    } else {
        command.to_owned()
    }
}

/// Refuse early, before any child runs, when the proxy image is absent.
pub(crate) fn require_image(proxy_image: &Path) -> Result<(), AppError> {
    if proxy_image.is_file() {
        Ok(())
    } else {
        Err(AppError::ProxyImage {
            path: proxy_image.display().to_string(),
        })
    }
}

/// Make `opt/bin` hold one current proxy copy per name in `current`: remove
/// the copies of names only `previous` reserved, then copy the image for new
/// names and overwrite copies whose bytes differ. Names that are not portable
/// command tokens are never written or removed.
///
/// ```
/// use std::collections::BTreeSet;
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let image = dir.path().join("vibe-app-proxy");
/// std::fs::write(&image, b"proxy").unwrap();
/// let tool = BTreeSet::from(["tool".to_owned()]);
/// vibe_app::sync_proxies(&settings, &BTreeSet::new(), &tool, &image).unwrap();
/// let copy = settings.proxy_dir().join(vibe_app::proxy_file_name("tool"));
/// assert_eq!(std::fs::read(&copy).unwrap(), b"proxy");
/// vibe_app::sync_proxies(&settings, &tool, &BTreeSet::new(), &image).unwrap();
/// assert!(!copy.exists());
/// ```
pub fn sync(
    settings: &SettingsRoot,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
    proxy_image: &Path,
) -> Result<(), AppError> {
    sync_into(&settings.proxy_dir(), previous, current, proxy_image)
}

/// [`sync`] for any proxy directory of a home: `opt/bin`, or the session
/// proxies of one installation (`PROP-059#hidden-installations`). A proxy
/// finds its home as the nearest ancestor holding the index, so `dir`
/// must lie inside the home.
pub(crate) fn sync_into(
    dir: &Path,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
    proxy_image: &Path,
) -> Result<(), AppError> {
    let dir = dir.to_path_buf();
    for name in previous
        .difference(current)
        .filter(|name| command_name(name))
    {
        let path = dir.join(proxy_file_name(name));
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(io("removing", &path, source).into()),
        }
    }
    let wanted: Vec<PathBuf> = current
        .iter()
        .filter(|name| command_name(name))
        .map(|name| dir.join(proxy_file_name(name)))
        .collect();
    if wanted.is_empty() {
        return Ok(());
    }
    require_image(proxy_image)?;
    let image = fs::read(proxy_image).map_err(|source| io("reading", proxy_image, source))?;
    fs::create_dir_all(&dir).map_err(|source| io("creating", &dir, source))?;
    for path in wanted {
        if fs::read(&path).is_ok_and(|bytes| bytes == image) {
            continue;
        }
        place(&path, &image)?;
    }
    Ok(())
}

fn command_name(name: &str) -> bool {
    is_command_name(name) && name != "vibe"
}

/// Write an executable sibling and rename it over `path`.
fn place(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let sibling = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let written = File::create(&sibling).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });
    #[cfg(unix)]
    let written = written.and_then(|()| {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&sibling, fs::Permissions::from_mode(0o755))
    });
    if let Err(source) = written.and_then(|()| fs::rename(&sibling, path)) {
        let _ = fs::remove_file(&sibling);
        return Err(io("writing", path, source).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn sync_adds_replaces_and_removes_only_previously_reserved_names() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let image = dir.path().join("vibe-app-proxy");
        fs::write(&image, b"image-1").unwrap();
        let bin = settings.proxy_dir();
        let copy = |name: &str| bin.join(proxy_file_name(name));

        sync(&settings, &names(&[]), &names(&["a", "b"]), &image).unwrap();
        assert_eq!(fs::read(copy("a")).unwrap(), b"image-1");
        assert_eq!(fs::read(copy("b")).unwrap(), b"image-1");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(copy("a")).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111);
        }

        fs::write(copy("a"), b"stale").unwrap();
        fs::write(bin.join("vibe-shim"), b"version manager").unwrap();
        fs::write(&image, b"image-2").unwrap();
        sync(&settings, &names(&["a", "b"]), &names(&["a"]), &image).unwrap();
        assert_eq!(fs::read(copy("a")).unwrap(), b"image-2");
        assert!(!copy("b").exists());
        assert_eq!(fs::read(bin.join("vibe-shim")).unwrap(), b"version manager");

        sync(&settings, &names(&["a", "../escape"]), &names(&[]), &image).unwrap();
        assert!(!copy("a").exists());
        assert!(bin.join("vibe-shim").exists());
        let leftovers: Vec<_> = fs::read_dir(&bin)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(leftovers.len(), 1, "{leftovers:?}");
    }

    #[test]
    fn a_missing_image_is_named_but_removal_still_happens() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let image = dir.path().join("absent-proxy");
        fs::create_dir_all(settings.proxy_dir()).unwrap();
        fs::write(settings.proxy_dir().join(proxy_file_name("old")), b"x").unwrap();
        let error = sync(&settings, &names(&["old"]), &names(&["new"]), &image).unwrap_err();
        assert!(error.to_string().contains("absent-proxy"), "{error}");
        assert!(!settings.proxy_dir().join(proxy_file_name("old")).exists());
        sync(&settings, &names(&[]), &names(&[]), &image).unwrap();
    }
}
