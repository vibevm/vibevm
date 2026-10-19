//! Bounded copy and portable path checks shared by direct artifact formats.

use super::*;

pub(super) fn copy_hashed(
    input: &mut impl Read,
    output: &mut impl Write,
    hash: &mut Sha256,
    maximum: u64,
) -> Result<u64> {
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| anyhow::anyhow!("distribution size overflow"))?;
        if total > maximum {
            bail!("distribution file exceeds its declared size");
        }
        hash.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
    }
    Ok(total)
}

pub(super) fn portable_path(value: &str) -> Result<PathBuf> {
    if value.is_empty()
        || value.len() > DISTRIBUTION_SOURCE_MAX_PATH_BYTES
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains(':')
    {
        bail!("distribution path is not portable");
    }
    let path = PathBuf::from(value);
    let components: Vec<_> = path.components().collect();
    if components.len() > DISTRIBUTION_SOURCE_MAX_DEPTH
        || components.iter().any(|part| match part {
            Component::Normal(value) => value.to_str().is_none_or(is_windows_unsafe_component),
            _ => true,
        })
    {
        bail!("distribution path contains traversal or an unsafe component");
    }
    Ok(path)
}

pub(super) fn special_mode(mode: Option<u32>) -> bool {
    mode.is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
}
