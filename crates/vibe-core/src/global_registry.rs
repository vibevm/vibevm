//! Machine-global registry config (`~/.vibe/registry.toml`) and its merge
//! with a project `vibe.toml` (PROP-002 §2.2.2 `#global-config`).
//!
//! A developer keeps machine-local registries — a `file://` checkout, a path
//! repo — in a per-user file instead of hard-coding a per-machine path into a
//! team-shared `vibe.toml`. The effective registry set is the project's
//! entries followed by the global file's, project-first with a `name`
//! collision resolved in the project's favour ([`merge_effective`]).
//! [`EffectiveRegistryConfig::local_only`] narrows that set for `--offline`
//! (PROP-002 §2.2.2.1 `#offline-local`): local sources resolve, remotes are
//! dropped, so no network round-trip or credential prompt is possible.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#global-config");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::manifest::{Manifest, MirrorSection, OverrideSection, RegistrySection};

/// Parsed `~/.vibe/registry.toml`. Same section shapes as a project
/// `vibe.toml`: `[[registry]]` / `[[mirror]]` / `[[override]]`.
///
/// ```
/// use vibe_core::GlobalRegistryConfig;
///
/// let g: GlobalRegistryConfig = toml::from_str(
///     "[[registry]]\nname = \"local\"\nurl = \"file:///home/u/repos\"\n",
/// )
/// .unwrap();
/// assert_eq!(g.registries.len(), 1);
/// assert!(g.mirrors.is_empty() && g.overrides.is_empty());
/// // The all-empty file is the default (an absent global config).
/// assert!(GlobalRegistryConfig::default().registries.is_empty());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalRegistryConfig {
    /// `[[registry]]` — machine-local (or extra) registries.
    #[serde(default, rename = "registry", skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistrySection>,
    /// `[[mirror]]` — mirrors, concatenated after the project's.
    #[serde(default, rename = "mirror", skip_serializing_if = "Vec::is_empty")]
    pub mirrors: Vec<MirrorSection>,
    /// `[[override]]` — source pins, project-first deduped by `pkgref`.
    #[serde(default, rename = "override", skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<OverrideSection>,
}

impl GlobalRegistryConfig {
    /// Load from the canonical `~/.vibe/registry.toml` (through the settings
    /// chokepoint, so `$VIBE_SETTINGS` relocates it). A missing file — or an
    /// unresolvable settings dir — is `Ok(default)`: the layer is optional.
    pub fn load() -> Result<Self, GlobalRegistryError> {
        let Some(path) = crate::settings::registry_config_path() else {
            return Ok(Self::default());
        };
        Self::load_from(&path)
    }

    /// Like [`Self::load`] but from an explicit path (tests, ad-hoc).
    /// Missing-file is `Ok(default)`; a parse error surfaces so a malformed
    /// file is noticed rather than silently ignored.
    pub fn load_from(path: &Path) -> Result<Self, GlobalRegistryError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let body = std::fs::read_to_string(path).map_err(|source| GlobalRegistryError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&body).map_err(|source| GlobalRegistryError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }
}

/// Why loading the global registry config failed — an I/O error or a TOML
/// parse error. Missing-file is *not* an error (the layer is optional); each
/// variant's `Display` cites the governing REQ.
///
/// ```
/// use vibe_core::GlobalRegistryError;
///
/// let e = GlobalRegistryError::Io {
///     path: "/x/registry.toml".into(),
///     source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
/// };
/// assert!(e.to_string().contains("could not read"));
/// assert!(e.to_string().contains("global-config"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#global-config")]
pub enum GlobalRegistryError {
    #[error(
        "could not read `{path}`: {source} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#global-config; \
          fix: check the file's permissions, or remove it to fall back to \
          project-only registry config)"
    )]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "`{path}` is malformed: {source} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#global-config; \
          fix: repair the TOML at the reported location)"
    )]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// The merged, resolution-ready registry config: a project manifest's
/// sections plus the machine-global file's (PROP-002 §2.2.2).
/// [`Self::local_only`] narrows it for `--offline`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveRegistryConfig {
    /// Priority-ordered registries (project first, then machine-global).
    pub registries: Vec<RegistrySection>,
    /// Mirrors (project first, then machine-global).
    pub mirrors: Vec<MirrorSection>,
    /// Overrides (project first, then machine-global).
    pub overrides: Vec<OverrideSection>,
}

impl EffectiveRegistryConfig {
    /// Narrow to local sources for `--offline` (PROP-002 §2.2.2.1
    /// `#offline-local`): keep `file://` and bare-path registries / mirrors /
    /// overrides; drop `http(s)`/`ssh`/`git` remotes. A machine-local
    /// registry still resolves offline; a github/gitverse one is simply
    /// absent — no network, no credential prompt.
    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#offline-local")]
    pub fn local_only(self) -> Self {
        EffectiveRegistryConfig {
            registries: self
                .registries
                .into_iter()
                .filter(|r| url_is_local(&r.url))
                .collect(),
            mirrors: self
                .mirrors
                .into_iter()
                .filter(|m| url_is_local(&m.url))
                .collect(),
            overrides: self
                .overrides
                .into_iter()
                .filter(|o| url_is_local(&o.source_url))
                .collect(),
        }
    }
}

