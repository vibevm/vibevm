//! Explicit personal-project observation and create-new initialization.
//!
//! All traversal and writes retain no-follow directory capabilities. Existing
//! projects are parsed in memory; no SYSTEM, member, lock or boot is consulted.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-INIT-SAFETY");

use specmark::spec;
use std::path::{Component, Path, PathBuf};
use vibe_core::cancellation::CancellationToken;
use vibe_core::manifest::Manifest;
use vibe_core::user_project::{
    Command, Effects, Outcome, Publication, Report, State, UserProjectRefusal,
};
use vibe_safefs::{Pinned, Project};

type FsResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn fs_error(message: &str) -> Box<dyn std::error::Error + Send + Sync> {
    std::io::Error::other(message).into()
}

/// Maximum admitted manifest size; oversized input refuses without truncation.
pub const MANIFEST_READ_CAP: usize = 64 * 1024;
/// The initialization visibility boundary.
pub const MANIFEST_NAME: &str = "vibe.toml";
/// The only authored directories produced by initialization.
pub const AUTHORED_DIRECTORIES: [&str; 2] = ["vibevm", "vibevm/vibespecs"];
/// Ordinary identity used for a fresh personal project.
pub const INITIAL_NAME: &str = "vibe-user";
/// Ordinary version used for a fresh personal project.
pub const INITIAL_VERSION: &str = "1.0.0";

/// Minimal ordinary project manifest; no SYSTEM, lock or boot inputs.
#[must_use]
pub fn initial_manifest() -> String {
    format!("[project]\nname = \"{INITIAL_NAME}\"\nversion = \"{INITIAL_VERSION}\"\n")
}

/// Bounded root observation, not a whole-project health check.
#[derive(Debug, Clone)]
pub enum Observation {
    Absent,
    Empty,
    Initialized { name: String },
    Unavailable(Box<UserProjectRefusal>),
}

/// The result of explicit initialization.
#[derive(Debug)]
pub enum Initialized {
    Created(Report),
    AlreadyInitialized(Report),
}
impl Initialized {
    /// This invocation's root, effects and publication facts.
    #[must_use]
    pub fn report(&self) -> &Report {
        match self {
            Self::Created(report) | Self::AlreadyInitialized(report) => report,
        }
    }
}

/// Retain the filesystem anchor separately from every selected component.
fn anchor(root: &Path) -> FsResult<(Project, Vec<String>)> {
    if !root.is_absolute() {
        return Err(fs_error("the personal project root must be absolute"));
    }
    let mut base = PathBuf::new();
    let mut names = Vec::new();
    for part in root.components() {
        match part {
            Component::Prefix(prefix) => base.push(prefix.as_os_str()),
            Component::RootDir => base.push(part.as_os_str()),
            Component::Normal(name) => {
                let name = name
                    .to_str()
                    .ok_or_else(|| fs_error("root component is not UTF-8"))?;
                vibe_safefs::ensure_safe_component(name)?;
                names.push(name.to_owned());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(fs_error(
                    "parent components are not admitted in the personal root",
                ));
            }
        }
    }
    if names.is_empty() {
        return Err(fs_error(
            "the personal project cannot be a filesystem anchor",
        ));
    }
    Ok((Project::open(&base)?, names))
}

fn walk(project: &Project, names: &[String]) -> FsResult<Option<Pinned>> {
    let mut directory = project.root_dir()?;
    for name in names {
        let Some(child) = directory.open_child_checked(name)? else {
            return Ok(None);
        };
        directory = child;
    }
    Ok(Some(directory))
}

/// Observe without mutation, lock acquisition or member traversal.
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-STATUS")]
pub fn status(root: &Path) -> Observation {
    let result = anchor(root).and_then(|(project, names)| {
        Ok(match walk(&project, &names)? {
            Some(directory) => observe(&project, &directory, root, Command::Status),
            None => Observation::Absent,
        })
    });
    result.unwrap_or_else(|_| {
        Observation::Unavailable(Box::new(refusal(
            Command::Status,
            root,
            "the root or an existing ancestor is unavailable or unsafe",
        )))
    })
}

