//! The builtin Cargo build provider — §5's seven laws, in process.
//!
//! 1. `cargo metadata` resolves the selected workspace/package/target
//!    before a compile starts, so a `select` typo refuses in a metadata
//!    call instead of after a full build;
//! 2. every invocation is an **argv**, never a shell string;
//! 3. the produced executable is taken ONLY from a `compiler-artifact`
//!    message — never from a guessed `target/<profile>/<name>` path;
//! 4. the target's `config` table is structured and validated at `plan`;
//! 5. Cargo/Rust toolchain identity enters the evidence, and the
//!    authoritative freshness probe is delegated to Cargo rather than
//!    skipped from an incomplete Vibe-side source census;
//! 6. the chosen output is verified and its digest recorded;
//! 7. Cargo owns its internal incremental compilation while VibeVM owns
//!    graph ordering, the output root and the artifact hand-off.
//!
//! Nothing here reads the operator's settings home. The child processes
//! inherit the ambient toolchain environment they need (`PATH`, `CARGO_*`,
//! `RUSTUP_*`) and receive no VibeVM-sourced bytes at all — no secret can
//! travel a path that does not exist.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::{ArtifactInput, ArtifactKind};

pub(crate) mod config;
pub(crate) mod message;

use crate::mechanism::contain::{FileFault, digest_file, forward_slashed, relative_to};
use crate::mechanism::error::preview;
use crate::mechanism::{
    BUILTIN_CARGO_PIN, BuildProvider, BuildTargetRequest, EffectClass, MechanismError, NetworkUse,
    PrivilegeNeed, ProviderDescriptor, ProviderOperation, Reversibility,
};
use config::{CargoBuildConfig, OutputSelect};

/// How much of a failed command's stderr a refusal carries.
const STDERR_TAIL: usize = 2000;

/// The artifact kinds this provider produces. A Cargo build target
/// produces an executable; every other kind of the closed §12 vocabulary
/// belongs to a packaging provider.
const PRODUCED_KINDS: [ArtifactKind; 1] = [ArtifactKind::Executable];

/// The §3.2 operations a build-role provider implements. `remove` and
/// `recover` are deploy-only by the architecture's own sentence.
const BUILD_OPERATIONS: [ProviderOperation; 4] = [
    ProviderOperation::Plan,
    ProviderOperation::Fingerprint,
    ProviderOperation::Apply,
    ProviderOperation::Verify,
];

/// One declared output, resolved against the provider's own grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedOutput {
    pub(crate) id: String,
    pub(crate) kind: ArtifactKind,
    pub(crate) select: OutputSelect,
}

/// What `plan` reports: the validated config, the resolved paths, the
/// declared inputs and outputs, and the exact argv the provider WOULD run.
/// Producing it spawns nothing and touches no path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildPlan {
    pub(crate) config: CargoBuildConfig,
    pub(crate) workdir: PathBuf,
    pub(crate) target_dir: PathBuf,
    pub(crate) metadata_argv: Vec<String>,
    pub(crate) build_argv: Vec<String>,
    pub(crate) inputs: Vec<String>,
    pub(crate) outputs: Vec<PlannedOutput>,
    /// Whether this plan, under the run's posture, may reach the network.
    pub(crate) network: bool,
}

/// The provider/toolchain half of the freshness digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolchainIdentity {
    pub(crate) cargo: String,
    pub(crate) rustc: String,
    pub(crate) host: Option<String>,
    /// SHA-256 over the canonical evidence above, 64 lowercase hex.
    pub(crate) digest: String,
}

/// One artifact the message stream named for one declared output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectedArtifact {
    pub(crate) output_id: String,
    pub(crate) kind: ArtifactKind,
    /// Exactly the path the `compiler-artifact` message carried.
    pub(crate) executable: PathBuf,
    /// Cargo's own freshness verdict for this artifact.
    pub(crate) fresh: bool,
    pub(crate) package_id: String,
    pub(crate) bin: String,
}

/// One artifact `verify` independently proved and digested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedArtifact {
    pub(crate) output_id: String,
    /// Forward-slashed absolute placement.
    pub(crate) path_absolute: String,
    /// Forward-slashed project-relative identity.
    pub(crate) path_relative: String,
    /// 64 lowercase hex over the exact produced bytes.
    pub(crate) digest: String,
    pub(crate) bytes: u64,
}

/// The builtin `org.vibevm/vibe#cargo` provider.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CargoProvider;

