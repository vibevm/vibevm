//! Concrete native sinks only; never exposes key bytes or accepts caller callbacks.
use super::*;
mod jsonc;
mod pi;
mod yaml_mapping;
pub use pi::PiNativePreview;
mod qwen;
pub use qwen::QwenNativePreview;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Pi,
    Qwen,
    Codex,
    OpenCode,
    Hermes,
}
impl Kind {
    fn id(self) -> &'static str {
        match self {
            Self::Pi => "pi",
            Self::Qwen => "qwen",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::Hermes => "hermes",
        }
    }
    fn file(self) -> &'static str {
        match self {
            Self::Pi => "models.json",
            Self::Qwen => "settings.json",
            Self::Codex => "config.toml",
            Self::OpenCode => "opencode.json",
            Self::Hermes => "config.yaml",
        }
    }
    fn domain(self) -> &'static [u8] {
        match self {
            Self::Pi => b"vibevm.pi-native-settings/v1\0",
            Self::Qwen => b"vibevm.qwen-native-settings/v1\0",
            Self::Codex => b"vibevm.codex-native-settings/v1\0",
            Self::OpenCode => b"vibevm.opencode-native-settings/v1\0",
            Self::Hermes => b"vibevm.hermes-native-settings/v1\0",
        }
    }
}

mod codex;
pub use codex::CodexNativePreview;

mod opencode;
pub use opencode::OpenCodeNativePreview;

mod hermes;
pub use hermes::HermesNativePreview;
