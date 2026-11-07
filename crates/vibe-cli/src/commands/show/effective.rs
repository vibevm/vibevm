//! `vibe show effective` — concatenate the boot files, the WAL, and
//! every installed package's written files, each with a `spec://`
//! provenance header (`VIBEVM-SPEC.md` §4.6).
//!
//! Declared authored sources use their captured media and selected native
//! interpretation. Named ordinary UTF-8 remains literal display data. Legacy
//! sources retain suffix projection; generated STATIC and INDEX are verbatim
//! artifacts, including tapes with multiple XML roots.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use vibe_core::machine_json_path;
use vibe_workspace::boot_artifacts;

use crate::cli::ShowEffectiveArgs;
use crate::output;

use super::resolve_project_root;
mod read;
use read::{Body, Reads};

// ===================== show effective =====================

#[derive(Debug, Serialize)]
struct EffectiveReport {
    ok: bool,
    command: &'static str,
    project: String,
    sections: Vec<EffectiveSection>,
}

#[derive(Debug, Serialize)]
struct EffectiveSection {
    /// `spec://` URI for this section. Composed from the originating
    /// package's `(kind, name)` plus the project-relative path.
    /// User-owned files (the boot foundation, WAL) get
    /// `spec://project/...`.
    spec_uri: String,
    /// Project-relative path of the file that produced this section.
    path: String,
    /// Origin of the section: `"package:<group>/<name>@<version>"`,
    /// `"user"`, or `"wal"`.
    origin: String,
    /// File content, verbatim.
    body: String,
}

/// Collect every effective section — the boot dir, the WAL, and the
/// installed packages' written files — without rendering anything. The
/// qualified legacy wrapper intentionally supplies no installed format.
#[cfg(test)]
fn collect_sections(project_root: &Path) -> Result<Vec<EffectiveSection>> {
    collect_bound_sections(project_root, None)
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-SHOW-READ",
    r = 1
)]
fn collect_bound_sections(
    project_root: &Path,
    format: Option<vibe_core::manifest::SpecFormat>,
) -> Result<Vec<EffectiveSection>> {
    let mut reads = Reads::new(project_root, format)?;
    let packages = reads.packages.clone();

    let mut sections: Vec<EffectiveSection> = Vec::new();
    // Layout-relative string prefixes, computed once: everything this
    // collector emits is relative to the live layout's specs root
    // (PROP-052 L2 — the names come from the layout module).
    let boot_dir = project_root.join(vibe_core::layout::current_boot_dir());
    let boot_prefix = format!(
        "{}/",
        machine_json_path(&vibe_core::layout::current_boot_dir())
    );
    let specs_prefix = format!(
        "{}/",
        machine_json_path(&vibe_core::layout::current_specs_root())
    );

    // 1. Boot dir — sorted by NN- prefix. Each file gets a
    // user-or-package origin: the lockfile's `boot_snippet` field
    // names which package contributed which `NN-…` file. Files not
    // claimed by any lockfile entry (00-core / 90-user / hand-edited)
    // surface as `user`. Both spec serialisations load (PROP-045
    // ##LOADER-LAW) — an `.xml` boot file renders as its projection.
    let own_boot = reads.own_boot()?;
    if boot_dir.is_dir() || own_boot.as_ref().is_some_and(|entries| !entries.is_empty()) {
        boot_artifacts::resolve_static_path(project_root)
            .context("resolving the generated STATIC tape")?;
        let mut entries: Vec<PathBuf> = fs::read_dir(&boot_dir)
            .with_context(|| format!("reading `{}`", boot_dir.display()))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.path())
            .filter(|p| {
                let relative = p.strip_prefix(project_root).unwrap_or(p);
                vibe_specdoc::is_spec_source(p)
                    && (vibe_core::layout::is_generated_boot_output(relative) || own_boot.is_none())
            })
            .collect();
        if let Some(own) = &own_boot {
            entries.extend(own.iter().map(|(relative, _)| project_root.join(relative)));
        }
        entries.sort();
        for path in entries {
            // Every entry came from `read_dir` filtered to files, so it
            // always has a final component; skip the unreachable `..`-shaped
            // path rather than unwrap.
            let Some(filename) = path.file_name() else {
                continue;
            };
            let filename = filename.to_string_lossy().into_owned();
            let rel = format!("{boot_prefix}{filename}");
            let origin = boot_origin(&filename, &packages);
            let spec_uri = format!("spec://project/boot/{filename}");
            let body = if let Some((_, body)) = own_boot
                .as_ref()
                .and_then(|entries| entries.iter().find(|(relative, _)| relative == &rel))
            {
                body.clone()
            } else {
                match reads.read(Path::new(&rel))? {
                    Body::Text(body) => body,
                    Body::Missing | Body::Skipped => continue,
                }
            };
            sections.push(EffectiveSection {
                spec_uri,
                path: rel,
                origin,
                body,
            });
        }
    }

    // 2. WAL — always one section, distinct origin.
    let wal = project_root.join(vibe_core::layout::current_wal_md());
    if wal.is_file()
        && let Body::Text(body) = reads.read(&vibe_core::layout::current_wal_md())?
    {
        sections.push(EffectiveSection {
            spec_uri: "spec://project/WAL".to_string(),
            path: machine_json_path(&vibe_core::layout::current_wal_md()),
            origin: "wal".to_string(),
            body,
        });
    }

    // 3. Per package, in lockfile order: every file in `files_written`
    // that we haven't already emitted (skip the boot snippet — it
    // landed in step 1). Lockfile order is the install order, which
    // is the same order the resolver pinned the graph in. Stable
    // enough for cold-reader use.
    for entry in &packages {
        let pkg_uri_root = format!("spec://{}/{}/{}", entry.group, entry.name, entry.version);
        let mut paths: Vec<PathBuf> = entry
            .files_written
            .iter()
            .map(|p| normalize_rel_path(p))
            .collect();
        paths.sort();
        for rel in paths {
            let rel_str = machine_json_path(&rel);
            if rel_str.starts_with(&boot_prefix) {
                // Already emitted under step 1.
                continue;
            }
            let body = reads.read(&rel)?;
            if matches!(body, Body::Skipped) {
                continue;
            }
            if matches!(body, Body::Missing) {
                // Missing file — surface as a section with empty
                // body and a warning header instead of crashing.
                // `vibe check` exists for the dedicated linter
                // path; `vibe show effective` is best-effort by
                // design.
                sections.push(EffectiveSection {
                    spec_uri: format!(
                        "{}/{}",
                        pkg_uri_root,
                        rel_str.trim_start_matches(&specs_prefix)
                    ),
                    path: rel_str.clone(),
                    origin: format!(
                        "package:{}/{}@{} (MISSING ON DISK)",
                        entry.group, entry.name, entry.version
                    ),
                    body: String::new(),
                });
                continue;
            }
            let Body::Text(body) = body else {
                unreachable!("handled missing and skipped display inputs");
            };
            let suffix = rel_str.trim_start_matches(&specs_prefix);
            sections.push(EffectiveSection {
                spec_uri: format!("{pkg_uri_root}/{suffix}"),
                path: rel_str,
                origin: format!("package:{}/{}@{}", entry.group, entry.name, entry.version),
                body,
            });
        }
    }
    reads.recheck()?;
    Ok(sections)
}

