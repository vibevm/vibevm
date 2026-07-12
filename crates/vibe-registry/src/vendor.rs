//! `vibe registry vendor`'s domain — generate a local `file://` mirror
//! directory from the per-package cache clones (CONVERT-PLAN v0.1 §4.2).
//!
//! The CLI (`vibe registry vendor`) keeps argument parsing, the `--force`
//! / empty-dir safety policy, and rendering; this module owns the
//! vendoring loop — for every `[[registry]]`-served lockfile entry,
//! refresh its per-package cache clone and copy the `.git/` tree into a
//! bare repo under the output directory — plus the `README.md` generation
//! and the `file://` URL derivation. Per-package progress crosses the
//! seam as typed [`VendorEvent`]s ([`VendorObserver`]); the structured
//! [`VendorSummary`] carries the result for the caller's report.
//!
//! Each `[[registry]]`-served lockfile entry produces a bare git repo
//! `<out>/<naming.repo_name(kind,name)>.git/` populated from the matching
//! per-package cache clone, ready to wire as
//! `[[mirror]] url = "file:///abs/path"` for offline / air-gapped
//! installs.
//!
//! Spec: [PROP-002 §2.3 (mirror layer)](../../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#mirror).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#mirror");

use std::path::Path;

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::Lockfile;

use crate::{MultiRegistryResolver, RegistryError};

/// Failure surface of vendoring — every refusal names the violated spec
/// unit and the fix surface (the product-error grammar, SHRINK-v0.1 §4).
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#mirror")]
pub enum VendorError {
    #[error(
        "refreshing the per-package clone for `{group}/{name}` at `{refname}` failed \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#mirror; \
          fix: ensure the registry — or one of its `[[mirror]]` URLs — is reachable at that ref): {source}"
    )]
    Refresh {
        group: String,
        name: String,
        refname: String,
        // Boxed: `RegistryError` is large, and nesting it unboxed would
        // push `VendorError` past clippy's `result_large_err` threshold.
        #[source]
        source: Box<RegistryError>,
    },

    #[error(
        "per-package clone for `{group}/{name}` lacks a `.git/` after refresh — the registry \
         returned without populating the cache at `{clone_dir}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#mirror; \
          fix: wipe `~/.vibe/registries` and re-run, or confirm the registry serves this ref)"
    )]
    CacheNotPopulated {
        group: String,
        name: String,
        clone_dir: String,
    },

    #[error(
        "deriving the vendor repo name for `{group}/{name}` failed \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#mirror; \
          fix: the registry's `naming` convention needs a `kind` this lockfile entry omits): {source}"
    )]
    RepoName {
        group: String,
        name: String,
        #[source]
        source: Box<vibe_core::Error>,
    },

    #[error(
        "I/O error vendoring into `{path}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#mirror; \
          fix: check permissions and free space at that path): {source}"
    )]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// One observable step of [`vendor_packages`] — a single package was
/// copied into the mirror directory. Fields carry exactly what a renderer
/// needs; no pre-formatted prose crosses the seam (R3-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorEvent {
    /// A `[[registry]]`-served package was vendored into a bare repo.
    Vendored {
        group: String,
        name: String,
        /// Git ref the bare repo carries — typically `v<version>`.
        refname: String,
        /// Forward-slashed path of the bare repo under the output dir.
        repo_dir: String,
    },
}

/// The caller's view into a running vendor pass. Implemented by the CLI
/// to render per-package progress; tests and headless callers use
/// [`NullObserver`].
///
/// ```
/// use vibe_registry::vendor::{VendorEvent, VendorObserver};
///
/// struct Collector(std::cell::RefCell<Vec<VendorEvent>>);
/// impl VendorObserver for Collector {
///     fn on(&self, event: VendorEvent) {
///         self.0.borrow_mut().push(event);
///     }
/// }
///
/// let observer = Collector(std::cell::RefCell::new(Vec::new()));
/// observer.on(VendorEvent::Vendored {
///     group: "org.vibevm".into(),
///     name: "wal".into(),
///     refname: "v0.1.0".into(),
///     repo_dir: "/tmp/vendor/org.vibevm.wal.git".into(),
/// });
/// assert_eq!(observer.0.into_inner().len(), 1);
/// ```
pub trait VendorObserver {
    fn on(&self, event: VendorEvent);
}

