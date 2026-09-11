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
pub(super) fn split_block(text: &str) -> (String, Vec<String>, String) {
    if let (Some(begin), Some(end)) = (text.find(BLOCK_BEGIN), text.find(BLOCK_END))
        && begin < end
    {
        let pre = text[..begin].to_string();
        let inner = &text[begin + BLOCK_BEGIN.len()..end];
        let block = inner
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        let post = text[end + BLOCK_END.len()..].to_string();
        return (pre, block, post);
    }
    (text.to_string(), Vec::new(), String::new())
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

pub(super) fn rebuild(pre: &str, block: &[String], post: &str) -> String {
    if block.is_empty() {
        return format!("{pre}{post}");
    }
    let pre = pre.trim_end_matches('\n');
    let post = post.trim_start_matches('\n');
    let mut out = String::new();
    if !pre.is_empty() {
        out.push_str(pre);
        out.push('\n');
    }
    out.push_str(BLOCK_BEGIN);
    out.push('\n');
    out.push_str(&block.join("\n"));
    out.push('\n');
    out.push_str(BLOCK_END);
    out.push('\n');
    if !post.is_empty() {
        out.push_str(post);
        if !post.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}
