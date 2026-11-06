//! Read-only source observations permit shared regular files, not write ownership.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
);

use super::{identity::file_identity, regular_read_options, stable::unix_mode};
use crate::{Pinned, Project};
use anyhow::{Context, Result, bail};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

impl Project {
    /// Observe one source file without following descendant links or copying
    /// shared inodes. The held initial length fences two agreeing passes;
    /// allocation failure, changed content and a replaced final name refuse.
    /// This grants no exclusive ownership or whole-tree snapshot guarantee.
    ///
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// let root = tempfile::tempdir()?;
    /// std::fs::write(root.path().join("source.txt"), b"source")?;
    /// let project = vibe_safefs::Project::open(root.path())?;
    /// assert_eq!(project.read_source_bytes("source.txt")?, Some(b"source".to_vec()));
    /// assert_eq!(project.read_source_bytes("absent.txt")?, None);
    /// # Ok(())
    /// # }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
    )]
    pub fn read_source_bytes(&self, relative: &str) -> Result<Option<Vec<u8>>> {
        self.read_source_bytes_in(&self.root_dir()?, relative)
    }

    /// The same source observation below a parent retained by the locator.
    ///
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// let root = tempfile::tempdir()?;
    /// std::fs::create_dir(root.path().join("docs"))?;
    /// std::fs::write(root.path().join("docs/one.md"), b"one")?;
    /// let project = vibe_safefs::Project::open(root.path())?;
    /// let docs = project.dir(&["docs"], false)?;
    /// assert_eq!(project.read_source_bytes_in(&docs, "one.md")?, Some(b"one".to_vec()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_source_bytes_in(
        &self,
        directory: &Pinned,
        relative: &str,
    ) -> Result<Option<Vec<u8>>> {
        let Some((holder, name)) = self.holder_of(directory, relative)? else {
            return Ok(None);
        };
        let path = holder.join(&name);
        let mut options = regular_read_options();
        let mut held = match holder.dir.open_with(&name, options.read(true)) {
            Ok(file) => file.into_std(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("opening source `{}` no-follow", path.display()));
            }
        };
        let before = regular(&held, &path)?;
        let identity = file_identity(&held, &path)?;
        let mut bytes = reserve_source(before.len())?;
        crate::race_hook::before_bounded_read(&holder, &name);
        held.read_exact(&mut bytes).with_context(|| {
            format!(
                "source `{}` changed length or could not be read",
                path.display()
            )
        })?;
        if read_chunk(&mut held, &mut [0_u8; 1])? != 0 {
            bail!(
                "source `{}` grew beyond its observed length",
                path.display()
            );
        }
        crate::race_hook::between_stream_passes(&holder, &name);
        held.seek(SeekFrom::Start(0))
            .context("rewinding held source")?;
        compare_pass(&mut held, &bytes)
            .with_context(|| format!("source `{}` changed between held passes", path.display()))?;
        let after = regular(&held, &path)?;
        if before.len() != after.len() || unix_mode(&before) != unix_mode(&after) {
            bail!(
                "source `{}` changed length or mode during observation",
                path.display()
            );
        }
        let current = holder
            .dir
            .open_with(&name, options.read(true))
            .with_context(|| format!("reopening source `{}` after observation", path.display()))?
            .into_std();
        regular(&current, &path)?;
        if !crate::race_hook::bounded_read_identity_matches(
            identity == file_identity(&current, &path)?,
        ) {
            bail!(
                "source `{}` was replaced during observation",
                path.display()
            );
        }
        Ok(Some(bytes))
    }

    /// Check source file membership without reading content. Shared regular
    /// files count; directories/absence do not. Links and special entries
    /// refuse. This finite metadata observation does not pin later bytes.
    ///
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// let root = tempfile::tempdir()?;
    /// std::fs::write(root.path().join("source.txt"), b"source")?;
    /// let project = vibe_safefs::Project::open(root.path())?;
    /// let parent = project.root_dir()?;
    /// assert!(project.source_file_present_in(&parent, "source.txt")?);
    /// assert!(!project.source_file_present_in(&parent, "absent")?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn source_file_present_in(&self, parent: &Pinned, name: &str) -> Result<bool> {
        crate::ensure_safe_component(name)?;
        let metadata = match parent.dir.symlink_metadata(name) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error).context("inspecting source candidate"),
        };
        if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
            bail!(
                "source candidate `{}` is a link or special entry",
                parent.join(name).display()
            );
        }
        Ok(metadata.is_file())
    }
}

fn regular(file: &std::fs::File, path: &Path) -> Result<std::fs::Metadata> {
    let metadata = file
        .metadata()
        .with_context(|| format!("inspecting source `{}`", path.display()))?;
    if !metadata.is_file() {
        bail!("source `{}` is not a regular file", path.display());
    }
    Ok(metadata)
}

fn reserve_source(length: u64) -> Result<Vec<u8>> {
    let length = usize::try_from(length).context("source length exceeds address space")?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .context("cannot allocate observed source length")?;
    bytes.resize(length, 0);
    Ok(bytes)
}

fn read_chunk(reader: &mut impl Read, chunk: &mut [u8]) -> std::io::Result<usize> {
    loop {
        match reader.read(chunk) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

fn compare_pass(reader: &mut impl Read, expected: &[u8]) -> Result<()> {
    let mut chunk = [0_u8; super::bounded::READ_CHUNK];
    let mut offset = 0;
    loop {
        let used = read_chunk(reader, &mut chunk)?;
        if used == 0 {
            if offset != expected.len() {
                bail!("source shortened during comparison");
            }
            return Ok(());
        }
        if expected.get(offset..).and_then(|tail| tail.get(..used)) != Some(&chunk[..used]) {
            bail!("source bytes differ during comparison");
        }
        offset += used;
    }
}

#[cfg(test)]
#[path = "source_read_tests.rs"]
mod tests;
