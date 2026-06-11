//! `vibe registry vendor` — generate a local `file://` mirror directory
//! from the per-package cache clones.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_registry::MultiRegistryResolver;

use crate::cli::RegistryVendorArgs;
use crate::output;

use super::{SkippedReportEntry, resolve_project_root};

// ===================== vendor =====================
//
// `vibe registry vendor [--out <dir>] [--force]` — generates a local
// directory that vibe can later use as `[[mirror]] url =
// "file:///abs/path"` for offline / air-gapped installs. Each
// `[[registry]]`-served lockfile entry produces a bare git repo
// `<out>/<naming.repo_name(kind,name)>.git/` populated from the
// matching per-package cache clone.
//
// Spec: PROP-002 §2.3 (mirror layer), §6 (Phase B preview).

#[derive(Debug, Serialize)]
struct VendorReport {
    ok: bool,
    command: &'static str,
    out_dir: String,
    /// Suggested `[[mirror]]` snippet the operator can paste into
    /// `vibe.toml`. The URL is `file://` + the absolute, forward-slash
    /// form of `out_dir`.
    suggested_mirror_url: String,
    vendored: Vec<VendoredReportEntry>,
    skipped: Vec<SkippedReportEntry>,
}

#[derive(Debug, Serialize)]
struct VendoredReportEntry {
    group: String,
    name: String,
    /// Registry that originally served this package — what `vibe.lock`
    /// records under `registry`.
    registry: String,
    repo_dir: String,
    /// What `vibe.lock` records under `source_ref` — typically
    /// `v<version>`. Vendored repo carries this tag.
    #[serde(rename = "ref")]
    refname: String,
}

