//! Binary build authorization and the shared in-slot cargo wire.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use specmark::spec;

use super::{BinsError, DeclaredBinary};
use crate::vibedeps;

/// Why this process is authorized to build package-supplied code.
///
/// Direct operator commands preserve the historical allow-list / explicit
/// `--assume-yes` gate. An installed extension is already authorized by the
/// dependency selection that installed its provider (PROP-054).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
pub enum BinaryProviderHome {
    InstalledSlot,
    AuthoredPackageRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#INSTALL-IS-CONSENT")]
pub enum BuildAuthorization {
    /// A direct `vibe bin build`-genre request.
    ExplicitOperator { assume_yes: bool },
    /// A binary handler supplied by an installed extension provider.
    InstalledExtension { home: BinaryProviderHome },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
pub enum BuildOutput {
    Inherit,
    Quiet,
}

/// The complete environment inherited by package-supplied cargo builds.
///
/// This list is deliberately positive and small: it contains only process,
/// home, temporary-directory, locale, and Rust toolchain discovery inputs.
/// Credentials, registry/publish settings, proxies, wrappers, and compiler
/// flags are consequently absent even when the parent `vibe` process has
/// them.
const CARGO_BUILD_ENV_KEYS: &[&str] = &[
    "PATH",
    "SystemRoot",
    "WINDIR",
    "HOME",
    "USERPROFILE",
    "TEMP",
    "TMP",
    "LANG",
    "LC_ALL",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
];

#[derive(Debug)]
struct CargoBuildInvocation {
    manifest_path: PathBuf,
    binary_name: String,
    environment: Vec<(OsString, OsString)>,
    output: BuildOutput,
}

#[derive(Clone, Debug, Default)]
struct CargoToolchainEnvironment {
    path_entries: Vec<PathBuf>,
    library_dirs: Vec<PathBuf>,
}

trait CargoBuildRunner {
    fn run(&self, invocation: &CargoBuildInvocation) -> io::Result<bool>;
}

struct SystemCargoBuildRunner;

impl CargoBuildRunner for SystemCargoBuildRunner {
    fn run(&self, invocation: &CargoBuildInvocation) -> io::Result<bool> {
        let mut command = Command::new("cargo");
        command
            .arg("build")
            .arg("--release")
            .arg("--manifest-path")
            .arg(&invocation.manifest_path)
            .arg("--bin")
            .arg(&invocation.binary_name)
            .env_clear()
            .envs(invocation.environment.iter().cloned());
        if invocation.output == BuildOutput::Quiet {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        command.status().map(|status| status.success())
    }
}

fn cargo_build_environment(
    parent: impl IntoIterator<Item = (OsString, OsString)>,
    toolchain: &CargoToolchainEnvironment,
) -> Vec<(OsString, OsString)> {
    let parent = parent.into_iter().collect::<Vec<_>>();
    let mut environment = CARGO_BUILD_ENV_KEYS
        .iter()
        .filter_map(|allowed| {
            parent
                .iter()
                .find(|(key, _)| environment_key_matches(key, allowed))
                .map(|(_, value)| (OsString::from(allowed), value.clone()))
        })
        .collect::<Vec<_>>();
    extend_path(&mut environment, toolchain.path_entries.iter().cloned());
    if !toolchain.library_dirs.is_empty()
        && let Ok(libraries) = std::env::join_paths(&toolchain.library_dirs)
    {
        // MSVC's linker does not search PATH for the Windows SDK libraries.
        // This LIB is derived exclusively from discovered toolchain directories;
        // an ambient parent LIB is never copied into package build code.
        environment.push(("LIB".into(), libraries));
    }
    environment
}

fn environment_key_matches(actual: &OsStr, allowed: &str) -> bool {
    if cfg!(windows) {
        actual.to_string_lossy().eq_ignore_ascii_case(allowed)
    } else {
        actual == OsStr::new(allowed)
    }
}

fn extend_path(
    environment: &mut [(OsString, OsString)],
    additions: impl IntoIterator<Item = PathBuf>,
) {
    let Some((_, path)) = environment
        .iter_mut()
        .find(|(key, _)| environment_key_matches(key, "PATH"))
    else {
        return;
    };
    // Toolchain executables must win over same-named utilities in the ambient
    // PATH (Git for Windows ships a POSIX `link.exe`, for example). Keep the
    // inherited PATH as fallback so `cargo`/`rustup` remain discoverable.
    let inherited = std::env::split_paths(path).collect::<Vec<_>>();
    let mut entries = Vec::new();
    for addition in additions {
        if !entries.iter().any(|entry| entry == &addition) {
            entries.push(addition);
        }
    }
    for entry in inherited {
        if !entries.iter().any(|existing| existing == &entry) {
            entries.push(entry);
        }
    }
    if let Ok(joined) = std::env::join_paths(entries) {
        *path = joined;
    }
}

#[cfg(not(windows))]
fn system_cargo_toolchain(_parent: &[(OsString, OsString)]) -> CargoToolchainEnvironment {
    CargoToolchainEnvironment::default()
}

#[cfg(windows)]
fn system_cargo_toolchain(parent: &[(OsString, OsString)]) -> CargoToolchainEnvironment {
    use std::sync::OnceLock;

    static DISCOVERED: OnceLock<CargoToolchainEnvironment> = OnceLock::new();
    DISCOVERED
        .get_or_init(|| discover_msvc_toolchain(parent))
        .clone()
}

#[cfg(windows)]
fn discover_msvc_toolchain(parent: &[(OsString, OsString)]) -> CargoToolchainEnvironment {
    let mut candidates = Vec::new();
    for key in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        let Some((_, value)) = parent
            .iter()
            .find(|(actual, _)| environment_key_matches(actual, key))
        else {
            continue;
        };
        collect_msvc_linker_dirs(
            &PathBuf::from(value).join("Microsoft Visual Studio"),
            &mut candidates,
        );
    }
    candidates.sort();
    candidates.dedup();
    let linker_dir = candidates
        .iter()
        .rev()
        .find(|path| path.ends_with("bin/Hostx64/x64"))
        .or_else(|| candidates.last())
        .cloned();
    let mut library_dirs = Vec::new();
    if let Some(linker_dir) = &linker_dir
        && let Some(msvc_version_root) = linker_dir.ancestors().nth(3)
    {
        let msvc_libraries = msvc_version_root.join("lib").join(msvc_architecture());
        if msvc_libraries.is_dir() {
            library_dirs.push(msvc_libraries);
        }
    }
    collect_windows_sdk_library_dirs(parent, &mut library_dirs);
    library_dirs.sort();
    library_dirs.dedup();
    CargoToolchainEnvironment {
        path_entries: linker_dir.into_iter().collect(),
        library_dirs,
    }
}

#[cfg(windows)]
fn collect_windows_sdk_library_dirs(
    parent: &[(OsString, OsString)],
    library_dirs: &mut Vec<PathBuf>,
) {
    for key in ["ProgramFiles(x86)", "ProgramFiles", "ProgramW6432"] {
        let Some((_, value)) = parent
            .iter()
            .find(|(actual, _)| environment_key_matches(actual, key))
        else {
            continue;
        };
        let root = PathBuf::from(value).join("Windows Kits/10/Lib");
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        let mut versions = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        versions.sort();
        let Some(latest) = versions.last() else {
            continue;
        };
        for family in ["ucrt", "um"] {
            let directory = latest.join(family).join(msvc_architecture());
            if directory.is_dir() {
                library_dirs.push(directory);
            }
        }
    }
}

#[cfg(windows)]
fn msvc_architecture() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "arm"
    }
}