pub(super) fn run_effective(ctx: &output::Context, args: ShowEffectiveArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest = vibe_core::manifest::Manifest::read(
        project_root.join(vibe_core::manifest::Manifest::FILENAME),
    )?;
    let format = crate::commands::reinstall::effective_rederive_format(&manifest)?;
    let sections = collect_bound_sections(&project_root, Some(format))?;

    if ctx.is_json() {
        let payload = EffectiveReport {
            ok: true,
            command: "show:effective",
            project: project_root.display().to_string(),
            sections,
        };
        ctx.emit_json(&payload)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show effective: {} section{} from `{}`",
            sections.len(),
            if sections.len() == 1 { "" } else { "s" },
            project_root.display()
        ));
        return Ok(());
    }
    if sections.is_empty() {
        ctx.summary(&format!(
            "vibe show effective: nothing to materialise — `{}` has no {} files, no WAL, and an empty lockfile",
            project_root.display(),
            machine_json_path(&vibe_core::layout::current_boot_dir())
        ));
        return Ok(());
    }
    for section in &sections {
        println!("--- {} ({})", section.spec_uri, section.origin);
        println!("--- path: {}", section.path);
        println!();
        // Trim trailing newline so we don't double up before the next
        // separator. The original file's content is preserved
        // verbatim modulo that trailing trim.
        if section.body.ends_with('\n') {
            print!("{}", section.body);
        } else {
            println!("{}", section.body);
        }
        println!();
    }
    ctx.summary(&format!(
        "vibe show effective: {} sections, project `{}`",
        sections.len(),
        project_root.display()
    ));
    Ok(())
}

