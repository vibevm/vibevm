//! `vibe cache clean` — reclaim store space (PROP-010 §2.8 CMD-CLEAN,
//! §2.1 EXPLICIT-RECLAIM). Reclaiming is an explicit operator action:
//! the bare command refuses and names its three targets (`--all`,
//! `--package`, `--older-than`) so «clean everything» is never a lazy
//! default, the store is never auto-evicted, and removal — unlike a
//! rewrite — does not touch the write-once rule (a reclaimed entry is
//! deleted, not overwritten; §2.7).
//!
//! Confirmation follows the established uninstall contract: `--all`
//! prompts (a mass deletion must never happen silently), with
//! `--assume-yes` / `--unattended` / `--json` implying yes and a
//! non-TTY without them a hard error. The targeted branches
//! (`--package`, `--older-than`) name exactly what dies already, so
//! they run unprompted — the operator's specificity is the consent.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use vibe_core::Group;

use crate::cli::CacheCleanArgs;
use crate::exit_code::InstallError;
use crate::output;

pub(crate) fn run(ctx: &output::Context, args: CacheCleanArgs) -> Result<()> {
    // The EXPLICIT-RECLAIM guard: without a target the command refuses.
    // This is the deliberate friction — a bare `vibe cache clean` must
    // not be able to mean «wipe everything» by accident.
    if !args.all && args.package.is_none() && args.older_than.is_none() {
        bail!(
            "`vibe cache clean` needs an explicit target — reclaiming space is an explicit \
             operator action, never a default. Pass one of:\n  \
             --all                     remove every entry in the store\n  \
             --package <group>/<name>[@<version>]   remove one name, or one version of it\n  \
             --older-than <days>       remove entries older than N days"
        );
    }

    let root = vibe_registry::store_root().context("resolving the machine store root")?;

    if args.all {
        return clean_all(ctx, args.assume_yes, &root);
    }
    if let Some(spec) = &args.package {
        return clean_package(ctx, spec, &root);
    }
    let days = args.older_than.unwrap_or(0);
    clean_older_than(ctx, days, &root)
}

/// `--all` — confirm, then remove every entry. The store root itself
/// survives (empty); a foreign file in it is not ours to touch.
fn clean_all(ctx: &output::Context, assume_yes: bool, root: &std::path::Path) -> Result<()> {
    let approved = if assume_yes || ctx.is_unattended() || ctx.is_json() {
        true
    } else if !console::user_attended() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to clean the \
             whole store non-interactively"
        );
    } else {
        Confirm::new()
            .with_prompt(format!(
                "Remove EVERY package entry under {}? Re-fetching them needs the network.",
                root.display()
            ))
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    let removed = vibe_registry::remove_all().context("removing the store contents")?;
    emit(ctx, root, "all", removed, &[])
}

/// `--package <group>/<name>[@<version>]` — one version, or the whole
/// name. An absent target is an error, not a silent zero: the operator
/// named a specific thing, and a typo should surface.
fn clean_package(ctx: &output::Context, spec: &str, root: &std::path::Path) -> Result<()> {
    let (group, name, version) = parse_package_spec(spec)?;
    let (count, labels) = match &version {
        Some(version) => {
            let removed = vibe_registry::remove_entry(&group, &name, version)
                .context("removing the store entry")?;
            if !removed {
                bail!(
                    "no entry `{group}/{name}@{version}` in the store ({})",
                    root.display()
                );
            }
            (1, vec![format!("{group}/{name}@{version}")])
        }
        None => {
            let removed =
                vibe_registry::remove_name(&group, &name).context("removing the store entries")?;
            if removed == 0 {
                bail!(
                    "no entries `{group}/{name}` in the store ({})",
                    root.display()
                );
            }
            let plural = if removed == 1 { "" } else { "s" };
            (
                removed,
                vec![format!("{group}/{name} ({removed} version{plural})")],
            )
        }
    };
    emit(ctx, root, "package", count, &labels)
}

/// `--older-than <days>` — entries whose store directory's mtime
/// predates the cutoff. Removing nothing is a legitimate outcome (a
/// young store), so unlike `--package` this succeeds with a zero.
fn clean_older_than(ctx: &output::Context, days: u64, root: &std::path::Path) -> Result<()> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(days.saturating_mul(86_400)))
        .context("computing the age cutoff")?;
    let targets = vibe_registry::list_older_than(cutoff);
    let mut labels = Vec::new();
    for (group, name, version) in &targets {
        let label = format!("{group}/{name}@{version}");
        let removed = vibe_registry::remove_entry(group, name, version)
            .with_context(|| format!("removing the store entry {label}"))?;
        if removed {
            labels.push(label);
        }
    }
    emit(ctx, root, "older-than", labels.len(), &labels)
}

/// Parse `<group>/<name>[@<version>]` into its parts.
fn parse_package_spec(spec: &str) -> Result<(Group, String, Option<semver::Version>)> {
    let (group_raw, rest) = spec.split_once('/').ok_or_else(|| {
        anyhow!("`{spec}` is not a `<group>/<name>[@<version>]` package reference")
    })?;
    let group =
        Group::parse(group_raw).map_err(|e| anyhow!("`{group_raw}` is not a valid group: {e}"))?;
    let (name, version) = match rest.split_once('@') {
        Some((name, version)) => (
            name.to_string(),
            Some(
                semver::Version::parse(version)
                    .with_context(|| format!("parsing version `{version}`"))?,
            ),
        ),
        None => (rest.to_string(), None),
    };
    if name.is_empty() {
        bail!("`{spec}` carries an empty package name");
    }
    Ok((group, name, version))
}

/// One shape for every branch's report: what was removed (count +
/// labels) against which store. JSON gets the labels as `entries`;
/// human gets a `- removed` line per label; quiet gets the count.
fn emit(
    ctx: &output::Context,
    root: &std::path::Path,
    mode: &str,
    count: usize,
    labels: &[String],
) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "cache:clean",
            "mode": mode,
            "root": root.display().to_string(),
            "removed": count,
            "entries": labels,
        }))?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe cache clean: removed {count} entr{}",
            if count == 1 { "y" } else { "ies" }
        ));
        return Ok(());
    }
    for label in labels {
        ctx.removed(label);
    }
    ctx.summary(&format!(
        "\nRemoved {count} entr{} from {}.",
        if count == 1 { "y" } else { "ies" },
        root.display()
    ));
    Ok(())
}
