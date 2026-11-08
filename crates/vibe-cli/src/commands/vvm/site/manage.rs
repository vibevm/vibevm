//! `vibe self apply --show` and `--remove` (`PROP-019#site-profile`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::path::PathBuf;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use vibe_core::site_profile::{SITE_PROFILE_FILE, SITE_RECEIPT_FILE, SiteReceipt};

use super::super::VvmEnv;
use super::super::bundle::release_source;
use super::ConfigLayers;
use super::apply::{Files, hex, read_optional};
use super::diff::line_diff;
use super::report::{ApplyReport, FileChange, PROTOCOL, ProfileInfo};
use super::steps::path_step;
use super::view::{effective, where_to_put};
use crate::output;

/// `vibe self apply --show`: the installation as every command reads it.
pub(super) fn show(ctx: &output::Context, env: &VvmEnv) -> Result<()> {
    let files = Files::resolve()?;
    let mut warnings = Vec::new();
    let site = vibe_core::site_profile::installed_at(&files.profile)?;
    let receipt = SiteReceipt::load_from(&files.receipt)?;
    let profile = site.as_ref().map(|site| {
        let sha256 = read_optional(&site.path)
            .ok()
            .flatten()
            .map(|text| hex(&Sha256::digest(text.as_bytes())))
            .unwrap_or_default();
        if let Some(receipt) = &receipt
            && receipt.sha256 != sha256
        {
            warnings.push(format!(
                "`{}` was edited after `vibe self apply`; it is read as it is now",
                site.path.display()
            ));
        }
        ProfileInfo {
            name: site.profile.name.clone(),
            source: receipt
                .as_ref()
                .map_or_else(|| "(no receipt)".to_owned(), |r| r.source.clone()),
            sha256,
            stored: site.path.display().to_string(),
            applied_at: receipt.as_ref().map(|r| r.applied_at.clone()),
        }
    });
    let policy = site
        .as_ref()
        .map(|site| site.profile.path_fix())
        .unwrap_or_default();
    let mut own_warnings = Vec::new();
    let layers = ConfigLayers::load(Some(&files.profile), &mut own_warnings, &mut warnings);
    warnings.extend(own_warnings);
    let active = release_source::active();
    let effective = effective(&layers, Some(active), &mut warnings)?;
    let where_to_put = where_to_put(&layers, &mut warnings);
    let store = env.store()?;
    let path = path_step(ctx, env, &store, None, policy, true)?;
    ApplyReport {
        protocol: PROTOCOL,
        ok: true,
        command: "self:apply",
        action: "show",
        changed: false,
        profile,
        files: Vec::new(),
        effective: Some(effective),
        where_to_put: Some(where_to_put),
        path: Some(path),
        imported: None,
        next_steps: Vec::new(),
        warnings,
    }
    .emit(ctx)
}

/// `vibe self apply --remove`: delete exactly the files the receipt says
/// apply owns — never a file a user wrote. The durable PATH is left as it
/// is: the shim directory on it is vibe's own and still needed.
pub(super) fn remove(ctx: &output::Context, dry_run: bool, with_diff: bool) -> Result<()> {
    let files = Files::resolve()?;
    let mut warnings = Vec::new();
    let receipt = match SiteReceipt::load_from(&files.receipt) {
        Ok(receipt) => receipt,
        Err(error) => {
            warnings.push(error.to_string());
            None
        }
    };
    let known = [
        (SITE_PROFILE_FILE, &files.profile),
        (SITE_RECEIPT_FILE, &files.receipt),
    ];
    let owned: Vec<&PathBuf> = match &receipt {
        Some(receipt) => {
            for name in &receipt.owns {
                if !known.iter().any(|(known, _)| known == name) {
                    warnings.push(format!(
                        "the receipt names `{name}`, which `vibe self apply` never writes; it is left alone"
                    ));
                }
            }
            known
                .iter()
                .filter(|(name, _)| receipt.owns.iter().any(|owned| owned == name))
                .map(|(_, path)| *path)
                .collect()
        }
        // Without a receipt, the two files apply writes are still its own.
        None => known.iter().map(|(_, path)| *path).collect(),
    };
    let profile = vibe_core::site_profile::installed_at(&files.profile)
        .ok()
        .flatten()
        .map(|site| ProfileInfo {
            name: site.profile.name,
            source: receipt
                .as_ref()
                .map_or_else(|| "(no receipt)".to_owned(), |r| r.source.clone()),
            sha256: receipt
                .as_ref()
                .map(|r| r.sha256.clone())
                .unwrap_or_default(),
            stored: files.profile.display().to_string(),
            applied_at: receipt.as_ref().map(|r| r.applied_at.clone()),
        });
    let mut changes = Vec::new();
    for path in owned {
        let Some(text) = read_optional(path)? else {
            continue;
        };
        changes.push(FileChange {
            path: path.display().to_string(),
            change: "delete",
            diff: with_diff.then(|| line_diff(&text, "")),
        });
        if !dry_run {
            std::fs::remove_file(path).with_context(|| format!("removing `{}`", path.display()))?;
        }
    }
    let changed = !changes.is_empty();
    let mut next_steps = Vec::new();
    if changed {
        next_steps.push(
            "the durable PATH was left as it is; the shim directory on it is vibe's own".to_owned(),
        );
    }
    ApplyReport {
        protocol: PROTOCOL,
        ok: true,
        command: "self:apply",
        action: if dry_run { "dry-run" } else { "remove" },
        changed,
        profile,
        files: changes,
        effective: None,
        where_to_put: None,
        path: None,
        imported: None,
        next_steps,
        warnings,
    }
    .emit(ctx)
}
