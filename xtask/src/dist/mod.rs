//! `cargo xtask dist` — native bundle production and mutable release assembly.

use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};
use clap::Subcommand;
use serde::Deserialize;
use vibe_publish::publication_effect_plan::CapturedPublicationRemote;
use vibe_publish::publication_subject::{
    PublicationSubjectExpectations, require_apply_ready_default_smoke,
};
use vibe_safefs::Project;
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedRelease, PublicationObservedTag,
};

mod archive;
mod build;
mod capture;
mod release;
mod smoke;
mod snapshot;

#[derive(Debug, Subcommand)]
pub(crate) enum DistCommand {
    /// Build the complete native bundle from one pinned committed snapshot.
    Build {
        /// Exact native Rust target. Omit to derive it from this host.
        #[arg(long)]
        target: Option<String>,

        /// Directory for the bundle, bootstrap and platform fragment.
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,

        /// Run workspace `cargo check` in the clean source snapshot before building.
        #[arg(long)]
        checks: bool,

        /// Run workspace tests in the clean source snapshot. Off by default.
        #[arg(long)]
        tests: bool,

        /// Run the repository self-check in an isolated scratch Git repository.
        #[arg(long)]
        self_check: bool,
    },

    /// Delete any same-version release and prepare a fresh mutable GitHub draft.
    Prepare {
        #[arg(long)]
        version: String,
    },

    /// Inspect exact 1.1.0 artifacts and save a local native smoke report.
    PrepareLocal {
        #[arg(long, default_value = "1.1.0")]
        version: String,
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,
        /// Explicitly skip only the named target; repeat for multiple targets.
        #[arg(long = "skip-smoke-target", requires = "skip_reason")]
        skip_targets: Vec<String>,
        /// Bounded operator reason shared by the explicitly skipped targets.
        #[arg(long = "skip-reason", requires = "skip_targets")]
        skip_reason: Option<String>,
    },

    /// Review a local 1.1.0 effect plan without probes, credentials or remote calls.
    PlanLocal {
        #[arg(long, default_value = "1.1.0")]
        version: String,
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,
        /// Relative content-addressed report path below the output directory.
        #[arg(long)]
        smoke_report: String,
        /// Operator-supplied typed observation; this command cannot verify its remote origin.
        #[arg(long)]
        remote_snapshot: PathBuf,
        /// Describe a future visibility change; this performs no publication.
        #[arg(long)]
        publish: bool,
    },

    /// Upload one previously built platform bundle, bootstrap and fragment.
    Upload {
        #[arg(long)]
        asset: PathBuf,
        #[arg(long)]
        fragment: PathBuf,
    },

    /// Upload the exact outputs just built for a target. Wrapper-only boundary.
    #[command(hide = true)]
    UploadBuilt {
        #[arg(long)]
        target: String,
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,
    },

    /// Show completeness of the five-target GitHub release.
    Status {
        #[arg(long)]
        version: String,
    },

    /// Verify all remote assets, publish DISTRIBUTIONS.json, and optionally publish the release.
    Finalize {
        #[arg(long)]
        version: String,
        #[arg(long)]
        publish: bool,
    },
}

/// Remove every publishing credential or credential-provider handle before a
/// release command starts Cargo, Git, a repository script, or a built binary.
/// The xtask process itself retains its environment so an explicit upload can
/// still load the token after the isolated build has completed.
pub(super) fn scrub_release_credentials(command: &mut Command) {
    let explicit = command
        .get_envs()
        .map(|(name, _)| name.to_os_string())
        .collect::<Vec<_>>();
    let inherited = std::env::vars_os().map(|(name, _)| name);
    for name in explicit.into_iter().chain(inherited) {
        if is_release_credential_name(&name) {
            command.env_remove(name);
        }
    }
}

fn is_release_credential_name(name: &OsStr) -> bool {
    let name = name.to_string_lossy().to_ascii_uppercase();
    name == "VIBEVM_PUBLISH_TOKEN"
        || name.starts_with("VIBEVM_PUBLISH_TOKEN_")
        || matches!(
            name.as_str(),
            "GITHUB_TOKEN"
                | "GH_TOKEN"
                | "GITHUB_ENTERPRISE_TOKEN"
                | "GH_ENTERPRISE_TOKEN"
                | "GITHUB_PAT"
                | "ACTIONS_ID_TOKEN_REQUEST_TOKEN"
                | "ACTIONS_RUNTIME_TOKEN"
                | "GIT_ASKPASS"
                | "SSH_ASKPASS"
                | "SSH_AUTH_SOCK"
                | "SSH_AGENT_PID"
                | "GIT_SSH"
                | "GIT_SSH_COMMAND"
        )
        || name.starts_with("GIT_CONFIG_")
}

