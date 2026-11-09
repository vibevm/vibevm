//! The body of `vibe dl`: into a temporary file beside the destination,
//! verified and renamed, or byte-exact to stdout.

use std::io::{IsTerminal, Read, Write};
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};
use vibe_core::progress::transfer;

use super::DlError;

pub(super) fn io_error(context: &str, error: std::io::Error) -> anyhow::Error {
    if error
        .get_ref()
        .is_some_and(|inner| inner.is::<vibe_core::cancellation::Cancelled>())
    {
        return anyhow::Error::new(vibe_core::cancellation::Cancelled);
    }
    DlError::local(
        "dl_local_io",
        18,
        format!("{context}: {error}"),
        "check the destination's free space and permissions",
    )
    .into()
}

pub(super) fn check_digest(expected: Option<&str>, actual: &str) -> Result<()> {
    if let Some(expected) = expected
        && expected != actual
    {
        return Err(DlError::local(
            "dl_sha256_mismatch",
            16,
            format!("the body hashes to {actual}, not {expected}"),
            "check the digest you were given; nothing was kept",
        )
        .into());
    }
    Ok(())
}

/// Stream into a temporary file beside `path`, verify, then rename. A
/// failure or a stop removes the partial file.
pub(super) fn write_file<R: Read>(
    mut reader: transfer::TransferReader<R>,
    path: &Path,
    force: bool,
    expected: Option<&str>,
) -> Result<(u64, String)> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::Builder::new()
        .prefix(".vibe-dl-")
        .suffix(".part")
        .tempfile_in(parent)
        .map_err(|error| io_error("creating the temporary file", error))?;
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) => {
                reader.fail(error.to_string());
                return Err(io_error("reading the response", error));
            }
        };
        hasher.update(&buffer[..read]);
        bytes += read as u64;
        if let Err(error) = temp.write_all(&buffer[..read]) {
            reader.fail(error.to_string());
            return Err(io_error("writing the temporary file", error));
        }
    }
    reader.finish();
    let digest = hex(&hasher.finalize());
    check_digest(expected, &digest)?;
    temp.as_file()
        .sync_all()
        .map_err(|error| io_error("syncing the temporary file", error))?;
    let persisted = if force {
        temp.persist(path)
    } else {
        temp.persist_noclobber(path)
    };
    persisted.map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            DlError::local(
                "dl_destination_exists",
                17,
                format!("`{}` appeared while downloading", path.display()),
                "pass `--force` to replace it, or choose another destination",
            )
            .into()
        } else {
            io_error("renaming into place", error.error)
        }
    })?;
    Ok((bytes, digest))
}

/// Whether `chunk` reads as text: valid UTF-8 (a cut final character
/// allowed) with no NUL byte.
pub(super) fn is_text(chunk: &[u8]) -> bool {
    if chunk.contains(&0) {
        return false;
    }
    match std::str::from_utf8(chunk) {
        Ok(_) => true,
        Err(error) => error.error_len().is_none(),
    }
}

/// `--show`: the body, byte-exact, on stdout. A binary body is refused on
/// a terminal unless `--force`.
pub(super) fn write_stdout<R: Read>(
    mut reader: transfer::TransferReader<R>,
    force: bool,
    expected: Option<&str>,
) -> Result<(u64, String)> {
    let terminal = std::io::stdout().is_terminal();
    let mut out = std::io::stdout().lock();
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = vec![0u8; 64 * 1024];
    let mut first = true;
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) => {
                reader.fail(error.to_string());
                return Err(io_error("reading the response", error));
            }
        };
        if first && terminal && !force && !is_text(&buffer[..read]) {
            reader.fail("binary body refused on a terminal");
            return Err(DlError::local(
                "dl_binary_terminal",
                19,
                "the body is binary and stdout is a terminal",
                "redirect stdout to a file (`> file`), or pass `--force` to print it anyway",
            )
            .into());
        }
        first = false;
        hasher.update(&buffer[..read]);
        bytes += read as u64;
        if let Err(error) = out.write_all(&buffer[..read]) {
            reader.fail(error.to_string());
            return Err(io_error("writing stdout", error));
        }
    }
    let _ = out.flush();
    reader.finish();
    let digest = hex(&hasher.finalize());
    check_digest(expected, &digest)?;
    Ok((bytes, digest))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