pub(super) fn run_vendor(ctx: &output::Context, args: RegistryVendorArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let lockfile_path = project_root.join(Lockfile::FILENAME);
    if !lockfile_path.exists() {
        bail!(
            "no `vibe.lock` in `{}`. Run `vibe install` first — vendoring is driven by the lockfile, not the manifest.",
            project_root.display()
        );
    }
    let lockfile = Lockfile::read(&lockfile_path)
        .with_context(|| format!("reading `{}`", lockfile_path.display()))?;

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` entries in `{}`. Vendor only mirrors registry-served packages; \
             projects using only `--registry <path>` or `[[override]]` have nothing to vendor.",
            manifest_path.display()
        );
    }

    let out_dir = args
        .out
        .as_ref()
        .map(|p| project_root.join(p))
        .unwrap_or_else(|| project_root.join("vendor"));

    // Safety: never silently overwrite operator content. `--force`
    // wipes; without it, a non-empty target dir is a hard error.
    if out_dir.exists() {
        let mut iter = std::fs::read_dir(&out_dir)
            .with_context(|| format!("reading `{}`", out_dir.display()))?;
        let non_empty = iter.next().is_some();
        if non_empty && !args.force {
            bail!(
                "`{}` exists and is not empty. Pass `--force` to wipe and re-vendor, \
                 or pick a different `--out`.",
                out_dir.display()
            );
        }
        if args.force {
            std::fs::remove_dir_all(&out_dir)
                .with_context(|| format!("wiping `{}`", out_dir.display()))?;
        }
    }
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating `{}`", out_dir.display()))?;

    let mrr =
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
            .context("opening multi-registry resolver")?;

    ctx.heading(&format!(
        "Vendoring {} lockfile entr{} into `{}`",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 {
            "y"
        } else {
            "ies"
        },
        out_dir.display()
    ));

    let mut vendored: Vec<VendoredReportEntry> = Vec::new();
    let mut skipped: Vec<SkippedReportEntry> = Vec::new();

    for entry in &lockfile.packages {
        if entry.overridden {
            skipped.push(SkippedReportEntry {
                group: entry.group.as_str().to_string(),
                name: entry.name.clone(),
                reason: format!(
                    "[[override]]-served (source_url `{}`); vendor it manually if you need offline coverage",
                    entry.source_url
                ),
            });
            continue;
        }
        let Some(reg_name) = entry.registry.as_deref() else {
            skipped.push(SkippedReportEntry {
                group: entry.group.as_str().to_string(),
                name: entry.name.clone(),
                reason: "lockfile entry has no `registry` (likely installed via `--registry <path>` or a legacy v1 path)"
                    .to_string(),
            });
            continue;
        };
        let Some(reg) = mrr.registries().iter().find(|r| r.name() == reg_name) else {
            skipped.push(SkippedReportEntry {
                group: entry.group.as_str().to_string(),
                name: entry.name.clone(),
                reason: format!(
                    "lockfile names registry `{reg_name}` but no `[[registry]]` with that name exists in `vibe.toml`"
                ),
            });
            continue;
        };

        let refname = entry
            .source_ref
            .clone()
            .unwrap_or_else(|| format!("v{}", entry.version));

        // Make sure the per-package clone is on disk and at the
        // requested ref. `refresh_package` is mirror-aware, so a fresh
        // `vibe registry vendor` against an unreachable primary still
        // works as long as some `[[mirror]]` URL is reachable.
        reg.refresh_package(&entry.group, &entry.name, &refname)
            .with_context(|| {
                format!(
                    "refreshing per-package clone for `{}/{}` against `{}`",
                    entry.group, entry.name, refname
                )
            })?;

        let clone_dir = reg.package_clone_dir(&entry.group, &entry.name);
        let clone_git = clone_dir.join(".git");
        if !clone_git.is_dir() {
            // Should not happen after a successful `refresh_package`,
            // but guard anyway — `bare_clone_from_clone` reads
            // `.git/` and an explicit error here beats a confusing
            // I/O error two layers down.
            bail!(
                "per-package clone for `{}/{}` lacks a `.git/` after refresh — registry returned without populating the cache (`{}`)",
                entry.group,
                entry.name,
                clone_dir.display()
            );
        }

        let repo_name = reg
            .naming()
            .repo_name(Some(entry.kind), &entry.group, &entry.name)
            .with_context(|| {
                format!(
                    "deriving the vendor repo name for `{}/{}`",
                    entry.group, entry.name
                )
            })?;
        let vendor_repo = out_dir.join(format!("{repo_name}.git"));
        if vendor_repo.exists() {
            std::fs::remove_dir_all(&vendor_repo)
                .with_context(|| format!("wiping stale vendor repo `{}`", vendor_repo.display()))?;
        }
        if let Some(parent) = vendor_repo.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating parent dir `{}`", parent.display()))?;
        }

        bare_clone_from_clone(&clone_git, &vendor_repo).with_context(|| {
            format!(
                "vendoring `{}/{}` into `{}`",
                entry.group,
                entry.name,
                vendor_repo.display()
            )
        })?;

        ctx.step(&format!(
            "{}/{} @ {} → {}",
            entry.group,
            entry.name,
            refname,
            forward_slash_display(&vendor_repo)
        ));
        vendored.push(VendoredReportEntry {
            group: entry.group.as_str().to_string(),
            name: entry.name.clone(),
            registry: reg_name.to_string(),
            repo_dir: forward_slash_display(&vendor_repo),
            refname,
        });
    }

    let suggested_url = file_url_for_dir(&out_dir);
    write_vendor_readme(&out_dir, &suggested_url, &vendored).context("writing vendor README.md")?;

    if !skipped.is_empty() {
        for s in &skipped {
            ctx.skipped(&format!("{}/{}", s.group, s.name), &s.reason);
        }
    }

    if ctx.is_json() {
        ctx.emit_json(&VendorReport {
            ok: true,
            command: "registry:vendor",
            out_dir: forward_slash_display(&out_dir),
            suggested_mirror_url: suggested_url.clone(),
            vendored,
            skipped,
        })?;
        return Ok(());
    }

    ctx.summary(&format!(
        "\nvibe registry vendor: {} vendored, {} skipped. \
         Wire as `[[mirror]] of = \"<registry>\" url = \"{}\"` to enable offline fallback.",
        vendored.len(),
        skipped.len(),
        suggested_url
    ));
    Ok(())
}

