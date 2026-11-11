//! The process's network route, installed once at the composition root
//! (`PROP-068#proxy`): the effective `[proxy]` table with its mode and PAC
//! switched at every layer — `VIBEVM_PROXY_MODE` / `VIBEVM_PROXY_PAC` /
//! `VIBEVM_PROXY_REVOCATION` over the user's file over the site profile —
//! so a user can turn a site's px off.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy");

use vibe_core::user_config::{
    ProxyConfig, ProxyMode, RevocationCheck, TlsConfig, UserConfig, effective_extra_ca,
};

/// Install the route `effective` (the merged `[proxy]` table) describes,
/// with the environment switches `VIBEVM_PROXY_MODE` / `VIBEVM_PROXY_PAC` /
/// `VIBEVM_PROXY_REVOCATION`
/// (read through the composition root's `env`) applied; return the
/// variables the table promotes into the environment. An unusable table is said once, with
/// its fix, and the run continues direct, as before `[proxy]` existed.
pub(crate) fn install_route(
    effective: &ProxyConfig,
    tls: &TlsConfig,
    env: fn(&str) -> Option<String>,
) -> Vec<(&'static str, String)> {
    use vibe_core::net::{MODE_ENV, PAC_ENV, REVOCATION_ENV};
    let (env_mode, env_pac) = (env(MODE_ENV), env(PAC_ENV));
    let env_revocation = env(REVOCATION_ENV);
    let warn = |text: &str| eprintln!("{}", crate::output::language::warning(text));
    let env_mode = env_mode.and_then(|raw| {
        let mode = ProxyMode::parse(&raw);
        if mode.is_none() {
            warn(&format!(
                "{MODE_ENV}=`{raw}` is not a proxy mode; ignored \
                 (fix: use `px`, `explicit` or `none`)"
            ));
        }
        mode
    });
    let env_revocation = env_revocation.and_then(|raw| {
        let choice = RevocationCheck::parse(&raw);
        if choice.is_none() {
            warn(&format!(
                "{REVOCATION_ENV}=`{raw}` is not a revocation setting; ignored \
                 (fix: use `check`, `best-effort` or `off`)"
            ));
        }
        choice
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
    let (revocation, revocation_origin) =
        vibe_core::net::effective_revocation(site.as_ref(), &own, env_revocation);
    table.revocation = Some(revocation);
    // PROP-068 `#proxy`: the trust set is additive across both spellings,
    // so the route carries the whole list rather than the one key.
    let extra_ca = effective_extra_ca(tls, &table);
    match vibe_core::net::ProxyRoute::from_config(&table) {
        Ok(route) => {
            vibe_core::net::install_route(
                route
                    .with_origin(origin)
                    .with_pac_origin(pac_origin)
                    .with_revocation(revocation, revocation_origin)
                    .with_extra_ca(extra_ca),
            );
        }
        Err(error) => warn(&error.to_string()),
    }
    register_px_store();
    table.child_env()
}

/// The route this invocation runs on: the installed one, with the fixed
/// loopback port `vibe tool px --port N` asked for (`PROP-068#proxy-tool`).
/// One session owns px for the whole invocation and is created before any
/// command runs, so the port belongs to the route it is created from.
pub(crate) fn route_for(command: &crate::cli::Command) -> vibe_core::net::ProxyRoute {
    let route = vibe_core::net::route().clone();
    match crate::commands::tool::px::requested_px_port(command) {
        Some(port) => route.with_px_port(port),
        None => route,
    }
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
