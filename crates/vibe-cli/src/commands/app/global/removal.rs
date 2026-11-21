//! `vibe uninstall -g` for one user application: the short name is answered
//! by this home's own inventory, and the removal plan by the `vibe-app`
//! library. Split from `global.rs` for the file-length budget; `global`
//! re-exports both items, so their paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");

use std::path::Path;

use anyhow::{Result, bail};
use vibe_app::{OperationContext, RemoveRequest};

use super::{Observation, operation, proxy_image, settings_root};
use crate::cli::UninstallArgs;
use crate::commands::app::display::StepDisplay;
use crate::commands::app::report::render_removal;
use crate::commands::app::short_name;
use crate::output;

/// The tree-removal flags of `uninstall` (PROP-065 `#uninstall-tree`).
pub(crate) fn remove_request(args: &UninstallArgs) -> RemoveRequest {
    RemoveRequest {
        force: args.force,
        cascade: args.cascade,
        autoremove: args.autoremove,
        plan_only: args.plan_only,
    }
}

pub fn uninstall(ctx: &output::Context, args: UninstallArgs, root_offline: bool) -> Result<()> {
    if args.path != Path::new(".") {
        bail!("global uninstall received project-only --path");
    }
    let settings = settings_root()?;
    // An uninstall acts only on installed state, so the home's inventory is
    // the whole source of the short name and the answer never leaves this
    // machine (PROP-008 `#INSTALLED-STATE-RESOLVES-LOCALLY`).
    let package = short_name::installed(ctx, "vibe uninstall -g", &settings, &args.package)?;
    vibe_app::qualified_ref(&package)?;
    let proxy_image = proxy_image()?;
    let flat = StepDisplay::flat();
    let observe = Observation::new(ctx, true, &flat)?;
    let removal = flat.around(ctx, || {
        let operation = OperationContext {
            chooser: None,
            feature_chooser: None,
            ..operation(ctx, &settings, root_offline, &proxy_image, None, &observe)
        };
        vibe_app::uninstall(&operation, &package, &remove_request(&args))
    })?;
    render_removal(ctx, &removal)
}
