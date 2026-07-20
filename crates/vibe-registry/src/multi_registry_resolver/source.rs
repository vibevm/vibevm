//! The registry-source model + the local-`[[registry]]` url parser.
//!
//! [`RegistrySource`] is the heterogeneous entry the multi-registry walk
//! iterates: git registries ([`GitPackageRegistry`]) and local-directory
//! registries ([`LocalRegistry`]) coexist in declared order, dispatched per
//! variant on the four core operations. [`local_path_from_url`] turns a local
//! `[[registry]]` url into the filesystem path `LocalRegistry` opens.
//!
//! Split out of the resolver root to hold the file-length budget (PROP-002
//! §2.2.2 — a local url is served from the filesystem, never git-cloned).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

/// One source in the priority-ordered registry walk. Git registries
/// ([`GitPackageRegistry`] — per-package repos under an org URL, versions by
/// git tag) and local-directory registries ([`LocalRegistry`] — a monorepo
/// laid out `<root>/<group>/<name>/v<version>/`) coexist in declared order.
///
/// A `[[registry]]` whose `url` is local (`file://` / bare path, per
/// [`url_is_local`]) is served by [`LocalRegistry`] straight off the
/// filesystem — never git-cloned — so a plain on-disk directory works as a
/// registry (PROP-002 §2.2.2). The two backends share the `<group>/<name>
/// /v<version>/` layout but resolve it differently (git per-package repos vs
/// a filesystem tree), so they cannot be wrapped one as the other; the
/// resolver dispatches per variant on the four core operations
/// (list / resolve / fetch-dep-manifest / fetch).
pub enum RegistrySource {
    /// Remote (or local-git) org-root registry: `<org>/<group>.<name>.git`
    /// per-package repos, versions by `git ls-remote` tags.
    Git(Arc<GitPackageRegistry>),
    /// Local-directory registry: a filesystem root laid out
    /// `<root>/<group>/<name>/v<version>/`, read with no git.
    Local(LocalRegistrySource),
}

/// The local-directory flavour of a [`RegistrySource`] — a [`LocalRegistry`]
/// plus the `[[registry]]` metadata the walk and the diagnostics still need
/// (the registry `name`, used to dispatch a fetch back to the source that
/// resolved it, and the original `url`, used for the failure-discriminator
/// attempts and the lockfile `source_url`).
pub struct LocalRegistrySource {
    pub name: String,
    pub url: String,
    pub registry: LocalRegistry,
}

impl RegistrySource {
    /// The `[[registry]].name` — identifies the source for the fetch
    /// dispatch (a resolution records `registry_name`; fetch finds the source
    /// by name) and for the failure-discriminator attempts.
    pub fn name(&self) -> &str {
        match self {
            RegistrySource::Git(g) => g.name(),
            RegistrySource::Local(ls) => &ls.name,
        }
    }

    /// The `[[registry]].url` — for the failure-discriminator attempts and
    /// the lockfile `source_url`.
    pub fn url(&self) -> &str {
        match self {
            RegistrySource::Git(g) => g.org_url(),
            RegistrySource::Local(ls) => &ls.url,
        }
    }
}

/// Turn a local `[[registry]]` url (`file://…` / bare path, already classified
/// local by [`url_is_local`] and not a `git+` transport — `git+file://` is a
/// local *git* repo, handled by the git-clone backend, not here) into the
/// filesystem path [`LocalRegistry`] opens. Handles the common empty-authority
/// forms — `file:///C:/x` (Windows drive), `file:///home/x` (POSIX),
/// `file:/x` — and bare paths verbatim. UNC (`file://host/share`) and anything
/// else with a non-empty authority are rejected: they are not the
/// plain-directory registry shape `LocalRegistry` serves.
pub(crate) fn local_path_from_url(url: &str) -> Result<PathBuf, RegistryError> {
    let s = url.trim();
    let raw = if let Some(after) = s
        .strip_prefix("file://")
        .or_else(|| s.strip_prefix("file:"))
    {
        // `file://<authority>/<path>` or `file:<path>`. Only the empty-authority
        // form (`file:///…`, `after` starts with `/`) maps to a plain path; a
        // non-empty authority would be a UNC share, which LocalRegistry does
        // not serve.
        let path_slash =
            after
                .strip_prefix('/')
                .ok_or_else(|| RegistryError::BadLocalRegistryUrl {
                    url: url.to_string(),
                })?;
        format!("/{path_slash}")
    } else {
        // No `file:` scheme — a bare filesystem path, used as-is.
        s.to_string()
    };
    let path = PathBuf::from(strip_leading_drive_slash(&raw));
    if path.as_os_str().is_empty() {
        return Err(RegistryError::BadLocalRegistryUrl {
            url: url.to_string(),
        });
    }
    Ok(path)
}

/// Drop the POSIX-style leading slash before a Windows drive letter:
/// `/C:/x` or `/C:\x` → `C:/x` / `C:\x` (the shape `file:///C:/x` produces
/// after stripping the scheme + authority marker). A no-op for POSIX roots
/// (`/home/x`) and for paths that do not start with `/<letter>:`.
fn strip_leading_drive_slash(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' {
        &s[1..]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `local_path_from_url` handles the `file://` Windows-drive and POSIX
    /// forms and bare paths — the shapes a local-directory `[[registry]]` url
    /// takes (per `url_is_local`). A `git+` transport is NOT a local-directory
    /// url (it is a local *git* repo the git-clone backend handles) and never
    /// reaches this parser.
    #[test]
    fn local_path_from_url_parses_file_and_bare_forms() {
        // file:// Windows drive: the empty-authority form `file:///C:/x`.
        assert_eq!(
            local_path_from_url("file:///C:/x/y").unwrap(),
            PathBuf::from("C:/x/y")
        );
        // file:// POSIX absolute.
        assert_eq!(
            local_path_from_url("file:///home/x/y").unwrap(),
            PathBuf::from("/home/x/y")
        );
        // Bare path (no scheme) — used verbatim.
        assert_eq!(
            local_path_from_url("/abs/path").unwrap(),
            PathBuf::from("/abs/path")
        );
        // A non-empty authority (UNC share) is rejected — not a plain directory.
        assert!(local_path_from_url("file://host/share/x").is_err());
    }
}