/// Produce a `file://` URL for an absolute directory path, forward-slashed
/// so the URL is well-formed on Windows (`file:///C:/Users/...`) and Unix
/// (`file:///path/...`).
fn file_url_for_dir(dir: &Path) -> String {
    let mut s = dir.to_string_lossy().replace('\\', "/");
    // Strip Windows UNC `\\?\` prefix that may survive `canonicalize`.
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    format!("file://{s}")
}

fn forward_slash_display(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    s
}

/// Build a bare repo at `dst` from the contents of a non-bare clone's
/// `.git/` at `src_git`. Implementation: copy every file under `src_git/`
/// recursively into `dst/`, preserving relative paths. The result is a
/// directory whose layout (`HEAD`, `refs/`, `objects/`, …) is what `git
/// clone <dst>` and `git ls-remote <dst>` consume — git auto-detects
/// bare-ness from the layout, the `core.bare` config flag is informational
/// from the consumer's side.
///
/// We deliberately do NOT shell out to `git clone --bare` because (a) it
/// would couple `vibe registry vendor` to git availability at vendor-time,
/// not just install-time, and (b) the copy is straightforward and easier
/// to test without spawning subprocesses. Hard-links would be faster but
/// would tie the vendor dir's lifetime to the source clone's filesystem;
/// a plain `fs::copy` produces a self-contained vendor that survives a
/// `~/.vibe/registries` wipe.
fn bare_clone_from_clone(src_git: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("creating `{}`", dst.display()))?;
    for entry in walkdir::WalkDir::new(src_git)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let rel = entry.path().strip_prefix(src_git).unwrap_or(entry.path());
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("creating `{}`", target.display()))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating `{}`", parent.display()))?;
            }
            std::fs::copy(entry.path(), &target)
                .with_context(|| format!("copying to `{}`", target.display()))?;
        }
    }
    Ok(())
}

