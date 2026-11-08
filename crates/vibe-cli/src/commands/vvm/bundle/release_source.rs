//! The one owner of "where `vibe self` reads its releases from"
//! (`PROP-019#release-sources`).
//!
//! Four inputs used to be able to answer that question, and each was read
//! somewhere else: `$VIBEVM_RELEASE_ORIGIN` (the install scripts' mirror),
//! the `[self]` table, the `[artifacts] vibe` drop, and the built-in GitHub
//! release page. This module resolves them once, in that precedence, into
//! one [`EffectiveReleaseSource`]. The composition root lowers it into the
//! remap row that redirects the GitHub download root, so `vibe remap
//! explain` shows the redirection; the release lane reads the same value
//! for what a row cannot say — how to list releases, and GitVerse's API.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#release-sources");

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use specmark::spec;
use vibe_core::remap::drop::release_source_row;
use vibe_core::remap::{ByteLocation, Layer, RemapKey, RemapRow, RemapTarget};
use vibe_core::user_config::{ReleaseSourceKind, UserConfig};

/// The release origin `install.sh` / `install.ps1` declare.
pub(crate) const CANONICAL_RELEASE_ORIGIN: &str = "https://github.com/vibevm/vibevm/releases";

/// The environment variable the install scripts honour; it outranks every
/// configured source so a bootstrap started by one of them agrees with it.
pub(crate) const RELEASE_ORIGIN_ENV: &str = "VIBEVM_RELEASE_ORIGIN";

/// The public GitVerse repository of vibevm.
pub(crate) const DEFAULT_GITVERSE_REPOSITORY: &str = "https://gitverse.ru/vibevm/vibevm";

/// Where releases come from, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-019#release-sources")]
pub(crate) enum ReleaseSource {
    /// The GitHub release page (`latest/download` names the newest).
    Github,
    /// `$VIBEVM_RELEASE_ORIGIN`: a GitHub-shaped mirror of the whole
    /// release page (`<origin>/download/v<version>/`, `<origin>/latest/download/`).
    Origin { origin: String },
    /// A GitVerse instance, read through its REST API.
    Gitverse {
        /// The repository web URL.
        web: String,
        /// The REST API base.
        api: String,
        owner: String,
        repo: String,
    },
    /// An HTTP(S) raw repository holding `v<version>/<asset>`.
    Nexus { base: String },
    /// A local release drop holding `v<version>/<asset>`.
    Localdir { path: PathBuf },
}

impl ReleaseSource {
    /// The type name `vibe self source` prints.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            ReleaseSource::Github => "github",
            ReleaseSource::Origin { .. } => "mirror",
            ReleaseSource::Gitverse { .. } => "gitverse",
            ReleaseSource::Nexus { .. } => "nexus",
            ReleaseSource::Localdir { .. } => "localdir",
        }
    }

    /// The location a human is shown.
    pub(crate) fn location(&self) -> String {
        match self {
            ReleaseSource::Github => CANONICAL_RELEASE_ORIGIN.to_owned(),
            ReleaseSource::Origin { origin } => origin.clone(),
            ReleaseSource::Gitverse { web, .. } => web.clone(),
            ReleaseSource::Nexus { base } => base.clone(),
            ReleaseSource::Localdir { path } => path.display().to_string(),
        }
    }
}

/// Which input decided the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceOrigin {
    BuiltIn,
    Environment,
    SelfTable(Option<PathBuf>),
    ArtifactsDrop(Option<PathBuf>),
}

impl SourceOrigin {
    pub(crate) fn describe(&self) -> String {
        let file = |path: &Option<PathBuf>| {
            path.as_ref()
                .map_or_else(|| "config.toml".to_owned(), |p| p.display().to_string())
        };
        match self {
            SourceOrigin::BuiltIn => "built-in default".to_owned(),
            SourceOrigin::Environment => format!("${RELEASE_ORIGIN_ENV}"),
            SourceOrigin::SelfTable(path) => format!("[self] in {}", file(path)),
            SourceOrigin::ArtifactsDrop(path) => format!("[artifacts] vibe in {}", file(path)),
        }
    }
}

/// The resolved source and the input that decided it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveReleaseSource {
    pub source: ReleaseSource,
    pub origin: SourceOrigin,
}