pub(crate) fn run_dist(repo_root: &Path, command: DistCommand) -> Result<()> {
    match command {
        DistCommand::Build {
            target,
            out_dir,
            checks,
            tests,
            self_check,
        } => {
            build::run(
                repo_root,
                &build::BuildOptions {
                    target,
                    out_dir,
                    checks,
                    tests,
                    self_check,
                },
            )?;
            Ok(())
        }
        DistCommand::Prepare { version } => release::prepare(repo_root, &version),
        DistCommand::PrepareLocal {
            version,
            out_dir,
            skip_targets,
            skip_reason,
        } => {
            let stdout = io::stdout();
            let mut locked = stdout.lock();
            run_prepare_local_into(
                repo_root,
                &version,
                &out_dir,
                skip_targets,
                skip_reason,
                &mut locked,
            )
        }
        DistCommand::PlanLocal {
            version,
            out_dir,
            smoke_report,
            remote_snapshot,
            publish,
        } => {
            let stdout = io::stdout();
            let mut locked = stdout.lock();
            run_plan_local_into(
                repo_root,
                &version,
                &out_dir,
                &smoke_report,
                &remote_snapshot,
                publish,
                &mut locked,
            )
        }
        DistCommand::Upload { asset, fragment } => release::upload(repo_root, &asset, &fragment),
        DistCommand::UploadBuilt { target, out_dir } => {
            release::upload_built(repo_root, &target, &out_dir)
        }
        DistCommand::Status { version } => release::status(repo_root, &version),
        DistCommand::Finalize { version, publish } => {
            release::finalize(repo_root, &version, publish)
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperatorRemoteSnapshot {
    generation_ref: String,
    tag: PublicationObservedTag,
    release: PublicationObservedRelease,
}

fn read_remote_snapshot(repo_root: &Path, path: &Path) -> Result<CapturedPublicationRemote> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    };
    let parent = absolute
        .parent()
        .ok_or_else(|| anyhow::anyhow!("remote snapshot has no parent"))?;
    let name = absolute
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow::anyhow!("remote snapshot filename is not UTF-8"))?;
    let project = Project::open(parent)?;
    let bytes = project
        .read_file_bounded(name, 256 * 1024)?
        .ok_or_else(|| anyhow::anyhow!("remote snapshot is absent"))?;
    let typed: OperatorRemoteSnapshot = serde_json::from_slice(&bytes)?;
    Ok(CapturedPublicationRemote {
        generation_ref: typed.generation_ref,
        tag: typed.tag,
        release: typed.release,
    })
}

fn run_plan_local_into(
    repo_root: &Path,
    version: &str,
    out_dir: &Path,
    smoke_report: &str,
    remote_snapshot: &Path,
    publish: bool,
    output: &mut dyn Write,
) -> Result<()> {
    if version != "1.1.0" {
        bail!("local distribution planning requires the mutable 1.1.0 version");
    }
    let remote = read_remote_snapshot(repo_root, remote_snapshot)?;
    let prepared = smoke::prepare::load_local_subject(repo_root, out_dir, smoke_report)?;
    let report_digest = prepared.report.digest;
    let composition = capture::plan_captured_subject(
        prepared.capture,
        &remote,
        if publish {
            capture::PublicationVisibilityChoice::Publish
        } else {
            capture::PublicationVisibilityChoice::RetainDraft
        },
        capture::PublishedReplacementChoice::Refuse,
    )?;
    let smoke_gate_passed = require_apply_ready_default_smoke(
        &composition.capture.subject,
        &PublicationSubjectExpectations {
            version: "1.1.0",
            repository: "vibevm/vibevm",
            smoke_policy_id: "distribution-smoke/e1",
            smoke_policy_epoch: 1,
        },
    )
    .is_ok();
    // Operator-supplied JSON is a review input, never an authenticated remote
    // generation or an approval to apply. Keep this verdict false even when
    // all native probes passed.
    let rendered = serde_json::to_vec_pretty(&serde_json::json!({
        "schema": 1,
        "version": "1.1.0",
        "remote_observation": "operator-supplied-unverified",
        "apply_ready": false,
        "smoke_gate_passed": smoke_gate_passed,
        "subject_sha256": composition.capture.subject_digest,
        "smoke_report_sha256": report_digest,
        "plan_sha256": composition.plan_digest,
        "effect_plan": composition.plan,
    }))?;
    output.write_all(&rendered)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

fn run_prepare_local_into(
    repo_root: &Path,
    version: &str,
    out_dir: &Path,
    skip_targets: Vec<String>,
    skip_reason: Option<String>,
    output: &mut dyn Write,
) -> Result<()> {
    if version != "1.1.0" {
        bail!("local distribution preparation requires the mutable 1.1.0 version");
    }
    let selection = smoke::prepare::SmokeSelection::new(skip_targets, skip_reason)?;
    let prepared = smoke::prepare::prepare_local_subject_selected(repo_root, out_dir, &selection)?;
    output.write_all(&prepared.report_bytes)?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
#[path = "credential_tests.rs"]
mod credential_tests;

#[cfg(test)]
#[path = "prepare_local_cli_tests.rs"]
mod prepare_local_cli_tests;
