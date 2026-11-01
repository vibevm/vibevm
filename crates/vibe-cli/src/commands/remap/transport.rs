//! `vibe remap export` and `vibe remap import` (`PROP-066#transport`).
//!
//! The two halves of getting a set of rows — and, with
//! `--with-artifacts`, the bytes they point at — into a network that
//! cannot reach the public hosts. `export` writes a directory that is
//! simultaneously a `file://` registry (so `vibe install --registry <dir>`
//! and a `[[registry]] url = "file:///…"` read it unchanged) and an
//! artifact tree laid out by digest. `import` installs the package from
//! that registry as an offer and places the artifacts.
//!
//! Nothing here invents a second packaging format: the registry half is
//! the `<group>/<name>/v<version>/` layout `vibe registry vendor` and
//! `LocalRegistry` already share, and the artifact half is the
//! `<sha256>/<filename>` layout the application download cache already
//! uses — which is exactly why an `--offline` install works from it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#transport");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::Lockfile;
use vibe_core::remap::{RemapKey, RemapPackage, RemapTarget};
use vibe_wire::generated::remap_bundle as wire;

use crate::cli::{RemapExportArgs, RemapImportArgs};
use crate::output;

use super::bundle;
use super::home::RemapHome;

/// `vibe remap export <package> --out <dir> [--with-artifacts]`.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#transport")]
pub(crate) fn export(ctx: &output::Context, args: RemapExportArgs) -> Result<()> {
    let home = RemapHome::open()?;
    let key = args
        .package
        .strip_prefix("remap:")
        .unwrap_or(&args.package)
        .to_owned();
    let Some(entry) = home.installed.find(&key).cloned() else {
        return Err(super::verbs::RemapPackageMissing {
            package: key,
            home: home.root().display().to_string(),
        }
        .into());
    };
    let out = &args.out;
    std::fs::create_dir_all(out).with_context(|| format!("creating `{}`", out.display()))?;
    let placed = write_registry(out, &entry.package, &super::packages::tree_of(&home, &key))?;
    let mut artifacts = Vec::new();
    if args.with_artifacts {
        artifacts = collect(out, &entry.package, &args.path)?;
    }
    let document = wire::RemapBundle {
        protocol: bundle::BUNDLE_PROTOCOL.into(),
        package: entry.package.key.clone(),
        version: entry.package.version.clone(),
        content_hash: entry.package.content_hash.clone(),
        written: crate::commands::init::current_timestamp_utc(),
        artifacts,
    };
    bundle::write_manifest(out, &document)?;
    emit_export(ctx, out, &document, &placed)
}

/// Write the remap package into the bundle's registry half and return the
/// relative path it landed at.
fn write_registry(out: &Path, package: &RemapPackage, tree: &Path) -> Result<String> {
    let (group, name) = package
        .key
        .split_once('/')
        .with_context(|| format!("`{}` is not `<group>/<name>`", package.key))?;
    let relative = PathBuf::from(group)
        .join(name)
        .join(format!("v{}", package.version));
    let destination = out.join(&relative);
    if destination.exists() {
        std::fs::remove_dir_all(&destination)
            .with_context(|| format!("replacing `{}`", destination.display()))?;
    }
    if !tree.exists() {
        bail!(
            "the home holds no tree for `{}` — reinstall it before exporting \
             (violates spec://org.vibevm.core/vibevm/common/PROP-066#transport; \
              fix: `vibe install -g {}` from a registry that serves it)",
            package.key,
            package.key
        );
    }
    copy_tree(tree, &destination)?;
    Ok(vibe_core::machine_json_path(&relative))
}

/// Every artifact the rows and the project's lock name, staged into the
/// bundle by digest.
///
/// A row names its artifacts in one of two ways. A `sha256` row names one
/// digest, and the bytes are wherever this machine already keeps them —
/// the application download cache, or the row's own `path` target. A
/// `package` row names a package, and a package's bytes are a tree in the
/// machine store, which is staged as a registry entry rather than a file.
/// Everything the project's lock names is staged the same way, so an
/// `--offline` install from the bundle resolves.
fn collect(
    out: &Path,
    package: &RemapPackage,
    project: &Path,
) -> Result<Vec<wire::BundledArtifact>> {
    let mut staged: Vec<wire::BundledArtifact> = Vec::new();
    let cache = vibe_core::settings::settings_dir()
        .map(|settings| vibe_app::SettingsRoot::new(&settings).app_sources_cache());
    let rows = package
        .loaded(vibe_core::remap::Layer::UserPackage)
        .map(|loaded| loaded.rows)
        .unwrap_or_default();
    for row in &rows {
        let RemapKey::Sha256(digest) = &row.key else {
            continue;
        };
        let origin = match &row.target {
            RemapTarget::Path(path) => found_by_digest(path, digest),
            _ => None,
        }
        .or_else(|| {
            cache
                .as_deref()
                .and_then(|cache| found_by_digest(cache, digest))
        });
        let Some(file) = origin else { continue };
        let filename = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "artifact".to_owned());
        staged.push(bundle::stage_file(
            out,
            &file,
            digest,
            &filename,
            Some(row.target.display()),
            row.reason.clone(),
        )?);
    }
    // The project's lock, when the command was run inside a project: every
    // locked package's tree, as a registry entry of the bundle.
    let lock = project.join(Lockfile::FILENAME);
    if !lock.exists() {
        return Ok(staged);
    }
    let lockfile =
        Lockfile::read(&lock).with_context(|| format!("reading `{}`", lock.display()))?;
    for locked in &lockfile.packages {
        let Some(entry) = vibe_registry::lookup(&locked.group, &locked.name, &locked.version)
        else {
            continue;
        };
        let relative = PathBuf::from(locked.group.as_str())
            .join(locked.name.as_str())
            .join(format!("v{}", locked.version));
        let destination = out.join(&relative);
        if destination.exists() {
            continue;
        }
        copy_tree(&entry, &destination)?;
        staged.push(wire::BundledArtifact {
            sha256: locked.content_hash.as_str().to_owned(),
            path: vibe_core::machine_json_path(&relative),
            filename: format!("{}/{}", locked.group.as_str(), locked.name),
            bytes: 0,
            place: wire::ArtifactPlace::Store,
            origin: Some(locked.source_url.as_str().to_owned()),
            package: Some(format!(
                "{}/{}@{}",
                locked.group.as_str(),
                locked.name,
                locked.version
            )),
            row: None,
        });
    }
    Ok(staged)
}

