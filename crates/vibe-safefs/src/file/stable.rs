//! Stable held-handle state and streaming capability-to-capability copy.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ZAI-GLM-LAUNCHER-DELIVERY");

use std::io::{Read, Seek, SeekFrom, Write};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use super::{
    cap_options, create_unique_stage, file_identity, injected_post_publication_failure,
    refuse_unpublishable_destination, verify_regular_single_link,
};
use crate::project::{Pinned, Project};
use crate::publish::{PublishError, Published};

const WINDOW: usize = 64 * 1024;

/// Stable identity of one regular single-link file read twice through one
/// held no-follow handle. No content-sized allocation is retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StableFileState {
    /// Lowercase SHA-256 over the file bytes.
    pub sha256: String,
    pub bytes: u64,
    /// Exact Unix permission bits, including special bits; absent elsewhere.
    pub unix_mode: Option<u32>,
}

impl Project {
    /// Observe one file through a held capability, proving two identical
    /// streaming passes plus stable length, mode, link count and name identity.
    pub fn stable_file_state(&self, relative: &str) -> Result<Option<StableFileState>> {
        let root = self.root_dir()?;
        read_stable_in(self, &root, relative, None)
    }

    /// Copy a source file to another capability root without reopening its
    /// ambient path or allocating its content. The source stays held across
    /// both stability passes; the destination is staged, mode-adjusted and
    /// renamed through its pinned parent capability.
    pub fn copy_stable_file_to(
        &self,
        source_relative: &str,
        destination: &Project,
        destination_relative: &str,
        desired_mode: Option<u32>,
    ) -> std::result::Result<(StableFileState, Published), PublishError> {
        self.copy_stable_file_to_inner(
            source_relative,
            destination,
            destination_relative,
            desired_mode,
            None,
            None,
        )
    }

    /// Copy guarded by a previously resolved content identity. The first
    /// held-handle pass must match before any destination parent or stage is
    /// created.
    #[allow(clippy::too_many_arguments, reason = "one guarded copy contract")]
    pub fn copy_stable_file_to_expected(
        &self,
        source_relative: &str,
        destination: &Project,
        destination_relative: &str,
        desired_mode: Option<u32>,
        expected_sha256: &str,
        expected_bytes: u64,
    ) -> std::result::Result<(StableFileState, Published), PublishError> {
        self.copy_stable_file_to_inner(
            source_relative,
            destination,
            destination_relative,
            desired_mode,
            None,
            Some((expected_sha256, expected_bytes)),
        )
    }

    /// The same streaming copy after emptying one engine-owned destination
    /// directory. The source handle is opened and proved before the reset, so
    /// an invalid source causes no destination mutation.
    pub fn copy_stable_file_to_fresh_dir(
        &self,
        source_relative: &str,
        destination: &Project,
        destination_dir: &str,
        filename: &str,
        desired_mode: Option<u32>,
    ) -> std::result::Result<(StableFileState, Published), PublishError> {
        let destination_relative = format!("{destination_dir}/{filename}");
        self.copy_stable_file_to_inner(
            source_relative,
            destination,
            &destination_relative,
            desired_mode,
            Some(destination_dir),
            None,
        )
    }

    /// Fresh-directory copy guarded by the content identity an earlier
    /// engine resolution recorded. The comparison happens on the first pass
    /// of this same held handle, before the destination directory is reset.
    #[allow(clippy::too_many_arguments, reason = "one guarded copy contract")]
    pub fn copy_stable_file_to_fresh_dir_expected(
        &self,
        source_relative: &str,
        destination: &Project,
        destination_dir: &str,
        filename: &str,
        desired_mode: Option<u32>,
        expected_sha256: &str,
        expected_bytes: u64,
    ) -> std::result::Result<(StableFileState, Published), PublishError> {
        let destination_relative = format!("{destination_dir}/{filename}");
        self.copy_stable_file_to_inner(
            source_relative,
            destination,
            &destination_relative,
            desired_mode,
            Some(destination_dir),
            Some((expected_sha256, expected_bytes)),
        )
    }

