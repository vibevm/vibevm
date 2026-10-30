//! The `source_build` recipe runner: fetch and verify every declared input
//! (URL downloads cached by digest under `cache/app-sources/`), lay them out
//! in a fresh work directory under `cache/app-build/`, run the authored
//! script, and hand back a non-empty `VIBE_OUT` as the payload. A failed
//! build keeps its work directory for inspection; nothing else is recorded.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#source-build");

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::{RecipeDecl, ShellInterpreter, SourceInputDecl};

use crate::error::InstallerError;
use crate::resolve::strip_verbatim;

use super::archive::{self, ArchiveFormat};
use super::shell;

/// Everything one build needs, threaded in by the caller.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#source-build")]
pub struct SourceBuild<'a> {
    pub sources: &'a [SourceInputDecl],
    pub recipe: &'a RecipeDecl,
    /// The resolved package tree (`VIBE_PACKAGE`).
    pub package_root: &'a Path,
    /// `<settings>/cache/app-sources`.
    pub cache_root: &'a Path,
    /// `<settings>/cache/app-build/<group>.<name>@<version>-<pid>`.
    pub work: &'a Path,
    pub platform: &'a str,
    pub arch: &'a str,
    pub offline: bool,
    pub json: bool,
}

/// One verified input, ready to be laid out as `sources/<name>/`.
#[derive(Debug)]
struct Fetched {
    name: String,
    file: PathBuf,
    filename: String,
    extract: Option<ArchiveFormat>,
}

