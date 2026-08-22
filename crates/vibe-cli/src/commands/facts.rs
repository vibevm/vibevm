//! `vibe facts` — CRUD and host-spec synchronization for adoption facts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#cli");

use std::path::Path;

use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use vibe_facts::{FactEntry, FactStatus, Registry, host_package, sync};

use crate::output;

#[derive(Debug, Args)]
pub struct FactsArgs {
    #[command(subcommand)]
    pub command: FactsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum FactsSubcommand {
    /// List registry entries in address order.
    List {
        /// Keep entries from this `<group>/<name>` source only.
        #[arg(long)]
        package: Option<String>,
        /// Keep entries at this progress-core `<stage>/<state>` only.
        #[arg(long, conflicts_with = "indeterminate")]
        status: Option<String>,
        /// Keep entries with no adoption status only.
        #[arg(long)]
        indeterminate: bool,
    },
    /// Show one entry, or report that it is not registered.
    Get { address: String },
    /// Create or update an entry.
    Set {
        address: String,
        status: String,
        #[arg(long)]
        comment: Option<String>,
    },
    /// Remove one entry.
    Rm { address: String },
    /// Compare host entries with spec markers; optionally reconcile them.
    Sync {
        #[arg(long)]
        write: bool,
    },
}

pub fn run(_ctx: &output::Context, args: FactsArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = super::resolve_project_root(&cwd)?;
    match args.command {
        FactsSubcommand::List {
            package,
            status,
            indeterminate,
        } => list(&root, package.as_deref(), status.as_deref(), indeterminate),
        FactsSubcommand::Get { address } => get(&root, &address),
        FactsSubcommand::Set {
            address,
            status,
            comment,
        } => set(&root, address, &status, comment),
        FactsSubcommand::Rm { address } => rm(&root, &address),
        FactsSubcommand::Sync { write } => sync_command(&root, write),
    }
}

fn list(
    root: &Path,
    package: Option<&str>,
    status: Option<&str>,
    indeterminate: bool,
) -> Result<()> {
    let registry = Registry::load(root)?;
    let wanted_status = status.map(FactStatus::parse).transpose()?;
    for entry in registry.entries() {
        if let Some(package) = package
            && entry.source_package()? != package
        {
            continue;
        }
        if let Some(status) = wanted_status
            && entry.status != Some(status)
        {
            continue;
        }
        if indeterminate && entry.status.is_some() {
            continue;
        }
        print_entry(entry);
    }
    Ok(())
}

fn get(root: &Path, address: &str) -> Result<()> {
    let registry = Registry::load(root)?;
    match registry.get(address) {
        Some(entry) => print_entry(entry),
        None => println!("not in registry: {address}"),
    }
    Ok(())
}

fn set(root: &Path, address: String, status: &str, comment: Option<String>) -> Result<()> {
    let host = host_package(root)?;
    let status = FactStatus::parse(status)?;
    let entry = FactEntry::for_host(address, &host, Some(status), comment)?;
    let mut registry = Registry::load(root)?;
    registry.upsert(root, entry.clone())?;
    println!("set {} {}", entry.address, status);
    Ok(())
}

fn rm(root: &Path, address: &str) -> Result<()> {
    let mut registry = Registry::load(root)?;
    if registry.remove(root, address)? {
        println!("removed {address}");
    } else {
        println!("not in registry: {address}");
    }
    Ok(())
}

fn sync_command(root: &Path, write: bool) -> Result<()> {
    let mut registry = Registry::load(root)?;
    if write {
        let applied = sync::reconcile(root, &mut registry)?;
        if applied.is_empty() {
            println!("facts sync: clean");
        } else {
            for mismatch in &applied {
                print_mismatch(mismatch);
            }
            println!(
                "facts sync: reconciled {} entrie(s) from spec",
                applied.len()
            );
        }
        let remaining = sync::check(root, &registry)?;
        if !remaining.is_empty() {
            bail!(
                "facts sync: {} mismatch(es) remain after write",
                remaining.len()
            );
        }
        return Ok(());
    }

    let mismatches = sync::check(root, &registry)?;
    if mismatches.is_empty() {
        println!("facts sync: clean");
        return Ok(());
    }
    for mismatch in &mismatches {
        print_mismatch(mismatch);
    }
    bail!(
        "facts sync: {} mismatch(es); spec is authoritative — run `vibe facts sync --write`",
        mismatches.len()
    )
}

fn print_entry(entry: &FactEntry) {
    let status = entry
        .status
        .map(|status| status.to_string())
        .unwrap_or_else(|| "indeterminate".to_string());
    println!("{}\t{}\t{}", entry.address, entry.origin, status);
}

fn print_mismatch(mismatch: &sync::SyncMismatch) {
    let location = match (&mismatch.path, mismatch.line) {
        (Some(path), Some(line)) => format!("{}:{line}", path.display()),
        _ => "not found in spec".to_string(),
    };
    println!(
        "{}\tspec={}\tregistry={}\t{}",
        mismatch.address,
        mismatch.spec_status_text(),
        mismatch.registry_status_text(),
        location
    );
}
