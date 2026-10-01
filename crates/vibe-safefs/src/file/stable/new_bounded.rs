//! Bounded streaming copy into a create-new destination.
//!
//! The hard-link claim is atomic against any occupant on Linux, macOS and
//! Windows local filesystems that support regular-file hard links. A foreign
//! writer that mutates the source or published file after the witnessed epoch
//! remains a detection-bound alias race; file-index/inode reuse is likewise
//! outside this same-host observation contract.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use super::{
    StableFileState, WINDOW, finish_source_state, hash_pass, mode_matches,
    read_stable_with_identity_in, recheck_name, set_unix_mode,
};
use crate::file::{
    cap_options, create_unique_stage, file_identity, injected_post_publication_failure,
    verify_regular_single_link,
};
use crate::project::{Pinned, Project};
use crate::publish::{PublishError, Published};

fn copy_bounded(
    source: &mut std::fs::File,
    staged: &mut std::fs::File,
    max_bytes: u64,
) -> Result<StableFileState> {
    source.seek(SeekFrom::Start(0))?;
    let mut hash = Sha256::new();
    let mut bytes = 0_u64;
    let mut window = [0_u8; WINDOW];
    loop {
        let read = source.read(&mut window)?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .context("source length overflow")?;
        if bytes > max_bytes {
            bail!("source grew beyond its {max_bytes}-byte copy cap");
        }
        staged.write_all(&window[..read])?;
        hash.update(&window[..read]);
    }
    Ok(StableFileState {
        sha256: format!("{:x}", hash.finalize()),
        bytes,
        unix_mode: None,
    })
}

fn before_with_stage(
    staged: std::fs::File,
    holder: &Pinned,
    name: &str,
    created: &[PathBuf],
    error: anyhow::Error,
) -> PublishError {
    drop(staged);
    match holder.dir.remove_file(name) {
        Ok(()) => PublishError::before(created.to_vec(), error),
        Err(cleanup) => PublishError::possibly(
            created.to_vec(),
            anyhow::anyhow!(
                "pre-publication failure: {error:#}; owned stage `{}` survived: {cleanup}",
                holder.join(name).display()
            ),
        ),
    }
}

