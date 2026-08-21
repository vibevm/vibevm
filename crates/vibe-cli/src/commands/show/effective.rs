//! `vibe show effective` — concatenate the boot files, the WAL, and
//! every installed package's written files, each with a `spec://`
//! provenance header (`VIBEVM-SPEC.md` §4.6).
//!
//! Every spec source enters through the PROP-045 projection dispatch
//! (`load_spec_text`): a `.xml` boot file or written file is shown as its
//! canonical Markdown projection — one effective view, whatever form each
//! document materialised in. The WAL stays host Markdown, read raw.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use vibe_core::machine_json_path;
use vibe_core::manifest::Lockfile;

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

    // 1. Boot dir — sorted by NN- prefix. Each file gets a
    // user-or-package origin: the lockfile's `boot_snippet` field
    // names which package contributed which `NN-…` file. Files not
    // claimed by any lockfile entry (00-core / 90-user / hand-edited)
    // surface as `user`. Both spec serialisations load (PROP-045
    // ##LOADER-LAW) — an `.xml` boot file renders as its projection.
    let boot_dir = project_root.join("spec/boot");
    if boot_dir.is_dir() {
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
            let rel = format!("spec/boot/{filename}");
            let origin = boot_origin(&filename, lockfile.as_ref());
            let spec_uri = format!("spec://project/boot/{filename}");
            let (body, _) = load_spec(&path)?;
            sections.push(EffectiveSection {
                spec_uri,
                path: rel,
                origin,
                body,
            });
        }
    }

    // 2. WAL — always one section, distinct origin.
    let wal = project_root.join("spec/WAL.md");
    if wal.is_file() {
        let body =
            fs::read_to_string(&wal).with_context(|| format!("reading `{}`", wal.display()))?;
        sections.push(EffectiveSection {
            spec_uri: "spec://project/WAL".to_string(),
            path: "spec/WAL.md".to_string(),
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
                if rel_str.starts_with("spec/boot/") {
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
                            rel_str.trim_start_matches("spec/")
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
                let (body, _) = load_spec(&abs)?;
                let suffix = rel_str.trim_start_matches("spec/");
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
            "vibe show effective: nothing to materialise — `{}` has no spec/boot files, no WAL, and an empty lockfile",
            project_root.display()
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

/// One spec source as the effective view shows it — the PROP-045 dispatch
/// (`load_spec_text`): `.md` verbatim, `.xml` as its canonical projection.
/// The kind is deliberately dropped here: a provenance concatenation has
/// no line-citing diagnostics to mark.
fn load_spec(path: &Path) -> Result<(String, vibe_specdoc::SourceKind)> {
    vibe_specdoc::load_spec_text(path)
        .map_err(|e| anyhow::Error::msg(e.to_string()))
        .with_context(|| format!("reading `{}`", path.display()))
}

fn boot_origin(filename: &str, lockfile: Option<&Lockfile>) -> String {
    if filename == "00-core.md" || filename == "90-user.md" {
        return "user".to_string();
    }
    let Some(lockfile) = lockfile else {
        return "user".to_string();
    };
    if let Some(pkg) = lockfile
        .packages
        .iter()
        .find(|p| p.boot_snippet.as_deref() == Some(filename))
    {
        return format!("package:{}/{}@{}", pkg.group, pkg.name, pkg.version);
    }
    "user".to_string()
}

fn normalize_rel_path(p: &Path) -> PathBuf {
    PathBuf::from(machine_json_path(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PROP-045 ##LOADER-LAW: an `.xml` boot file is a first-class boot
    /// section, and its body is the canonical MD projection — the
    /// effective view renders one form regardless of how each document
    /// materialised.
    #[test]
    fn an_xml_boot_file_lands_as_its_projection() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("spec/boot")).unwrap();
        std::fs::write(root.join("vibe.toml"), "[project]\nname = \"p\"\n").unwrap();
        std::fs::write(
            root.join("spec/boot/rules.xml"),
            "<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
             <p><fact id=\"ONLY\" status=\"impl/done\">one rule</fact></p>\n</spec>\n",
        )
        .unwrap();
        let sections = collect_sections(root).expect("collect");
        let boot = sections
            .iter()
            .find(|s| s.path == "spec/boot/rules.xml")
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