    fn copy_stable_file_to_inner(
        &self,
        source_relative: &str,
        destination: &Project,
        destination_relative: &str,
        desired_mode: Option<u32>,
        reset_dir: Option<&str>,
        expected: Option<(&str, u64)>,
    ) -> std::result::Result<(StableFileState, Published), PublishError> {
        let source_root = self
            .root_dir()
            .map_err(|error| PublishError::before(Vec::new(), error))?;
        let Some((source_holder, source_name)) = self
            .holder_of(&source_root, source_relative)
            .map_err(|error| PublishError::before(Vec::new(), error))?
        else {
            return Err(PublishError::before(
                Vec::new(),
                anyhow::anyhow!("source file `{source_relative}` is absent"),
            ));
        };
        let source_display = source_holder.join(&source_name);
        let mut source_options = cap_options();
        let mut source = source_holder
            .dir
            .open_with(&source_name, source_options.read(true))
            .map_err(|error| {
                PublishError::before(
                    Vec::new(),
                    anyhow::Error::new(error).context(format!(
                        "opening source `{}` without following links",
                        source_display.display()
                    )),
                )
            })?
            .into_std();
        verify_regular_single_link(&source, &source_display)
            .map_err(|error| PublishError::before(Vec::new(), error))?;
        let before = source.metadata().map_err(|error| {
            PublishError::before(
                Vec::new(),
                anyhow::Error::new(error)
                    .context(format!("inspecting source `{}`", source_display.display())),
            )
        })?;
        let first = hash_pass(&mut source, &source_display, None)
            .and_then(|state| finish_source_state(&source, &source_display, &before, state))
            .map_err(|error| PublishError::before(Vec::new(), error))?;
        recheck_name(&source_holder, &source_name, &source, &source_display)
            .map_err(|error| PublishError::before(Vec::new(), error))?;
        if expected.is_some_and(|(digest, bytes)| first.sha256 != digest || first.bytes != bytes) {
            return Err(PublishError::before(
                Vec::new(),
                anyhow::anyhow!(
                    "source `{}` no longer matches its expected digest and length",
                    source_display.display()
                ),
            ));
        }
        if let Some(relative) = reset_dir {
            destination
                .reset_dir(relative)
                .map_err(|error| PublishError::before(Vec::new(), error))?;
        }
        let mut created = Vec::new();
        let destination_root = destination
            .root_dir()
            .map_err(|error| PublishError::before(created.clone(), error))?;
        let (parents, name) = crate::component::split_relative(destination_relative)
            .map_err(|error| PublishError::before(created.clone(), error))?;
        let holder = if parents.is_empty() {
            destination_root
        } else {
            let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
            destination
                .dir_at_recording(&destination_root, &chain, &mut created)
                .map_err(|error| PublishError::before(created.clone(), error))?
        };
        refuse_unpublishable_destination(&holder, &name)
            .map_err(|error| PublishError::before(created.clone(), error))?;
        let (staged_name, staged) = create_unique_stage(&holder)
            .map_err(|error| PublishError::before(created.clone(), error))?;
        let mut staged = staged.into_std();
        let second = match copy_pass(&mut source, &mut staged, &source_display) {
            Ok(state) => state,
            Err(error) => {
                drop(staged);
                let _ = holder.dir.remove_file(&staged_name);
                return Err(PublishError::before(created, error));
            }
        };
        let staged_finish = staged
            .flush()
            .and_then(|()| set_unix_mode(&staged, desired_mode))
            .and_then(|()| staged.sync_all());
        drop(staged);
        if let Err(error) = staged_finish {
            let _ = holder.dir.remove_file(&staged_name);
            return Err(PublishError::before(
                created,
                anyhow::Error::new(error).context(format!(
                    "finishing staged `{}`",
                    holder.join(&staged_name).display()
                )),
            ));
        }
        let source_state = match finish_source_state(&source, &source_display, &before, second) {
            Ok(state) if state.sha256 == first.sha256 && state.bytes == first.bytes => state,
            Ok(_) => {
                let _ = holder.dir.remove_file(&staged_name);
                return Err(PublishError::before(
                    created,
                    anyhow::anyhow!(
                        "source `{}` changed between its two held-handle passes",
                        source_display.display()
                    ),
                ));
            }
            Err(error) => {
                let _ = holder.dir.remove_file(&staged_name);
                return Err(PublishError::before(created, error));
            }
        };
        if let Err(error) = recheck_name(&source_holder, &source_name, &source, &source_display) {
            let _ = holder.dir.remove_file(&staged_name);
            return Err(PublishError::before(created, error));
        }
        holder
            .dir
            .rename(&staged_name, &holder.dir, &name)
            .map_err(|error| {
                let _ = holder.dir.remove_file(&staged_name);
                PublishError::before(
                    created.clone(),
                    anyhow::Error::new(error)
                        .context(format!("publishing `{}`", holder.join(&name).display())),
                )
            })?;
        let possibly = |error| PublishError::possibly(created.clone(), error);
        crate::race_hook::before_publish_verify(&holder, &name);
        let visible = read_stable_in(destination, &holder, &name, Some(source_state.bytes))
            .map_err(&possibly)?;
        if visible.as_ref().is_none_or(|state| {
            state.sha256 != source_state.sha256
                || state.bytes != source_state.bytes
                || !mode_matches(state.unix_mode, desired_mode)
        }) {
            return Err(possibly(anyhow::anyhow!(
                "published state of `{}` does not match the held source",
                holder.join(&name).display()
            )));
        }
        if let Ok(handle) = holder.dir.try_clone() {
            let _ = handle.into_std_file().sync_all();
        }
        if let Some(error) = injected_post_publication_failure(destination_relative) {
            return Err(possibly(error));
        }
        Ok((
            source_state,
            Published {
                created_directories: created,
            },
        ))
    }
}

