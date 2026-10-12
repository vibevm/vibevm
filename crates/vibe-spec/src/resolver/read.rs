//! Bounded read consistency for internal source callers; not a retained-handle grant.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#INTERNAL-LOOKUP-FAILURES");

use std::fs;
use std::io;
use std::path::Path;

use crate::SpecAddress;

use super::{FileResolver, ResolveError, portable};

impl FileResolver {
    pub(crate) fn read_projected_source(
        &self,
        addr: &SpecAddress,
        path: &Path,
    ) -> Result<String, String> {
        let bytes = self
            .read_preflighted_source(addr, path)
            .map_err(|error| error.to_string())?;
        project_source(path, &bytes)
    }

    pub(crate) fn read_preflighted_source(
        &self,
        addr: &SpecAddress,
        path: &Path,
    ) -> Result<Vec<u8>, ResolveError> {
        self.read_preflighted_with(addr, path, |path| fs::read(path))
    }

    fn read_preflighted_with(
        &self,
        addr: &SpecAddress,
        path: &Path,
        read: impl FnMut(&Path) -> io::Result<Vec<u8>>,
    ) -> Result<Vec<u8>, ResolveError> {
        let base = self.spec_root(&addr.authority)?;
        if !portable::checked_file(&base, path)? {
            return Err(ResolveError::SourceUnstable {
                path: path.display().to_string(),
                reason: "selected source disappeared before reading".to_owned(),
            });
        }
        read_stable_with(path, read)
    }
}

pub(crate) fn project_source(file: &Path, bytes: &[u8]) -> Result<String, String> {
    let raw = std::str::from_utf8(bytes).map_err(|error| {
        format!(
            "overlay spec source `{}` is not UTF-8: {error}",
            file.display()
        )
    })?;
    vibe_specdoc::project_spec_text(file, raw)
        .map(|(src, _kind)| src)
        .map_err(|error| error.to_string())
}

fn read_stable_with(
    path: &Path,
    mut read: impl FnMut(&Path) -> io::Result<Vec<u8>>,
) -> Result<Vec<u8>, ResolveError> {
    let first = read(path).map_err(|error| read_error(path, error))?;
    let second = read(path).map_err(|error| read_error(path, error))?;
    if first != second {
        return Err(ResolveError::SourceUnstable {
            path: path.display().to_string(),
            reason: "bytes differed between two reads".to_owned(),
        });
    }
    Ok(first)
}

fn read_error(path: &Path, error: io::Error) -> ResolveError {
    match error.kind() {
        io::ErrorKind::NotFound => ResolveError::SourceUnstable {
            path: path.display().to_string(),
            reason: "source disappeared after lookup".to_owned(),
        },
        _ => ResolveError::SourceUnreadable {
            path: path.display().to_string(),
            reason: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn changing_bytes_between_reads_is_unstable_not_absent() {
        let root = tempfile::tempdir().unwrap();
        let specs = root.path().join("spec");
        fs::create_dir(&specs).unwrap();
        let path = specs.join("source.md");
        fs::write(&path, "first").unwrap();
        let resolver = FileResolver::new(
            root.path(),
            crate::SelfCoordinate::new(Some("org.demo".into()), "self".into()),
        );
        let address = SpecAddress::parse("spec://org.demo/self/source").unwrap();
        let mut calls = 0;
        let error = resolver
            .read_preflighted_with(&address, &path, |path| {
                calls += 1;
                let bytes = fs::read(path)?;
                if calls == 1 {
                    fs::write(path, "second")?;
                }
                Ok(bytes)
            })
            .unwrap_err();
        assert!(matches!(error, ResolveError::SourceUnstable { .. }));
    }

    #[test]
    fn a_non_file_source_is_unreadable_not_absent() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("spec/dir");
        fs::create_dir_all(&path).unwrap();
        let resolver = FileResolver::new(
            root.path(),
            crate::SelfCoordinate::new(Some("org.demo".into()), "self".into()),
        );
        let address = SpecAddress::parse("spec://org.demo/self/dir").unwrap();
        assert!(matches!(
            resolver.read_preflighted_source(&address, &path),
            Err(ResolveError::SourceUnreadable { .. })
        ));
    }
}
