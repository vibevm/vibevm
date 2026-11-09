//! `cargo xtask dist` — native bundle production and mutable release assembly.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use clap::Subcommand;

mod archive;
mod build;
mod px_tool;
mod release;
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

    /// Mirror one complete, verified release onto GitVerse. A separate explicit
    /// step: no other command runs it.
    GitversePublish {
        #[arg(long)]
        version: String,

        /// Read the complete asset set from this directory instead of the
        /// published GitHub release.
        #[arg(long)]
        from_dir: Option<PathBuf>,

        /// Print the planned requests; load no token and send nothing to GitVerse.
        #[arg(long)]
        dry_run: bool,

        /// Replace a same-named GitVerse asset whose bytes differ (refused otherwise).
        #[arg(long)]
        replace: bool,

        /// GitVerse API base URL (an on-prem GitVerse's API root).
        #[arg(long, default_value = release::gitverse::DEFAULT_GITVERSE_API_BASE)]
        api_base: String,

        #[arg(long, default_value = release::gitverse::DEFAULT_GITVERSE_OWNER)]
        owner: String,

        #[arg(long, default_value = release::gitverse::DEFAULT_GITVERSE_REPO)]
        repo: String,
    },

    /// Stage the pinned px release (`xtask/dist/px.toml`) as `<out-dir>/tools/px/`
    /// for one target: fetch or take the asset, verify its SHA-256, unpack
    /// it, add px's LICENSE and a NOTICE (PROP-068#proxy).
    Px {
        #[arg(long)]
        target: String,
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,
        /// A local copy of the pinned asset instead of downloading it.
        #[arg(long)]
        archive: Option<PathBuf>,
        /// A local copy of px's LICENSE.txt (default: the archive's own).
        #[arg(long)]
        license: Option<PathBuf>,
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
        DistCommand::Upload { asset, fragment } => release::upload(repo_root, &asset, &fragment),
        DistCommand::UploadBuilt { target, out_dir } => {
            release::upload_built(repo_root, &target, &out_dir)
        }
        DistCommand::Status { version } => release::status(repo_root, &version),
        DistCommand::GitversePublish {
            version,
            from_dir,
            dry_run,
            replace,
            api_base,
            owner,
            repo,
        } => release::gitverse::publish(
            repo_root,
            &release::gitverse::GitverseOptions {
                version,
                from_dir,
                dry_run,
                replace,
                api_base,
                owner,
                repo,
            },
        ),
        DistCommand::Px {
            target,
            out_dir,
            archive,
            license,
        } => px_tool::run(
            repo_root,
            &px_tool::Inputs {
                target: &target,
                out_dir: &out_dir,
                archive: archive.as_deref(),
                license: license.as_deref(),
            },
        ),
        DistCommand::Finalize { version, publish } => {
            release::finalize(repo_root, &version, publish)
        }
    }
}

#[cfg(test)]
#[path = "credential_tests.rs"]
mod credential_tests;
