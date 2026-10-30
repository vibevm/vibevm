//! `vibe app use SLOT KEY` and `vibe app alternatives SLOT`: thin rendering
//! around the `vibe-app` alternatives seam.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#alternatives");

use anyhow::Result;
use vibe_app::SettingsRoot;
use vibe_wire::generated::application_alternatives::{ApplicationAlternatives, Status};

use super::global;
use crate::output;

pub(super) fn use_default(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    key: &str,
) -> Result<()> {
    let proxy_image = global::proxy_image()?;
    let change = vibe_app::set_default(settings, slot, key, &proxy_image)?;
    if ctx.is_json() {
        // The machine answer is the slot's listing after the change.
        return ctx.emit_json(&vibe_app::alternatives_listing(settings, slot)?);
    }
    ctx.summary(&match &change.previous {
        Some(previous) if previous != &change.default => format!(
            "vibe app use: slot `{}` default is now {} (was {previous})",
            change.slot, change.default
        ),
        _ => format!(
            "vibe app use: slot `{}` default is now {}",
            change.slot, change.default
        ),
    });
    Ok(())
}

pub(super) fn alternatives(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
) -> Result<()> {
    let listing = vibe_app::alternatives_listing(settings, slot)?;
    if ctx.is_json() {
        return ctx.emit_json(&listing);
    }
    print(&listing, ctx.is_quiet());
    Ok(())
}

fn print(listing: &ApplicationAlternatives, quiet: bool) {
    println!(
        "slot `{}`: {} provider(s), default {}",
        listing.slot,
        listing.providers.len(),
        listing.default.as_deref().unwrap_or("(none)")
    );
    if quiet {
        return;
    }
    for provider in &listing.providers {
        let marker = if provider.default { " [default]" } else { "" };
        let status = match (&provider.status, &provider.reason) {
            (Status::Ready, _) => "ready".to_owned(),
            (Status::Unavailable, Some(reason)) => format!("unavailable: {reason}"),
            (Status::Unavailable, None) => "unavailable".to_owned(),
        };
        println!("  {}{marker} ({}, {status})", provider.key, provider.method);
        if !provider.commands.is_empty() {
            println!("    commands: {}", provider.commands.join(", "));
        }
    }
}
