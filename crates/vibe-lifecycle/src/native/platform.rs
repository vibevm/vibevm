//! Closed native platform keys and library suffixes.

use specmark::spec;

use super::NativeArtifactError;

/// The only native binary platforms VibeVM currently admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PREBUILT-CLOSED")]
pub enum NativePlatform {
    WindowsX86_64,
    LinuxX86_64,
    MacosAarch64,
}

impl NativePlatform {
    /// Map one exact Rust OS/architecture pair into the closed platform set.
    pub fn from_pair(os: &str, arch: &str) -> Result<Self, NativeArtifactError> {
        match (os, arch) {
            ("windows", "x86_64") => Ok(Self::WindowsX86_64),
            ("linux", "x86_64") => Ok(Self::LinuxX86_64),
            ("macos", "aarch64") => Ok(Self::MacosAarch64),
            _ => Err(NativeArtifactError::UnsupportedPlatform {
                os: bounded(os),
                arch: bounded(arch),
            }),
        }
    }

    /// Select the current process platform once from Rust's exact constants.
    pub fn current() -> Result<Self, NativeArtifactError> {
        Self::from_pair(std::env::consts::OS, std::env::consts::ARCH)
    }

    /// Exact manifest map key.
    pub const fn key(self) -> &'static str {
        match self {
            Self::WindowsX86_64 => "windows-x86_64",
            Self::LinuxX86_64 => "linux-x86_64",
            Self::MacosAarch64 => "macos-aarch64",
        }
    }

    /// Exact current-platform dynamic-library suffix.
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::WindowsX86_64 => ".dll",
            Self::LinuxX86_64 => ".so",
            Self::MacosAarch64 => ".dylib",
        }
    }
}

fn bounded(value: &str) -> String {
    value.chars().take(80).collect()
}
