use std::io::{self, Read, Seek, Write};
use std::path::Path;

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};

use super::super::super::store::open_regular_no_follow;

pub(super) fn read_regular_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let (mut file, metadata) = open_regular_no_follow(path)?;
    if metadata.len() > maximum {
        bail!("`{}` is not a bounded regular file", path.display());
    }
    let mut bytes = Vec::with_capacity(metadata.len().min(64 * 1024) as usize);
    let copied = copy_bounded(&mut file, &mut bytes, maximum)?;
    if copied != metadata.len() {
        bail!("`{}` changed size while reading", path.display());
    }
    Ok(bytes)
}

pub(super) fn hash_regular_exact(
    path: &Path,
    expected_size: u64,
    maximum: u64,
    executable: bool,
) -> Result<String> {
    open_hashed_regular_exact(path, expected_size, maximum, executable).map(|(_, digest)| digest)
}

pub(super) fn open_hashed_regular_exact(
    path: &Path,
    expected_size: u64,
    maximum: u64,
    executable: bool,
) -> Result<(std::fs::File, String)> {
    let (mut file, metadata) = open_regular_no_follow(path)?;
    if metadata.len() != expected_size || expected_size > maximum {
        bail!(
            "`{}` is not the expected bounded regular file",
            path.display()
        );
    }
    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            bail!("`{}` is not executable", path.display());
        }
    }
    #[cfg(not(unix))]
    let _ = executable;
    let (size, digest) = copy_and_hash_bounded(&mut file, &mut io::sink(), expected_size)?;
    if size != expected_size {
        bail!("`{}` changed size while hashing", path.display());
    }
    file.rewind()?;
    Ok((file, digest))
}

fn copy_and_hash(reader: &mut impl Read, writer: &mut impl Write) -> Result<(u64, String)> {
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        writer.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        total += read as u64;
    }
    Ok((total, format!("sha256:{:x}", hasher.finalize())))
}

pub(super) fn copy_and_hash_bounded(
    reader: &mut impl Read,
    writer: &mut impl Write,
    maximum: u64,
) -> Result<(u64, String)> {
    let mut limited = reader.take(maximum.saturating_add(1));
    let result = copy_and_hash(&mut limited, writer)?;
    if result.0 > maximum {
        bail!("archive entry exceeds its {maximum}-byte verified bound");
    }
    Ok(result)
}

pub(super) fn copy_bounded(
    reader: &mut impl Read,
    writer: &mut impl Write,
    maximum: u64,
) -> Result<u64> {
    let copied = io::copy(&mut reader.take(maximum.saturating_add(1)), writer)?;
    if copied > maximum {
        bail!("archive entry exceeds its {maximum}-byte bound");
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::fs;
    use std::io::Cursor;

    #[test]
    fn bounded_copy_stops_after_one_byte_beyond_the_limit() {
        let mut source = Cursor::new(vec![7_u8; 32]);
        let mut destination = Vec::new();
        let error = copy_bounded(&mut source, &mut destination, 8)
            .unwrap_err()
            .to_string();
        assert!(error.contains("8-byte bound"));
        assert_eq!(destination.len(), 9);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_read_and_hash_never_follow_a_symlink() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::write(&target, b"owner bytes").unwrap();
        symlink(&target, &link).unwrap();

        assert!(read_regular_bounded(&link, 1024).is_err());
        assert!(hash_regular_exact(&link, 11, 1024, false).is_err());
    }
}
