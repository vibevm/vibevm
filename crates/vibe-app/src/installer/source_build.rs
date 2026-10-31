//! The parts of a source build the `sources` and `recipe` steps reuse:
//! fetch and verify one declared input (URL downloads cached by digest
//! under `cache/app-sources/`), lay inputs out as `<work>/sources/<name>/`,
//! build the child that runs a package-relative script, and move a
//! finished `VIBE_OUT` into place.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#source-build");

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use vibe_core::manifest::{ShellInterpreter, SourceInputDecl};

use crate::error::InstallerError;
use crate::resolve::strip_verbatim;

use super::archive::{self, ArchiveFormat};
use super::shell;

/// One verified input, ready to be laid out as `sources/<name>/`.
#[derive(Debug)]
pub(crate) struct Fetched {
    name: String,
    pub(crate) file: PathBuf,
    filename: String,
    extract: Option<ArchiveFormat>,
}

/// The file name a source keeps: the last URL path segment (`source` when
/// empty) or the file name of its package path.
fn source_filename(source: &SourceInputDecl) -> Result<String, InstallerError> {
    let raw = match (&source.url, &source.path) {
        (Some(url), _) => {
            let rest = url.split_once("://").map_or(url.as_str(), |(_, rest)| rest);
            let segment = rest
                .split_once('/')
                .map_or("", |(_, path)| path.rsplit('/').next().unwrap_or(""));
            if segment.is_empty() {
                "source".to_owned()
            } else {
                segment.to_owned()
            }
        }
        (None, Some(path)) => path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        (None, None) => String::new(),
    };
    let single = archive::portable_path(&raw).is_ok_and(|path| path.components().count() == 1)
        && raw != "."
        && raw != "..";
    if !single {
        return Err(layout(
            &source.name,
            format!("`{raw}` is not a portable file name"),
        ));
    }
    Ok(raw)
}

/// Whether a source is extracted: `extract` when declared, otherwise true
/// exactly for `.zip`, `.tar.gz` and `.tgz` file names.
fn extraction(
    source: &SourceInputDecl,
    filename: &str,
) -> Result<Option<ArchiveFormat>, InstallerError> {
    let lower = filename.to_ascii_lowercase();
    let format = if lower.ends_with(".zip") {
        Some(ArchiveFormat::Zip)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        Some(ArchiveFormat::TarGz)
    } else {
        None
    };
    match (source.extract, format) {
        (Some(false), _) => Ok(None),
        (Some(true), None) => Err(layout(
            &source.name,
            format!("`{filename}` is not a .zip, .tar.gz or .tgz archive but extract = true"),
        )),
        (_, format) => Ok(format),
    }
}

/// A URL input comes from `<cache>/<sha256>/<filename>` when the cached
/// file still matches its digest, otherwise from the network into the
/// cache; a path input is the verified package file itself.
pub(crate) fn fetch(
    cache_root: &Path,
    package_root: &Path,
    source: &SourceInputDecl,
    offline: bool,
) -> Result<Fetched, InstallerError> {
    let filename = source_filename(source)?;
    let extract = extraction(source, &filename)?;
    let sha256 = source.sha256.to_ascii_lowercase();
    let file = match (&source.url, &source.path) {
        (Some(url), _) => {
            let directory = cache_root.join(&sha256);
            let cached = directory.join(&filename);
            if digest_of(&cached).ok().as_deref() != Some(sha256.as_str()) {
                if offline {
                    return Err(InstallerError::Offline {
                        source_ref: url.clone(),
                    });
                }
                fs::create_dir_all(&directory)
                    .map_err(|source| super::io("creating", &directory, source))?;
                let partial = directory.join(format!(".{filename}.{}.part", std::process::id()));
                super::fetch_verified(url, &sha256, package_root, &partial, false)?;
                fs::rename(&partial, &cached).map_err(|source| {
                    let _ = fs::remove_file(&partial);
                    super::io("caching", &cached, source)
                })?;
            }
            cached
        }
        (None, Some(path)) => {
            let relative = path.to_string_lossy();
            let file = super::package_file(package_root, &relative).map_err(|reason| {
                InstallerError::Fetch {
                    source_ref: relative.clone().into_owned(),
                    reason,
                }
            })?;
            let actual = digest_of(&file).map_err(|source| super::io("reading", &file, source))?;
            if actual != sha256 {
                return Err(InstallerError::Digest {
                    source_ref: relative.into_owned(),
                });
            }
            file
        }
        (None, None) => {
            return Err(layout(
                &source.name,
                "it declares neither url nor path".into(),
            ));
        }
    };
    Ok(Fetched {
        name: source.name.clone(),
        file,
        filename,
        extract,
    })
}

