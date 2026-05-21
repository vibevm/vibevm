//! Structured `vibe install` error types.
//!
//! Since PROP-009 (M1.18) the install / uninstall / update workflows live
//! in `vibe-cli` (the command layer) and `vibe-workspace` (the computed
//! loading model — materialisation, boot composition, artifact
//! generation). The legacy `[writes]` mirror-layout machinery — the
//! per-file plan / apply / register code that this crate used to hold —
//! is retired.
//!
//! What remains is the one structured error the CLI maps to a process
//! exit code: a user declining the plan at the confirmation prompt
//! (`VIBEVM-SPEC.md` §9.4, exit code 5).

#![forbid(unsafe_code)]

use thiserror::Error;

/// A structured `vibe install` / `vibe uninstall` failure that the CLI
/// maps to a specific process exit code.
#[derive(Debug, Error)]
pub enum InstallError {
    /// The user declined the plan at the interactive confirmation prompt.
    #[error("user declined the plan")]
    UserDeclined,
}

impl InstallError {
    /// The process exit code for this failure (`VIBEVM-SPEC.md` §9.4).
    pub fn exit_code(&self) -> u8 {
        match self {
            InstallError::UserDeclined => 5,
        }
    }
}
