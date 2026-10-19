//! Preserve a native application's termination status across the CLI boundary.

use std::process::ExitStatus;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

pub(super) fn code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return 128 + signal;
    }
    1
}