#[cfg(windows)]
fn collect_msvc_linker_dirs(root: &std::path::Path, candidates: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_msvc_linker_dirs(&path, candidates);
        } else if path
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("link.exe"))
            && let Some(parent) = path.parent()
        {
            candidates.push(parent.to_path_buf());
        }
    }
}

/// The PROP-020-shaped consent gate retained for direct binary builds.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
pub fn consent_to_build(bin: &DeclaredBinary, assume_yes: bool) -> Result<(), BinsError> {
    if bin.group == "org.vibevm" || assume_yes {
        return Ok(());
    }
    Err(BinsError::ConsentRequired {
        name: bin.decl.name.clone(),
        package: bin.package.clone(),
        group: bin.group.clone(),
    })
}

pub(super) fn authorize_build(
    bin: &DeclaredBinary,
    authorization: BuildAuthorization,
) -> Result<(), BinsError> {
    match authorization {
        BuildAuthorization::ExplicitOperator { assume_yes } => consent_to_build(bin, assume_yes),
        BuildAuthorization::InstalledExtension { .. } => Ok(()),
    }
}

/// Historical direct-build entry point. Its bool retains exactly the old
/// operator semantics and is translated to the explicit enum at the seam.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
pub fn build_binary(bin: &DeclaredBinary, assume_yes: bool) -> Result<(), BinsError> {
    build_binary_authorized(bin, BuildAuthorization::ExplicitOperator { assume_yes })
}