/// Generate a small `README.md` at the root of the vendor directory
/// explaining what it is and how to wire it as `[[mirror]]`. Idempotent:
/// any prior README is overwritten as part of `--force` / first vendor.
fn write_vendor_readme(
    out_dir: &Path,
    suggested_url: &str,
    vendored: &[VendoredReportEntry],
) -> Result<()> {
    let mut body = String::new();
    body.push_str("# vibe vendor\n\n");
    body.push_str(
        "Local mirror directory generated by `vibe registry vendor`. Each entry \
        below is a bare git repository populated from the per-package cache clone \
        for the package referenced by `vibe.lock`.\n\n\
        Wire it into your `vibe.toml` as a `[[mirror]]` for offline / air-gapped \
        installs:\n\n",
    );
    body.push_str("```toml\n");
    body.push_str("[[mirror]]\n");
    body.push_str("of = \"<registry-name>\"  # or \"*\" to mirror every registry\n");
    body.push_str(&format!("url = \"{suggested_url}\"\n"));
    body.push_str("priority = 0\n");
    body.push_str("```\n\n");
    body.push_str(
        "When the primary registry is reachable, `vibe install` walks it first per \
        PROP-002 §2.3; the file:// mirror takes over only if the primary is \
        unavailable, which is the offline / air-gapped path.\n\n",
    );
    if vendored.is_empty() {
        body.push_str("_(No registry-served lockfile entries were vendored on this run.)_\n");
    } else {
        body.push_str("## Contents\n\n");
        for v in vendored {
            body.push_str(&format!(
                "- `{}/{}` @ `{}` — `{}` (from registry `{}`)\n",
                v.group, v.name, v.refname, v.repo_dir, v.registry
            ));
        }
    }
    let readme_path = out_dir.join("README.md");
    std::fs::write(&readme_path, body)
        .with_context(|| format!("writing `{}`", readme_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{bare_clone_from_clone, file_url_for_dir};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn bare_clone_copies_git_tree_recursively() {
        // Synthesize a minimal `.git/`-shape tree and verify
        // `bare_clone_from_clone` reproduces the layout at the target.
        // No real git; just files + directories in the shape git would
        // produce.
        let src = tempdir().unwrap();
        let src_git = src.path().join(".git");
        fs::create_dir_all(src_git.join("refs/heads")).unwrap();
        fs::create_dir_all(src_git.join("refs/tags")).unwrap();
        fs::create_dir_all(src_git.join("objects/pack")).unwrap();
        fs::write(src_git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(
            src_git.join("config"),
            "[core]\n\trepositoryformatversion = 0\n",
        )
        .unwrap();
        fs::write(src_git.join("refs/heads/main"), "abc123\n").unwrap();
        fs::write(src_git.join("refs/tags/v0.1.0"), "def456\n").unwrap();
        fs::write(src_git.join("objects/pack/pack-x.idx"), b"binary").unwrap();

        let dst_root = tempdir().unwrap();
        let dst = dst_root.path().join("flow-wal.git");
        bare_clone_from_clone(&src_git, &dst).unwrap();

        // Every file the helper saw is present at the same relative
        // path under `dst`.
        assert_eq!(
            fs::read_to_string(dst.join("HEAD")).unwrap(),
            "ref: refs/heads/main\n"
        );
        assert_eq!(
            fs::read_to_string(dst.join("refs/heads/main")).unwrap(),
            "abc123\n"
        );
        assert_eq!(
            fs::read_to_string(dst.join("refs/tags/v0.1.0")).unwrap(),
            "def456\n"
        );
        assert_eq!(
            fs::read(dst.join("objects/pack/pack-x.idx")).unwrap(),
            b"binary".to_vec()
        );
        // Empty directories survive the copy too — `objects/` is
        // implicitly preserved by walking, even when only `pack/`
        // contains files.
        assert!(dst.join("objects/pack").is_dir());
    }

    #[test]
    fn bare_clone_creates_dst_when_absent() {
        let src = tempdir().unwrap();
        let src_git = src.path().join(".git");
        fs::create_dir_all(&src_git).unwrap();
        fs::write(src_git.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let dst_root = tempdir().unwrap();
        let dst = dst_root.path().join("nested/flow-wal.git");
        // Caller (run_vendor) guarantees parent exists; the helper
        // creates the leaf. Pre-create the parent here to mirror that
        // contract.
        fs::create_dir_all(dst.parent().unwrap()).unwrap();
        bare_clone_from_clone(&src_git, &dst).unwrap();
        assert!(dst.join("HEAD").is_file());
    }

    #[test]
    fn file_url_for_dir_unix_absolute() {
        let url = file_url_for_dir(&PathBuf::from("/abs/path/to/vendor"));
        assert_eq!(url, "file:///abs/path/to/vendor");
    }

    #[test]
    fn file_url_for_dir_windows_drive_letter() {
        // PathBuf::from on a non-Windows platform won't drive-letter-
        // canonicalize, but the helper just transforms the string —
        // platform-independent.
        let url = file_url_for_dir(&PathBuf::from(r"C:\Users\foo\vendor"));
        assert_eq!(url, "file:///C:/Users/foo/vendor");
    }

    #[test]
    fn file_url_for_dir_strips_unc_prefix() {
        // `canonicalize` on Windows returns paths with the `\\?\`
        // prefix; the helper drops that so the URL is portable.
        let url = file_url_for_dir(&PathBuf::from(r"\\?\C:\Users\foo\vendor"));
        assert_eq!(url, "file:///C:/Users/foo/vendor");
    }
}