/// Merge a project manifest with the machine-global registry config
/// (PROP-002 §2.2.2 `#global-config`): registries project-first, deduped by
/// `name` (a collision resolves to the project entry); mirrors concatenated
/// project-first; overrides project-first, deduped by `pkgref`. Pure —
/// verified in isolation.
///
/// ```
/// use vibe_core::{merge_effective, GlobalRegistryConfig};
/// use vibe_core::manifest::Manifest;
///
/// let project = Manifest::parse_str(
///     "[package]\ngroup=\"org.x\"\nname=\"p\"\nkind=\"flow\"\nversion=\"0.1.0\"\n\
///      [[registry]]\nname=\"team\"\nurl=\"https://github.com/team\"\n",
/// )
/// .unwrap();
/// let global: GlobalRegistryConfig = toml::from_str(
///     "[[registry]]\nname=\"team\"\nurl=\"file:///shadow\"\n\
///      [[registry]]\nname=\"local\"\nurl=\"file:///repos\"\n",
/// )
/// .unwrap();
/// let eff = merge_effective(&project, &global);
/// // Project first; the global `team` is dropped (name collision → project
/// // wins); the machine-only `local` is appended.
/// assert_eq!(eff.registries.len(), 2);
/// assert_eq!(eff.registries[0].name, "team");
/// assert_eq!(eff.registries[0].url, "https://github.com/team");
/// assert_eq!(eff.registries[1].name, "local");
/// ```
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#global-config")]
pub fn merge_effective(
    project: &Manifest,
    global: &GlobalRegistryConfig,
) -> EffectiveRegistryConfig {
    use std::collections::HashSet;

    let mut registries = project.registries.clone();
    let have: HashSet<String> = registries.iter().map(|r| r.name.clone()).collect();
    registries.extend(
        global
            .registries
            .iter()
            .filter(|r| !have.contains(&r.name))
            .cloned(),
    );

    let mut mirrors = project.mirrors.clone();
    mirrors.extend(global.mirrors.iter().cloned());

    let mut overrides = project.overrides.clone();
    let have_ov: HashSet<String> = overrides.iter().map(|o| o.pkgref.clone()).collect();
    overrides.extend(
        global
            .overrides
            .iter()
            .filter(|o| !have_ov.contains(&o.pkgref))
            .cloned(),
    );

    EffectiveRegistryConfig {
        registries,
        mirrors,
        overrides,
    }
}

