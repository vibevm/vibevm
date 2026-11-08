specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) const BLOCK_BEGIN: &str = "# >>> vibevm (VVM) — managed, do not edit by hand >>>";
const BLOCK_END: &str = "# <<< vibevm (VVM) <<<";

pub(super) fn read_rc_file(path: &Path) -> Result<(String, Option<fs::Permissions>)> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((String::new(), None));
        }
        Err(error) => return Err(error).with_context(|| format!("statting `{}`", path.display())),
    };
    if !metadata.file_type().is_file() {
        bail!(
            "refusing non-regular or symlink shell rc `{}`",
            path.display()
        );
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Stable platform ABI values for O_NOFOLLOW on our supported Unix
        // targets (Linux and macOS), avoiding a new runtime dependency.
        #[cfg(target_os = "linux")]
        options.custom_flags(0x2_0000);
        #[cfg(target_os = "macos")]
        options.custom_flags(0x100);
    }
    let mut file = options.open(path).with_context(|| {
        format!(
            "opening shell rc `{}` without following links",
            path.display()
        )
    })?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .with_context(|| format!("reading shell rc `{}`", path.display()))?;
    Ok((text, Some(metadata.permissions())))
}

pub(super) fn write_rc_atomic(
    path: &Path,
    bytes: &[u8],
    permissions: Option<fs::Permissions>,
) -> Result<()> {
    let parent = path
        .parent()
        .context("shell rc path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("creating `{}`", parent.display()))?;
    let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("rc");
    let temporary = parent.join(format!(".{name}.vvm-{}-{nonce}.tmp", std::process::id()));
    write_rc_atomic_at(path, &temporary, bytes, permissions)
}

pub(super) fn write_rc_atomic_at(
    path: &Path,
    temporary: &Path,
    bytes: &[u8],
    permissions: Option<fs::Permissions>,
) -> Result<()> {
    let mut owned = false;
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temporary)
            .with_context(|| format!("creating `{}`", temporary.display()))?;
        owned = true;
        file.write_all(bytes)?;
        file.sync_all()?;
        if let Some(permissions) = permissions {
            fs::set_permissions(temporary, permissions)?;
        }
        if fs::symlink_metadata(path).is_ok_and(|metadata| !metadata.file_type().is_file()) {
            bail!(
                "refusing replaced non-regular or symlink shell rc `{}`",
                path.display()
            );
        }
        fs::rename(temporary, path)
            .with_context(|| format!("atomically replacing shell rc `{}`", path.display()))?;
        Ok(())
    })();
    if result.is_err() && owned {
        let _ = fs::remove_file(temporary);
    }
    result
}

/// Split a file into (text before the managed block, the block's inner
/// lines, text after the block). No block yields an empty managed section.
pub(super) fn split_block(text: &str) -> Result<(String, Vec<String>, String)> {
    if let Some(span) = block_span(text)? {
        let pre = text[..span.begin].to_string();
        let inner = &text[span.inner_begin..span.inner_end];
        let block = inner
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        let post = text[span.end..].to_string();
        return Ok((pre, block, post));
    }
    Ok((text.to_string(), Vec::new(), String::new()))
}

struct BlockSpan {
    begin: usize,
    inner_begin: usize,
    inner_end: usize,
    end: usize,
}

/// One pair of complete marker lines is the only managed-block envelope.
/// Broken, embedded or duplicate markers are never repaired by guessing ownership.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
fn block_span(text: &str) -> Result<Option<BlockSpan>> {
    let mut opening = None;
    let mut result = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        let content = if offset == 0 {
            content.strip_prefix('\u{feff}').unwrap_or(content)
        } else {
            content
        };
        let bom = if offset == 0 && line.starts_with('\u{feff}') {
            '\u{feff}'.len_utf8()
        } else {
            0
        };
        if content == BLOCK_BEGIN && opening.is_none() && result.is_none() {
            opening = Some((offset + bom, offset + line.len()));
        } else if content == BLOCK_END && result.is_none() {
            let Some((begin, inner_begin)) = opening.take() else {
                return malformed_block();
            };
            result = Some(BlockSpan {
                begin,
                inner_begin,
                inner_end: offset,
                end: offset + line.len(),
            });
        } else if content.contains(BLOCK_BEGIN) || content.contains(BLOCK_END) {
            return malformed_block();
        }
        offset += line.len();
    }
    if opening.is_some() {
        return malformed_block();
    }
    Ok(result)
}

fn malformed_block<T>() -> Result<T> {
    bail!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-019#path: malformed or duplicate managed profile markers; fix surface: inspect the existing profile block before requesting a repair"
    )
}

pub(super) fn managed_block_present(text: &str) -> Result<bool> {
    Ok(block_span(text)?.is_some())
}

/// Replace the block line beginning with `prefix`, or append `line`.
pub(super) fn set_or_add(block: &mut Vec<String>, prefix: &str, line: &str) -> bool {
    if let Some(slot) = block.iter_mut().find(|line| line.starts_with(prefix)) {
        if slot == line {
            return false;
        }
        *slot = line.to_string();
        true
    } else {
        block.push(line.to_string());
        true
    }
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(super) fn rebuild(pre: &str, block: &[String], post: &str, newline: &str) -> String {
    if block.is_empty() {
        return format!("{pre}{post}");
    }
    let mut out = pre.to_string();
    if !pre.is_empty() && pre != "\u{feff}" && !pre.ends_with('\n') {
        out.push_str(newline);
    }
    out.push_str(BLOCK_BEGIN);
    out.push_str(newline);
    out.push_str(&block.join(newline));
    out.push_str(newline);
    out.push_str(BLOCK_END);
    out.push_str(newline);
    out.push_str(post);
    out
}
