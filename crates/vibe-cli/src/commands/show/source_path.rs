//! `vibe show source-path` — resolve an installed package or authenticated
//! embedded source to its canonical physical root.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::PackageRef;
use vibe_core::manifest::{LockedEmbeddedSource, LockedPackage, Lockfile, Manifest};
use vibe_registry::CachedEmbeddedSource;

use crate::cli::ShowSourcePathArgs;
use crate::output;

use super::resolve_project_root;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum RootKind {
    Package,
    Embedded,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct SourcePathReport {
    ok: bool,
    command: &'static str,
    package: String,
    root_kind: RootKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedded: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tree_oid: Option<String>,
    content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license_file_sha256: Option<String>,
    path: String,
}

pub(super) fn run_source_path(
    ctx: &output::Context,
    args: ShowSourcePathArgs,
    offline: bool,
) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let report = resolve_source_path(
        &project_root,
        &args.package,
        args.embedded.as_deref(),
        offline,
        |locked, offline| {
            vibe_registry::cache_locked_embedded_source_with(locked, offline)
                .map_err(anyhow::Error::from)
        },
    )?;

    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    // Intentionally one raw line in both normal and quiet human modes: this
    // command is a path primitive for shells, build tools, and static loaders.
    ctx.summary(&report.path);
    Ok(())
}

fn resolve_source_path<F>(
    project_root: &Path,
    requested: &str,
    embedded_name: Option<&str>,
    offline: bool,
    hydrate: F,
) -> Result<SourcePathReport>
where
    F: FnOnce(&LockedEmbeddedSource, bool) -> Result<CachedEmbeddedSource>,
{
    let lock_path = project_root.join(Lockfile::FILENAME);
    if !lock_path.is_file() {
        bail!(
            "project `{}` has no vibe.lock; install `{requested}` before asking for its source path",
            project_root.display()
        );
    }
    let lock =
        Lockfile::read(&lock_path).with_context(|| format!("reading `{}`", lock_path.display()))?;
    let package_ref = PackageRef::parse(requested)
        .with_context(|| format!("invalid package reference `{requested}`"))?;
    let package = resolve_locked_package(&lock, &package_ref, requested)?;
    let package_id = format!("{}/{}@{}", package.group, package.name, package.version);

    let slot = if package.materialization.is_in_place() {
        vibe_workspace::vibedeps::in_place_slot_abs_path(
            project_root,
            &package.group,
            package.name.as_ref(),
        )
    } else {
        vibe_workspace::vibedeps::slot_abs_path(
            project_root,
            &package.group,
            package.name.as_ref(),
            &package.version,
        )
    };
    let slot = canonical_existing_dir(&slot).with_context(|| {
        format!(
            "installed package `{package_id}` has no usable vibedeps slot; run `vibe reinstall`"
        )
    })?;

    let Some(name) = embedded_name else {
        let license = read_package_license(&slot)?;
        return Ok(SourcePathReport {
            ok: true,
            command: "show:source-path",
            package: package_id,
            root_kind: RootKind::Package,
            embedded: None,
            source_url: Some(package.source_url.as_str().to_owned()),
            source_ref: package.source_ref.clone(),
            resolved_commit: package.resolved_commit.clone(),
            tree_oid: None,
            content_hash: package.content_hash.to_string(),
            license,
            license_path: None,
            license_url: None,
            license_file_sha256: None,
            path: slot.display().to_string(),
        });
    };

    let locked = package
        .embedded_sources
        .iter()
        .find(|source| source.name == name)
        .ok_or_else(|| {
            let available = package
                .embedded_sources
                .iter()
                .map(|source| source.name.as_str())
                .collect::<Vec<_>>();
            anyhow::anyhow!(
                "installed package `{package_id}` has no embedded source `{name}`{}",
                if available.is_empty() {
                    String::new()
                } else {
                    format!("; available: {}", available.join(", "))
                }
            )
        })?;
    let cached = hydrate(locked, offline).with_context(|| {
        format!("opening embedded source `{name}` of installed package `{package_id}`")
    })?;
    if cached.resolved_commit != locked.resolved_commit
        || cached.tree_oid != locked.tree_oid
        || cached.content_hash != locked.content_hash
    {
        bail!(
            "verified cache result for embedded source `{name}` of `{package_id}` disagrees with vibe.lock"
        );
    }
    let tree = canonical_existing_dir(&cached.tree)?;

    Ok(SourcePathReport {
        ok: true,
        command: "show:source-path",
        package: package_id,
        root_kind: RootKind::Embedded,
        embedded: Some(locked.name.clone()),
        source_url: Some(locked.source_url.as_str().to_owned()),
        source_ref: locked.source_ref.clone(),
        resolved_commit: Some(locked.resolved_commit.clone()),
        tree_oid: Some(locked.tree_oid.clone()),
        content_hash: locked.content_hash.to_string(),
        license: Some(locked.upstream_license.clone()),
        license_path: Some(locked.license_path.to_string_lossy().replace('\\', "/")),
        license_url: Some(locked.license_url.clone()),
        license_file_sha256: Some(locked.license_file_sha256.to_string()),
        path: tree.display().to_string(),
    })
}

