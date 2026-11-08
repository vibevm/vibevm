//! Stage-handle publication receipts for cooperative MCP batch undo.
//!
//! No after-state read mints mutation authority. Stable observations must
//! match the original snapshot or the actual writer's OwnedFileReceipt.
//! Safefs documents a final uncooperative syscall window: this is not kernel CAS.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION");
use super::*;
use vibe_safefs::{OwnedFileReceipt, Project, PublishStage, StableFileSnapshot};

const CONFIG_CAP: usize = 64 * 1024 * 1024;

#[cfg(test)]
type BeforeWrite = Box<dyn FnMut(&Path) -> Result<()>>;

#[derive(Default)]
pub(super) struct PublicationBatch {
    files: BTreeMap<PathBuf, PublishedFile>,
    attempted: bool,
    inverting: bool,
    created: std::collections::BTreeSet<PathBuf>,
    #[cfg(test)]
    before_write: Option<BeforeWrite>,
}

struct PublishedFile {
    project: Project,
    relative: String,
    before: Option<StableFileSnapshot>,
    expected: Expected,
}

enum Expected {
    Before,
    Published(OwnedFileReceipt),
    Removed,
    Uncertain,
    // Terminal for this file: verification only, never fresh mutation authority.
    Restored,
}

pub(super) struct Mutation<'a> {
    pub(super) dry_run: bool,
    pub(super) batch: &'a mut PublicationBatch,
}

impl PublishedFile {
    fn capture(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            bail!("MCP publication requires an absolute destination");
        }
        let mut anchor = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("MCP file has no parent"))?;
        loop {
            match fs::symlink_metadata(anchor) {
                Ok(_) => break,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    anchor = anchor.parent().ok_or_else(|| {
                        anyhow::anyhow!("MCP destination has no existing ancestor")
                    })?;
                }
                Err(error) => return Err(error).context("inspecting MCP publication ancestor"),
            }
        }
        let project = Project::open(anchor)?;
        let relative = path
            .strip_prefix(anchor)?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("MCP relative destination is not UTF-8"))?
            .replace('\\', "/");
        let before = project.read_file_snapshot_bounded(&relative, CONFIG_CAP)?;
        Ok(Self {
            project,
            relative,
            before,
            expected: Expected::Before,
        })
    }

    fn observe(&self) -> Result<Option<StableFileSnapshot>> {
        let visible = Project::open(self.project.root_path())?;
        if visible.root_identity()? != self.project.root_identity()? {
            bail!("MCP publication ancestor was replaced; newer namespace preserved");
        }
        self.project
            .read_file_snapshot_bounded(&self.relative, CONFIG_CAP)
    }

    fn matches_expected(&self, current: Option<&StableFileSnapshot>) -> bool {
        match &self.expected {
            Expected::Before => current == self.before.as_ref(),
            Expected::Published(receipt) => current.is_some_and(|s| receipt.matches_snapshot(s)),
            Expected::Removed => current.is_none(),
            Expected::Uncertain => false,
            Expected::Restored => self.matches_before_bytes(current),
        }
    }

    fn matches_before_bytes(&self, current: Option<&StableFileSnapshot>) -> bool {
        match (self.before.as_ref(), current) {
            (None, None) => true,
            (Some(before), Some(current)) => {
                before.bytes == current.bytes && before.unix_mode == current.unix_mode
            }
            _ => false,
        }
    }

    fn require_expected(&self, path: &Path) -> Result<()> {
        if !self.matches_expected(self.observe()?.as_ref()) {
            bail!(
                "MCP publication identity/content/mode changed at `{}`; recovery required; newer state preserved",
                path.display()
            );
        }
        Ok(())
    }

    fn parent(&self) -> Result<(vibe_safefs::Pinned, String)> {
        let (parents, name) = vibe_safefs::split_relative(&self.relative)?;
        let components = parents.iter().map(String::as_str).collect::<Vec<_>>();
        Ok((self.project.dir(&components, false)?, name))
    }
}

impl PublicationBatch {
    pub(super) fn capture(&mut self, path: &Path) -> Result<()> {
        if self.inverting {
            bail!("MCP batch is already in inverse; start a fresh operation after recovery");
        }
        if !self.files.contains_key(path) {
            self.files
                .insert(path.into(), PublishedFile::capture(path)?);
        }
        self.files
            .get(path)
            .ok_or_else(|| anyhow::anyhow!("uncaptured MCP publication"))?
            .require_expected(path)
    }

    pub(super) fn capture_config(&mut self, config: &Path) -> Result<()> {
        self.capture(config)?;
        self.capture(&receipt_path(config)?)
    }

    pub(super) fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<()> {
        self.publish(path, Some(bytes))
    }
    pub(super) fn remove(&mut self, path: &Path) -> Result<()> {
        self.publish(path, None)
    }