fn observe(project: &Project, directory: &Pinned, root: &Path, command: Command) -> Observation {
    let refuse = |reason| Observation::Unavailable(Box::new(refusal(command, root, reason)));
    let bytes =
        match project.read_file_snapshot_bounded_in(directory, MANIFEST_NAME, MANIFEST_READ_CAP) {
            Ok(Some(snapshot)) => snapshot.bytes,
            Ok(None) => {
                return match project.child_names_bounded(directory, 1) {
                    Ok(names) if names.is_empty() => Observation::Empty,
                    _ => refuse("the root is nonempty but carries no valid `vibe.toml`"),
                };
            }
            Err(_) => {
                return refuse("the root `vibe.toml` is not a bounded ordinary single-link file");
            }
        };
    let parsed = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| Manifest::parse_str(text).ok());
    let Some(manifest) = parsed else {
        return refuse("the root `vibe.toml` does not parse as an ordinary manifest");
    };
    match manifest.require_project() {
        Ok(project) => Observation::Initialized {
            name: project.name.clone(),
        },
        Err(_) => {
            refuse("the root manifest is package-only or a virtual workspace, not a `[project]`")
        }
    }
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-INIT-SAFETY")]
pub fn init(
    root: &Path,
    cancellation: &CancellationToken,
) -> Result<Initialized, UserProjectRefusal> {
    cancellation
        .checkpoint()
        .map_err(|_| refusal(Command::Init, root, "stopped before the first write"))?;
    let (project, names) = anchor(root).map_err(|_| {
        refusal(
            Command::Init,
            root,
            "the personal project root is unavailable or unsafe",
        )
    })?;
    let existing = walk(&project, &names).map_err(|_| {
        refusal(
            Command::Init,
            root,
            "the root or an existing ancestor is unavailable or unsafe",
        )
    })?;
    if let Some(directory) = &existing {
        match observe(&project, directory, root, Command::Init) {
            Observation::Initialized { .. } => return Ok(already(root)),
            Observation::Unavailable(reason) => return Err(*reason),
            Observation::Empty => {}
            Observation::Absent => unreachable!(),
        }
    }
    let mut owned = Vec::new();
    let mut uncertain = false;
    let result = (|| -> FsResult<()> {
        let directory = match existing {
            Some(directory) => directory,
            None => {
                let mut directory = project.root_dir()?;
                let mut prefix = Vec::new();
                for name in &names {
                    cancellation.checkpoint()?;
                    prefix.push(name.clone());
                    directory = match directory.open_child_checked(name)? {
                        Some(child) => child,
                        None => create_owned(
                            &directory,
                            name,
                            &prefix,
                            cancellation,
                            &mut owned,
                            &mut uncertain,
                        )?,
                    };
                }
                directory
            }
        };
        let root_identity = directory.identity()?;
        cancellation.checkpoint()?;
        // Revalidate after the checkpoint, immediately before the first
        // authored mutation; a poll may itself observe a concurrent writer.
        if !project.child_names_bounded(&directory, 1)?.is_empty() {
            return Err(fs_error("the root became nonempty during initialization"));
        }
        let mut authored_path = names.clone();
        authored_path.push("vibevm".to_owned());
        let authored = create_owned(
            &directory,
            "vibevm",
            &authored_path,
            cancellation,
            &mut owned,
            &mut uncertain,
        )?;
        let authored_identity = authored.identity()?;
        cancellation.checkpoint()?;
        authored_path.push("vibespecs".to_owned());
        let specs = create_owned(
            &authored,
            "vibespecs",
            &authored_path,
            cancellation,
            &mut owned,
            &mut uncertain,
        )?;
        let specs_identity = specs.identity()?;
        cancellation.checkpoint()?;
        verify_layout(
            &project,
            &names,
            root_identity,
            authored_identity,
            specs_identity,
            false,
        )?;
        match project.publish_new_in(&directory, MANIFEST_NAME, initial_manifest().as_bytes()) {
            Ok(()) => cancellation.committed(),
            Err(error) => {
                uncertain |= error.stage == vibe_safefs::PublishStage::PossiblyPublished;
                return Err(fs_error("the manifest could not be published and verified"));
            }
        }
        // The publisher verifies its held root. The selected path can move
        // independently: observe that movement without pretending rollback.
        verify_layout(
            &project,
            &names,
            root_identity,
            authored_identity,
            specs_identity,
            true,
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            let mut report = Report::success(
                Command::Init,
                Some(root.to_path_buf()),
                State::Initialized,
                Effects::Changed,
                Publication::Committed,
            );
            report.outcome = Outcome::Created;
            Ok(Initialized::Created(report))
        }
        Err(error) => {
            let mut report = Report::refused(
                Command::Init,
                Some(root.to_path_buf()),
                format!(
                    "{}: initialization stopped or could not complete safely: {error}",
                    vibe_core::machine_json_path(root)
                ),
                "inspect the root; any partial directories were kept; resolve concurrent or incomplete state before retrying",
            );
            if !owned.is_empty() || uncertain {
                report.outcome = Outcome::Incomplete;
                report.effects = if !uncertain
                    && owned.iter().all(|(path, identity)| {
                        walk(&project, path)
                            .ok()
                            .flatten()
                            .and_then(|entry| entry.identity().ok())
                            .as_ref()
                            == Some(identity)
                    }) {
                    Effects::Changed
                } else {
                    Effects::Unknown
                };
                report.publication = if cancellation.publication_state() == "committed" {
                    report.reason = Some(format!(
                        "{}: the manifest was committed in the held root, but the selected path or authored layout no longer verifies: {error}",
                        vibe_core::machine_json_path(root)
                    ));
                    Publication::Committed
                } else {
                    Publication::RecoveryRequired
                };
            } else if cancellation.publication_state() == "recovery-required" {
                // No syscall created anything and no outcome is uncertain.
                // End the token's mutation intent, never reopen this attempt.
                cancellation.restored();
            }
            Err(UserProjectRefusal::new(report))
        }
    }
}

fn create_owned(
    parent: &Pinned,
    name: &str,
    path: &[String],
    cancellation: &CancellationToken,
    owned: &mut Vec<(Vec<String>, vibe_safefs::FileIdentity)>,
    uncertain: &mut bool,
) -> FsResult<Pinned> {
    cancellation.publishing();
    match parent.create_child_exclusive(name) {
        Ok(child) => {
            let identity = match child.identity() {
                Ok(identity) => identity,
                Err(error) => {
                    *uncertain = true;
                    return Err(fs_error(&error.to_string()));
                }
            };
            owned.push((path.to_vec(), identity));
            Ok(child)
        }
        Err(vibe_safefs::ExclusiveChildError::NotCreated(_)) => Err(fs_error(
            "a directory creation was refused before creating an entry",
        )),
        Err(vibe_safefs::ExclusiveChildError::CreatedNotReopened { .. }) => {
            *uncertain = true;
            Err(fs_error(
                "a directory was created but could not be reopened safely",
            ))
        }
    }
}

fn verify_layout(
    project: &Project,
    names: &[String],
    root_identity: vibe_safefs::FileIdentity,
    authored_identity: vibe_safefs::FileIdentity,
    specs_identity: vibe_safefs::FileIdentity,
    published: bool,
) -> FsResult<()> {
    let root = walk(project, names)?.ok_or_else(|| fs_error("the selected root vanished"))?;
    if root.identity()? != root_identity {
        return Err(fs_error("the selected root changed"));
    }
    let authored = root.open_child("vibevm")?;
    let specs = authored.open_child("vibespecs")?;
    if authored.identity()? != authored_identity || specs.identity()? != specs_identity {
        return Err(fs_error(
            "the authored directories changed during initialization",
        ));
    }
    let mut root_names = project.child_names_bounded(&root, 2)?;
    root_names.sort();
    let expected = if published {
        vec![MANIFEST_NAME, "vibevm"]
    } else {
        vec!["vibevm"]
    };
    if root_names != expected
        || project.child_names_bounded(&authored, 1)? != ["vibespecs"]
        || !project.child_names_bounded(&specs, 1)?.is_empty()
    {
        return Err(fs_error("the root layout changed during initialization"));
    }
    Ok(())
}

fn already(root: &Path) -> Initialized {
    Initialized::AlreadyInitialized(Report::success(
        Command::Init,
        Some(root.to_path_buf()),
        State::Initialized,
        Effects::None,
        Publication::NotStarted,
    ))
}
fn refusal(command: Command, root: &Path, reason: &str) -> UserProjectRefusal {
    let mut report = Report::refused(
        command,
        Some(root.to_path_buf()),
        format!("{}: {reason}", vibe_core::machine_json_path(root)),
        "inspect or repair the selected root, then retry",
    );
    if matches!(command, Command::Status) {
        report.outcome = Outcome::Observed;
    }
    UserProjectRefusal::new(report)
}

#[cfg(test)]
#[path = "user_project/tests.rs"]
mod tests;
