//! Explicit platform selection. These constructors stay fail-closed until an
//! adversarially proven backend replaces the corresponding stub.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

use super::UnsupportedBackend;

#[must_use]
pub fn native_backend() -> UnsupportedBackend {
    #[cfg(target_os = "windows")]
    return windows::backend();
    #[cfg(target_os = "linux")]
    return linux::backend();
    #[cfg(target_os = "macos")]
    return macos::backend();
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return other::backend();
}
