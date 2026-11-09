//! The process's network route, installed once at the composition root
//! (`PROP-068#proxy`): the effective `[proxy]` table with its mode and PAC
//! switched at every layer — `VIBEVM_PROXY_MODE` / `VIBEVM_PROXY_PAC` over
//! the user's file over the site profile — so a user can turn a site's px
//! off.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy");

use vibe_core::user_config::{ProxyConfig, ProxyMode, UserConfig};

/// Install the route `effective` (the merged `[proxy]` table) describes,
/// with the environment switches `VIBEVM_PROXY_MODE` / `VIBEVM_PROXY_PAC`
/// (read through the composition root's `env`) applied; return the
/// variables the table promotes into the environment. An unusable table is said once, with
/// its fix, and the run continues direct, as before `[proxy]` existed.
pub(crate) fn install_route(
    effective: &ProxyConfig,
    env: fn(&str) -> Option<String>,
) -> Vec<(&'static str, String)> {
    let (env_mode, env_pac) = (env("VIBEVM_PROXY_MODE"), env("VIBEVM_PROXY_PAC"));
    let warn = |text: &str| eprintln!("{}", crate::output::language::warning(text));
    let env_mode = env_mode.and_then(|raw| {
        let mode = ProxyMode::parse(&raw);
        if mode.is_none() {
            warn(&format!(
                "VIBEVM_PROXY_MODE=`{raw}` is not a proxy mode; ignored \
                 (fix: use `px`, `explicit` or `none`)"
            ));
        }
        mode
    });
    let own = UserConfig::load_own()
        .map(|own| own.proxy)
        .unwrap_or_default();
    let site = vibe_core::site_profile::installed()
        .ok()
        .flatten()
        .map(|site| site.profile.proxy);
    let mut table = effective.clone();
    let (mode, origin) = vibe_core::net::effective_mode(site.as_ref(), &own, env_mode);
    table.mode = mode;
    let (pac, pac_origin) = vibe_core::net::effective_pac(site.as_ref(), &own, env_pac.as_deref());
    table.pac = pac;
    match vibe_core::net::ProxyRoute::from_config(&table) {
        Ok(route) => {
            vibe_core::net::install_route(route.with_origin(origin).with_pac_origin(pac_origin));
        }
        Err(error) => warn(&error.to_string()),
    }
    register_px_store();
    table.child_env()
}

/// Name the px tool store every installation kind shares
/// (PROP-019 `#companion-tools`): `<install root>/opt/vibevm/tools/px`, the
/// root resolved as `vibe self` resolves it.
fn register_px_store() {
    let managed = crate::commands::vvm::derive_self(std::env::current_exe().ok().as_deref());
    if let Some(root) = crate::commands::vvm::resolve_root(
        managed.map(|location| location.root),
        crate::composition::install_base_override(),
        dirs::home_dir(),
    ) {
        vibe_net::px_store::register_store(root.join("vibevm").join("tools").join("px"));
    }
}

/// End the operation's network: px is killed and reaped now, and a px that
/// could not serve a failed operation is named once with its cause.
pub(crate) fn finish(ctx: &crate::output::Context, network: vibe_net::NetworkGuard, failed: bool) {
    if failed && let Some(failure) = network.session().px_failure() {
        ctx.diagnostic(&format!("vibe-managed px: {failure}"));
    }
    drop(network);
}
