//! Taking a remap package out of a home (`PROP-066#uninstall`).
//!
//! Uninstalling a package that offers accepted rows never silently changes
//! what the machine fetches from. Before anything is removed the two
//! blocks are shown — the rows that would stop applying, because no other
//! accepted package offers them, and the rows that stay because one still
//! does — and the question is `keep these N rows as local rows? (y/n)`.
//! `yes` copies the orphans into the user home's own `[[remap]]` table
//! with `from = "<package>@<version>"` and the original `reason`, so the
//! machine keeps working and `remap list` still says where each row came
//! from. `no` drops them.
//!
//! The acceptance record is deleted with the package either way: the
//! answer was about a package that is gone, and leaving it would make a
//! reinstall apply rows nobody was asked about again.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#uninstall");

use anyhow::{Context, Result};
use dialoguer::Confirm;
use vibe_core::remap::{KeptRow, RowIdentity};

use crate::cli::UninstallArgs;
use crate::output;

use super::home::RemapHome;

/// Whether `package` is an installed remap package of the user home — the
/// question `vibe uninstall` asks before it reaches the application path.
pub(crate) fn claims(package: &str) -> bool {
    let key = package.strip_prefix("remap:").unwrap_or(package);
    RemapHome::open()
        .map(|home| home.installed.find(key).is_some())
        .unwrap_or(false)
}

/// Remove one remap package, answering the kept-rows question.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#uninstall")]
pub(crate) fn run(ctx: &output::Context, args: &UninstallArgs) -> Result<()> {
    let key = args
        .package
        .strip_prefix("remap:")
        .unwrap_or(&args.package)
        .to_owned();
    let mut home = RemapHome::open()?;
    let Some(entry) = home.installed.find(&key).cloned() else {
        return Err(super::verbs::RemapPackageMissing {
            package: key,
            home: home.root().display().to_string(),
        }
        .into());
    };
    let label = entry.package.label();
    let (orphaned, surviving) = home.providers().split_on_removal(&label);
    let keep = decide(ctx, args, &orphaned, &surviving)?;
    let kept = if keep {
        copy_into_local_layer(&home, &entry.package, &orphaned)?
    } else {
        Vec::new()
    };
    {
        let _lock = home.lock()?;
        home.ledger.forget(&key);
        home.installed.remove(&key);
        let tree = super::packages::tree_of(&home, &key);
        if tree.exists() {
            std::fs::remove_dir_all(&tree)
                .with_context(|| format!("removing `{}`", tree.display()))?;
        }
        home.save()?;
    }
    report(ctx, &home, &label, &orphaned, &surviving, &kept, keep)
}

/// Answer the kept-rows question: the flags first, then the terminal, then
/// the refusal (`PROP-066#uninstall`).
fn decide(
    ctx: &output::Context,
    args: &UninstallArgs,
    orphaned: &[RowIdentity],
    surviving: &[RowIdentity],
) -> Result<bool> {
    if args.keep_remap {
        return Ok(true);
    }
    if args.drop_remap || orphaned.is_empty() {
        // With nothing orphaned there is nothing to keep, so the
        // question has no content and asking it would be noise.
        return Ok(false);
    }
    if ctx.is_json() || ctx.is_unattended() || !console::user_attended() {
        return Err(super::consent::ConsentRequired {
            count: orphaned.len(),
            what: "orphaned remap row(s)",
            packages: orphaned
                .iter()
                .map(RowIdentity::display)
                .collect::<Vec<_>>()
                .join("; "),
            accept: "vibe uninstall … --keep-remap".to_owned(),
            reject: "vibe uninstall … --drop-remap".to_owned(),
        }
        .into());
    }
    ctx.suspend_progress(|| {
        eprintln!();
        eprintln!("rows that would stop applying:");
        for row in orphaned {
            eprintln!("  {}", row.display());
        }
        if !surviving.is_empty() {
            eprintln!("rows another accepted package still offers:");
            for row in surviving {
                eprintln!("  {}", row.display());
            }
        }
        crate::output::prompts::space_before_question();
        Confirm::with_theme(&crate::output::prompt_theme())
            .with_prompt(format!("keep these {} rows as local rows?", orphaned.len()))
            .default(true)
            .interact()
            .context("reading the kept-rows answer")
    })
}

/// Append the orphaned rows to the user home's `settings.toml` as local
/// `[[remap]]` tables, each carrying its original reason and the package
/// it came from.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#uninstall")]
fn copy_into_local_layer(
    home: &RemapHome,
    package: &vibe_core::remap::RemapPackage,
    orphaned: &[RowIdentity],
) -> Result<Vec<String>> {
    if orphaned.is_empty() {
        return Ok(Vec::new());
    }
    // The reason is the package's own words, found by matching the row's
    // identity back to the row that declared it.
    let declared = package
        .loaded(vibe_core::remap::Layer::UserPackage)
        .map(|loaded| loaded.rows)
        .unwrap_or_default();
    let path = home.root().join("settings.toml");
    let mut body = std::fs::read_to_string(&path).unwrap_or_default();
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    let mut kept = Vec::new();
    for identity in orphaned {
        let reason = declared
            .iter()
            .find(|row| RowIdentity::of(row) == *identity)
            .and_then(|row| row.reason.clone());
        let row = KeptRow {
            identity: identity.clone(),
            reason,
            from: package.label(),
        };
        body.push_str(
            &row.to_toml()
                .map_err(|reason| anyhow::anyhow!("writing a kept remap row: {reason}"))?,
        );
        kept.push(identity.display());
    }
    std::fs::write(&path, body)
        .with_context(|| format!("appending the kept remap rows to `{}`", path.display()))?;
    Ok(kept)
}

/// The report: what stopped applying, what stayed, and what was kept.
fn report(
    ctx: &output::Context,
    home: &RemapHome,
    label: &str,
    orphaned: &[RowIdentity],
    surviving: &[RowIdentity],
    kept: &[String],
    keep: bool,
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "remap:uninstall",
            "home": home.root().display().to_string(),
            "package": label,
            "orphaned": orphaned.iter().map(RowIdentity::display).collect::<Vec<_>>(),
            "surviving": surviving.iter().map(RowIdentity::display).collect::<Vec<_>>(),
            "kept": kept,
            "kept_as_local_rows": keep,
        }));
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe uninstall {label}: {} row(s) kept, {} dropped",
            kept.len(),
            orphaned.len() - kept.len()
        ));
        return Ok(());
    }
    ctx.removed(label);
    for row in surviving {
        ctx.skipped(&row.display(), "another accepted package still offers it");
    }
    for row in kept {
        ctx.created(&format!("{row} — kept as a local row"));
    }
    if !keep {
        for row in orphaned {
            ctx.removed(&row.display());
        }
    }
    ctx.summary(&format!(
        "\n{label} removed from {}; {} row(s) kept as local rows.",
        home.root().display(),
        kept.len()
    ));
    Ok(())
}
