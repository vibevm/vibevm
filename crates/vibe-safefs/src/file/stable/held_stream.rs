//! One held, verified source streamed without a content-sized allocation.
//!
//! A successful completion proves the exact bytes were *read* by the body;
//! it does not prove a remote host received them. A transport failure or
//! lost response remains uncertain even if this local witness is complete.
//! Hostile writes after the final observation, or an ABA identity reuse, are
//! outside this cooperative Linux/macOS/Windows detection-bound contract.

use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use super::{StableFileState, WINDOW, finish_source_state, hash_pass, recheck_name, unix_mode};
use crate::file::{cap_options, file_identity, verify_regular_single_link};
use crate::{FileIdentity, Pinned, Project};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamOutcome {
    Incomplete,
    Complete { bytes: u64, sha256: String },
    Uncertain(String),
}

/// Read-only shared witness retained after the stream reader moves into a
/// streaming HTTP body. The caller must also verify the HTTP result and the
/// remote object; this witness alone grants no effect or retry.
#[derive(Debug, Clone)]
pub struct StreamCompletion(Arc<Mutex<StreamOutcome>>);

impl StreamCompletion {
    #[must_use]
    pub fn outcome(&self) -> StreamOutcome {
        self.0.lock().map_or_else(
            |_| StreamOutcome::Uncertain("stream witness lock poisoned".into()),
            |state| state.clone(),
        )
    }
}

/// Move-only, no-follow reader. A `Read` call emits at most one fixed window;
/// it never buffers a whole release asset.
pub struct VerifiedStreamReader {
    file: std::fs::File,
    holder: Pinned,
    name: String,
    display: PathBuf,
    opening: std::fs::Metadata,
    identity: FileIdentity,
    expected_sha256: String,
    expected_bytes: u64,
    delivered: u64,
    hash: Sha256,
    complete: StreamCompletion,
    finished: bool,
}

impl std::fmt::Debug for VerifiedStreamReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedStreamReader")
            .field("display", &self.display)
            .field("expected_bytes", &self.expected_bytes)
            .field("delivered", &self.delivered)
            .finish_non_exhaustive()
    }
}

impl VerifiedStreamReader {
    fn uncertain(&mut self, reason: impl Into<String>) -> io::Error {
        let reason = reason.into();
        if let Ok(mut state) = self.complete.0.lock() {
            *state = StreamOutcome::Uncertain(reason.clone());
        }
        io::Error::other(reason)
    }

    fn finish(&mut self) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        let mut check = || -> Result<String> {
            let mut extra = [0_u8; 1];
            if self.file.read(&mut extra)? != 0 {
                bail!("held stream grew beyond its exact length");
            }
            verify_regular_single_link(&self.file, &self.display)?;
            let state = StableFileState {
                sha256: format!("{:x}", self.hash.clone().finalize()),
                bytes: self.delivered,
                unix_mode: unix_mode(&self.opening),
            };
            let state = finish_source_state(&self.file, &self.display, &self.opening, state)?;
            if file_identity(&self.file, &self.display)? != self.identity {
                bail!("held stream file identity changed");
            }
            recheck_name(&self.holder, &self.name, &self.file, &self.display)?;
            if state.bytes != self.expected_bytes || state.sha256 != self.expected_sha256 {
                bail!("held stream bytes differ from exact expected digest or length");
            }
            Ok(state.sha256)
        };
        let digest = check().map_err(|error| self.uncertain(format!("{error:#}")))?;
        self.finished = true;
        if let Ok(mut state) = self.complete.0.lock() {
            *state = StreamOutcome::Complete {
                bytes: self.delivered,
                sha256: digest,
            };
        }
        Ok(())
    }
}

impl Read for VerifiedStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.finished {
            return Ok(0);
        }
        if self.delivered == self.expected_bytes {
            self.finish()?;
            return Ok(0);
        }
        let remaining = usize::try_from(self.expected_bytes - self.delivered).unwrap_or(usize::MAX);
        let wanted = buf.len().min(WINDOW).min(remaining);
        let read = loop {
            match self.file.read(&mut buf[..wanted]) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                value => break value,
            }
        }
        .map_err(|error| self.uncertain(format!("held stream read: {error}")))?;
        if read == 0 {
            return Err(self.uncertain("held stream ended before expected length"));
        }
        self.hash.update(&buf[..read]);
        self.delivered += read as u64;
        if self.delivered == self.expected_bytes {
            self.finish()?;
        }
        Ok(read)
    }
}

impl Drop for VerifiedStreamReader {
    fn drop(&mut self) {
        if !self.finished
            && let Ok(mut state) = self.complete.0.lock()
        {
            *state = StreamOutcome::Uncertain("held stream dropped before completion".into());
        }
    }
}

impl Project {
    /// Open one source through this held project, prove two capped passes on
    /// its no-follow single-link handle, then rewind that SAME handle for a
    /// future `reqwest::blocking::Body::sized` or fake-port stream.
    pub fn open_verified_stream_bounded(
        &self,
        relative: &str,
        expected_sha256: &str,
        expected_bytes: u64,
        max_bytes: u64,
    ) -> Result<(VerifiedStreamReader, StreamCompletion)> {
        if max_bytes == 0
            || expected_bytes > max_bytes
            || expected_sha256.len() != 64
            || !expected_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            bail!("invalid held stream digest, length or cap");
        }
        let root = self.root_dir()?;
        let (holder, name) = self
            .holder_of(&root, relative)?
            .with_context(|| format!("held stream `{relative}` is absent"))?;
        let display = holder.join(&name);
        let mut options = cap_options();
        let mut file = holder
            .dir
            .open_with(&name, options.read(true))
            .with_context(|| format!("opening `{}` without following links", display.display()))?
            .into_std();
        verify_regular_single_link(&file, &display)?;
        let opening = file.metadata()?;
        if opening.len() > max_bytes {
            bail!("held stream exceeds its {max_bytes}-byte cap");
        }
        let identity = file_identity(&file, &display)?;
        let first = hash_pass(&mut file, &display, Some(max_bytes))
            .and_then(|state| finish_source_state(&file, &display, &opening, state))?;
        crate::race_hook::between_stream_passes(&holder, &name);
        let second = hash_pass(&mut file, &display, Some(max_bytes))
            .and_then(|state| finish_source_state(&file, &display, &opening, state))?;
        if first != second || first.sha256 != expected_sha256 || first.bytes != expected_bytes {
            bail!("held stream changed or differs from expected bytes");
        }
        recheck_name(&holder, &name, &file, &display)?;
        file.seek(SeekFrom::Start(0))?;
        let complete = StreamCompletion(Arc::new(Mutex::new(StreamOutcome::Incomplete)));
        let mut reader = VerifiedStreamReader {
            file,
            holder,
            name,
            display,
            opening,
            identity,
            expected_sha256: expected_sha256.into(),
            expected_bytes,
            delivered: 0,
            hash: Sha256::new(),
            complete: complete.clone(),
            finished: false,
        };
        if expected_bytes == 0 {
            reader.finish()?;
        }
        Ok((reader, complete))
    }
}

#[cfg(test)]
#[path = "held_stream_tests.rs"]
mod tests;
