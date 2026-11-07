//! Admit the exact image and refresh declared existing readers before index4.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#proxy-index-upgrade");
use super::*;
use crate::{Index, StateError};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// An operation-local byte snapshot; capability is checked on these same bytes.
pub(crate) struct CapturedImage {
    pub(super) bytes: Vec<u8>,
    pub(super) reader: u8,
}
impl CapturedImage {
    pub(crate) fn read_for_reader(path: &Path, reader: u8) -> Result<Self, StateError> {
        let bytes = fs::read(path).map_err(|source| io("reading", path, source))?;
        if reader >= 4 {
            probe(&bytes, path, reader)?;
        }
        Ok(Self { bytes, reader })
    }
}
fn failure(path: &Path, reason: impl Into<String>) -> StateError {
    StateError::ProxyUpgrade {
        path: path.display().to_string(),
        reason: reason.into(),
    }
}
fn probe(bytes: &[u8], source: &Path, reader: u8) -> Result<(), StateError> {
    let temp = tempfile::tempdir().map_err(|e| io("creating proxy snapshot", source, e))?;
    let image = temp.path().join(proxy_file_name("vibe-app-proxy"));
    fs::write(&image, bytes).map_err(|e| io("writing proxy snapshot", &image, e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&image, fs::Permissions::from_mode(0o700))
            .map_err(|e| io("making proxy snapshot executable", &image, e))?;
    }
    for version in 3..=reader {
        query(
            &image,
            temp.path(),
            source,
            &format!("vibe-user-applications/{version}"),
        )?;
    }
    Ok(())
}
fn query(image: &Path, directory: &Path, source: &Path, protocol: &str) -> Result<(), StateError> {
    let output_path = directory.join("capability-output");
    let output_file = File::create(&output_path)
        .map_err(|e| io("creating capability output", &output_path, e))?;
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut command = Command::new(image);
    command
        .args(["--supports-index", protocol])
        .stdin(Stdio::null())
        .stdout(Stdio::from(output_file))
        .stderr(Stdio::null());
    let mut child = loop {
        match command.spawn() {
            Ok(child) => break child,
            // A concurrent fork can briefly inherit the just-closed writable
            // snapshot descriptor until exec. Retry only this OS condition,
            // on the same captured bytes, within the query's total deadline.
            Err(e)
                if e.kind() == std::io::ErrorKind::ExecutableFileBusy
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(e) => return Err(io("probing proxy snapshot", source, e)),
        }
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(failure(
                    source,
                    if result.is_err() {
                        "proxy capability query could not be observed"
                    } else {
                        "proxy capability query exceeded two seconds"
                    },
                ));
            }
        }
    };
    let mut output = Vec::new();
    File::open(&output_path)
        .map_err(|e| io("opening capability output", &output_path, e))?
        .take(64)
        .read_to_end(&mut output)
        .map_err(|e| io("reading proxy capability", source, e))?;
    if !status.success() || output != format!("{protocol}\n").as_bytes() {
        return Err(failure(
            source,
            format!("proxy image does not advertise {protocol} support"),
        ));
    }
    Ok(())
}

/// Only declared paths below the actual home are eligible. A root alias is
/// allowed; symlinks or special files inside the owned layout are refused.
pub(super) fn regular(settings: &SettingsRoot, path: &Path) -> Result<bool, StateError> {
    let relative = path
        .strip_prefix(settings.path())
        .map_err(|_| failure(path, "proxy path is outside the selected home"))?;
    let mut current = settings.path().to_path_buf();
    let components: Vec<_> = relative.components().collect();
    for (i, component) in components.iter().enumerate() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(failure(path, "proxy path contains a nonliteral component"));
        }
        current.push(component);
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(io("inspecting proxy path", &current, e)),
        };
        let valid = if i + 1 == components.len() {
            metadata.is_file()
        } else {
            metadata.is_dir()
        };
        if metadata.file_type().is_symlink() || !valid {
            return Err(failure(
                &current,
                "declared proxy path is occupied by a symlink or nonregular entry",
            ));
        }
    }
    Ok(true)
}
fn existing(settings: &SettingsRoot, previous: &Index) -> Result<Vec<PathBuf>, StateError> {
    let mut paths = BTreeSet::new();
    for command in crate::alternatives::reserved_names(previous) {
        if command_name(&command) {
            paths.insert(settings.proxy_dir().join(proxy_file_name(&command)));
        }
    }
    for record in previous.installations.values() {
        let directory = settings
            .app_proxies_dir()
            .join(record.package.directory_name())
            .join("bin");
        for command in &record.public_commands {
            if command_name(&command.name) {
                paths.insert(directory.join(proxy_file_name(&command.name)));
            }
        }
    }
    paths
        .into_iter()
        .filter_map(|path| match regular(settings, &path) {
            Ok(true) => Some(Ok(path)),
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        })
        .collect()
}

pub(crate) fn for_publication(
    settings: &SettingsRoot,
    next: &Index,
    image: &Path,
) -> Result<Option<CapturedImage>, StateError> {
    let (previous, stored_four) = Index::load_with_protocol(settings)?;
    let migrating = next.minimum_reader() > stored_four;
    let needed = crate::alternatives::reserved_names(next)
        .iter()
        .any(|name| command_name(name))
        || (migrating && !existing(settings, &previous)?.is_empty());
    needed
        .then(|| CapturedImage::read_for_reader(image, next.minimum_reader().max(stored_four)))
        .transpose()
}

/// Raw first4 publication with extant owned readers requires an admitted image.
/// Same4 metadata saves create no copies; all current copy creators check4.
pub(crate) fn before_publication(
    settings: &SettingsRoot,
    next: &Index,
    image: Option<&CapturedImage>,
) -> Result<(), StateError> {
    if next.minimum_reader() < 4 {
        return Ok(());
    }
    let (previous, stored_four) = Index::load_with_protocol(settings)?;
    if stored_four >= next.minimum_reader() {
        return Ok(());
    }
    let paths = existing(settings, &previous)?;
    if paths.is_empty() {
        return Ok(());
    }
    let image = image.filter(|i| i.reader >= next.minimum_reader()).ok_or_else(|| failure(&settings.index_path(), "new index protocol publication needs a compatible proxy image for existing commands; use the application operation API"))?;
    // Preflight the entire inventory before replacing any executable. No removal
    // or new session exposure occurs here; new bytes also understand index3.
    for path in paths {
        if fs::read(&path).map_err(|e| io("reading proxy", &path, e))? != image.bytes {
            super::place(&path, &image.bytes).map_err(|e| failure(&path, e.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;