fn boot_origin(filename: &str, packages: &[vibe_core::manifest::LockedPackage]) -> String {
    let logical_name = logical_spec_name(Path::new(filename));
    if matches!(logical_name, Some("00-core" | "90-user")) {
        return "user".to_string();
    }
    if let Some(pkg) = packages.iter().find(|p| {
        p.boot_snippet
            .as_deref()
            .and_then(|snippet| logical_spec_name(Path::new(snippet)))
            == logical_name
    }) {
        return format!("package:{}/{}@{}", pkg.group, pkg.name, pkg.version);
    }
    "user".to_string()
}

/// A boot contribution keeps its identity when materialisation changes only
/// its spec representation (`10-flow-wal.md` -> `10-flow-wal.xml`).
fn logical_spec_name(path: &Path) -> Option<&str> {
    path.file_stem().and_then(|stem| stem.to_str())
}

fn normalize_rel_path(p: &Path) -> PathBuf {
    PathBuf::from(machine_json_path(p))
}

fn authored_guards(
    root: &Path,
    path: &Path,
) -> Result<Option<Vec<vibe_spec::source_policy::PackagePolicyCheckpoint>>> {
    use vibe_spec::source_policy::{PackageSourceMode, observe_package_policy};
    let mut guards = Vec::new();
    for parent in path
        .parent()
        .into_iter()
        .flat_map(Path::ancestors)
        .take_while(|parent| parent.starts_with(root))
    {
        // A legacy written-file diagnostic may name a parent that is absent.
        // Inspect its nearest existing owner instead of losing that diagnostic.
        match std::fs::symlink_metadata(parent) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("observing source parent `{}`", parent.display()));
            }
            Ok(_) => {}
        }
        let policy = observe_package_policy(parent)?;
        match policy.mode() {
            PackageSourceMode::ExplicitEmpty => {
                policy.recheck_unchanged()?;
                return Ok(None);
            }
            PackageSourceMode::Explicit => {
                return Err(policy
                    .unsupported("show effective authored-source projection")
                    .into());
            }
            PackageSourceMode::Legacy => {
                guards.push(policy.into_checkpoint());
                break;
            }
            PackageSourceMode::MissingManifest => guards.push(policy.into_checkpoint()),
        }
    }
    Ok(Some(guards))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_static_tape_is_read_verbatim_in_both_formats() {
        for format in [
            vibe_core::manifest::SpecFormat::Markdown,
            vibe_core::manifest::SpecFormat::Xml,
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            std::fs::create_dir_all(root.join(vibe_core::layout::current_boot_dir())).unwrap();
            std::fs::write(
                root.join("vibe.toml"),
                "[project]\nname = \"p\"\nversion='1.0.0'\n",
            )
            .unwrap();
            let tape = "<!-- vibe:static one -->\n<spec xmlns=\"https://vibevm.org/spec/1\"><p>one</p></spec>\n\n\
                        <!-- vibe:static two -->\n<spec xmlns=\"https://vibevm.org/spec/1\"><p>two</p></spec>\n";
            let rel = boot_artifacts::static_path(format);
            std::fs::write(root.join(rel), tape).unwrap();

            let sections = collect_sections(root).expect("collect the STATIC tape");
            let static_tape = sections
                .iter()
                .find(|section| section.path == rel)
                .expect("the generated tape is a section");
            assert_eq!(static_tape.body, tape);
        }
    }

    /// PROP-045 ##LOADER-LAW: an `.xml` boot file is a first-class boot
    /// section, and its body is the canonical MD projection — the
    /// effective view renders one form regardless of how each document
    /// materialised.
    #[test]
    fn an_xml_boot_file_lands_as_its_projection() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(vibe_core::layout::current_boot_dir())).unwrap();
        std::fs::write(
            root.join("vibe.toml"),
            "[project]\nname = \"p\"\nversion='1.0.0'\n",
        )
        .unwrap();
        let rules = vibe_core::layout::current_boot_dir().join("rules.xml");
        let rules_rel = machine_json_path(&rules);
        std::fs::write(
            root.join(&rules),
            "<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
             <p><fact id=\"ONLY\" status=\"impl/done\">one rule</fact></p>\n</spec>\n",
        )
        .unwrap();
        let sections = collect_sections(root).expect("collect");
        let boot = sections
            .iter()
            .find(|s| s.path == rules_rel)
            .expect("the xml boot file is a section");
        assert_eq!(boot.origin, "user");
        assert!(boot.body.contains("@fact:ONLY"), "{}", boot.body);
        assert!(
            !boot.body.contains("<spec"),
            "the body is the projection, not raw XML: {}",
            boot.body
        );
    }
}
