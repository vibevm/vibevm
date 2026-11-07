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
use crate::settings::{MutationLock, SettingsRoot, io};
mod upgrade;
pub(crate) use upgrade::{CapturedImage, before_publication, for_publication};

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
/// names and overwrite copies whose bytes differ. This entrypoint takes the
/// home mutation lock; use [`sync_locked`] inside an existing mutation.
/// Names that are not portable
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
    let lock = settings.lock()?;
    sync_locked(settings, previous, current, proxy_image, &lock)
}

/// Reconcile global proxies inside an already-held home mutation. The supplied
/// image must advertise reader4 support when this home uses index4.
///
/// ```
/// use std::collections::BTreeSet;
/// let temp = tempfile::tempdir().unwrap();
/// let home = vibe_app::SettingsRoot::new(temp.path());
/// let lock = home.lock().unwrap();
/// vibe_app::sync_proxies_locked(&home, &BTreeSet::new(), &BTreeSet::new(),
///     &temp.path().join("unused"), &lock).unwrap();
/// ```
pub fn sync_locked(
    settings: &SettingsRoot,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
    proxy_image: &Path,
    lock: &MutationLock,
) -> Result<(), AppError> {
    lock.require_home(settings)?;
    if !current.iter().any(|name| command_name(name)) {
        return sync_captured(
            settings,
            &settings.proxy_dir(),
            previous,
            current,
            None,
            lock,
        );
    }
    let four = crate::Index::load_with_protocol(settings)?.1;
    if four < 4 {
        // Preserve the incumbent3 contract: obsolete names are removed even
        // when the image for a new command is missing.
        let obsolete = previous.difference(current).cloned().collect();
        sync_captured(
            settings,
            &settings.proxy_dir(),
            &obsolete,
            &BTreeSet::new(),
            None,
            lock,
        )?;
    }
    let image = CapturedImage::read_for_reader(proxy_image, four)?;
    sync_captured(
        settings,
        &settings.proxy_dir(),
        previous,
        current,
        Some(&image),
        lock,
    )
}

/// Reconcile names using the same captured bytes admitted before publication.
pub(crate) fn sync_captured(
    settings: &SettingsRoot,
    dir: &Path,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
    image: Option<&CapturedImage>,
    lock: &MutationLock,
) -> Result<(), AppError> {
    lock.require_home(settings)?;
    let wanted: Vec<_> = current
        .iter()
        .filter(|name| command_name(name))
        .map(|name| dir.join(proxy_file_name(name)))
        .collect();
    for path in wanted.iter().chain(
        previous
            .difference(current)
            .filter(|name| command_name(name))
            .map(|name| dir.join(proxy_file_name(name)))
            .collect::<Vec<_>>()
            .iter(),
    ) {
        upgrade::regular(settings, path)?;
    }
    if !wanted.is_empty() {
        let image = image.ok_or_else(|| crate::StateError::ProxyUpgrade {
            path: dir.display().to_string(),
            reason: "proxy writes require the captured image".into(),
        })?;
        if crate::Index::load_with_protocol(settings)?.1 > image.reader {
            return Err(crate::StateError::ProxyUpgrade {
                path: dir.display().to_string(),
                reason: "index4 proxy writes require an admitted compatible image".into(),
            }
            .into());
        }
    }
    for name in previous
        .difference(current)
        .filter(|name| command_name(name))
    {
        let path = dir.join(proxy_file_name(name));
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io("removing", &path, e).into()),
        }
    }
    if wanted.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(dir).map_err(|e| io("creating", dir, e))?;
    let Some(image) = image else {
        return Ok(());
    };
    for path in wanted {
        if fs::read(&path).is_ok_and(|bytes| bytes == image.bytes) {
            continue;
        }
        place(&path, &image.bytes)?;
    }
    Ok(())
}

fn command_name(name: &str) -> bool {
    is_command_name(name) && name != "vibe"
}

/// Write an executable sibling and rename it over `path`.
fn place(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let mut sibling = tempfile::NamedTempFile::new_in(parent)
        .map_err(|source| io("creating proxy sibling", parent, source))?;
    sibling
        .write_all(bytes)
        .and_then(|()| sibling.as_file().sync_all())
        .map_err(|source| io("writing", path, source))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        sibling
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o755))
            .map_err(|source| io("making proxy executable", path, source))?;
    }
    sibling
        .persist(path)
        .map_err(|e| io("replacing proxy", path, e.error))?;
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
    #[test]
    fn invalid_only_requested_commands_need_no_image_and_keep_filtered_removals() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let missing = dir.path().join("missing");
        let invalid = names(&["vibe", "../escape"]);
        sync(&settings, &names(&[]), &invalid, &missing).unwrap();
        fs::create_dir_all(settings.proxy_dir()).unwrap();
        let obsolete = settings.proxy_dir().join(proxy_file_name("old"));
        fs::write(&obsolete, b"old").unwrap();
        sync(&settings, &names(&["old"]), &invalid, &missing).unwrap();
        assert!(!obsolete.exists());
        assert!(!dir.path().join("escape").exists());
    }
}
