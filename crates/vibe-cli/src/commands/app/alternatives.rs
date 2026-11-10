//! `vibe app use SLOT KEY` and `vibe app alternatives SLOT`: thin rendering
//! around the `vibe-app` alternatives seam.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#alternatives");

use anyhow::Result;
use vibe_app::SettingsRoot;
use vibe_wire::generated::application_alternatives::{ApplicationAlternatives, Status};

use super::global;
use crate::output;

/// What `vibe app use` was asked to do: name a provider, switch the slot
/// off, or switch it back on at its remembered default (PROP-059
/// `#slot-off`).
pub(crate) enum UseRequest {
    Provider(String),
    Off,
    On,
}

impl UseRequest {
    /// Read the three spellings; naming a key and a switch together, or
    /// neither, is a usage error named here rather than guessed at.
    pub(crate) fn of(key: Option<String>, none: bool, default: bool) -> Result<Self> {
        match (key, none, default) {
            (Some(key), false, false) => Ok(Self::Provider(key)),
            (None, true, false) => Ok(Self::Off),
            (None, false, true) => Ok(Self::On),
            (None, true, true) | (Some(_), _, _) => anyhow::bail!(
                "vibe app use takes an installation key, `--none` or `--default` — exactly one of \
                 the three"
            ),
            (None, false, false) => anyhow::bail!(
                "vibe app use needs an installation key, `--none` to switch the slot off, or \
                 `--default` to switch it back on"
            ),
        }
    }
}

pub(super) fn use_default(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    request: UseRequest,
) -> Result<()> {
    let proxy_image = global::proxy_image()?;
    let home = super::facts::user_home();
    let environment = super::environment::for_host(settings.path(), home.as_deref());
    let exposure = vibe_app::SlotExposure {
        proxy_image: &proxy_image,
        environment: environment.as_deref(),
    };
    let change = match request {
        UseRequest::Provider(key) => vibe_app::set_default(settings, slot, &key, exposure)?,
        UseRequest::Off => vibe_app::set_slot_off(settings, slot, true, exposure)?,
        UseRequest::On => vibe_app::set_slot_off(settings, slot, false, exposure)?,
    };
    if ctx.is_json() {
        // The machine answer is the slot's listing after the change.
        return ctx.emit_json(&vibe_app::alternatives_listing(settings, slot)?);
    }
    if change.off {
        ctx.summary(&format!(
            "vibe app use: slot `{}` is off; its commands fall through to the system program, and \
             `vibe app use {} --default` turns it back on at {}",
            change.slot,
            change.slot,
            if change.default.is_empty() {
                "its next default"
            } else {
                change.default.as_str()
            }
        ));
        return Ok(());
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
    // Moving the default is the third thing that may leave state behind in
    // another version; it offers and never migrates (PROP-075
    // `#MIGRATION-FAILURE`).
    super::migrate::offer(ctx, settings, &change.slot, &change.default);
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
        "slot `{}`: {} provider(s), default {}{}",
        listing.slot,
        listing.providers.len(),
        listing.default.as_deref().unwrap_or("(none)"),
        if listing.off {
            " [off: the system program answers; `vibe app use <slot> --default` turns it on]"
        } else {
            ""
        }
    );
    if quiet {
        return;
    }
    for provider in &listing.providers {
        let marker = match (provider.default, listing.off) {
            (true, false) => " [default]",
            (true, true) => " [default, off]",
            (false, _) => "",
        };
        let status = match (&provider.status, &provider.reason) {
            (Status::Ready, _) => "ready".to_owned(),
            (Status::Unavailable, Some(reason)) => format!("unavailable: {reason}"),
            (Status::Unavailable, None) => "unavailable".to_owned(),
        };
        println!("  {}{marker} ({}, {status})", provider.key, provider.method);
        if !provider.commands.is_empty() {
            println!("    commands: {}", provider.commands.join(", "));
        }
        if !provider.path_dirs.is_empty() {
            println!("    path_dirs: {}", provider.path_dirs.join(", "));
        }
    }
}