impl Project {
    /// Copy exact expected bytes through one held no-follow source into an
    /// absent destination. Both source passes and the final destination
    /// witness obey `max_bytes`; a fixed stack window holds the payload.
    /// `BeforePublication` proves the destination name was not created by
    /// this call; `PossiblyPublished` requires caller recovery without retry.
    #[allow(clippy::too_many_arguments, reason = "one exact guarded copy")]
    pub fn copy_stable_file_to_expected_new_bounded(
        &self,
        source_relative: &str,
        destination: &Project,
        destination_relative: &str,
        expected_sha256: &str,
        expected_bytes: u64,
        max_bytes: u64,
        desired_mode: Option<u32>,
    ) -> std::result::Result<(StableFileState, Published), PublishError> {
        let before = |error| PublishError::before(Vec::new(), error);
        if max_bytes == 0 || expected_bytes > max_bytes {
            return Err(before(anyhow::anyhow!(
                "invalid bounded copy limit or expected length"
            )));
        }
        let source_root = self.root_dir().map_err(&before)?;
        let Some((source_holder, source_name)) = self
            .holder_of(&source_root, source_relative)
            .map_err(&before)?
        else {
            return Err(before(anyhow::anyhow!(
                "source `{source_relative}` is absent"
            )));
        };
        let source_display = source_holder.join(&source_name);
        let mut options = cap_options();
        let mut source = source_holder
            .dir
            .open_with(&source_name, options.read(true))
            .map_err(|error| before(anyhow::Error::new(error)))?
            .into_std();
        verify_regular_single_link(&source, &source_display).map_err(&before)?;
        let metadata = source.metadata().map_err(|error| before(error.into()))?;
        if metadata.len() > max_bytes {
            return Err(before(anyhow::anyhow!(
                "source exceeds its {max_bytes}-byte copy cap"
            )));
        }
        let first = hash_pass(&mut source, &source_display, Some(max_bytes))
            .and_then(|state| finish_source_state(&source, &source_display, &metadata, state))
            .map_err(&before)?;
        recheck_name(&source_holder, &source_name, &source, &source_display).map_err(&before)?;
        if first.sha256 != expected_sha256 || first.bytes != expected_bytes {
            return Err(before(anyhow::anyhow!(
                "source differs from expected digest or length"
            )));
        }
        crate::race_hook::between_stream_passes(&source_holder, &source_name);

        let mut created = Vec::new();
        let destination_root = destination
            .root_dir()
            .map_err(|error| PublishError::before(created.clone(), error))?;
        let (parents, name) = crate::component::split_relative(destination_relative)
            .map_err(|error| PublishError::before(created.clone(), error))?;
        let holder = if parents.is_empty() {
            destination_root
        } else {
            let parts = parents.iter().map(String::as_str).collect::<Vec<_>>();
            destination
                .dir_at_recording(&destination_root, &parts, &mut created)
                .map_err(|error| PublishError::before(created.clone(), error))?
        };
        let before_dest = |error| PublishError::before(created.clone(), error);
        let (stage_name, staged_file) = create_unique_stage(&holder).map_err(&before_dest)?;
        let stage_display = holder.join(&stage_name);
        let mut staged = staged_file.into_std();
        let stage_identity = match file_identity(&staged, &stage_display) {
            Ok(identity) => identity,
            Err(error) => {
                return Err(before_with_stage(
                    staged,
                    &holder,
                    &stage_name,
                    &created,
                    error,
                ));
            }
        };
        let copied = copy_bounded(&mut source, &mut staged, max_bytes)
            .and_then(|state| finish_source_state(&source, &source_display, &metadata, state));
        let second = match copied {
            Ok(state) if state.sha256 == first.sha256 && state.bytes == first.bytes => state,
            Ok(_) => {
                return Err(before_with_stage(
                    staged,
                    &holder,
                    &stage_name,
                    &created,
                    anyhow::anyhow!("source changed between copy passes"),
                ));
            }
            Err(error) => {
                return Err(before_with_stage(
                    staged,
                    &holder,
                    &stage_name,
                    &created,
                    error,
                ));
            }
        };
        if let Err(error) = recheck_name(&source_holder, &source_name, &source, &source_display)
            .and_then(|()| staged.flush().map_err(Into::into))
            .and_then(|()| set_unix_mode(&staged, desired_mode).map_err(Into::into))
            .and_then(|()| staged.sync_all().map_err(Into::into))
        {
            return Err(before_with_stage(
                staged,
                &holder,
                &stage_name,
                &created,
                error,
            ));
        }
        // The link is the create-new authority. In particular an occupant
        // planted after source proof is not replaced on any supported host.
        crate::race_hook::before_link(&holder, &name);
        if let Err(error) = holder.dir.hard_link(&stage_name, &holder.dir, &name) {
            drop(staged);
            let cleanup = holder.dir.remove_file(&stage_name);
            return Err(if cleanup.is_ok() {
                before_dest(anyhow::Error::new(error))
            } else {
                PublishError::possibly(
                    created,
                    anyhow::anyhow!(
                        "create-new link failed and owned stage cleanup failed: {error}"
                    ),
                )
            });
        }
        let possibly = |error| PublishError::possibly(created.clone(), error);
        if let Err(error) = holder.dir.remove_file(&stage_name) {
            return Err(possibly(anyhow::anyhow!(
                "destination may exist with a surviving owned stage: {error}"
            )));
        }
        crate::race_hook::before_publish_verify(&holder, &name);
        let visible =
            read_stable_with_identity_in(destination, &holder, &name, Some(expected_bytes))
                .map_err(&possibly)?;
        if !matches!(visible, Some((ref state, identity))
            if identity == stage_identity
                && state.sha256 == second.sha256
                && state.bytes == second.bytes
                && mode_matches(state.unix_mode, desired_mode))
        {
            return Err(possibly(anyhow::anyhow!(
                "create-new destination changed or differs from held source"
            )));
        }
        if let Ok(handle) = holder.dir.try_clone() {
            let _ = handle.into_std_file().sync_all();
        }
        if let Some(error) = injected_post_publication_failure(destination_relative) {
            return Err(possibly(error));
        }
        drop(staged);
        Ok((
            second,
            Published {
                created_directories: created,
            },
        ))
    }
}
