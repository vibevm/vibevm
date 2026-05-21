//! Exit codes.
//!
//! Spec: `VIBEVM-SPEC.md` §9.4.

use std::process::ExitCode;

use vibe_install::InstallError;

// Spec: VIBEVM-SPEC.md §9.4. These are the catalogue of CLI exit codes.
// The runtime path exits via `InstallError::exit_code()` (which maps specific
// failures onto per-category codes). The constants here are referenced by
// tests and make the catalogue literal in one place; they're otherwise
// allowed to be dead code in the binary itself.

#[allow(dead_code)] pub const OK: u8 = 0;
pub const GENERAL: u8 = 1;
#[allow(dead_code)] pub const USAGE: u8 = 2;
#[allow(dead_code)] pub const PACKAGE_CONFLICT: u8 = 3;
#[allow(dead_code)] pub const TYPE_MISMATCH: u8 = 4;
#[allow(dead_code)] pub const USER_DECLINED: u8 = 5;
#[allow(dead_code)] pub const LLM_PROVIDER: u8 = 6;

pub fn as_exit_code(err: &anyhow::Error) -> ExitCode {
    if let Some(install_err) = err.downcast_ref::<InstallError>() {
        return ExitCode::from(install_err.exit_code());
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
}