/// Fetch every input, then build into a fresh work directory and move
/// `VIBE_OUT` to `payload`. `work_created` turns true once the work
/// directory exists, so the caller knows a failure left it behind.
pub fn produce(
    build: &SourceBuild<'_>,
    payload: &Path,
    work_created: &mut bool,
) -> Result<(), InstallerError> {
    let fetched = build
        .sources
        .iter()
        .map(|source| fetch(build, source))
        .collect::<Result<Vec<_>, _>>()?;
    fresh_work(build.work)?;
    *work_created = true;
    lay_out(build.work, &fetched)?;
    let out = run_recipe(build)?;
    move_payload(&out, payload)
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
fn fetch(build: &SourceBuild<'_>, source: &SourceInputDecl) -> Result<Fetched, InstallerError> {
    let filename = source_filename(source)?;
    let extract = extraction(source, &filename)?;
    let sha256 = source.sha256.to_ascii_lowercase();
    let file = match (&source.url, &source.path) {
        (Some(url), _) => {
            let directory = build.cache_root.join(&sha256);
            let cached = directory.join(&filename);
            if digest_of(&cached).ok().as_deref() != Some(sha256.as_str()) {
                if build.offline {
                    return Err(InstallerError::Offline {
                        source_ref: url.clone(),
                    });
                }
                fs::create_dir_all(&directory)
                    .map_err(|source| super::io("creating", &directory, source))?;
                let partial = directory.join(format!(".{filename}.{}.part", std::process::id()));
                super::fetch_verified(url, &sha256, build.package_root, &partial, false)?;
                fs::rename(&partial, &cached).map_err(|source| {
                    let _ = fs::remove_file(&partial);
                    super::io("caching", &cached, source)
                })?;
            }
            cached
        }
        (None, Some(path)) => {
            let relative = path.to_string_lossy();
            let file = super::package_file(build.package_root, &relative).map_err(|reason| {
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

/// Create `<work>/sources` and an empty `<work>/out`, replacing a stale
/// directory of the same name.
fn fresh_work(work: &Path) -> Result<(), InstallerError> {
    if work.exists() {
        fs::remove_dir_all(work).map_err(|source| super::io("clearing", work, source))?;
    }
    for directory in [work.join("sources"), work.join("out")] {
        fs::create_dir_all(&directory)
            .map_err(|source| super::io("creating", &directory, source))?;
    }
    Ok(())
}

/// `sources/<name>/` holds the extracted archive or the file itself.
fn lay_out(work: &Path, fetched: &[Fetched]) -> Result<(), InstallerError> {
    for input in fetched {
        let destination = work.join("sources").join(&input.name);
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

/// Run the recipe script in `<work>` and return `VIBE_OUT` when the child
/// exits 0 and left a non-empty directory there.
fn run_recipe(build: &SourceBuild<'_>) -> Result<PathBuf, InstallerError> {
    let relative = build.recipe.script.to_string_lossy();
    let script = super::package_file(build.package_root, &relative)
        .map(strip_verbatim)
        .map_err(|reason| InstallerError::Fetch {
            source_ref: relative.into_owned(),
            reason,
        })?;
    let work = build.work;
    let out = work.join("out");
    let mut command = recipe_command(build.recipe.interpreter, &script)?;
    command
        .current_dir(work)
        .env("VIBE_SOURCES", work.join("sources"))
        .env("VIBE_WORK", work)
        .env("VIBE_OUT", &out)
        .env("VIBE_PACKAGE", build.package_root)
        .env("VIBE_PLATFORM", build.platform)
        .env("VIBE_ARCH", build.arch);
    super::run_child(&mut command, "source_build recipe", build.json)?;
    let populated = fs::read_dir(&out).is_ok_and(|mut entries| entries.next().is_some());
    if !out.is_dir() || !populated {
        return Err(InstallerError::EmptyOutput {
            path: out.display().to_string(),
        });
    }
    Ok(out)
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
fn move_payload(out: &Path, payload: &Path) -> Result<(), InstallerError> {
    if fs::rename(out, payload).is_ok() {
        return Ok(());
    }
    crate::fs_util::copy_tree(out, payload)
        .map_err(|failure| super::io(failure.action, &failure.path, failure.source))
}

fn digest_of(path: &Path) -> io::Result<String> {
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

    struct Tree {
        _dir: tempfile::TempDir,
        package: PathBuf,
        cache: PathBuf,
        work: PathBuf,
        recipe: RecipeDecl,
    }

    fn tree() -> Tree {
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        fs::create_dir_all(&package).unwrap();
        Tree {
            package,
            cache: dir.path().join("cache/app-sources"),
            work: dir.path().join("cache/app-build/org.example.demo@1.0.0-1"),
            recipe: RecipeDecl {
                interpreter: None,
                script: PathBuf::from("build.sh"),
            },
            _dir: dir,
        }
    }

    fn build<'a>(tree: &'a Tree, sources: &'a [SourceInputDecl], offline: bool) -> SourceBuild<'a> {
        SourceBuild {
            sources,
            recipe: &tree.recipe,
            package_root: &tree.package,
            cache_root: &tree.cache,
            work: &tree.work,
            platform: "linux",
            arch: "x86_64",
            offline,
            json: true,
        }
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
        let tree = tree();
        let digest = sha(b"upstream");
        let url = "http://127.0.0.1:9/dist/upstream.txt";
        let sources = [input("up", Some(url), None, &digest)];
        let missing = fetch(&build(&tree, &sources, true), &sources[0]).unwrap_err();
        assert!(
            matches!(missing, InstallerError::Offline { .. }),
            "{missing}"
        );

        let cached = tree.cache.join(&digest).join("upstream.txt");
        fs::create_dir_all(cached.parent().unwrap()).unwrap();
        fs::write(&cached, b"tampered").unwrap();
        let stale = fetch(&build(&tree, &sources, true), &sources[0]).unwrap_err();
        assert!(matches!(stale, InstallerError::Offline { .. }), "{stale}");

        fs::write(&cached, b"upstream").unwrap();
        let hit = fetch(&build(&tree, &sources, true), &sources[0]).unwrap();
        assert_eq!(hit.file, cached);
        assert_eq!(hit.filename, "upstream.txt");
        assert_eq!(hit.extract, None);
    }

    #[test]
    fn path_sources_are_verified_and_laid_out_without_a_cache() {
        let tree = tree();
        fs::write(tree.package.join("fix.patch"), b"patch").unwrap();
        let wrong = [input("fix", None, Some("fix.patch"), &"0".repeat(64))];
        let mut created = false;
        let refused = produce(&build(&tree, &wrong, false), &tree.work, &mut created);
        assert!(matches!(refused, Err(InstallerError::Digest { .. })));
        assert!(!created && !tree.work.exists(), "refused before any work");

        let sources = [input("fix", None, Some("fix.patch"), &sha(b"patch"))];
        let fetched = fetch(&build(&tree, &sources, true), &sources[0]).unwrap();
        fresh_work(&tree.work).unwrap();
        lay_out(&tree.work, &[fetched]).unwrap();
        assert_eq!(
            fs::read(tree.work.join("sources/fix/fix.patch")).unwrap(),
            b"patch"
        );
        assert!(!tree.cache.exists(), "path inputs are never cached");
    }

    #[cfg(unix)]
    #[test]
    fn an_empty_out_fails_and_keeps_the_work_directory() {
        let tree = tree();
        fs::write(tree.package.join("data.txt"), b"data").unwrap();
        fs::write(
            tree.package.join("build.sh"),
            "test \"$PWD\" = \"$VIBE_WORK\"\n",
        )
        .unwrap();
        let sources = [input("data", None, Some("data.txt"), &sha(b"data"))];
        let payload = tree.package.join("payload");
        let mut created = false;
        let error = produce(&build(&tree, &sources, true), &payload, &mut created).unwrap_err();
        assert!(
            matches!(error, InstallerError::EmptyOutput { .. }),
            "{error}"
        );
        assert!(created);
        assert!(tree.work.join("sources/data/data.txt").is_file());
        assert!(!payload.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_recipe_sees_its_variables_and_its_out_becomes_the_payload() {
        let tree = tree();
        let archive = tree.package.join("src.zip");
        let mut zip = zip::ZipWriter::new(File::create(&archive).unwrap());
        zip.start_file("hello.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        io::Write::write_all(&mut zip, b"hello").unwrap();
        zip.finish().unwrap();
        fs::write(
            tree.package.join("build.sh"),
            "set -e\nmkdir \"$VIBE_OUT/bin\"\ncat \"$VIBE_SOURCES/src/hello.txt\" > \"$VIBE_OUT/bin/hello\"\n\
             echo \"$VIBE_PLATFORM/$VIBE_ARCH\" > \"$VIBE_OUT/target\"\ntest -f \"$VIBE_PACKAGE/build.sh\"\n",
        )
        .unwrap();
        let sources = [input(
            "src",
            None,
            Some("src.zip"),
            &digest_of(&archive).unwrap(),
        )];
        let payload = tree.package.join("payload");
        let mut created = false;
        produce(&build(&tree, &sources, true), &payload, &mut created).unwrap();
        assert_eq!(fs::read(payload.join("bin/hello")).unwrap(), b"hello");
        assert_eq!(
            fs::read_to_string(payload.join("target")).unwrap(),
            "linux/x86_64\n"
        );

        let copied = tree.package.join("copied");
        crate::fs_util::copy_tree(&payload, &copied).unwrap();
        assert_eq!(fs::read(copied.join("bin/hello")).unwrap(), b"hello");
    }
}