/// Build one declared binary under an explicit authorization provenance.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
pub fn build_binary_authorized(
    bin: &DeclaredBinary,
    authorization: BuildAuthorization,
) -> Result<(), BinsError> {
    build_binary_authorized_with_output(bin, authorization, BuildOutput::Inherit)
}

#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
pub fn build_binary_authorized_with_output(
    bin: &DeclaredBinary,
    authorization: BuildAuthorization,
    output: BuildOutput,
) -> Result<(), BinsError> {
    authorize_build(bin, authorization)?;
    build_binary_inner(bin, authorization, output)
}

fn build_binary_inner(
    bin: &DeclaredBinary,
    authorization: BuildAuthorization,
    output: BuildOutput,
) -> Result<(), BinsError> {
    let parent_environment = std::env::vars_os().collect::<Vec<_>>();
    let toolchain = system_cargo_toolchain(&parent_environment);
    build_binary_inner_with_runner(
        bin,
        authorization,
        output,
        &SystemCargoBuildRunner,
        parent_environment,
        toolchain,
    )
}

fn build_binary_inner_with_runner(
    bin: &DeclaredBinary,
    authorization: BuildAuthorization,
    output: BuildOutput,
    runner: &dyn CargoBuildRunner,
    parent_environment: impl IntoIterator<Item = (OsString, OsString)>,
    toolchain: CargoToolchainEnvironment,
) -> Result<(), BinsError> {
    if output == BuildOutput::Inherit {
        eprintln!(
            "bin build: `{}` ({}) — cargo build --release in {}",
            bin.decl.name,
            bin.package,
            bin.slot.display()
        );
    }
    if !matches!(
        authorization,
        BuildAuthorization::InstalledExtension {
            home: BinaryProviderHome::AuthoredPackageRoot
        }
    ) {
        prepare_build_output_ignores(bin)?;
    }
    let invocation = CargoBuildInvocation {
        manifest_path: bin.slot.join("Cargo.toml"),
        binary_name: bin.decl.name.clone(),
        environment: cargo_build_environment(parent_environment, &toolchain),
        output,
    };
    let success = runner.run(&invocation).map_err(|e| BinsError::CargoSpawn {
        name: bin.decl.name.clone(),
        detail: e.to_string(),
    })?;
    if !success {
        return Err(BinsError::BuildFailed {
            name: bin.decl.name.clone(),
        });
    }
    if !bin.release_artifact().exists() {
        return Err(BinsError::ArtifactMissing {
            artifact: bin.release_artifact(),
        });
    }
    Ok(())
}

#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build")]
pub(super) fn prepare_build_output_ignores(bin: &DeclaredBinary) -> Result<(), BinsError> {
    let root = &bin.vibedeps_root;
    if root.file_name() != Some(OsStr::new(vibedeps::VIBEDEPS_DIR)) {
        return Err(BinsError::MalformedSlot {
            name: bin.decl.name.clone(),
            slot: bin.slot.clone(),
            root: root.clone(),
            reason: format!(
                "the authoritative root's final component must be exactly `{}`",
                vibedeps::VIBEDEPS_DIR
            ),
        });
    }
    let relative = bin
        .slot
        .strip_prefix(root)
        .map_err(|_| BinsError::MalformedSlot {
            name: bin.decl.name.clone(),
            slot: bin.slot.clone(),
            root: root.clone(),
            reason: "slot is not beneath the authoritative root".to_string(),
        })?;
    let components: Vec<_> = relative.components().collect();
    if components.len() != 2
        || !components
            .iter()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(BinsError::MalformedSlot {
            name: bin.decl.name.clone(),
            slot: bin.slot.clone(),
            root: root.clone(),
            reason: "slot must be exactly `<root>/<coordinate>/<version>` using two normal path components"
                .to_string(),
        });
    }
    vibedeps::ensure_build_output_ignores(root).map_err(|error| BinsError::BuildOutputIgnore {
        name: bin.decl.name.clone(),
        root: root.to_path_buf(),
        detail: error.to_string(),
    })?;
    Ok(())
}

