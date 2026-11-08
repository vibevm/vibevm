//! Demand snapshot of the existing registry file; unused rows are not validated.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::{GlobalRegistryConfig, GlobalRegistryError};
use crate::manifest::SourcePolicyDecl;
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};
/// Demand-read view of the machine registry configuration.
///
/// ```
/// use vibe_core::{GlobalRegistryConfig, GlobalRegistrySource};
///
/// // An in-memory value is known up front.
/// let ready = GlobalRegistrySource::ready(GlobalRegistryConfig::default());
/// assert!(ready.known().is_some());
/// // A deferred file is read only on demand; an absent file yields `missing`.
/// let deferred = GlobalRegistrySource::deferred(None, GlobalRegistryConfig::default());
/// assert!(deferred.known().is_none());
/// assert!(deferred.catalog().unwrap().registries.is_empty());
/// ```
#[derive(Clone)]
pub struct GlobalRegistrySource(Arc<Input>);
enum Input {
    Ready(GlobalRegistryConfig),
    File {
        path: Option<PathBuf>,
        missing: GlobalRegistryConfig,
        body: OnceLock<Option<String>>,
        gate: Mutex<()>,
    },
}
impl std::fmt::Debug for GlobalRegistrySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GlobalRegistrySource(..)")
    }
}
fn invalid(path: &Path, reason: &'static str) -> GlobalRegistryError {
    GlobalRegistryError::Io {
        path: path.into(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, reason),
    }
}
impl GlobalRegistrySource {
    pub fn ready(value: GlobalRegistryConfig) -> Self {
        Self(Arc::new(Input::Ready(value)))
    }
    /// Capture only address/default data; no filesystem observation occurs here.
    pub fn deferred(path: Option<PathBuf>, missing: GlobalRegistryConfig) -> Self {
        Self(Arc::new(Input::File {
            path,
            missing,
            body: OnceLock::new(),
            gate: Mutex::new(()),
        }))
    }
    pub fn known(&self) -> Option<&GlobalRegistryConfig> {
        match self.0.as_ref() {
            Input::Ready(value) => Some(value),
            _ => None,
        }
    }
    fn body(&self) -> Result<Option<&str>, GlobalRegistryError> {
        let Input::File {
            path, body, gate, ..
        } = self.0.as_ref()
        else {
            return Ok(None);
        };
        if body.get().is_none() {
            let _guard = gate.lock().map_err(|_| {
                invalid(
                    path.as_deref().unwrap_or(Path::new("registry.toml")),
                    "registry snapshot lock poisoned",
                )
            })?;
            if body.get().is_none() {
                let value = match path {
                    None => None,
                    Some(path) => read(path)?,
                };
                let _ = body.set(value);
            }
        }
        Ok(body.get().and_then(Option::as_deref))
    }
    /// Decode only policy, so malformed unused URL/auth rows cannot block contact
    /// to an independently selected locked source. Malformed TOML still refuses.
    pub fn policy(&self) -> Result<SourcePolicyDecl, GlobalRegistryError> {
        let Input::File { path, missing, .. } = self.0.as_ref() else {
            return Ok(self
                .known()
                .map(|g| g.source_policy.clone())
                .unwrap_or_default());
        };
        let Some(body) = self.body()? else {
            return Ok(missing.source_policy.clone());
        };
        #[derive(serde::Deserialize)]
        struct Policy {
            #[serde(default)]
            source_policy: SourcePolicyDecl,
        }
        toml::from_str::<Policy>(body)
            .map(|p| p.source_policy)
            .map_err(|_| {
                invalid(
                    path.as_deref().unwrap_or(Path::new("registry.toml")),
                    "global source-policy document is malformed",
                )
            })
    }
    /// Full catalog decoding is a distinct new-source/index demand.
    pub fn catalog(&self) -> Result<GlobalRegistryConfig, GlobalRegistryError> {
        match self.0.as_ref() {
            Input::Ready(value) => Ok(value.clone()),
            Input::File { path, missing, .. } => match self.body()? {
                None => Ok(missing.clone()),
                Some(body) => toml::from_str(body).map_err(|_| {
                    invalid(
                        path.as_deref().unwrap_or(Path::new("registry.toml")),
                        "global registry catalog is malformed",
                    )
                }),
            },
        }
    }
}
fn read(path: &Path) -> Result<Option<String>, GlobalRegistryError> {
    let fail = |source| GlobalRegistryError::Io {
        path: path.into(),
        source,
    };
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(fail(e)),
    };
    let before = file.metadata().map_err(fail)?;
    if !before.is_file() || before.len() > 4 * 1024 * 1024 {
        return Err(invalid(
            path,
            "registry snapshot requires a bounded regular file",
        ));
    }
    let mut text = String::new();
    (&file)
        .take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut text)
        .map_err(fail)?;
    let after = file.metadata().map_err(fail)?;
    if text.len() > 4 * 1024 * 1024
        || text.len() as u64 != before.len()
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(invalid(path, "registry snapshot changed while reading"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let current = std::fs::metadata(path).map_err(fail)?;
        if (
            before.dev(),
            before.ino(),
            before.ctime(),
            before.ctime_nsec(),
        ) != (after.dev(), after.ino(), after.ctime(), after.ctime_nsec())
            || (after.dev(), after.ino()) != (current.dev(), current.ino())
        {
            return Err(invalid(path, "registry snapshot identity changed"));
        }
    }
    Ok(Some(text))
}

impl From<GlobalRegistryConfig> for GlobalRegistrySource {
    fn from(value: GlobalRegistryConfig) -> Self {
        Self::ready(value)
    }
}
impl Default for GlobalRegistrySource {
    fn default() -> Self {
        Self::ready(GlobalRegistryConfig::default())
    }
}