/// The file `<dir>/<sha256>/<name>` holds, when one is there.
fn found_by_digest(dir: &Path, digest: &str) -> Option<PathBuf> {
    let entry = dir.join(digest.to_ascii_lowercase());
    if entry.is_file() {
        return Some(entry);
    }
    let mut files = std::fs::read_dir(&entry)
        .ok()?
        .filter_map(Result::ok)
        .map(|found| found.path())
        .filter(|path| path.is_file());
    let only = files.next()?;
    Some(only)
}

/// Copy a package tree, skipping `.git`.
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_name() == ".git" {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// `vibe remap import <dir>` — install the bundle's package as an offer
/// and place its artifacts.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#transport")]
pub(crate) fn import(ctx: &output::Context, args: RemapImportArgs, accept: bool) -> Result<()> {
    let dir = args
        .dir
        .canonicalize()
        .with_context(|| format!("resolving `{}`", args.dir.display()))?;
    let document = bundle::read_manifest(&dir)?;
    let pkgref =
        vibe_core::PackageRef::parse(&format!("{}@={}", document.package, document.version))
            .with_context(|| format!("parsing `{}`", document.package))?;
    // The bundle's registry half is read as a local registry — the same
    // `--registry <dir>` path an operator would type, so an import is not
    // a second way to install a package.
    let store_root = vibe_registry::store_root().context("resolving the machine store root")?;
    let resolver = super::packages::local_resolver(&dir)?;
    let mut home = RemapHome::open()?;
    let installed = {
        let _lock = home.lock()?;
        let installed = super::packages::adopt(
            &resolver,
            &store_root,
            &mut home,
            &pkgref,
            Some(&vibe_registry::vendor::file_url_for_dir(&dir)),
        )?;
        if accept {
            home.ledger.accept(
                &installed.offer.key,
                &installed.offer.content_hash,
                &crate::commands::init::current_timestamp_utc(),
            );
        }
        home.save()?;
        installed
    };
    let imported = bundle::place(&dir, &document)?;
    emit_import(ctx, &dir, &home, &installed.offer, &imported)
}

/// The export report.
fn emit_export(
    ctx: &output::Context,
    out: &Path,
    document: &wire::RemapBundle,
    registry_path: &str,
) -> Result<()> {
    let url = vibe_registry::vendor::file_url_for_dir(out);
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "remap:export",
            "out": out.display().to_string(),
            "registry_url": url,
            "package": document.package,
            "version": document.version,
            "package_path": registry_path,
            "artifacts": document.artifacts.len(),
        }));
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe remap export: {}@{} + {} artifact(s) → {}",
            document.package,
            document.version,
            document.artifacts.len(),
            out.display()
        ));
        return Ok(());
    }
    ctx.created(&format!(
        "{registry_path} — {}@{}",
        document.package, document.version
    ));
    for artifact in &document.artifacts {
        ctx.created(&artifact.path);
    }
    ctx.summary(&format!(
        "\nWrote a `file://` registry and {} artifact(s) to {}.\n\
         Carry the directory over, then `vibe remap import <dir>` — or install \
         straight from it with `--registry {}`.",
        document.artifacts.len(),
        out.display(),
        url
    ));
    Ok(())
}

/// The import report.
fn emit_import(
    ctx: &output::Context,
    dir: &Path,
    home: &RemapHome,
    offer: &RemapPackage,
    imported: &bundle::Imported,
) -> Result<()> {
    let state = home
        .state(&offer.key)
        .map(|state| state.label())
        .unwrap_or("offered");
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "remap:import",
            "dir": dir.display().to_string(),
            "home": home.root().display().to_string(),
            "package": offer.key,
            "version": offer.version,
            "state": state,
            "placed": imported.placed,
            "already_present": imported.already,
            "refused": imported.refused
                .iter()
                .map(|(what, why)| serde_json::json!({ "artifact": what, "reason": why }))
                .collect::<Vec<_>>(),
        }));
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe remap import: {}@{} {state}, {} artifact(s) placed",
            offer.key,
            offer.version,
            imported.placed.len()
        ));
        return Ok(());
    }
    ctx.created(&format!("{}@{} — {state}", offer.key, offer.version));
    for label in &imported.placed {
        ctx.created(label);
    }
    for label in &imported.already {
        ctx.skipped(label, "already present and verified — bytes untouched");
    }
    for (label, reason) in &imported.refused {
        ctx.step(&format!("{label} not placed — {reason}"));
    }
    ctx.summary(&format!(
        "\n{} artifact(s) placed, {} already present, {} refused.{}",
        imported.placed.len(),
        imported.already.len(),
        imported.refused.len(),
        if state == "offered" {
            format!(
                "\nNothing applies yet — `vibe remap accept {}` applies the rows.",
                offer.key
            )
        } else {
            String::new()
        }
    ));
    Ok(())
}
