//! Durable staging of desired bytes for a receipt transaction.
//!
//! Before an `applying` intent is published, every desired digest is staged
//! content-addressably under `.vibe/package-skills/staged/<nonce>/files/
//! <sha256>` in an exclusively created nonce directory. The staged bytes are
//! the only proof that an interrupted new file is ours: recovery accepts a
//! present file only when it hashes to the staged desired digest (or was
//! previously owned), never the intent alone. Bytes are always served from
//! the durable files, never from in-memory desired copies.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};

use super::nofollow::{Pinned, Project};
use super::state::{digest, fresh_nonce};

const STAGE_CHAIN: [&str; 2] = ["package-skills", "staged"];

/// One transaction's staged desired bytes, keyed by `sha256:<hex>` digest.
#[derive(Debug)]
pub(super) struct Stage {
    pub nonce: String,
    files: BTreeMap<String, Vec<u8>>,
}

impl Stage {
    /// Stage every unique desired digest for a new transaction under a fresh
    /// nonce directory created exclusively — a collision refuses rather than
    /// reusing crash or attacker bytes. After writing, every file is reopened
    /// through the capability, verified regular/single-link, and byte-hashed
    /// to its filename; the durable bytes are authoritative.
    pub(super) fn create(project: &Project, desired: &BTreeMap<String, Vec<u8>>) -> Result<Self> {
        let nonce = fresh_nonce();
        let staged_root = project.dir(&[".vibe", STAGE_CHAIN[0], STAGE_CHAIN[1]], true)?;
        let nonce_dir = staged_root.create_child_exclusive(&nonce)?;
        let directory = nonce_dir.ensure_child("files")?;
        let mut files = BTreeMap::new();
        for bytes in desired.values() {
            let sha = digest(bytes);
            let name = staged_name(&sha);
            project.write_atomic(&directory, &name, bytes)?;
            let durable = project
                .read_file(&directory, &name)?
                .with_context(|| format!("verifying staged `{name}` after publication"))?;
            let durable_sha = digest(&durable);
            if durable_sha != sha {
                bail!(
                    "staged file `{name}` hashes to `{durable_sha}` instead of `{sha}`; \
                     refusing to trust the durable stage"
                );
            }
            files.insert(durable_sha, durable);
        }
        Ok(Self { nonce, files })
    }

    /// Reference an already-durable stage from a published intent (recovery)
    /// without creating anything, loading **only** the exact required digest
    /// set. A required file that is missing, corrupt (hash mismatch), or
    /// hardlinked refuses. A different correctly content-addressed file in
    /// the same nonce directory is never adopted into the Stage and is
    /// therefore never deleted by cleanup.
    pub(super) fn existing(
        project: &Project,
        nonce: &str,
        required: &std::collections::BTreeSet<String>,
    ) -> Result<Self> {
        let directory = open_stage_directory(project, nonce)?;
        let mut files = BTreeMap::new();
        for sha in required {
            let name = staged_name(sha);
            let bytes = project
                .read_file(&directory, &name)
                .with_context(|| format!("loading required staged file `{name}`"))?
                .with_context(|| format!("required staged file `{name}` is missing"))?;
            let durable_sha = digest(&bytes);
            if durable_sha != *sha {
                bail!(
                    "required staged file `{name}` hashes to `{durable_sha}` instead of `{sha}`; \
                     refusing to trust the durable stage"
                );
            }
            files.insert(durable_sha, bytes);
        }
        Ok(Self {
            nonce: nonce.to_string(),
            files,
        })
    }

    /// The staged bytes for one `sha256:<hex>` digest; a required digest that
    /// is missing or was corrupt on disk is a hard refusal.
    pub(super) fn require(&self, sha256: &str) -> Result<&[u8]> {
        self.files.get(sha256).map(Vec::as_slice).with_context(|| {
            format!(
                "required staged bytes `{sha256}` are missing or corrupt in the durable stage; \
                     restore `.vibe/package-skills/staged/{}` or remove the target manually",
                self.nonce
            )
        })
    }

    /// Remove **only the validated digest files referenced by this Stage**,
    /// then the now-empty `files`, nonce, and parent stage directories, in
    /// that order. Unexpected neighbours inside `files` are preserved and
    /// keep their directories alive; removal errors propagate.
    pub(super) fn cleanup(&self, project: &Project) -> Result<()> {
        let referenced = self
            .files
            .keys()
            .map(|sha| staged_name(sha))
            .collect::<Vec<_>>();
        {
            let directory = open_stage_directory(project, &self.nonce)?;
            for name in referenced {
                project.remove_file(&directory, &name)?;
            }
        }
        {
            let nonce_dir = project.dir(
                &[".vibe", STAGE_CHAIN[0], STAGE_CHAIN[1], &self.nonce],
                false,
            )?;
            project.remove_dir_if_empty(&nonce_dir, "files")?;
        }
        {
            let staged = project.dir(&[".vibe", STAGE_CHAIN[0], STAGE_CHAIN[1]], false)?;
            project.remove_dir_if_empty(&staged, &self.nonce)?;
        }
        {
            let skills = project.dir(&[".vibe", STAGE_CHAIN[0]], false)?;
            project.remove_dir_if_empty(&skills, STAGE_CHAIN[1])?;
        }
        Ok(())
    }
}

fn open_stage_directory(project: &Project, nonce: &str) -> Result<Pinned> {
    project.dir(
        &[".vibe", STAGE_CHAIN[0], STAGE_CHAIN[1], nonce, "files"],
        false,
    )
}

fn staged_name(sha256: &str) -> String {
    sha256.strip_prefix("sha256:").unwrap_or(sha256).to_string()
}
