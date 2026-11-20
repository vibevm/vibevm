//! Concrete native sinks only; never exposes key bytes or accepts caller callbacks.
use super::*;
mod pi;
pub use pi::PiNativePreview;
