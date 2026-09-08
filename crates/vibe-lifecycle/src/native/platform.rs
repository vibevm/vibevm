//! Closed native platform keys and library suffixes.

use specmark::spec;
use vibe_core::manifest::MechanismKey;
use vibe_extension_registry::resolve_mechanism;

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
    /// Recover the exact platform selected and stored by the install epoch.
    pub fn from_key(key: &str) -> Result<Self, NativeArtifactError> {
        match key {
            "windows-x86_64" => Ok(Self::WindowsX86_64),
            "linux-x86_64" => Ok(Self::LinuxX86_64),
            "macos-aarch64" => Ok(Self::MacosAarch64),
            _ => Err(NativeArtifactError::UnsupportedPlatform {
                os: bounded(key),
                arch: bounded("stored-key"),
            }),
        }
    }

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

    /// Build the replay adapter for this already-selected platform.
    pub fn replay_factory(
        self,
    ) -> impl vibe_workspace::extension_world::CompilerNativeReplayFactory {
        super::compiler::ArtifactCompilerNativeReplayFactory::new(self)
    }

    /// Resolve the exact build provider pin without admitting or executing it.
    pub fn resolved_build_provider_pin(
        self,
        execution: &super::NativeBuildExecution<'_>,
    ) -> Result<String, NativeArtifactError> {
        debug_assert_eq!(self, execution.platform);
        let key = "build:cargo".parse::<MechanismKey>().map_err(|error| {
            NativeArtifactError::MechanismSelection {
                reason: format!("engine-owned key is invalid: {error}"),
            }
        })?;
        resolve_mechanism(execution.registry, &key, None, execution.routes)
            .map(|selected| selected.row().pin().to_string())
            .map_err(|error| NativeArtifactError::MechanismSelection {
                reason: error.to_string(),
            })
    }

    /// Admit that resolved provider without starting Cargo.
    pub fn admit_build_provider(
        self,
        execution: &super::NativeBuildExecution<'_>,
    ) -> Result<(), NativeArtifactError> {
        debug_assert_eq!(self, execution.platform);
        super::select_build_provider(execution).map(|_| ())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_keys_round_trip_without_observing_the_current_host() {
        for platform in [
            NativePlatform::WindowsX86_64,
            NativePlatform::LinuxX86_64,
            NativePlatform::MacosAarch64,
        ] {
            assert_eq!(NativePlatform::from_key(platform.key()).unwrap(), platform);
        }
        assert!(NativePlatform::from_key("other-x86_64").is_err());
    }
}
