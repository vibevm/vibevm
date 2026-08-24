//! `vibe show effective` — concatenate the boot files, the WAL, and
//! every installed package's written files, each with a `spec://`
//! provenance header (`VIBEVM-SPEC.md` §4.6).
//!
//! Authored spec sources enter through the PROP-045 projection dispatch
//! (`load_spec_text`): a `.xml` boot file or written file is shown as its
//! canonical Markdown projection. Generated STATIC is different: it is a
//! provenance-delimited tape of contributions, not one dialect document, so
//! the effective view reads either generated spelling verbatim. The WAL stays
//! host Markdown and is also read raw.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use vibe_core::machine_json_path;
use vibe_core::manifest::Lockfile;
use vibe_workspace::boot_artifacts;

use crate::cli::ShowEffectiveArgs;
use crate::output;

use super::resolve_project_root;

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
/// pure half of [`run_effective`], split so the projection law (every
/// spec source through `load_spec_text`) is testable on the collected
/// bodies alone.
fn collect_sections(project_root: &Path) -> Result<Vec<EffectiveSection>> {
    let lockfile_path = project_root.join(Lockfile::FILENAME);
    let lockfile = if lockfile_path.exists() {
        Some(
            Lockfile::read(&lockfile_path)
                .with_context(|| format!("reading `{}`", lockfile_path.display()))?,
        )
    } else {
        None
    };

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
    if boot_dir.is_dir() {
        boot_artifacts::resolve_static_path(project_root)
            .context("resolving the generated STATIC tape")?;
        let mut entries: Vec<PathBuf> = fs::read_dir(&boot_dir)
            .with_context(|| format!("reading `{}`", boot_dir.display()))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.path())
            .filter(|p| vibe_specdoc::is_spec_source(p))
            .collect();
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
            let origin = boot_origin(&filename, lockfile.as_ref());
            let spec_uri = format!("spec://project/boot/{filename}");
            let body = load_boot_entry(&path)?;
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
    if wal.is_file() {
        let body =
            fs::read_to_string(&wal).with_context(|| format!("reading `{}`", wal.display()))?;
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
    if let Some(lockfile) = &lockfile {
        for entry in &lockfile.packages {
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
                let abs = project_root.join(&rel);
                if !abs.is_file() {
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
                let body = load_spec(&abs)?;
                let suffix = rel_str.trim_start_matches(&specs_prefix);
                sections.push(EffectiveSection {
                    spec_uri: format!("{pkg_uri_root}/{suffix}"),
                    path: rel_str,
                    origin: format!("package:{}/{}@{}", entry.group, entry.name, entry.version),
                    body,
                });
            }
        }
    }

    Ok(sections)
}

pub(super) fn run_effective(ctx: &output::Context, args: ShowEffectiveArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let sections = collect_sections(&project_root)?;

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

/// Load one boot entry. Generated STATIC is a tape, so feeding its complete
/// contents to a single-document parser would merge provenance boundaries or
/// reject a valid multi-document XML lane. Show needs only its text and keeps
/// it verbatim.
fn load_boot_entry(path: &Path) -> Result<String> {
    let is_static_tape = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name == boot_artifacts::STATIC_FILE || name == boot_artifacts::STATIC_XML_FILE
        });
    if is_static_tape {
        return fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()));
    }
    load_spec(path)
}

/// One authored spec source as the effective view shows it — the PROP-045
/// dispatch (`load_spec_text`): `.md` verbatim, `.xml` as its canonical
/// projection. The source kind is deliberately dropped here: a provenance
/// concatenation has no line-citing diagnostics to mark.
fn load_spec(path: &Path) -> Result<String> {
    vibe_specdoc::load_spec_text(path)
        .map_err(|e| anyhow::Error::msg(e.to_string()))
        .with_context(|| format!("reading `{}`", path.display()))
        .map(|(body, _)| body)
}

fn boot_origin(filename: &str, lockfile: Option<&Lockfile>) -> String {
    let logical_name = logical_spec_name(Path::new(filename));
    if matches!(logical_name, Some("00-core" | "90-user")) {
        return "user".to_string();
    }
    let Some(lockfile) = lockfile else {
        return "user".to_string();
    };
    if let Some(pkg) = lockfile.packages.iter().find(|p| {
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
            std::fs::write(root.join("vibe.toml"), "[project]\nname = \"p\"\n").unwrap();
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
        std::fs::write(root.join("vibe.toml"), "[project]\nname = \"p\"\n").unwrap();
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