/// Ignores every event — for headless callers and tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullObserver;

impl VendorObserver for NullObserver {
    fn on(&self, _event: VendorEvent) {}
}

/// A package successfully vendored into the mirror directory.
#[derive(Debug, Clone)]
pub struct VendoredPackage {
    pub group: String,
    pub name: String,
    /// `[[registry]]` name that originally served this package.
    pub registry: String,
    /// Forward-slashed path of the bare repo under the output dir.
    pub repo_dir: String,
    /// Git ref the bare repo carries — typically `v<version>`.
    pub refname: String,
}

/// A lockfile entry that was not vendored, with the operator-facing reason.
#[derive(Debug, Clone)]
pub struct SkippedPackage {
    pub group: String,
    pub name: String,
    pub reason: String,
}

/// Result of a vendor pass — the suggested `[[mirror]]` URL plus the
/// per-package vendored / skipped breakdown the caller renders.
#[derive(Debug, Clone)]
pub struct VendorSummary {
    /// Forward-slashed display form of the output directory.
    pub out_dir: String,
    /// `file://` + absolute, forward-slashed output dir — paste into
    /// `vibe.toml` as `[[mirror]] url = …`.
    pub suggested_mirror_url: String,
    pub vendored: Vec<VendoredPackage>,
    pub skipped: Vec<SkippedPackage>,
}

