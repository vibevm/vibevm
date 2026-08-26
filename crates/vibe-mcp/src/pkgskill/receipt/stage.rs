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
        // The nonce is created exclusively, so this invocation is the one that
        // put an entry at that name — which is what licenses cleaning up after
        // a failure, and what bounds the cleanup to that name. It is a fact
        // about the creation, not a standing claim about the entry: `clean`
        // reopens and verifies before it removes anything. The nonce capability
        // is scoped so it is released before any cleanup — Windows refuses to
        // remove a directory an open handle still names.
        let mut owned = OwnedNonce::new(&nonce);
        let opened = match staged_root.create_child_exclusive(&nonce) {
            Ok(nonce_dir) => {
                owned.created();
                nonce_dir.ensure_child("files")
            }
            // Nothing was created, so there is nothing here to clean and the
            // guard stays disarmed: touching that name would be touching an
            // entry another process put there.
            Err(error @ vibe_safefs::ExclusiveChildError::NotCreated(_)) => {
                return Err(anyhow::Error::from(error)
                    .context(format!("staging bytes under a fresh nonce `{nonce}`")));
            }
            // This call created the entry and could not then reopen it, so what
            // is at that name now is unverified — possibly the directory we
            // made, possibly something swapped in. A bare `?` here is exactly
            // how an unreferenced nonce is born: no intent will ever name it,
            // so no later run can attribute it. Arm the guard and let cleanup
            // revalidate — remove what it can prove, name what it cannot.
            Err(error) => {
                owned.created();
                return Err(owned.clean(project, &staged_root, None, anyhow::Error::from(error)));
            }
        };
        let directory = match opened {
            Ok(directory) => directory,
            Err(error) => return Err(owned.clean(project, &staged_root, None, error)),
        };
        match Self::fill(project, &directory, desired, &mut owned) {
            Ok(files) => {
                owned.commit();
                Ok(Self { nonce, files })
            }
            Err(error) => {
                // No durable intent references this nonce yet, so leaving it
                // behind would be an orphan nobody ever collects. Remove only
                // what this invocation created, and if that fails say exactly
                // what remains rather than implying a clean tree.
                //
                // `clean` takes the capability by value because Windows
                // refuses to remove a directory an open handle still names —
                // the same reason `remove_dir_if_empty` drops its child probe.
                Err(owned.clean(project, &staged_root, Some(directory), error))
            }
        }
    }

    /// Write and verify every staged digest, recording each file this
    /// invocation published so a later failure knows precisely what it owns.
    fn fill(
        project: &Project,
        directory: &Pinned,
        desired: &BTreeMap<String, Vec<u8>>,
        owned: &mut OwnedNonce,
    ) -> Result<BTreeMap<String, Vec<u8>>> {
        let mut files = BTreeMap::new();
        for bytes in desired.values() {
            let sha = digest(bytes);
            let name = staged_name(&sha);
            // The typed report is the point: a failure after the rename must
            // stay distinguishable from one before it, exactly as it is for
            // every other caller of the shared writer.
            match project.write_atomic_in(directory, &name, bytes) {
                Ok(_) => owned.published(&name),
                Err(error) => {
                    if error.stage == vibe_safefs::PublishStage::PossiblyPublished {
                        owned.possibly_published(&name);
                    }
                    return Err(error.into_report());
                }
            }
            let durable = project
                .read_file_in(directory, &name)?
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
        Ok(files)
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
                .read_file_in(&directory, &name)
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
                project.remove_file_in(&directory, &name)?;
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

/// What one `Stage::create` invocation made, so a failure removes exactly that
/// and nothing else.
///
/// The nonce directory was created exclusively, so every name recorded here is
/// one this invocation put there. Cleanup never walks outside it, never removes
/// a file it did not publish, and never deletes a non-empty directory it does
/// not recognise — the alternative, "remove the nonce tree", would be ownership
/// by assumption rather than by evidence.
///
/// Having created a name licenses *attempting* to clean it, not deleting it
/// unverified: an entry can be removed, renamed or replaced between the create
/// and the cleanup, so every removal below reopens no-follow first and what
/// fails that reopen is named as residue instead.
struct OwnedNonce {
    nonce: String,
    /// Whether the exclusive creation actually happened. Having created the
    /// entry is the licence to *try* to clean it, so it is recorded as
    /// evidence rather than assumed from having reached this line: a `clean`
    /// that ran without it would be touching a name another process put there.
    /// It licenses no unguarded deletion — `clean` reopens no-follow and
    /// removes only what verifies.
    created: bool,
    published: Vec<String>,
    possibly_published: Vec<String>,
    committed: bool,
}

impl OwnedNonce {
    fn new(nonce: &str) -> Self {
        Self {
            nonce: nonce.to_string(),
            created: false,
            published: Vec::new(),
            possibly_published: Vec::new(),
            committed: false,
        }
    }

    /// Record that this invocation's own exclusive create succeeded at the
    /// nonce name. Not a claim that the entry there is still that directory.
    fn created(&mut self) {
        self.created = true;
    }

    fn published(&mut self, name: &str) {
        self.published.push(name.to_string());
    }

    fn possibly_published(&mut self, name: &str) {
        self.possibly_published.push(name.to_string());
    }

    fn commit(&mut self) {
        self.committed = true;
    }

    /// Remove this invocation's staged files and its own nonce directories.
    /// Whatever survives is named in the returned error, together with any
    /// file whose publication could not be ruled out.
    fn clean(
        &self,
        project: &Project,
        staged_root: &Pinned,
        directory: Option<Pinned>,
        cause: anyhow::Error,
    ) -> anyhow::Error {
        debug_assert!(!self.committed);
        // Not an assertion: "this call never created that name" is a state a
        // caller can reach, and the safe answer is to remove nothing at all.
        if !self.created {
            return cause;
        }
        let mut residue: Vec<String> = Vec::new();
        if let Some(directory) = directory {
            for name in self.published.iter().chain(&self.possibly_published) {
                match project.remove_file_in(&directory, name) {
                    Ok(_) => {}
                    Err(error) => residue.push(format!("{}/files/{name} ({error:#})", self.nonce)),
                }
            }
            // Release the handle before asking the OS to remove what it names.
            drop(directory);
        }
        // Each capability is released before the OS is asked to remove what it
        // names: Windows refuses to delete a directory an open handle holds.
        match staged_root.open_child_checked(&self.nonce) {
            Ok(Some(nonce_dir)) => {
                // Absent is not residue. `remove_dir_if_empty` answers `false`
                // for both "not empty" and "not there", and reporting the
                // second as leftover state would send an operator looking for
                // a directory that was never created.
                match nonce_dir.open_child_checked("files") {
                    Ok(None) => {}
                    Ok(Some(files)) => {
                        drop(files);
                        match project.remove_dir_if_empty(&nonce_dir, "files") {
                            Ok(true) => {}
                            Ok(false) => residue.push(format!("{}/files (not empty)", self.nonce)),
                            Err(error) => residue.push(format!("{}/files ({error:#})", self.nonce)),
                        }
                    }
                    Err(error) => residue.push(format!("{}/files ({error:#})", self.nonce)),
                }
                drop(nonce_dir);
            }
            // Nothing of ours is left to remove.
            Ok(None) => return annotate(cause, &self.possibly_published, &residue),
            Err(error) => {
                residue.push(format!("{} ({error:#})", self.nonce));
                return annotate(cause, &self.possibly_published, &residue);
            }
        }
        match project.remove_dir_if_empty(staged_root, &self.nonce) {
            Ok(true) => {}
            Ok(false) => residue.push(format!("{} (not empty)", self.nonce)),
            Err(error) => residue.push(format!("{} ({error:#})", self.nonce)),
        }
        annotate(cause, &self.possibly_published, &residue)
    }
}

fn annotate(cause: anyhow::Error, possibly: &[String], residue: &[String]) -> anyhow::Error {
    if possibly.is_empty() && residue.is_empty() {
        return cause;
    }
    let mut note = String::from("staging failed");
    if !possibly.is_empty() {
        note.push_str(&format!(
            "; these staged file(s) may have been published: {}",
            possibly.join(", ")
        ));
    }
    if !residue.is_empty() {
        note.push_str(&format!(
            "; this residue could not be removed and remains under \
             `.vibe/package-skills/staged/`: {}",
            residue.join(", ")
        ));
    }
    cause.context(note)
}
