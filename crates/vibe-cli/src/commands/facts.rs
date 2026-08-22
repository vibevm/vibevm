//! `vibe facts` — CRUD and host-spec synchronization for adoption facts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#cli");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use vibe_core::Group;
use vibe_core::manifest::Lockfile;
use vibe_facts::{FactEntry, FactOrigin, FactStatus, Registry, host_package, sync};
use vibe_specdoc::doc::{Block, Section, SpecDoc, Unit};
use vibe_workspace::{Workspace, vibedeps};

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
    /// Copy authored package statuses into absent consumer records.
    Adopt {
        /// Source package in `<group>/<name>` form.
        #[arg(long)]
        package: String,
        /// Keep full fact addresses beginning with this prefix only.
        #[arg(long)]
        prefix: Option<String>,
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
        FactsSubcommand::Adopt { package, prefix } => adopt(&root, &package, prefix.as_deref()),
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
    if entry.origin == FactOrigin::Package {
        rederive_or_defer(root, entry.package.as_deref())?;
    }
    Ok(())
}

fn rm(root: &Path, address: &str) -> Result<()> {
    let mut registry = Registry::load(root)?;
    let package = registry.get(address).and_then(|entry| {
        (entry.origin == FactOrigin::Package)
            .then(|| entry.package.clone())
            .flatten()
    });
    if registry.remove(root, address)? {
        println!("removed {address}");
        rederive_or_defer(root, package.as_deref())?;
    } else {
        println!("not in registry: {address}");
    }
    Ok(())
}

fn rederive_or_defer(root: &Path, package: Option<&str>) -> Result<()> {
    let Some(package) = package else {
        return Ok(());
    };
    if !super::reinstall::rederive_package(root, package)? {
        println!("{package}: not installed; will apply at install");
    }
    Ok(())
}

fn adopt(root: &Path, package: &str, prefix: Option<&str>) -> Result<()> {
    let slot = installed_slot(root, package)?;
    let inputs = slot_spec_inputs(&slot)?;
    let mut authored = Vec::new();
    for input in inputs {
        let text = fs::read_to_string(&input.output)
            .with_context(|| format!("reading `{}`", input.output.display()))?;
        let doc = match input.output.extension().and_then(|ext| ext.to_str()) {
            Some("md") => vibe_specdoc::from_markdown(&text),
            Some("xml") => vibe_specdoc::from_xml(&text),
            _ => continue,
        }
        .with_context(|| format!("parsing `{}` through vibe-specdoc", input.output.display()))?;
        let address_prefix = format!(
            "spec://{package}/{}#",
            vibe_spec::canonical_doc_path(&input.source)
        );
        collect_authored(&doc, &address_prefix, &mut authored);
    }
    authored.sort_by(|a, b| a.0.cmp(&b.0));

    let mut registry = Registry::load(root)?;
    let mut added = 0usize;
    let mut kept = 0usize;
    for (address, status) in authored {
        if prefix.is_some_and(|prefix| !address.starts_with(prefix)) {
            continue;
        }
        if registry.get(&address).is_some() {
            kept += 1;
            continue;
        }
        registry.upsert(
            root,
            FactEntry {
                address,
                origin: FactOrigin::Package,
                package: Some(package.to_string()),
                status: Some(status),
                comment: None,
            },
        )?;
        added += 1;
    }
    println!("facts adopt {package}: added={added} kept={kept}");
    rederive_or_defer(root, Some(package))?;
    Ok(())
}

struct SlotSpecInput {
    source: String,
    output: PathBuf,
}

fn installed_slot(root: &Path, package: &str) -> Result<PathBuf> {
    let Some((group, name)) = package.split_once('/') else {
        bail!("package must use the `<group>/<name>` form: {package}");
    };
    if name.is_empty() || name.contains('/') {
        bail!("package must use the `<group>/<name>` form: {package}");
    }
    let group = Group::parse(group)?;
    let workspace = Workspace::discover(root)?;
    let lock_path = workspace.root.join(Lockfile::FILENAME);
    let lock = Lockfile::read(&lock_path)
        .with_context(|| format!("reading `{}` to locate `{package}`", lock_path.display()))?;
    let locked = lock
        .packages
        .iter()
        .find(|locked| locked.group == group && locked.name == name)
        .with_context(|| format!("package `{package}` is not installed"))?;
    let slot = vibedeps::slot_abs_path(
        &workspace.root,
        &locked.group,
        &locked.name,
        &locked.version,
    );
    if !slot.is_dir() {
        bail!(
            "package `{package}` has no materialised slot at `{}`",
            slot.display()
        );
    }
    Ok(slot)
}

fn slot_spec_inputs(slot: &Path) -> Result<Vec<SlotSpecInput>> {
    if let Ok(manifest) = vibedeps::read_derived_manifest(slot) {
        return Ok(manifest
            .files
            .into_iter()
            .filter(|file| is_spec_document(&file.source))
            .map(|file| SlotSpecInput {
                source: file.source,
                output: slot.join(file.output),
            })
            .collect());
    }
    let mut inputs = Vec::new();
    collect_slot_specs(slot, slot, &mut inputs)?;
    inputs.sort_by(|a, b| a.source.cmp(&b.source));
    Ok(inputs)
}

fn collect_slot_specs(root: &Path, dir: &Path, out: &mut Vec<SlotSpecInput>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("walking `{}`", dir.display()))? {
        let entry = entry.with_context(|| format!("walking `{}`", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_slot_specs(root, &path, out)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(root).map_err(|_| {
                anyhow::anyhow!(
                    "walked path `{}` escaped `{}`",
                    path.display(),
                    root.display()
                )
            })?;
            let source = rel.to_string_lossy().replace('\\', "/");
            if is_spec_document(&source)
                && source != "spec/boot/STATIC.md"
                && source != "spec/boot/INDEX.md"
            {
                out.push(SlotSpecInput {
                    source,
                    output: path,
                });
            }
        }
    }
    Ok(())
}

fn is_spec_document(rel: &str) -> bool {
    let path = Path::new(rel);
    let spec_extension = matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "xml")
    );
    spec_extension && (rel.starts_with("spec/") || matches!(rel, "README.md" | "README.xml"))
}

fn collect_authored(doc: &SpecDoc, prefix: &str, out: &mut Vec<(String, FactStatus)>) {
    collect_blocks(&doc.preamble, prefix, out);
    for section in &doc.sections {
        collect_section(section, prefix, out);
    }
}

fn collect_section(section: &Section, prefix: &str, out: &mut Vec<(String, FactStatus)>) {
    collect_blocks(&section.blocks, prefix, out);
    for child in &section.sections {
        collect_section(child, prefix, out);
    }
}

fn collect_blocks(blocks: &[Block], prefix: &str, out: &mut Vec<(String, FactStatus)>) {
    for block in blocks {
        match block {
            Block::Paragraph(unit) | Block::Quote(unit) => collect_unit(unit, prefix, out),
            Block::List { items, .. } => {
                for unit in items {
                    collect_unit(unit, prefix, out);
                }
            }
            Block::Table { rows } => {
                for unit in rows.iter().flatten() {
                    collect_unit(unit, prefix, out);
                }
            }
            Block::Fence { .. } => {}
        }
    }
}

fn collect_unit(unit: &Unit, prefix: &str, out: &mut Vec<(String, FactStatus)>) {
    let Some(fact) = unit.fact.as_ref() else {
        return;
    };
    let (Some(id), Some(status)) = (fact.id.as_deref(), fact.status.as_ref()) else {
        return;
    };
    out.push((
        format!("{prefix}{id}"),
        FactStatus::new(status.stage, status.state),
    ));
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
