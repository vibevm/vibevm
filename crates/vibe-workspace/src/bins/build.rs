//! Binary build authorization and the shared in-slot cargo wire.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use specmark::spec;

use super::{BinsError, DeclaredBinary};
#[cfg(test)]
use crate::cargo_build::{CARGO_BUILD_ENV_KEYS, environment_key_matches};
use crate::cargo_build::{
    CargoToolchainEnvironment, cargo_build_environment, system_cargo_toolchain,
};
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

#[derive(Debug)]
struct CargoBuildInvocation {
    manifest_path: PathBuf,
    binary_name: String,
    environment: Vec<(OsString, OsString)>,
    output: BuildOutput,
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
    fn cargo_build_runner_receives_only_the_fixed_toolchain_environment() {
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