/// `<sources>/<name>/` holds the extracted archive or the file itself; a
/// stale directory of the same name is replaced.
pub(crate) fn lay_out(sources: &Path, fetched: &[Fetched]) -> Result<(), InstallerError> {
    fs::create_dir_all(sources).map_err(|source| super::io("creating", sources, source))?;
    for input in fetched {
        let destination = sources.join(&input.name);
        if destination.exists() {
            fs::remove_dir_all(&destination)
                .map_err(|source| super::io("clearing", &destination, source))?;
        }
        match input.extract {
            Some(format) => archive::extract(&input.file, format, None, &destination)?,
            None => {
                fs::create_dir(&destination)
                    .map_err(|source| super::io("creating", &destination, source))?;
                let target = destination.join(&input.filename);
                fs::copy(&input.file, &target)
                    .map_err(|source| super::io("copying", &target, source))?;
            }
        }
    }
    Ok(())
}

/// The child that runs a package-relative recipe script.
pub(crate) fn script_command(
    interpreter: Option<ShellInterpreter>,
    package_root: &Path,
    relative: &Path,
) -> Result<Command, InstallerError> {
    let relative = relative.to_string_lossy();
    let script = super::package_file(package_root, &relative)
        .map(strip_verbatim)
        .map_err(|reason| InstallerError::Fetch {
            source_ref: relative.into_owned(),
            reason,
        })?;
    recipe_command(interpreter, &script)
}

/// The recipe child: `/bin/sh <script>`, `cmd.exe /D /S /C <script>` or
/// `powershell.exe … -File <script>`.
fn recipe_command(
    interpreter: Option<ShellInterpreter>,
    script: &Path,
) -> Result<Command, InstallerError> {
    let interpreter = interpreter.unwrap_or_else(shell::host_default);
    let mut command = match interpreter {
        ShellInterpreter::PosixSh => Command::new(if cfg!(windows) { "sh" } else { "/bin/sh" }),
        ShellInterpreter::WindowsCmd if cfg!(windows) => {
            let mut command = Command::new("cmd.exe");
            command.args(["/D", "/S", "/C"]);
            command
        }
        ShellInterpreter::WindowsPowershell if cfg!(windows) => {
            let mut command = Command::new("powershell.exe");
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ]);
            command
        }
        other => {
            return Err(InstallerError::Interpreter {
                interpreter: shell::interpreter_name(other),
            });
        }
    };
    command.arg(script);
    Ok(command)
}

/// Move `VIBE_OUT` to `payload`; across filesystems, copy it instead.
pub(crate) fn move_payload(out: &Path, payload: &Path) -> Result<(), InstallerError> {
    if fs::rename(out, payload).is_ok() {
        return Ok(());
    }
    crate::fs_util::copy_tree(out, payload)
        .map_err(|failure| super::io(failure.action, &failure.path, failure.source))
}

pub(crate) fn digest_of(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(super::hex(&hash.finalize()))
}

fn layout(name: &str, reason: String) -> InstallerError {
    InstallerError::SourceLayout {
        name: name.into(),
        reason,
    }
}

