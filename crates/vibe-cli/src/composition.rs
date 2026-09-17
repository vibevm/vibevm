//! Small composition-root helpers that keep ambient input assembly out of dispatch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#vars");

use anyhow::Result;

use crate::cli::VarsArgs;
use crate::commands;

pub(crate) fn run_vars(
    args: VarsArgs,
    self_location: Option<&commands::vvm::SelfLocation>,
    unattended: bool,
    invoked_by: Option<&str>,
) -> Result<()> {
    let install_base = commands::vvm::resolve_root(
        self_location.map(|location| location.root.clone()),
        super::read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV).map(Into::into),
        dirs::home_dir(),
    )
    .and_then(|root| root.parent().map(|parent| parent.display().to_string()))
    .unwrap_or_default();
    let home = self_location
        .map(|location| location.home.display().to_string())
        .or_else(|| super::read_env_opt(commands::vvm::VIBEVM_HOME_ENV))
        .unwrap_or_else(|| "(none)".to_string());
    let (invoked_by, _) = crate::output::resolve_invoked_by(invoked_by);
    commands::vars::run(
        args,
        vec![
            commands::vars::VarRow::new(
                "VIBEVM_INSTALL_ROOT",
                install_base,
                super::read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV),
            ),
            commands::vars::VarRow::new(
                "VIBEVM_HOME",
                home,
                super::read_env_opt(commands::vvm::VIBEVM_HOME_ENV),
            ),
            commands::vars::VarRow::new(
                "VIBE_INVOKED_BY",
                invoked_by.unwrap_or_default(),
                super::read_env_opt("VIBE_INVOKED_BY"),
            ),
            commands::vars::VarRow::new(
                "VIBE_UNATTENDED",
                crate::output::resolve_unattended(unattended).to_string(),
                super::read_env_opt("VIBE_UNATTENDED"),
            ),
            commands::vars::VarRow::new(
                "VIBE_LOG",
                super::read_env_opt("VIBE_LOG").unwrap_or_else(|| "warn".to_string()),
                super::read_env_opt("VIBE_LOG"),
            ),
        ],
    )
}
