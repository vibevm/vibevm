//! `vibe facts` — CRUD and host-spec synchronization for adoption facts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#cli");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use vibe_core::Group;
use vibe_core::manifest::Lockfile;
use vibe_facts::{
    AuthoredFact, FactEntry, FactOrigin, FactStatus, Registry, adoption_counts, authored_facts,
    host_package, orphans, remove_package_file, sync,
};
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
    /// Remove registry files whose source packages are no longer installed.
    Clean {
        /// Name orphaned files without deleting them.
        #[arg(long)]
        dry_run: bool,
    },
    /// Summarise consumer adoption against facts authored by each package.
    Report {
        /// Report only this `<group>/<name>` source package.
        #[arg(long)]
        package: Option<String>,
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
        FactsSubcommand::Clean { dry_run } => clean(&root, dry_run),
        FactsSubcommand::Report { package } => report(&root, package.as_deref()),
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
    let authored = slot_authored_facts(&slot, package)?;

    let mut registry = Registry::load(root)?;
    let mut added = 0usize;
    let mut kept = 0usize;
    for authored in authored {
        if prefix.is_some_and(|prefix| !authored.address.starts_with(prefix)) {
            continue;
        }
        let Some(status) = authored.status else {
            continue;
        };
        if registry.get(&authored.address).is_some() {
            kept += 1;
            continue;
        }
        registry.upsert(
            root,
            FactEntry {
                address: authored.address,
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

fn clean(root: &Path, dry_run: bool) -> Result<()> {
    let installed = installed_packages(root)?;
    let registry = Registry::load(root)?;
    let package_files: BTreeSet<String> = registry
        .entries()
        .filter(|entry| entry.origin == FactOrigin::Package)
        .filter_map(|entry| entry.package.clone())
        .collect();
    let orphaned = orphans(root, &installed)?;
    let mut removed = 0usize;
    for orphan in &orphaned {
        let file = project_relative(root, &orphan.file);
        if dry_run {
            println!(
                "would remove {file} ({} entries; package {})",
                orphan.entries, orphan.package
            );
        } else if remove_package_file(root, &orphan.package)? {
            removed += 1;
            println!(
                "removed {file} ({} entries; package {})",
                orphan.entries, orphan.package
            );
        }
    }
    let kept = package_files.len().saturating_sub(removed);
    if dry_run {
        println!(
            "facts clean --dry-run: removed=0 kept={} orphaned={}",
            package_files.len(),
            orphaned.len()
        );
    } else {
        println!("facts clean: removed={removed} kept={kept}");
    }
    Ok(())
}

fn report(root: &Path, selected: Option<&str>) -> Result<()> {
    let registry = Registry::load(root)?;
    let installed = installed_packages(root)?;
    let packages: BTreeSet<String> = match selected {
        Some(package) => [package.to_string()].into_iter().collect(),
        None => registry
            .entries()
            .filter(|entry| entry.origin == FactOrigin::Package)
            .filter_map(|entry| entry.package.clone())
            .collect(),
    };
    let mut adopted = 0usize;
    let mut indeterminate = 0usize;
    let mut total_authored = 0usize;
    let mut unavailable = 0usize;
    for package in &packages {
        let authored = if installed.contains(package) {
            materialised_slot(root, package)?
                .map(|slot| slot_authored_facts(&slot, package))
                .transpose()?
        } else {
            None
        };
        let counts = adoption_counts(&registry, package, authored.as_deref().unwrap_or_default());
        adopted += counts.adopted;
        indeterminate += counts.indeterminate;
        if authored.is_some() {
            total_authored += counts.total_authored;
            println!(
                "{package}  {}/{} (indeterminate {})",
                counts.adopted, counts.total_authored, counts.indeterminate
            );
        } else {
            unavailable += 1;
            println!(
                "{package}  {}/? (indeterminate {}) — total unavailable: package has no materialised slot",
                counts.adopted, counts.indeterminate
            );
        }
    }
    println!(
        "TOTAL  adopted={adopted} authored={total_authored} indeterminate={indeterminate} packages={} unavailable={unavailable}",
        packages.len()
    );
    Ok(())
}

fn installed_packages(root: &Path) -> Result<BTreeSet<String>> {
    let workspace = Workspace::discover(root)?;
    let lock_path = workspace.root.join(Lockfile::FILENAME);
    if !lock_path.is_file() {
        return Ok(BTreeSet::new());
    }
    let lock = Lockfile::read(&lock_path)
        .with_context(|| format!("reading `{}` as installedness source", lock_path.display()))?;
    Ok(lock
        .packages
        .iter()
        .map(|package| format!("{}/{}", package.group, package.name))
        .collect())
}

fn project_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

struct SlotSpecInput {
    source: String,
    output: PathBuf,
}

fn installed_slot(root: &Path, package: &str) -> Result<PathBuf> {
    materialised_slot(root, package)?
        .with_context(|| format!("package `{package}` has no materialised installed slot"))
}

fn materialised_slot(root: &Path, package: &str) -> Result<Option<PathBuf>> {
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
        .find(|locked| locked.group == group && locked.name == name);
    let Some(locked) = locked else {
        return Ok(None);
    };
    let slot = if locked.materialization.is_in_place() {
        vibedeps::in_place_slot_abs_path(&workspace.root, &locked.group, &locked.name)
    } else {
        vibedeps::slot_abs_path(
            &workspace.root,
            &locked.group,
            &locked.name,
            &locked.version,
        )
    };
    Ok(slot.is_dir().then_some(slot))
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

fn slot_authored_facts(slot: &Path, package: &str) -> Result<Vec<AuthoredFact>> {
    let inputs = slot_spec_inputs(slot)?;
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
        authored.extend(authored_facts(&doc, &address_prefix));
    }
    authored.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(authored)
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