/// The hex digest of `bytes`, for tests elsewhere in the crate.
#[cfg(test)]
pub(crate) fn tests_digest(bytes: &[u8]) -> String {
    super::hex(&Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, url: Option<&str>, path: Option<&str>, sha256: &str) -> SourceInputDecl {
        SourceInputDecl {
            name: name.into(),
            url: url.map(Into::into),
            path: path.map(PathBuf::from),
            sha256: sha256.into(),
            extract: None,
        }
    }

    fn sha(bytes: &[u8]) -> String {
        super::super::hex(&Sha256::digest(bytes))
    }

    #[test]
    fn file_names_and_extraction_default_by_extension() {
        let named = |url: &str| source_filename(&input("s", Some(url), None, "")).unwrap();
        assert_eq!(named("https://host/a/tool-1.0.tar.gz"), "tool-1.0.tar.gz");
        assert_eq!(named("https://host/"), "source");
        assert_eq!(named("https://host"), "source");
        let local = input("s", None, Some("vendor/fix.patch"), "");
        assert_eq!(source_filename(&local).unwrap(), "fix.patch");
        let mut source = input("s", None, Some("x"), "");
        for (file, expected) in [
            ("a.zip", Some(ArchiveFormat::Zip)),
            ("a.TAR.GZ", Some(ArchiveFormat::TarGz)),
            ("a.tgz", Some(ArchiveFormat::TarGz)),
            ("a.tar", None),
            ("fix.patch", None),
        ] {
            assert_eq!(extraction(&source, file).unwrap(), expected, "{file}");
        }
        source.extract = Some(false);
        assert_eq!(extraction(&source, "a.zip").unwrap(), None);
        source.extract = Some(true);
        assert!(matches!(
            extraction(&source, "fix.patch"),
            Err(InstallerError::SourceLayout { .. })
        ));
    }

    #[test]
    fn url_sources_are_cached_by_digest_and_offline_needs_a_hit() {
        let dir = tempfile::tempdir().unwrap();
        let (package, cache) = (dir.path().join("package"), dir.path().join("cache"));
        let digest = sha(b"upstream");
        let source = input(
            "up",
            Some("http://127.0.0.1:9/dist/upstream.txt"),
            None,
            &digest,
        );
        let missing = fetch(&cache, &package, &source, true).unwrap_err();
        assert!(
            matches!(missing, InstallerError::Offline { .. }),
            "{missing}"
        );
        let cached = cache.join(&digest).join("upstream.txt");
        fs::create_dir_all(cached.parent().unwrap()).unwrap();
        fs::write(&cached, b"tampered").unwrap();
        let stale = fetch(&cache, &package, &source, true).unwrap_err();
        assert!(matches!(stale, InstallerError::Offline { .. }), "{stale}");
        fs::write(&cached, b"upstream").unwrap();
        let hit = fetch(&cache, &package, &source, true).unwrap();
        assert_eq!(hit.file, cached);
        assert_eq!(hit.extract, None);
    }

    #[test]
    fn path_sources_are_verified_and_laid_out_replacing_stale_ones() {
        let dir = tempfile::tempdir().unwrap();
        let (package, cache) = (dir.path().join("package"), dir.path().join("cache"));
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("fix.patch"), b"patch").unwrap();
        let wrong = input("fix", None, Some("fix.patch"), &"0".repeat(64));
        assert!(matches!(
            fetch(&cache, &package, &wrong, false),
            Err(InstallerError::Digest { .. })
        ));
        let right = input("fix", None, Some("fix.patch"), &sha(b"patch"));
        let sources = dir.path().join("work/sources");
        fs::create_dir_all(sources.join("fix")).unwrap();
        fs::write(sources.join("fix/stale"), b"old").unwrap();
        let fetched = fetch(&cache, &package, &right, true).unwrap();
        lay_out(&sources, &[fetched]).unwrap();
        assert_eq!(fs::read(sources.join("fix/fix.patch")).unwrap(), b"patch");
        assert!(!sources.join("fix/stale").exists());
        assert!(!cache.exists(), "path inputs are never cached");
    }

    #[cfg(unix)]
    #[test]
    fn a_script_runs_contained_and_its_out_moves_into_place() {
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("build.sh"),
            "mkdir -p \"$OUT/bin\" && echo hi > \"$OUT/bin/x\"\n",
        )
        .unwrap();
        let out = dir.path().join("out");
        let mut command = script_command(None, &package, Path::new("build.sh")).unwrap();
        command.env("OUT", &out);
        super::super::run_child(&mut command, "recipe", true).unwrap();
        let payload = dir.path().join("payload");
        move_payload(&out, &payload).unwrap();
        assert_eq!(fs::read(payload.join("bin/x")).unwrap(), b"hi\n");
        assert!(script_command(None, &package, Path::new("../escape.sh")).is_err());
    }
}
