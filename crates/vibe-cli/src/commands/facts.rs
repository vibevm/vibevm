//! `vibe facts` — CRUD and host-spec synchronization for adoption facts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#cli");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use vibe_core::Group;
use vibe_core::manifest::{Lockfile, Manifest, SourceSelector, SpecFormat};
use vibe_facts::{
    AuthoredFact, CapturedDeclaredDocument, FactEntry, FactOrigin, FactStatus, Registry,
    SourceFileWitness, SourceKind, adoption_counts, host_package, observe_declared_source, orphans,
    remove_package_file, scan_authored_facts, sync,
};
use vibe_safefs::Project;
use vibe_workspace::{Workspace, vibedeps};

use crate::{cli::ProgressCheckArgs, output};

#[derive(Debug, Args)]
pub struct FactsArgs {
    #[command(subcommand)]
    pub command: FactsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum FactsSubcommand {
    /// Validate facts markup. This is the durable spelling of the markup
    /// lint also available through the transitional `progress check` alias.
    Check(ProgressCheckArgs),
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

pub fn run(ctx: &output::Context, args: FactsArgs) -> Result<()> {
    match args.command {
        FactsSubcommand::Check(args) => super::facts_check::run(ctx, &args),
        FactsSubcommand::List {
            package,
            status,
            indeterminate,
        } => with_project_root(|root| {
            list(root, package.as_deref(), status.as_deref(), indeterminate)
        }),
        FactsSubcommand::Get { address } => with_project_root(|root| get(root, &address)),
        FactsSubcommand::Set {
            address,
            status,
            comment,
        } => with_project_root(|root| set(root, address, &status, comment)),
        FactsSubcommand::Rm { address } => with_project_root(|root| rm(root, &address)),
        FactsSubcommand::Sync { write } => with_project_root(|root| sync_command(root, write)),
        FactsSubcommand::Adopt { package, prefix } => {
            with_project_root(|root| adopt(root, &package, prefix.as_deref()))
        }
        FactsSubcommand::Clean { dry_run } => with_project_root(|root| clean(root, dry_run)),
        FactsSubcommand::Report { package } => {
            with_project_root(|root| report(root, package.as_deref()))
        }
    }
}

fn with_project_root<T>(operation: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
    let cwd = std::env::current_dir()?;
    let root = super::resolve_project_root(&cwd)?;
    operation(&root)
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
    let authored = authored_for_selected(&slot, package)?;

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

const ADOPT_MANIFEST_MAX_BYTES: usize = 1024 * 1024;
const ADOPT_SOURCE_MAX_BYTES: usize = 16 * 1024 * 1024;

/// The lock, rather than a slot-controlled record, supplies the expected
/// source identity and version for both facts consumers.
struct SelectedFactsSlot {
    path: PathBuf,
    version: String,
    source_hash: vibe_core::ContentHash,
}

/// Explicit descriptors are opt-in. An old package with no declarations keeps
/// the original suffix scanner; a declared package never falls back to it.
fn authored_for_selected(slot: &SelectedFactsSlot, package: &str) -> Result<Vec<AuthoredFact>> {
    let pinned = Project::open(&slot.path)
        .with_context(|| format!("pinning package slot `{}`", slot.path.display()))?;
    let manifest_bytes = pinned
        .read_file_bounded("vibe.toml", ADOPT_MANIFEST_MAX_BYTES)?
        .context("installed package has no vibe.toml")?;
    let manifest_text =
        std::str::from_utf8(&manifest_bytes).context("installed package manifest is not UTF-8")?;
    let manifest = Manifest::parse_str(manifest_text)?;
    let meta = manifest.require_package()?;
    ensure_package_coordinate(meta, package)?;
    if meta.version.to_string() != slot.version {
        bail!(
            "installed package version `{}` differs from lock-selected `{}`",
            meta.version,
            slot.version
        );
    }
    if meta.sources.is_empty() {
        return scan_authored_facts(&slot.path, package, SourceKind::Package).map_err(Into::into);
    }
    if meta
        .sources
        .iter()
        .any(|descriptor| matches!(descriptor.selector(), SourceSelector::Directory(_)))
    {
        bail!("declared directory sources need a complete captured census before facts adopt");
    }
    let record_before = pinned
        .read_file_bounded(vibedeps::SLOT_RECORD_FILENAME, ADOPT_MANIFEST_MAX_BYTES)?
        .context("declared package has no slot provenance record")?;
    let record =
        vibedeps::read_slot_record(&slot.path).map_err(|reason| anyhow::anyhow!(reason))?;
    let record_after = pinned
        .read_file_bounded(vibedeps::SLOT_RECORD_FILENAME, ADOPT_MANIFEST_MAX_BYTES)?
        .context("declared package slot record disappeared")?;
    if record_before != record_after || record.spec_format != SpecFormat::Mixed {
        bail!(
            "declared facts adopt requires an unchanged Mixed slot record and original source bytes"
        );
    }
    if record.source_hash != slot.source_hash {
        bail!("declared facts slot source hash differs from lock-selected source hash");
    }
    verify_slot_source(&record, "vibe.toml", &manifest_bytes)?;
    let mut held = Vec::with_capacity(meta.sources.len());
    let mut captured_paths = BTreeSet::new();
    for descriptor in &meta.sources {
        let SourceSelector::File(path) = descriptor.selector() else {
            unreachable!()
        };
        if !captured_paths.insert(path.as_str()) {
            continue;
        }
        let bytes = pinned
            .read_file_bounded(path, ADOPT_SOURCE_MAX_BYTES)
            .with_context(|| format!("capturing declared package source `{path}`"))?;
        if let Some(raw) = bytes.as_deref() {
            verify_slot_source(&record, path, raw)?;
        }
        held.push((path.as_str(), bytes));
    }
    let inventory = held
        .iter()
        .map(|(path, bytes)| CapturedDeclaredDocument {
            path,
            bytes: bytes.as_deref(),
        })
        .collect::<Vec<_>>();
    let observed = observe_declared_source(meta, &inventory)
        .map_err(|reason| anyhow::anyhow!("declared source walk refused: {reason:?}"))?;
    if !observed.complete {
        let path = observed
            .documents
            .iter()
            .find(|(_, row)| {
                !matches!(
                    row,
                    vibe_facts::DeclaredDocumentObservation::Facts { .. }
                        | vibe_facts::DeclaredDocumentObservation::Raw { .. }
                        | vibe_facts::DeclaredDocumentObservation::Structure { .. }
                )
            })
            .map(|(path, _)| path.as_str())
            .unwrap_or("<unknown>");
        bail!("declared source `{path}` is incomplete; facts adopt made no registry change");
    }
    Ok(observed.facts)
}

fn ensure_package_coordinate(
    meta: &vibe_core::manifest::PackageMeta,
    selected: &str,
) -> Result<()> {
    let observed = format!("{}/{}", meta.group, meta.name);
    if observed != selected {
        bail!("installed package coordinate `{observed}` differs from selected `{selected}`");
    }
    Ok(())
}

fn verify_slot_source(record: &vibedeps::SlotRecord, path: &str, bytes: &[u8]) -> Result<()> {
    let witness = SourceFileWitness::of(path, bytes);
    let row = record
        .files
        .iter()
        .find(|row| row.path == path)
        .with_context(|| format!("slot record has no declared source `{path}`"))?;
    if witness.digest.strip_prefix("sha256:") != Some(row.sha256.as_str()) {
        bail!("declared source `{path}` differs from recorded slot payload");
    }
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
                .map(|slot| authored_for_selected(&slot, package))
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

fn installed_slot(root: &Path, package: &str) -> Result<SelectedFactsSlot> {
    materialised_slot(root, package)?
        .with_context(|| format!("package `{package}` has no materialised installed slot"))
}

fn materialised_slot(root: &Path, package: &str) -> Result<Option<SelectedFactsSlot>> {
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
    Ok(slot.is_dir().then(|| SelectedFactsSlot {
        path: slot,
        version: locked.version.to_string(),
        source_hash: locked.content_hash.clone(),
    }))
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
