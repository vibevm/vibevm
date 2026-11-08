//! Config/trust content and identity metadata witnesses; no private-key decoding.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use super::*;
#[derive(Default)]
pub(super) struct Witness {
    files: BTreeMap<PathBuf, Option<Content>>,
    identities: BTreeMap<PathBuf, Option<Stamp>>,
    patterns: BTreeMap<String, Vec<PathBuf>>,
}
#[derive(PartialEq, Eq)]
struct Content {
    bytes: Vec<u8>,
    stamp: Stamp,
}
#[derive(PartialEq, Eq)]
struct Stamp {
    size: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    unix: (u64, u64, u64, u32, i64, i64),
}
fn stamp(path: &Path) -> Result<Option<Stamp>, GitError> {
    let metadata = match std::fs::metadata(path) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io(e)),
    };
    if !metadata.is_file() {
        return Err(unsupported("SSH identity reference is not a regular file"));
    }
    Ok(Some(metadata_stamp(&metadata)))
}
fn metadata_stamp(metadata: &std::fs::Metadata) -> Stamp {
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    Stamp {
        size: metadata.len(),
        modified: metadata.modified().ok(),
        #[cfg(unix)]
        unix: (
            metadata.dev(),
            metadata.ino(),
            u64::from(metadata.uid()),
            metadata.mode(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        ),
    }
}
fn read_content(path: &Path) -> Result<Option<Content>, GitError> {
    use std::io::Read;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    #[cfg(not(unix))]
    {
        return Err(unsupported(
            "bounded SSH configuration reads are not qualified on this platform",
        ));
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io(e)),
    };
    let before = file.metadata().map_err(io)?;
    if !before.is_file() || before.len() > 1024 * 1024 {
        return Err(unsupported(
            "SSH config/trust input is not a bounded regular file",
        ));
    }
    let mut bytes = Vec::new();
    (&file)
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    let after = file.metadata().map_err(io)?;
    if bytes.len() > 1024 * 1024
        || bytes.len() as u64 != before.len()
        || metadata_stamp(&before) != metadata_stamp(&after)
        || stamp(path)?.as_ref() != Some(&metadata_stamp(&after))
    {
        return Err(unsupported(
            "SSH configuration input changed or exceeded its read bound",
        ));
    }
    Ok(Some(Content {
        bytes,
        stamp: metadata_stamp(&after),
    }))
}

pub(super) fn expand(pattern: &str) -> Result<Vec<PathBuf>, GitError> {
    if pattern.contains("**") {
        return Err(unsupported("recursive SSH Include globs are not qualified"));
    }
    let mut files = glob::glob(pattern)
        .map_err(|_| unsupported("unsupported SSH Include glob"))?
        .take(257)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| unsupported("cannot read SSH Include glob"))?;
    files.sort();
    if files.len() > 256 {
        return Err(unsupported("SSH Include expansion exceeds capture bound"));
    }
    Ok(files)
}
impl Witness {
    pub(super) fn content(&mut self, path: &Path) -> Result<Option<Vec<u8>>, GitError> {
        let data = read_content(path)?;
        let bytes = data.as_ref().map(|v| v.bytes.clone());
        self.files.insert(path.into(), data);
        Ok(bytes)
    }
    pub(super) fn config_permissions(
        &self,
        path: &Path,
        owned_root: &Path,
    ) -> Result<(), GitError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let uid = u64::from(std::fs::metadata(owned_root).map_err(io)?.uid());
            if let Some(Some(content)) = self.files.get(path)
                && ((content.stamp.unix.2 != uid && content.stamp.unix.2 != 0)
                    || content.stamp.unix.3 & 0o022 != 0)
            {
                return Err(unsupported(
                    "original SSH config ownership or permissions are unsafe",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn identity(&mut self, path: &Path) -> Result<bool, GitError> {
        let value = stamp(path)?;
        let exists = value.is_some();
        self.identities.insert(path.into(), value);
        Ok(exists)
    }
    pub(super) fn glob(&mut self, pattern: &str) -> Result<Vec<PathBuf>, GitError> {
        let paths = expand(pattern)?;
        self.patterns.insert(pattern.into(), paths.clone());
        Ok(paths)
    }
    pub(super) fn recheck(&self) -> Result<(), GitError> {
        for (path, expected) in &self.files {
            let actual = read_content(path)?;
            if &actual != expected {
                return Err(unsupported("captured SSH configuration or trust changed"));
            }
        }
        for (path, expected) in &self.identities {
            if &stamp(path)? != expected {
                return Err(unsupported("captured SSH identity reference changed"));
            }
        }
        for (pattern, expected) in &self.patterns {
            if &expand(pattern)? != expected {
                return Err(unsupported("captured SSH Include expansion changed"));
            }
        }
        Ok(())
    }
}
