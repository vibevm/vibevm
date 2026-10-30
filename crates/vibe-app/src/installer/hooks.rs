//! `pre_install` / `post_install` hooks: authored commands run before the
//! installer and after it succeeds, both before the index is written.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#force-and-hooks");

use std::path::Path;

use vibe_core::manifest::ShellInterpreter;

use specmark::spec;

use crate::error::InstallerError;

/// The two hook positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#force-and-hooks")]
pub enum Hook {
    PreInstall,
    PostInstall,
}

impl Hook {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreInstall => "pre_install",
            Self::PostInstall => "post_install",
        }
    }
}

/// Run one hook, when authored, in the package tree with `VIBE_APP_ROOT`.
pub fn run(
    hook: Hook,
    script: Option<&str>,
    interpreter: Option<ShellInterpreter>,
    package_root: &Path,
    app_root: &Path,
    json: bool,
) -> Result<(), InstallerError> {
    match script {
        Some(script) => super::shell::run(
            interpreter,
            script,
            package_root,
            app_root,
            hook.as_str(),
            json,
        ),
        None => Ok(()),
    }
}
