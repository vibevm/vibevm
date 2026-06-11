//! Exit codes.
//!
//! Spec: `VIBEVM-SPEC.md` §9.4.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#exit-codes");

use std::process::ExitCode;

use thiserror::Error;

// Spec: VIBEVM-SPEC.md §9.4. These are the catalogue of CLI exit codes.
// The runtime path exits via `InstallError::exit_code()` (which maps specific
// failures onto per-category codes). The constants here are referenced by
// tests and make the catalogue literal in one place; they're otherwise
// allowed to be dead code in the binary itself.

#[allow(dead_code)]
pub const OK: u8 = 0;
pub const GENERAL: u8 = 1;
#[allow(dead_code)]
pub const USAGE: u8 = 2;
pub const PACKAGE_CONFLICT: u8 = 3;
#[allow(dead_code)]
pub const TYPE_MISMATCH: u8 = 4;
#[allow(dead_code)]
pub const USER_DECLINED: u8 = 5;
#[allow(dead_code)]
pub const LLM_PROVIDER: u8 = 6;
/// PROP-008 §2.7 — a short name (`wal`) matched two or more packages
/// across different groups. Distinct from `3` (a real package
/// conflict): a collision is a naming ambiguity the operator clears by
/// qualifying the pkgref, not a dependency-graph failure.
pub const AMBIGUOUS_PACKAGE: u8 = 7;

/// A structured `vibe install` / `vibe uninstall` failure the CLI maps to
/// a specific process exit code (`VIBEVM-SPEC.md` §9.4).
#[derive(Debug, Error)]
pub enum InstallError {
    /// The user declined the plan at the interactive confirmation prompt.
    #[error("user declined the plan")]
    UserDeclined,
    /// A bare short name resolved to two or more packages in different
    /// groups (PROP-008 §2.7) — the resolver never guesses past a
    /// collision. The payload is the fully-rendered, multi-line
    /// operator message (the numbered candidate list and the re-run
    /// hint), built at the resolution site where the candidate groups
    /// are in hand.
    #[error("{0}")]
    AmbiguousPackage(String),
}

impl InstallError {
    /// The process exit code for this failure.
    pub fn exit_code(&self) -> u8 {
        match self {
            InstallError::UserDeclined => USER_DECLINED,
            InstallError::AmbiguousPackage(_) => AMBIGUOUS_PACKAGE,
        }
    }
}

pub fn as_exit_code(err: &anyhow::Error) -> ExitCode {
    if let Some(install_err) = err.downcast_ref::<InstallError>() {
        return ExitCode::from(install_err.exit_code());
    }
    // A malformed `<vibevm>` block (PROP-012 §2.3) is conflict-shaped —
    // vibevm and the instruction file disagree on the managed region.
    // Walk the chain so a `.context()` wrapper cannot hide it.
    for cause in err.chain() {
        if let Some(vibe_workspace::WorkspaceError::MalformedRedirectBlock { .. }) =
            cause.downcast_ref::<vibe_workspace::WorkspaceError>()
        {
            return ExitCode::from(PACKAGE_CONFLICT);
        }
    }
    if err.downcast_ref::<vibe_core::Error>().is_some() {
        return ExitCode::from(GENERAL);
    }
    ExitCode::from(GENERAL)
}

#[cfg(test)]
fn code_of_install(err: &InstallError) -> u8 {
    err.exit_code()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_error_maps_to_one() {
        let err = anyhow::anyhow!("whoops");
        let code = as_exit_code(&err);
        assert_eq!(code, ExitCode::from(GENERAL));
    }

    #[test]
    fn install_declined_maps_to_five() {
        let err = anyhow::Error::from(InstallError::UserDeclined);
        assert_eq!(code_of_install(err.downcast_ref().unwrap()), USER_DECLINED);
    }

    #[test]
    fn ambiguous_package_maps_to_seven() {
        let err = anyhow::Error::from(InstallError::AmbiguousPackage(
            "the short name `wal` is ambiguous".to_string(),
        ));
        assert_eq!(as_exit_code(&err), ExitCode::from(AMBIGUOUS_PACKAGE));
    }

    #[test]
    fn malformed_redirect_block_maps_to_three() {
        let err = anyhow::Error::from(vibe_workspace::WorkspaceError::MalformedRedirectBlock {
            path: std::path::PathBuf::from("CLAUDE.md"),
            reason: "two `<vibevm>` markers".to_string(),
        });
        assert_eq!(as_exit_code(&err), ExitCode::from(PACKAGE_CONFLICT));
    }
}