fn resolve_locked_package<'a>(
    lock: &'a Lockfile,
    requested: &PackageRef,
    raw: &str,
) -> Result<&'a LockedPackage> {
    let mut matches = lock
        .packages
        .iter()
        .filter(|package| package.name == requested.name)
        .filter(|package| requested.version.matches(&package.version))
        .filter(|package| {
            requested
                .group
                .as_ref()
                .is_none_or(|group| group == &package.group)
        })
        .filter(|package| requested.kind.is_none_or(|kind| kind == package.kind));
    let Some(first) = matches.next() else {
        bail!("package `{raw}` is not installed in this project's vibe.lock");
    };
    if let Some(second) = matches.next() {
        bail!(
            "package reference `{raw}` is ambiguous in this project's vibe.lock; use `{}/{}` or `{}/{}`",
            first.group,
            first.name,
            second.group,
            second.name
        );
    }
    Ok(first)
}

fn canonical_existing_dir(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        bail!(
            "source root `{}` does not exist or is not a directory",
            path.display()
        );
    }
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing source root `{}`", path.display()))?;
    Ok(crate::commands::init::strip_unc_public(canonical))
}

fn read_package_license(slot: &Path) -> Result<Option<String>> {
    let manifest_path = slot.join(Manifest::FILENAME);
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading installed manifest `{}`", manifest_path.display()))?;
    let manifest = Manifest::parse_str(&raw)
        .with_context(|| format!("parsing installed manifest `{}`", manifest_path.display()))?;
    Ok(manifest.package.and_then(|package| package.license))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use clap::Parser;
    use vibe_core::manifest::CURRENT_SCHEMA_VERSION;

    use super::*;

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";
    const HASH: &str =
        "sha256-tree/1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn project(lock_packages: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("vibe.toml"),
            "[project]\nname = \"fixture\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        fs::write(
            root.path().join("vibe.lock"),
            format!(
                "[meta]\ngenerated_by = \"test\"\ngenerated_at = \"2026-09-11T00:00:00Z\"\nschema_version = {CURRENT_SCHEMA_VERSION}\n\n{lock_packages}"
            ),
        )
        .unwrap();
        root
    }

    fn bridge_package(group: &str, embedded: bool) -> String {
        let mut text = format!(
            "[[package]]\nkind = \"tool\"\nname = \"bridge\"\ngroup = \"{group}\"\nversion = \"1.0.0\"\nbridge = true\nsource_url = \"https://github.com/vibespecs/bridge.git\"\nsource_ref = \"refs/tags/v1.0.0\"\nresolved_commit = \"{COMMIT}\"\ncontent_hash = \"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"\n"
        );
        if embedded {
            text.push_str(&format!(
                "\n[[package.embedded_source]]\nname = \"upstream\"\nsource_url = \"https://github.com/example/upstream.git\"\nsource_ref = \"refs/tags/v1.2.3\"\nresolved_commit = \"{COMMIT}\"\ntree_oid = \"{TREE}\"\ncontent_hash = \"{HASH}\"\nupstream_authors = [\"Example Upstream Authors\"]\nupstream_license = \"MIT\"\nlicense_path = \"LICENSE\"\nlicense_url = \"https://github.com/example/upstream/blob/{COMMIT}/LICENSE\"\nlicense_file_sha256 = \"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\"\n"
            ));
        }
        text
    }

    fn materialise_slot(root: &Path, group: &str) -> PathBuf {
        let group = vibe_core::Group::parse(group).unwrap();
        let version = semver::Version::parse("1.0.0").unwrap();
        let slot = vibe_workspace::vibedeps::slot_abs_path(root, &group, "bridge", &version);
        fs::create_dir_all(&slot).unwrap();
        fs::write(
            slot.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"{group}\"\nname = \"bridge\"\nkind = \"tool\"\nversion = \"1.0.0\"\nlicense = \"UPL-1.0\"\n"
            ),
        )
        .unwrap();
        slot
    }

    #[test]
    fn package_root_is_the_canonical_normal_vibedeps_slot() {
        let project = project(&bridge_package("org.example", false));
        let slot = materialise_slot(project.path(), "org.example");
        let report = resolve_source_path(
            project.path(),
            "org.example/bridge@=1.0.0",
            None,
            true,
            |_, _| unreachable!("package root must not hydrate an embedded source"),
        )
        .unwrap();

        assert_eq!(report.root_kind, RootKind::Package);
        assert_eq!(
            report.path,
            crate::commands::init::strip_unc_public(slot.canonicalize().unwrap())
                .display()
                .to_string()
        );
        assert_eq!(report.license.as_deref(), Some("UPL-1.0"));
    }

    #[test]
    fn embedded_root_uses_exact_lock_row_and_threads_offline() {
        let project = project(&bridge_package("org.example", true));
        materialise_slot(project.path(), "org.example");
        let cached = tempfile::tempdir().unwrap();
        let observed_offline = Cell::new(false);
        let report = resolve_source_path(
            project.path(),
            "tool:org.example/bridge",
            Some("upstream"),
            true,
            |locked, offline| {
                assert_eq!(locked.resolved_commit, COMMIT);
                observed_offline.set(offline);
                Ok(CachedEmbeddedSource {
                    tree: cached.path().to_path_buf(),
                    resolved_commit: COMMIT.into(),
                    tree_oid: TREE.into(),
                    content_hash: locked.content_hash.clone(),
                })
            },
        )
        .unwrap();

        assert!(observed_offline.get());
        assert_eq!(report.root_kind, RootKind::Embedded);
        assert_eq!(report.embedded.as_deref(), Some("upstream"));
        assert_eq!(report.license.as_deref(), Some("MIT"));
        assert_eq!(report.resolved_commit.as_deref(), Some(COMMIT));
        assert_eq!(report.content_hash, HASH);
    }

    #[test]
    fn short_package_name_must_be_unambiguous() {
        let packages = format!(
            "{}\n{}",
            bridge_package("org.one", false),
            bridge_package("org.two", false)
        );
        let project = project(&packages);
        materialise_slot(project.path(), "org.one");
        materialise_slot(project.path(), "org.two");

        let error =
            resolve_source_path(project.path(), "bridge", None, true, |_, _| unreachable!())
                .unwrap_err();
        assert!(error.to_string().contains("ambiguous"));
    }

    #[test]
    fn cli_accepts_source_path_with_embedded_and_project_path() {
        let cli = crate::cli::Cli::try_parse_from([
            "vibe",
            "--offline",
            "show",
            "source-path",
            "org.example/bridge@=1.0.0",
            "--embedded",
            "upstream",
            "--path",
            ".",
        ])
        .unwrap();
        assert!(cli.offline);
        let crate::cli::Command::Show(show) = cli.command else {
            panic!("expected show command");
        };
        let crate::cli::ShowSubcommand::SourcePath(args) = show.command else {
            panic!("expected source-path command");
        };
        assert_eq!(args.package, "org.example/bridge@=1.0.0");
        assert_eq!(args.embedded.as_deref(), Some("upstream"));
    }
}
