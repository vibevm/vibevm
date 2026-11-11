//! How one `vibe app` command names the package it acts on, and the one
//! shape the state verbs share.
//!
//! PROP-008 §2.6 at the application input boundary: a bare
//! `<name>[@<version>]` is qualified **once**, here, before the spelling
//! reaches `vibe-app`, so everything below this line — the index record,
//! the version file, every receipt — holds `<group>/<name>`
//! (PROP-029 `#ADDR-SHORT-NAMES`).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

use anyhow::Result;
use vibe_app::ResolvedPackage;

use super::home::AppHome;
use super::{global, lifecycle, short_name};
use crate::output;

/// The registry side: a short name is resolved through the very chain the
/// command would have resolved the package with, preferring what this
/// home already installed.
pub(super) fn from_registry(
    ctx: &output::Context,
    command: &str,
    app_home: &AppHome,
    package: &str,
    inputs: short_name::Inputs<'_>,
) -> Result<String> {
    let settings = global::settings_root()?;
    short_name::from_registries(
        ctx,
        command,
        &short_name::Sources {
            home: app_home.settings(),
            settings: &settings,
            inputs,
        },
        package,
    )
}

/// The installed side: the home's own inventory answers, with no index
/// and no request (PROP-008 `#INSTALLED-STATE-RESOLVES-LOCALLY`).
pub(super) fn from_home(
    ctx: &output::Context,
    command: &str,
    app_home: &AppHome,
    package: &str,
) -> Result<String> {
    short_name::installed(ctx, command, app_home.settings(), package)
}

/// `installers` and `plan` both resolve one package out of a registry;
/// both qualify a bare name against that same chain first, and both need
/// the resolution afterwards.
pub(super) fn resolved(
    ctx: &output::Context,
    command: &str,
    app_home: &AppHome,
    package: &str,
    inputs: short_name::Inputs<'_>,
) -> Result<(String, ResolvedPackage)> {
    let package = from_registry(ctx, command, app_home, package, inputs)?;
    let resolved = global::resolve_package_with_operation(
        &global::settings_root()?,
        inputs.registry,
        inputs.embedded,
        &package,
        inputs.offline,
        inputs.operation,
    )?;
    Ok((package, resolved))
}

/// The six verbs that only change the recorded state of one installation —
/// `expose`, `hide`, `mark`, `hold`, `unhold`, `rollback`. One shape: name
/// the installation, change its row, answer nothing.
pub(super) fn state_verb(
    ctx: &output::Context,
    app_home: &AppHome,
    command: &str,
    package: &str,
    verb: lifecycle::StateVerb,
) -> Result<Option<i32>> {
    let package = from_home(ctx, command, app_home, package)?;
    lifecycle::change(ctx, app_home.settings(), &package, verb)?;
    Ok(None)
}