impl Default for EffectiveReleaseSource {
    fn default() -> Self {
        EffectiveReleaseSource {
            source: ReleaseSource::Github,
            origin: SourceOrigin::BuiltIn,
        }
    }
}

/// A release source that cannot be used as configured.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#release-sources")]
pub(crate) enum ReleaseSourceError {
    #[error(
        "release source `{kind}` needs {what} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
          fix: `vibe self source set {kind} {flag} …`, or set it in the `[self]` table)"
    )]
    Missing {
        kind: &'static str,
        what: &'static str,
        flag: &'static str,
    },
    #[error(
        "release source `{kind}` location `{location}` is unusable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#release-sources; \
          fix: {fix})"
    )]
    Location {
        kind: &'static str,
        location: String,
        reason: String,
        fix: &'static str,
    },
}

/// Resolve the effective release source, highest first:
/// `$VIBEVM_RELEASE_ORIGIN`, `[self] source`, the `[artifacts] vibe` drop,
/// GitHub. `config_path` only labels where the tables came from.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#release-sources")]
pub(crate) fn resolve(
    env_origin: Option<&str>,
    config: &UserConfig,
    config_path: Option<&Path>,
) -> Result<EffectiveReleaseSource, ReleaseSourceError> {
    let label = config_path.map(Path::to_path_buf);
    if let Some(origin) = env_origin
        .map(str::trim)
        .filter(|origin| !origin.is_empty() && *origin != CANONICAL_RELEASE_ORIGIN)
    {
        return Ok(EffectiveReleaseSource {
            source: ReleaseSource::Origin {
                origin: origin.trim_end_matches('/').to_owned(),
            },
            origin: SourceOrigin::Environment,
        });
    }
    let table = &config.self_source;
    let drop = config.artifacts.vibe.as_ref();
    let source = match table.source {
        Some(ReleaseSourceKind::Github) => ReleaseSource::Github,
        Some(ReleaseSourceKind::Localdir) => {
            let path = table
                .path
                .as_ref()
                .or(drop)
                .ok_or(ReleaseSourceError::Missing {
                    kind: "localdir",
                    what: "a directory (`[self] path` or `[artifacts] vibe`)",
                    flag: "--path",
                })?;
            localdir(path)?
        }
        Some(ReleaseSourceKind::Nexus) => {
            let url = table.url.as_deref().ok_or(ReleaseSourceError::Missing {
                kind: "nexus",
                what: "the base URL of the raw repository (`[self] url`)",
                flag: "--url",
            })?;
            nexus(url)?
        }
        Some(ReleaseSourceKind::Gitverse) => gitverse(table.url.as_deref(), table.api.as_deref())?,
        None => match drop {
            Some(path) => {
                return Ok(EffectiveReleaseSource {
                    source: localdir(path)?,
                    origin: SourceOrigin::ArtifactsDrop(label),
                });
            }
            None => return Ok(EffectiveReleaseSource::default()),
        },
    };
    Ok(EffectiveReleaseSource {
        source,
        origin: SourceOrigin::SelfTable(label),
    })
}

fn localdir(path: &Path) -> Result<ReleaseSource, ReleaseSourceError> {
    if !path.is_absolute() {
        return Err(ReleaseSourceError::Location {
            kind: "localdir",
            location: path.display().to_string(),
            reason: "the path is relative".to_owned(),
            fix: "give the release drop as an absolute path",
        });
    }
    Ok(ReleaseSource::Localdir {
        path: path.to_path_buf(),
    })
}

fn nexus(url: &str) -> Result<ReleaseSource, ReleaseSourceError> {
    match ByteLocation::parse(url) {
        Ok(ByteLocation::Remote(base)) => Ok(ReleaseSource::Nexus {
            base: base.trim_end_matches('/').to_owned(),
        }),
        Ok(ByteLocation::Local(_)) => Err(ReleaseSourceError::Location {
            kind: "nexus",
            location: url.to_owned(),
            reason: "it is a local location".to_owned(),
            fix: "use `vibe self source set localdir --path DIR` for a directory",
        }),
        Err(error) => Err(ReleaseSourceError::Location {
            kind: "nexus",
            location: url.to_owned(),
            reason: error.to_string(),
            fix: "give an `https://` or `http://` base URL",
        }),
    }
}