/// Whether a registry / mirror / override URL is **local** — resolvable
/// without the network (PROP-002 §2.2.2.1 `#offline-local`). Local: a `file:`
/// URL or a bare filesystem path (absolute, relative, or a Windows drive
/// path). Remote: an explicit `http(s)`/`ssh`/`git`/`ftp` scheme or an
/// scp-form `user@host:path`. A `git+` transport prefix is peeled first.
///
/// ```
/// use vibe_core::url_is_local;
///
/// assert!(url_is_local("file:///home/u/repos"));
/// assert!(url_is_local("/abs/path/registry"));
/// assert!(url_is_local("./rel/registry"));
/// assert!(url_is_local(r"C:\Users\u\repos"));
/// assert!(url_is_local("git+file:///repos"));
///
/// assert!(!url_is_local("https://github.com/vibespecs"));
/// assert!(!url_is_local("ssh://git@host/org"));
/// assert!(!url_is_local("git@github.com:org/repo"));
/// assert!(!url_is_local("git+https://github.com/org"));
/// ```
pub fn url_is_local(url: &str) -> bool {
    let u = url.trim();
    let u = u.strip_prefix("git+").unwrap_or(u);
    let lower = u.to_ascii_lowercase();
    const REMOTE_SCHEMES: [&str; 6] = [
        "http://", "https://", "ssh://", "git://", "ftp://", "ftps://",
    ];
    if REMOTE_SCHEMES.iter().any(|s| lower.starts_with(s)) {
        return false;
    }
    if u.starts_with("file:") {
        return true;
    }
    // scp-form `user@host:path` (ssh under the hood): an `@` followed later by
    // a `:`. A Windows drive path (`C:\…`) has no `@`, so it stays local.
    if let Some(at) = u.find('@')
        && u[at..].contains(':')
    {
        return false;
    }
    // No recognised remote scheme and not scp-form → a filesystem path.
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    fn project_with(sections: &str) -> Manifest {
        Manifest::parse_str(&format!(
            "[package]\ngroup=\"org.x\"\nname=\"p\"\nkind=\"flow\"\nversion=\"0.1.0\"\n{sections}"
        ))
        .unwrap()
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-registry/PROP-002#global-config")]
    fn merge_is_project_first_and_dedupes_registries_by_name() {
        let project =
            project_with("[[registry]]\nname=\"team\"\nurl=\"https://github.com/team\"\n");
        let global: GlobalRegistryConfig = toml::from_str(
            "[[registry]]\nname=\"team\"\nurl=\"file:///shadow\"\n\
             [[registry]]\nname=\"local\"\nurl=\"file:///repos\"\n",
        )
        .unwrap();
        let eff = merge_effective(&project, &global);
        assert_eq!(eff.registries.len(), 2);
        // Project's `team` wins the name collision — its URL survives.
        assert_eq!(eff.registries[0].name, "team");
        assert_eq!(eff.registries[0].url, "https://github.com/team");
        // Machine-only `local` is appended after the project's.
        assert_eq!(eff.registries[1].name, "local");
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-registry/PROP-002#global-config")]
    fn merge_dedupes_overrides_by_pkgref_and_concatenates_mirrors() {
        let project = project_with(
            "[[override]]\npkgref=\"feat:wal\"\nsource_url=\"https://github.com/me/wal\"\n\
             [[mirror]]\nof=\"team\"\nurl=\"https://m1/team\"\n",
        );
        let global: GlobalRegistryConfig = toml::from_str(
            "[[override]]\npkgref=\"feat:wal\"\nsource_url=\"file:///shadow\"\n\
             [[override]]\npkgref=\"flow:x\"\nsource_url=\"file:///x\"\n\
             [[mirror]]\nof=\"team\"\nurl=\"file:///m2\"\n",
        )
        .unwrap();
        let eff = merge_effective(&project, &global);
        // Override collision → project wins; the machine-only one is appended.
        assert_eq!(eff.overrides.len(), 2);
        assert_eq!(eff.overrides[0].pkgref, "feat:wal");
        assert_eq!(eff.overrides[0].source_url, "https://github.com/me/wal");
        assert_eq!(eff.overrides[1].pkgref, "flow:x");
        // Mirrors are concatenated (no dedupe), project first.
        assert_eq!(eff.mirrors.len(), 2);
        assert_eq!(eff.mirrors[0].url, "https://m1/team");
        assert_eq!(eff.mirrors[1].url, "file:///m2");
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-registry/PROP-002#offline-local")]
    fn local_only_keeps_local_drops_remote() {
        let project = project_with(
            "[[registry]]\nname=\"team\"\nurl=\"https://github.com/team\"\n\
             [[registry]]\nname=\"local\"\nurl=\"file:///repos\"\n",
        );
        let eff = merge_effective(&project, &GlobalRegistryConfig::default()).local_only();
        assert_eq!(eff.registries.len(), 1);
        assert_eq!(eff.registries[0].name, "local");
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-registry/PROP-002#offline-local")]
    fn url_is_local_truth_table() {
        for (url, want_local) in [
            ("file:///home/u/repos", true),
            ("/abs/registry", true),
            ("./rel", true),
            (r"C:\repos", true),
            ("git+file:///repos", true),
            ("https://github.com/x", false),
            ("http://x/y", false),
            ("ssh://git@h/o", false),
            ("git://h/o", false),
            ("git@github.com:o/r", false),
            ("git+https://h/o", false),
        ] {
            assert_eq!(url_is_local(url), want_local, "url = {url}");
        }
    }

    #[test]
    fn load_from_missing_is_default() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("registry.toml");
        assert_eq!(
            GlobalRegistryConfig::load_from(&p).unwrap(),
            GlobalRegistryConfig::default()
        );
    }

    #[test]
    fn load_from_parses_registry_toml() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("registry.toml");
        std::fs::write(&p, "[[registry]]\nname=\"local\"\nurl=\"file:///repos\"\n").unwrap();
        let g = GlobalRegistryConfig::load_from(&p).unwrap();
        assert_eq!(g.registries.len(), 1);
        assert_eq!(g.registries[0].name, "local");
    }

    #[test]
    fn load_from_malformed_errors() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("registry.toml");
        std::fs::write(&p, "this is = not = toml").unwrap();
        assert!(matches!(
            GlobalRegistryConfig::load_from(&p).unwrap_err(),
            GlobalRegistryError::Parse { .. }
        ));
    }

    #[test]
    fn load_from_rejects_unknown_top_level_key() {
        // `deny_unknown_fields` catches a typo'd section (`[[registy]]`)
        // instead of silently ignoring it.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("registry.toml");
        std::fs::write(&p, "[[registy]]\nname=\"x\"\nurl=\"file:///r\"\n").unwrap();
        assert!(matches!(
            GlobalRegistryConfig::load_from(&p).unwrap_err(),
            GlobalRegistryError::Parse { .. }
        ));
    }
}