#[cfg(test)]
mod environment_tests {
    use std::sync::Mutex;

    use vibe_core::manifest::BinaryDecl;

    use super::*;

    #[derive(Default)]
    struct RecordingRunner {
        environment: Mutex<Vec<(OsString, OsString)>>,
    }

    impl CargoBuildRunner for RecordingRunner {
        fn run(&self, invocation: &CargoBuildInvocation) -> io::Result<bool> {
            *self.environment.lock().unwrap() = invocation.environment.clone();
            Ok(true)
        }
    }

    #[test]
    fn cargo_runner_receives_only_the_fixed_toolchain_environment() {
        let temp = tempfile::tempdir().unwrap();
        let bin = DeclaredBinary {
            decl: BinaryDecl {
                name: "fixture".into(),
                crate_dir: ".".into(),
                description: None,
            },
            package: "org.example/fixture".into(),
            group: "org.example".into(),
            vibedeps_root: temp.path().join("unused-vibedeps"),
            slot: temp.path().to_path_buf(),
        };
        let artifact = bin.release_artifact();
        std::fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        std::fs::write(&artifact, b"fixture").unwrap();

        let mut parent = CARGO_BUILD_ENV_KEYS
            .iter()
            .map(|key| (OsString::from(key), OsString::from("allowed")))
            .collect::<Vec<_>>();
        let ambient_tool = temp.path().join("ambient/bin");
        std::fs::create_dir_all(&ambient_tool).unwrap();
        std::fs::write(ambient_tool.join("link.exe"), b"wrong linker").unwrap();
        let ambient_path = std::env::join_paths([ambient_tool.clone()]).unwrap();
        parent
            .iter_mut()
            .find(|(key, _)| environment_key_matches(key, "PATH"))
            .unwrap()
            .1 = ambient_path;
        parent.extend([
            ("VIBEVM_PUBLISH_TOKEN_GITHUB".into(), "secret".into()),
            ("CARGO_REGISTRIES_PRIVATE_TOKEN".into(), "secret".into()),
            ("HTTPS_PROXY".into(), "http://proxy".into()),
            ("RUSTC_WRAPPER".into(), "wrapper".into()),
            ("RUSTFLAGS".into(), "--cfg leaked".into()),
            ("LIB".into(), "ambient-library-path".into()),
        ]);
        let derived_library = temp.path().join("toolchain/lib");
        let derived_tool = temp.path().join("toolchain/bin");
        std::fs::create_dir_all(&derived_tool).unwrap();
        std::fs::write(derived_tool.join("link.exe"), b"MSVC linker").unwrap();
        let runner = RecordingRunner::default();

        build_binary_inner_with_runner(
            &bin,
            BuildAuthorization::InstalledExtension {
                home: BinaryProviderHome::AuthoredPackageRoot,
            },
            BuildOutput::Quiet,
            &runner,
            parent,
            CargoToolchainEnvironment {
                path_entries: vec![derived_tool.clone()],
                library_dirs: vec![derived_library.clone()],
            },
        )
        .unwrap();

        let environment = runner.environment.lock().unwrap();
        let actual = environment
            .iter()
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let mut expected = CARGO_BUILD_ENV_KEYS
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        expected.push("LIB".into());
        assert_eq!(actual, expected);
        let path_value = environment
            .iter()
            .find(|(key, _)| environment_key_matches(key, "PATH"))
            .map(|(_, value)| value)
            .unwrap();
        assert_eq!(
            std::env::split_paths(path_value).collect::<Vec<_>>(),
            [derived_tool, ambient_tool]
        );
        let derived_lib_value = environment
            .iter()
            .find(|(key, _)| key == "LIB")
            .map(|(_, value)| value)
            .unwrap();
        assert_eq!(
            std::env::split_paths(derived_lib_value).collect::<Vec<_>>(),
            [derived_library]
        );
    }
}