/// Vendor every `[[registry]]`-served lockfile entry into bare git repos
/// under `out_dir`, writing a `README.md` that explains the mirror wiring.
///
/// `out_dir` must already exist — the caller owns the `--force` /
/// empty-dir safety policy. Per-package progress is reported via
/// `observer`; the returned [`VendorSummary`] carries the full breakdown
/// for the caller's report.
///
/// Entries served by `[[override]]`, entries with no `registry` field
/// (installed via `--registry <path>` or a legacy v1 path), and entries
/// naming a registry absent from `vibe.toml` are recorded as
/// [`SkippedPackage`]s rather than failing the pass — partial offline
/// coverage is more useful than an all-or-nothing refusal.
///
/// `refresh_package` is mirror-aware, so vendoring against an unreachable
/// primary still works as long as some `[[mirror]]` URL is reachable.
pub fn vendor_packages(
    resolver: &MultiRegistryResolver,
    lockfile: &Lockfile,
    out_dir: &Path,
    observer: &dyn VendorObserver,
) -> Result<VendorSummary, VendorError> {
    let mut vendored: Vec<VendoredPackage> = Vec::new();
    let mut skipped: Vec<SkippedPackage> = Vec::new();

    for entry in &lockfile.packages {
        if entry.overridden {
            skipped.push(SkippedPackage {
                group: entry.group.to_string(),
                name: entry.name.to_string(),
                reason: format!(
                    "[[override]]-served (source_url `{}`); vendor it manually if you need offline coverage",
                    entry.source_url
                ),
            });
            continue;
        }
        let Some(reg_name) = entry.registry.as_deref() else {
            skipped.push(SkippedPackage {
                group: entry.group.to_string(),
                name: entry.name.to_string(),
                reason: "lockfile entry has no `registry` (likely installed via `--registry <path>` or a legacy v1 path)"
                    .to_string(),
            });
            continue;
        };
        let Some(reg) = resolver.registries().iter().find(|r| r.name() == reg_name) else {
            skipped.push(SkippedPackage {
                group: entry.group.to_string(),
                name: entry.name.to_string(),
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

        // Make sure the per-package clone is on disk and at the requested
        // ref before reading its `.git/`.
        reg.refresh_package(&entry.group, &entry.name, &refname)
            .map_err(|source| VendorError::Refresh {
                group: entry.group.to_string(),
                name: entry.name.to_string(),
                refname: refname.clone(),
                source: Box::new(source),
            })?;

        let clone_dir = reg.package_clone_dir(&entry.group, &entry.name);
        let clone_git = clone_dir.join(".git");
        if !clone_git.is_dir() {
            // Should not happen after a successful `refresh_package`, but
            // guard anyway — an explicit error here beats a confusing I/O
            // error two layers down in `bare_clone_from_clone`.
            return Err(VendorError::CacheNotPopulated {
                group: entry.group.to_string(),
                name: entry.name.to_string(),
                clone_dir: forward_slash_display(&clone_dir),
            });
        }

        let repo_name = reg
            .naming()
            .repo_name(Some(entry.kind), &entry.group, &entry.name)
            .map_err(|source| VendorError::RepoName {
                group: entry.group.to_string(),
                name: entry.name.to_string(),
                source: Box::new(source),
            })?;
        let vendor_repo = out_dir.join(format!("{repo_name}.git"));
        if vendor_repo.exists() {
            std::fs::remove_dir_all(&vendor_repo).map_err(|source| VendorError::Io {
                path: forward_slash_display(&vendor_repo),
                source,
            })?;
        }
        if let Some(parent) = vendor_repo.parent() {
            std::fs::create_dir_all(parent).map_err(|source| VendorError::Io {
                path: forward_slash_display(parent),
                source,
            })?;
        }

        bare_clone_from_clone(&clone_git, &vendor_repo)?;

        let repo_dir = forward_slash_display(&vendor_repo);
        observer.on(VendorEvent::Vendored {
            group: entry.group.to_string(),
            name: entry.name.to_string(),
            refname: refname.clone(),
            repo_dir: repo_dir.clone(),
        });
        vendored.push(VendoredPackage {
            group: entry.group.to_string(),
            name: entry.name.to_string(),
            registry: reg_name.to_string(),
            repo_dir,
            refname,
        });
    }

    let suggested_mirror_url = file_url_for_dir(out_dir);
    write_vendor_readme(out_dir, &suggested_mirror_url, &vendored).map_err(|source| {
        VendorError::Io {
            path: forward_slash_display(&out_dir.join("README.md")),
            source,
        }
    })?;

    Ok(VendorSummary {
        out_dir: forward_slash_display(out_dir),
        suggested_mirror_url,
        vendored,
        skipped,
    })
}

/// Produce a `file://` URL for an absolute directory path, forward-slashed
/// so the URL is well-formed on Windows (`file:///C:/Users/...`) and Unix
/// (`file:///path/...`).
///
/// ```
/// use std::path::PathBuf;
/// use vibe_registry::vendor::file_url_for_dir;
///
/// assert_eq!(
///     file_url_for_dir(&PathBuf::from("/abs/path/to/vendor")),
///     "file:///abs/path/to/vendor",
/// );
/// ```
pub fn file_url_for_dir(dir: &Path) -> String {
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
fn bare_clone_from_clone(src_git: &Path, dst: &Path) -> Result<(), VendorError> {
    std::fs::create_dir_all(dst).map_err(|source| VendorError::Io {
        path: forward_slash_display(dst),
        source,
    })?;
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
            std::fs::create_dir_all(&target).map_err(|source| VendorError::Io {
                path: forward_slash_display(&target),
                source,
            })?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|source| VendorError::Io {
                    path: forward_slash_display(parent),
                    source,
                })?;
            }
            std::fs::copy(entry.path(), &target).map_err(|source| VendorError::Io {
                path: forward_slash_display(&target),
                source,
            })?;
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
    vendored: &[VendoredPackage],
) -> std::io::Result<()> {
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
}

#[cfg(test)]
mod tests {
    use super::{VendoredPackage, bare_clone_from_clone, file_url_for_dir};
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
        // Caller (vendor_packages) guarantees parent exists; the helper
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

    #[test]
    fn write_readme_lists_vendored_contents() {
        let dir = tempdir().unwrap();
        let vendored = vec![VendoredPackage {
            group: "org.vibevm".into(),
            name: "wal".into(),
            registry: "vibespecs".into(),
            repo_dir: "/tmp/vendor/org.vibevm.wal.git".into(),
            refname: "v0.1.0".into(),
        }];
        super::write_vendor_readme(dir.path(), "file:///tmp/vendor", &vendored).unwrap();
        let body = fs::read_to_string(dir.path().join("README.md")).unwrap();
        assert!(body.contains("file:///tmp/vendor"));
        assert!(body.contains("`org.vibevm.world/wal` @ `v0.1.0`"));
        assert!(body.contains("[[mirror]]"));
    }
}
