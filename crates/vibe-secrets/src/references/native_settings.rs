//! Concrete native sinks only; never exposes key bytes or accepts caller callbacks.
use super::*;
mod jsonc;
mod pi;
pub use pi::PiNativePreview;
mod qwen;
pub use qwen::QwenNativePreview;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Pi,
    Qwen,
}
impl Kind {
    fn id(self) -> &'static str {
        match self {
            Self::Pi => "pi",
            Self::Qwen => "qwen",
        }
    }
    fn file(self) -> &'static str {
        match self {
            Self::Pi => "models.json",
            Self::Qwen => "settings.json",
        }
    }
    fn domain(self) -> &'static [u8] {
        match self {
            Self::Pi => b"vibevm.pi-native-settings/v1\0",
            Self::Qwen => b"vibevm.qwen-native-settings/v1\0",
        }
    }
}