    fn publish(&mut self, path: &Path, bytes: Option<&[u8]>) -> Result<()> {
        self.capture(path)?;
        #[cfg(test)]
        if let Some(hook) = &mut self.before_write {
            hook(path)?;
        }
        let file = self
            .files
            .get_mut(path)
            .ok_or_else(|| anyhow::anyhow!("lost MCP publication"))?;
        file.require_expected(path)?;
        let current = file.observe()?;
        if current.as_ref().map(|s| s.bytes.as_slice()) == bytes {
            return Ok(());
        }
        self.attempted = true;
        if let Some(bytes) = bytes {
            let mode = current.as_ref().and_then(|s| s.unix_mode).or(Some(0o600));
            match file.project.write_atomic_owned_in_with_mode(
                &file.project.root_dir()?,
                &file.relative,
                bytes,
                mode,
            ) {
                Ok(written) => {
                    self.created
                        .extend(written.publication.published.created_directories);
                    file.expected = Expected::Published(written.receipt);
                }
                Err(error) => {
                    self.created
                        .extend(error.publication.created_directories.iter().cloned());
                    if error.publication.stage == PublishStage::PossiblyPublished {
                        file.expected = error
                            .receipt
                            .map_or(Expected::Uncertain, Expected::Published);
                    }
                    return Err(error.publication.into_report());
                }
            }
        } else {
            // The final unlink window is safefs' documented cooperative boundary.
            file.require_expected(path)?;
            if !file.project.remove_file(&file.relative)? {
                file.expected = Expected::Uncertain;
                bail!("MCP entry vanished before owned removal; recovery required");
            }
            file.expected = Expected::Removed;
        }
        Ok(())
    }

    pub(super) fn receipt(&mut self, config: &Path, receipt: &Receipt) -> Result<()> {
        self.write(&receipt_path(config)?, &encode_receipt(receipt)?)
    }

    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION"
    )]

    pub(super) fn rollback(&mut self) -> Result<()> {
        self.inverting = true;
        // Preflight the COMPLETE config+receipt group, including untouched members.
        for (path, file) in &self.files {
            file.require_expected(path)?;
        }
        for (path, file) in &mut self.files {
            if matches!(file.expected, Expected::Before | Expected::Restored) {
                continue;
            }
            file.require_expected(path)?;
            match &file.before {
                Some(before) => {
                    if matches!(file.expected, Expected::Removed) {
                        // Restore a deleted file with no-replace, never replace a new occupant.
                        let (parent, name) = file.parent()?;
                        file.project
                            .publish_new_in_with_mode(
                                &parent,
                                &name,
                                &before.bytes,
                                before.unix_mode,
                            )
                            .map_err(vibe_safefs::PublishError::into_report)?;
                    } else {
                        match file.project.write_atomic_owned_in_with_mode(
                            &file.project.root_dir()?,
                            &file.relative,
                            &before.bytes,
                            before.unix_mode,
                        ) {
                            Ok(_restored) => {}
                            Err(error) => {
                                if error.publication.stage == PublishStage::PossiblyPublished {
                                    // Retain evidence; a later retry must not treat uncertain bytes as ours.
                                    file.expected = error
                                        .receipt
                                        .map_or(Expected::Uncertain, Expected::Published);
                                }
                                return Err(error.publication.into_report())
                                    .context("MCP inverse publication failed; recovery required");
                            }
                        }
                    }
                }
                None => {
                    file.require_expected(path)?;
                    if !file.project.remove_file(&file.relative)? {
                        bail!("MCP inverse entry vanished; recovery required");
                    }
                }
            }
            // This state is verification-only; forward publication after inverse is forbidden.
            file.expected = Expected::Restored;
        }
        for (path, file) in &self.files {
            if !file.matches_before_bytes(file.observe()?.as_ref()) {
                bail!(
                    "MCP rollback verification failed at `{}`; recovery required",
                    path.display()
                );
            }
        }
        Ok(())
    }

    pub(super) fn failed(&mut self, error: anyhow::Error, operation: &str) -> anyhow::Error {
        if !self.attempted {
            return error.context(format!("{operation} refused before any owned publication"));
        }
        match self.rollback() {
            Ok(())=>error.context(format!("{operation} failed; prior config and receipt bytes/modes restored; {} created parent directories retained",self.created.len())),
            Err(conflict)=>anyhow::anyhow!("{operation} failed: {error}; recovery required: {conflict}"),
        }
    }

    #[cfg(test)]
    pub(super) fn on_before_write(&mut self, hook: impl FnMut(&Path) -> Result<()> + 'static) {
        self.before_write = Some(Box::new(hook));
    }
}