impl BuildProvider for CargoProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            key: BUILTIN_CARGO_PIN,
            kinds: &PRODUCED_KINDS,
            effect: EffectClass::Workspace,
            network: NetworkUse::WhenNotOffline,
            privilege: PrivilegeNeed::None,
            reversibility: Reversibility::NotApplicable,
            operations: &BUILD_OPERATIONS,
        }
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn plan(&self, request: &BuildTargetRequest<'_>) -> Result<BuildPlan, MechanismError> {
        let descriptor = self.descriptor();
        let target = request.target;
        let config = CargoBuildConfig::parse(&target.id, target.config.as_ref())?;
        let mut outputs = Vec::with_capacity(target.outputs.len());
        for output in &target.outputs {
            if !descriptor.supports(output.kind) {
                return Err(MechanismError::UnsupportedKind {
                    target: target.id.clone(),
                    provider: BUILTIN_CARGO_PIN.to_owned(),
                    output: output.id.clone(),
                    kind: output.kind.to_string(),
                    supported: PRODUCED_KINDS
                        .iter()
                        .map(ArtifactKind::as_str)
                        .collect::<Vec<_>>()
                        .join(", "),
                });
            }
            outputs.push(PlannedOutput {
                id: output.id.clone(),
                kind: output.kind,
                select: OutputSelect::parse(&target.id, &output.id, output.select.as_ref())?,
            });
        }
        let target_dir = request.target_dir();
        Ok(BuildPlan {
            metadata_argv: metadata_argv(&config),
            build_argv: build_argv(&config, &target_dir),
            workdir: request.workdir(),
            target_dir,
            inputs: declared_inputs(target.inputs.as_deref()),
            outputs,
            network: config.reaches_network(request.offline),
            config,
        })
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn fingerprint(
        &self,
        request: &BuildTargetRequest<'_>,
    ) -> Result<ToolchainIdentity, MechanismError> {
        let workdir = request.workdir();
        let cargo = run(&request.target.id, &workdir, &["-Vv".to_owned()], "cargo")?;
        let rustc = run(&request.target.id, &workdir, &["-V".to_owned()], "rustc")?;
        let cargo_version = first_line(&cargo);
        let rustc_version = first_line(&rustc);
        let host = cargo
            .lines()
            .find_map(|line| line.trim().strip_prefix("host:"))
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let mut hash = Sha256::new();
        hash.update(b"cargo\x00");
        hash.update(cargo_version.as_bytes());
        hash.update(b"\x00rustc\x00");
        hash.update(rustc_version.as_bytes());
        hash.update(b"\x00host\x00");
        hash.update(host.as_deref().unwrap_or("").as_bytes());
        Ok(ToolchainIdentity {
            cargo: cargo_version,
            rustc: rustc_version,
            host,
            digest: format!("{:x}", hash.finalize()),
        })
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn apply(
        &self,
        request: &BuildTargetRequest<'_>,
        plan: &BuildPlan,
    ) -> Result<Vec<SelectedArtifact>, MechanismError> {
        let target = &request.target.id;
        let metadata_stdout = run(target, &plan.workdir, &plan.metadata_argv, "cargo")?;
        let metadata = message::parse_metadata(target, &metadata_stdout)?;
        for output in &plan.outputs {
            message::confirm_against_metadata(target, &output.id, &output.select, &metadata)?;
        }
        let build_stdout = run(target, &plan.workdir, &plan.build_argv, "cargo")?;
        let messages = message::parse_stream(target, &build_stdout)?;
        let mut selected = Vec::with_capacity(plan.outputs.len());
        for output in &plan.outputs {
            let chosen = message::select_message(target, &output.id, &output.select, &messages)?;
            let Some(executable) = chosen.executable.as_deref() else {
                // `select_message` already refused a null executable; this
                // branch cannot be reached and refuses rather than unwraps.
                return Err(MechanismError::NoExecutable {
                    target: target.clone(),
                    output: output.id.clone(),
                    bin: output.select.describe(),
                });
            };
            selected.push(SelectedArtifact {
                output_id: output.id.clone(),
                kind: output.kind,
                executable: PathBuf::from(executable),
                fresh: chosen.fresh.unwrap_or(false),
                package_id: chosen.package_id.clone().unwrap_or_default(),
                bin: chosen
                    .target
                    .as_ref()
                    .map(|value| value.name.clone())
                    .unwrap_or_default(),
            });
        }
        Ok(selected)
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
    fn verify(
        &self,
        request: &BuildTargetRequest<'_>,
        selected: &SelectedArtifact,
    ) -> Result<VerifiedArtifact, MechanismError> {
        let target = &request.target.id;
        let absolute = selected.executable.clone();
        let target_dir = request.target_dir();
        // The engine-owned root first: an artifact outside it has no
        // project-relative identity the engine is willing to mint, and
        // that is the refusal worth naming.
        let outside = || MechanismError::OutputOutsideBuildRoot {
            target: target.clone(),
            output: selected.output_id.clone(),
            path: preview(&forward_slashed(&absolute)),
            build_root: forward_slashed(&target_dir),
        };
        if relative_to(&absolute, &target_dir).is_none() {
            return Err(outside());
        }
        let relative = relative_to(&absolute, request.project_root).ok_or_else(outside)?;
        let (digest, bytes) = digest_produced_file(request, selected, &absolute, &relative)?;
        Ok(VerifiedArtifact {
            output_id: selected.output_id.clone(),
            path_absolute: forward_slashed(&absolute),
            path_relative: relative,
            digest,
            bytes,
        })
    }
}

/// Stream one produced artifact and digest its exact bytes, through the
/// mechanism layer's ONE containment cell.
///
/// The containment/link/absence laws and the streaming digest itself live
/// in [`crate::mechanism::contain`] because the packaging providers assert
/// exactly the same three things; what stays here is the translation into
/// THIS provider's named refusals, which is the part a human repairing a
/// build target reads.
fn digest_produced_file(
    request: &BuildTargetRequest<'_>,
    selected: &SelectedArtifact,
    absolute: &Path,
    relative: &str,
) -> Result<(String, u64), MechanismError> {
    digest_file(absolute).map_err(|fault| match fault {
        FileFault::Read(reason) => MechanismError::Digest {
            target: request.target.id.clone(),
            output: selected.output_id.clone(),
            path: preview(relative),
            reason,
        },
        other => MechanismError::OutputMissing {
            target: request.target.id.clone(),
            output: selected.output_id.clone(),
            path: preview(relative),
            reason: other.reason(),
        },
    })
}

/// `cargo metadata` argv — §5's law 1, as an argv and never a shell string.
fn metadata_argv(config: &CargoBuildConfig) -> Vec<String> {
    let mut argv = vec![
        "metadata".to_owned(),
        "--format-version".to_owned(),
        "1".to_owned(),
        "--no-deps".to_owned(),
    ];
    if let Some(path) = &config.manifest_path {
        argv.push("--manifest-path".to_owned());
        argv.push(path.clone());
    }
    argv.extend(config.posture_arguments());
    argv
}

/// `cargo build` argv. The message format and the output root are fixed by
/// the engine: the first is where the artifact identity comes from, the
/// second is a path a provider may not mint.
fn build_argv(config: &CargoBuildConfig, target_dir: &Path) -> Vec<String> {
    let mut argv = vec![
        "build".to_owned(),
        "--message-format=json-render-diagnostics".to_owned(),
        "--target-dir".to_owned(),
        target_dir.display().to_string(),
    ];
    if let Some(path) = &config.manifest_path {
        argv.push("--manifest-path".to_owned());
        argv.push(path.clone());
    }
    argv.extend(config.build_arguments());
    argv
}

/// The declared input rows, rendered for the plan report. `plan` resolves
/// them as DECLARATIONS — it opens nothing, because a pure operation that
/// stats the tree is no longer pure.
fn declared_inputs(inputs: Option<&[ArtifactInput]>) -> Vec<String> {
    inputs.map_or_else(Vec::new, |rows| {
        rows.iter()
            .map(|row| match row {
                ArtifactInput::Path { path } => format!("path:{}", forward_slashed(path)),
                ArtifactInput::Artifact { artifact } => format!("artifact:{artifact}"),
            })
            .collect()
    })
}

/// Run one toolchain command as an argv in a fixed directory.
///
/// `CARGO_TARGET_DIR` is removed from the child environment: the engine
/// owns the output root and passes it explicitly, so an ambient value must
/// not be able to argue with it.
fn run(
    target: &str,
    workdir: &Path,
    argv: &[String],
    program: &str,
) -> Result<String, MechanismError> {
    let output = Command::new(program)
        .args(argv)
        .current_dir(workdir)
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .map_err(|error| MechanismError::Spawn {
            target: target.to_owned(),
            program: format!("{program} {}", argv.join(" ")),
            reason: error.to_string(),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr
            .chars()
            .rev()
            .take(STDERR_TAIL)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return Err(MechanismError::NonZero {
            target: target.to_owned(),
            program: format!("{program} {}", argv.join(" ")),
            status: output.status.to_string(),
            detail: if tail.trim().is_empty() {
                "no stderr output".to_owned()
            } else {
                tail
            },
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The first line of a version banner, trimmed.
fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or("").trim().to_owned()
}

// `pub(crate)` under cfg(test) only: the build executor's own tests reuse
// this cell's target fixtures rather than keeping a second, drifting copy
// of the canonical build target.
#[cfg(test)]
#[path = "cargo/plan_tests.rs"]
pub(crate) mod plan_tests;

#[cfg(test)]
#[path = "cargo/verify_tests.rs"]
mod verify_tests;
