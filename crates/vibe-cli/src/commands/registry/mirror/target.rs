//! `--into`: a layout registry name, a Git url or bare repository, or a
//! working copy (PROP-002 `#registry-mirror`).
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{RegistrySection, RegistryType};
use vibe_publish::git_publish::redact_credentials;
use vibe_publish::layout_publish::LayoutTarget;
use vibe_wire::generated::registry_mirror_report::TargetKind;

use crate::cli::RegistryMirrorArgs;

/// The resolved `--into`.
pub(super) struct Target {
    pub layout: LayoutTarget,
    pub kind: TargetKind,
    /// The registry name recorded in a new index.
    pub name: String,
    /// The url recorded in a new index.
    pub url: String,
    pub shown: String,
}

impl Target {
    pub(super) fn identity(&self) -> vibe_index::layout::LayoutIdentity {
        vibe_index::layout::LayoutIdentity {
            registry: self.name.clone(),
            registry_url: self.url.clone(),
            branch: match &self.layout {
                LayoutTarget::Monorepo { branch, .. } => Some(branch.clone()),
                LayoutTarget::WorkingCopy { .. } => Some("HEAD".to_string()),
                LayoutTarget::Localdir { .. } => None,
            },
        }
    }
}

pub(super) fn resolve_target(
    args: &RegistryMirrorArgs,
    registries: &[RegistrySection],
) -> Result<Target> {
    let into = args.into.trim();
    if let Some(reg) = registries.iter().find(|r| r.name == into) {
        let name = args.name.clone().unwrap_or_else(|| reg.name.clone());
        return match reg.checked_type()? {
            RegistryType::Monorepo => Ok(Target {
                layout: LayoutTarget::Monorepo {
                    url: reg.url.clone(),
                    branch: args.branch.clone().unwrap_or_else(|| reg.r#ref.clone()),
                },
                kind: TargetKind::Monorepo,
                name,
                url: reg.url.clone(),
                shown: format!("monorepo `{}` ({})", reg.name, redact_credentials(&reg.url)),
            }),
            RegistryType::Localdir => {
                let root = vibe_registry::localdir_path(&reg.url)?;
                Ok(Target {
                    shown: format!("localdir `{}` ({})", reg.name, root.display()),
                    layout: LayoutTarget::Localdir { root },
                    kind: TargetKind::Localdir,
                    name,
                    url: reg.url.clone(),
                })
            }
            other => bail!(
                "`--into {into}` names a `{}` registry; a mirror writes only into a `monorepo` \
                 or `localdir` registry \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
                 fix: name a monorepo or localdir registry, or give a repository url or path)",
                other.as_str()
            ),
        };
    }
    let path = Path::new(into);
    if path.exists() {
        let path = path
            .canonicalize()
            .with_context(|| format!("canonicalizing `{into}`"))?;
        let path = crate::commands::init::strip_unc_public(path);
        if path.join(".git").exists() {
            let url = origin_url(&path).unwrap_or_else(|| file_url(&path));
            return Ok(Target {
                name: args.name.clone().unwrap_or_else(|| derived_name(&url)),
                shown: format!("working copy {}", path.display()),
                layout: LayoutTarget::WorkingCopy { path },
                kind: TargetKind::WorkingCopy,
                url,
            });
        }
        if path.join("HEAD").is_file() && path.join("objects").is_dir() {
            let url = file_url(&path);
            let branch = args.branch.clone().unwrap_or_else(|| "main".to_string());
            return Ok(Target {
                name: args.name.clone().unwrap_or_else(|| derived_name(&url)),
                shown: format!("bare repository {} (branch `{branch}`)", path.display()),
                layout: LayoutTarget::Monorepo {
                    url: url.clone(),
                    branch,
                },
                kind: TargetKind::Monorepo,
                url,
            });
        }
        bail!(
            "`--into {into}` is a directory that is neither a Git working copy nor a bare \
             repository \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
             fix: to mirror into a plain directory declare it as a `localdir` registry \
             (`vibe registry add <name> file:///… --type localdir --scope user`) and pass its name)"
        );
    }
    if looks_like_git_url(into) {
        let branch = args.branch.clone().unwrap_or_else(|| "main".to_string());
        return Ok(Target {
            name: args.name.clone().unwrap_or_else(|| derived_name(into)),
            shown: format!(
                "repository {} (branch `{branch}`)",
                redact_credentials(into)
            ),
            layout: LayoutTarget::Monorepo {
                url: into.to_string(),
                branch,
            },
            kind: TargetKind::Monorepo,
            url: into.to_string(),
        });
    }
    bail!(
        "`--into {into}` is neither a configured registry, an existing path nor a Git url \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
         fix: pass a monorepo/localdir registry name, `ssh://…` / `git@host:path` / `file://…`, \
         or the path of a bare repository or working copy)"
    )
}

fn looks_like_git_url(value: &str) -> bool {
    if value.contains("://") {
        return true;
    }
    // scp form `[user@]host:path` — not a Windows drive (`C:\…`).
    match value.split_once(':') {
        Some((host, rest)) => host.len() > 1 && !host.contains('/') && !rest.is_empty(),
        None => false,
    }
}

fn file_url(path: &Path) -> String {
    let shown = path.display().to_string().replace('\\', "/");
    if shown.starts_with('/') {
        format!("file://{shown}")
    } else {
        format!("file:///{shown}")
    }
}

fn origin_url(path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .ok()?;
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !url.is_empty()).then_some(url)
}

fn derived_name(url: &str) -> String {
    let tail = url
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or_default()
        .trim_end_matches(".git");
    if tail.is_empty() {
        "mirror".to_string()
    } else {
        tail.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_urls_are_told_from_paths() {
        assert!(looks_like_git_url("ssh://git@corp.example/vibe.git"));
        assert!(looks_like_git_url("git@corp.example:tools/vibe.git"));
        assert!(looks_like_git_url("file:///srv/vibe.git"));
        assert!(!looks_like_git_url("C:\\corp\\vibe"));
        assert!(!looks_like_git_url("relative/dir"));
    }

    #[test]
    fn the_index_name_derives_from_the_repository() {
        assert_eq!(
            derived_name("ssh://git@corp.example/tools/vibe-packages.git"),
            "vibe-packages"
        );
        assert_eq!(derived_name("git@corp.example:vibe.git"), "vibe");
        assert_eq!(derived_name("file:///srv/repos/corp/"), "corp");
    }
}