pub(super) fn read_stable_in(
    project: &Project,
    directory: &Pinned,
    relative: &str,
    cap: Option<u64>,
) -> Result<Option<StableFileState>> {
    let Some((holder, name)) = project.holder_of(directory, relative)? else {
        return Ok(None);
    };
    let display = holder.join(&name);
    let mut options = cap_options();
    let mut file = match holder.dir.open_with(&name, options.read(true)) {
        Ok(file) => file.into_std(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(anyhow::Error::new(error).context(format!(
                "opening `{}` without following links",
                display.display()
            )));
        }
    };
    verify_regular_single_link(&file, &display)?;
    let before = file.metadata()?;
    let first = hash_pass(&mut file, &display, cap)?;
    let second = hash_pass(&mut file, &display, cap)?;
    let state = finish_source_state(&file, &display, &before, second)?;
    if state.sha256 != first.sha256 || state.bytes != first.bytes {
        bail!(
            "`{}` changed between its two held-handle passes",
            display.display()
        );
    }
    recheck_name(&holder, &name, &file, &display)?;
    Ok(Some(state))
}

fn copy_pass(
    source: &mut std::fs::File,
    destination: &mut impl Write,
    display: &std::path::Path,
) -> Result<StableFileState> {
    source.seek(SeekFrom::Start(0))?;
    let mut hash = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; WINDOW];
    loop {
        let read = source
            .read(&mut buffer)
            .with_context(|| format!("reading `{}`", display.display()))?;
        if read == 0 {
            break;
        }
        destination.write_all(&buffer[..read])?;
        hash.update(&buffer[..read]);
        bytes += read as u64;
    }
    Ok(StableFileState {
        sha256: format!("{:x}", hash.finalize()),
        bytes,
        unix_mode: None,
    })
}

fn hash_pass(
    file: &mut std::fs::File,
    display: &std::path::Path,
    cap: Option<u64>,
) -> Result<StableFileState> {
    file.seek(SeekFrom::Start(0))?;
    let mut hash = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; WINDOW];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        if let Some(limit) = cap
            && bytes > limit
        {
            bail!("`{}` exceeds its {limit}-byte read cap", display.display());
        }
        hash.update(&buffer[..read]);
    }
    Ok(StableFileState {
        sha256: format!("{:x}", hash.finalize()),
        bytes,
        unix_mode: None,
    })
}

fn finish_source_state(
    file: &std::fs::File,
    display: &std::path::Path,
    before: &std::fs::Metadata,
    mut state: StableFileState,
) -> Result<StableFileState> {
    verify_regular_single_link(file, display)?;
    let after = file.metadata()?;
    if before.len() != after.len()
        || state.bytes != after.len()
        || unix_mode(before) != unix_mode(&after)
    {
        bail!(
            "`{}` changed length or mode during its stable read",
            display.display()
        );
    }
    state.unix_mode = unix_mode(&after);
    Ok(state)
}

fn recheck_name(
    holder: &Pinned,
    name: &str,
    held: &std::fs::File,
    display: &std::path::Path,
) -> Result<()> {
    let mut options = cap_options();
    let current = holder
        .dir
        .open_with(name, options.read(true))
        .with_context(|| format!("rechecking the name `{}`", display.display()))?
        .into_std();
    verify_regular_single_link(&current, display)?;
    if file_identity(&current, display)? != file_identity(held, display)? {
        bail!("`{}` no longer names the held file", display.display());
    }
    Ok(())
}

pub(super) fn mode_matches(observed: Option<u32>, desired: Option<u32>) -> bool {
    match desired {
        Some(mode) => observed == Some(mode),
        None => true,
    }
}

#[cfg(unix)]
fn unix_mode(metadata: &std::fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(metadata.permissions().mode() & 0o7777)
}

#[cfg(not(unix))]
fn unix_mode(_metadata: &std::fs::Metadata) -> Option<u32> {
    None
}

#[cfg(unix)]
pub(super) fn set_unix_mode(file: &std::fs::File, mode: Option<u32>) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let Some(mode) = mode else { return Ok(()) };
    if mode > 0o7777 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Unix permission mode {mode:o} exceeds 07777"),
        ));
    }
    file.set_permissions(std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
pub(super) fn set_unix_mode(_file: &std::fs::File, mode: Option<u32>) -> std::io::Result<()> {
    if mode.is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "an exact Unix mode is unavailable on this platform",
        ));
    }
    Ok(())
}