/// `https://<host>/<owner>/<repo>[/releases]` and the API base for it.
fn gitverse(web: Option<&str>, api: Option<&str>) -> Result<ReleaseSource, ReleaseSourceError> {
    let raw = web.unwrap_or(DEFAULT_GITVERSE_REPOSITORY);
    let bad = |reason: &str| ReleaseSourceError::Location {
        kind: "gitverse",
        location: raw.to_owned(),
        reason: reason.to_owned(),
        fix: "give the repository web URL, `https://<host>/<owner>/<repo>`",
    };
    let url = reqwest::Url::parse(raw).map_err(|_| bad("it is not a URL"))?;
    if !matches!(url.scheme(), "https" | "http") {
        return Err(bad("it is not an HTTP(S) URL"));
    }
    let host = url.host_str().ok_or_else(|| bad("it names no host"))?;
    let mut segments = url
        .path_segments()
        .map(|parts| parts.filter(|part| !part.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();
    if segments.last() == Some(&"releases") {
        segments.pop();
    }
    let [owner, repo] = segments.as_slice() else {
        return Err(bad("the path is not `/<owner>/<repo>`"));
    };
    let port = url
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    let api = match api {
        Some(api) => match ByteLocation::parse(api) {
            Ok(ByteLocation::Remote(api)) => api.trim_end_matches('/').to_owned(),
            _ => {
                return Err(ReleaseSourceError::Location {
                    kind: "gitverse",
                    location: api.to_owned(),
                    reason: "the API base is not an HTTP(S) URL".to_owned(),
                    fix: "give `[self] api` as `https://api.<host>`",
                });
            }
        },
        None => format!("{}://api.{host}{port}", url.scheme()),
    };
    Ok(ReleaseSource::Gitverse {
        web: format!("{}://{host}{port}/{owner}/{repo}", url.scheme()),
        api,
        owner: (*owner).to_owned(),
        repo: (*repo).to_owned(),
    })
}

impl EffectiveReleaseSource {
    /// The remap row this source lowers to, when it redirects by layout.
    /// GitHub needs none; GitVerse is read through its API, so the row
    /// would have nothing to say.
    pub(crate) fn row(&self) -> Option<RemapRow> {
        let reason = format!(
            "release source `{}` from {}",
            self.source.kind(),
            self.origin.describe()
        );
        match &self.source {
            ReleaseSource::Github | ReleaseSource::Gitverse { .. } => None,
            ReleaseSource::Origin { origin } => {
                let mut row = release_source_row(
                    RemapTarget::Url(origin.clone()),
                    Layer::Invocation,
                    RELEASE_ORIGIN_ENV,
                    &reason,
                );
                // The variable has always replaced the whole release page,
                // `latest/download` included.
                row.key = RemapKey::Prefix(CANONICAL_RELEASE_ORIGIN.to_owned());
                row.only = None;
                Some(row)
            }
            ReleaseSource::Nexus { base } => Some(release_source_row(
                RemapTarget::Url(base.clone()),
                Layer::Local,
                "[self] source",
                &reason,
            )),
            ReleaseSource::Localdir { path } => Some(release_source_row(
                RemapTarget::Path(path.clone()),
                Layer::Local,
                match self.origin {
                    SourceOrigin::ArtifactsDrop(_) => "[artifacts] vibe",
                    _ => "[self] source",
                },
                &reason,
            )),
        }
    }
}

/// The release target this build installs (`x86_64-unknown-linux-gnu`, …).
pub(crate) fn release_target() -> anyhow::Result<&'static str> {
    super::current_target()
}

static ACTIVE: OnceLock<Result<EffectiveReleaseSource, ReleaseSourceError>> = OnceLock::new();

/// Record this invocation's release source; write-once, like the Locator.
pub(crate) fn install(resolved: Result<EffectiveReleaseSource, ReleaseSourceError>) {
    let _ = ACTIVE.set(resolved);
}

/// This invocation's release source. A run that installed none (a unit
/// test, an embedder) reads the built-in default.
pub(crate) fn active() -> Result<EffectiveReleaseSource, ReleaseSourceError> {
    ACTIVE
        .get()
        .cloned()
        .unwrap_or_else(|| Ok(EffectiveReleaseSource::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::user_config::{ArtifactsConfig, SelfConfig};

    fn config(table: SelfConfig, vibe: Option<&str>) -> UserConfig {
        UserConfig {
            self_source: table,
            artifacts: ArtifactsConfig {
                vibe: vibe.map(PathBuf::from),
                packages: None,
            },
            ..UserConfig::default()
        }
    }

    #[test]
    fn precedence_is_environment_then_self_then_drop_then_github() {
        let both = config(
            SelfConfig {
                source: Some(ReleaseSourceKind::Nexus),
                url: Some("https://nexus.example/repository/vibe/".into()),
                ..SelfConfig::default()
            },
            Some("/srv/rel"),
        );
        let env = resolve(Some("https://mirror.example/rel"), &both, None).unwrap();
        assert_eq!(env.origin, SourceOrigin::Environment);
        let table = resolve(None, &both, None).unwrap();
        assert_eq!(
            table.source,
            ReleaseSource::Nexus {
                base: "https://nexus.example/repository/vibe".into()
            }
        );
        let drop = resolve(None, &config(SelfConfig::default(), Some("/srv/rel")), None).unwrap();
        assert!(matches!(drop.origin, SourceOrigin::ArtifactsDrop(_)));
        assert_eq!(
            resolve(None, &UserConfig::default(), None).unwrap(),
            EffectiveReleaseSource::default()
        );
        // The canonical origin in the variable is no redirection at all.
        assert_eq!(
            resolve(Some(CANONICAL_RELEASE_ORIGIN), &UserConfig::default(), None).unwrap(),
            EffectiveReleaseSource::default()
        );
    }

    #[test]
    fn incomplete_or_wrong_sources_say_how_to_fix_them() {
        let nexus = config(
            SelfConfig {
                source: Some(ReleaseSourceKind::Nexus),
                ..SelfConfig::default()
            },
            None,
        );
        let error = resolve(None, &nexus, None).unwrap_err().to_string();
        assert!(error.contains("--url"), "{error}");
        let relative = config(
            SelfConfig {
                source: Some(ReleaseSourceKind::Localdir),
                path: Some("rel".into()),
                ..SelfConfig::default()
            },
            None,
        );
        let error = resolve(None, &relative, None).unwrap_err().to_string();
        assert!(error.contains("absolute path"), "{error}");
    }

    #[test]
    fn gitverse_defaults_to_the_public_repository_and_its_api_host() {
        let table = config(
            SelfConfig {
                source: Some(ReleaseSourceKind::Gitverse),
                ..SelfConfig::default()
            },
            None,
        );
        let resolved = resolve(None, &table, None).unwrap();
        assert_eq!(
            resolved.source,
            ReleaseSource::Gitverse {
                web: DEFAULT_GITVERSE_REPOSITORY.into(),
                api: "https://api.gitverse.ru".into(),
                owner: "vibevm".into(),
                repo: "vibevm".into(),
            }
        );
        assert!(resolved.row().is_none());
        let onprem = config(
            SelfConfig {
                source: Some(ReleaseSourceKind::Gitverse),
                url: Some("https://git.corp.example:8443/tools/vibevm/releases".into()),
                ..SelfConfig::default()
            },
            None,
        );
        let ReleaseSource::Gitverse { api, owner, .. } =
            resolve(None, &onprem, None).unwrap().source
        else {
            panic!("gitverse")
        };
        assert_eq!(api, "https://api.git.corp.example:8443");
        assert_eq!(owner, "tools");
    }

    #[test]
    fn the_environment_row_keeps_replacing_the_whole_release_page() {
        let resolved =
            resolve(Some("http://127.0.0.1:1/rel"), &UserConfig::default(), None).unwrap();
        let row = resolved.row().unwrap();
        assert_eq!(row.key.what(), CANONICAL_RELEASE_ORIGIN);
        assert_eq!(row.from.layer, Layer::Invocation);
        let local = resolve(None, &config(SelfConfig::default(), Some("/srv/rel")), None)
            .unwrap()
            .row()
            .unwrap();
        assert_eq!(local.target, RemapTarget::Path(PathBuf::from("/srv/rel")));
        assert_eq!(local.from.source, "[artifacts] vibe");
    }
}
