//! macOS: the parent image comes from libproc's `proc_pidpath`, which the
//! workspace `libc` dependency already declares.

use std::ffi::OsString;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

/// `PROC_PIDPATHINFO_MAXSIZE` from `<libproc.h>`.
const PATH_CAPACITY: usize = 4096;

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: proc_pidpath has no standard-library wrapper; it writes at most the owned buffer's advertised capacity and retains no pointer, and the returned length is checked before use (spec://org.vibevm.core/vibevm/common/PROP-059#shell-command)."
)]
#[allow(unsafe_code)]
pub(super) fn parent_image() -> Option<PathBuf> {
    let pid = i32::try_from(std::os::unix::process::parent_id()).ok()?;
    let mut bytes = vec![0_u8; PATH_CAPACITY];
    // SAFETY: `bytes` is writable for its full length, which is passed as the
    // capacity; the synchronous call keeps no reference to the buffer.
    let used = unsafe { libc::proc_pidpath(pid, bytes.as_mut_ptr().cast(), PATH_CAPACITY as u32) };
    let used = usize::try_from(used)
        .ok()
        .filter(|used| *used > 0 && *used < PATH_CAPACITY)?;
    bytes.truncate(used);
    Some(PathBuf::from(OsString::from_vec(bytes)))
}

pub(super) fn fallback() -> io::Result<PathBuf> {
    Ok(PathBuf::from("/bin/zsh"))
}
