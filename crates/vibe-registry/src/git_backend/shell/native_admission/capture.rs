//! Fixed metadata readers preserve selected configuration meaning without execution.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION");
use super::*;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::process::{Command, Stdio};

/// Immutable invocation inputs. Construct before preparing any source; apply
/// never samples fresh ambient environment. Values are deliberately not Debug.
#[derive(Clone)]
pub struct NativeConfigSelection {
    cwd: PathBuf,
    environment: BTreeMap<OsString, OsString>,
}
impl std::fmt::Debug for NativeConfigSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeConfigSelection")
            .field("cwd", &self.cwd)
            .finish_non_exhaustive()
    }
}
impl NativeConfigSelection {
    /// Select an explicit invocation directory and home. Git itself discovers
    /// its compiled system paths and the ordinary global/XDG defaults.
    pub fn new(cwd: PathBuf, home: PathBuf) -> Result<Self, GitError> {
        if !cwd.is_absolute() || !cwd.is_dir() || !home.is_absolute() {
            return Err(unsupported(
                "configuration selection needs absolute cwd and HOME",
            ));
        }
        Ok(Self {
            cwd,
            environment: BTreeMap::from([("HOME".into(), home.into_os_string())]),
        })
    }
    /// Set one captured Git configuration-selection input. Executable,
    /// transport, tracing, loader and pager environment is never accepted.
    pub fn with_variable(mut self, key: &str, value: OsString) -> Result<Self, GitError> {
        if !selection_key(key) {
            return Err(unsupported("unsupported configuration-selection variable"));
        }
        self.environment.insert(key.into(), value);
        Ok(self)
    }
}
fn selection_key(key: &str) -> bool {
    matches!(
        key,
        "HOME"
            | "XDG_CONFIG_HOME"
            | "GIT_CONFIG_SYSTEM"
            | "GIT_CONFIG_GLOBAL"
            | "GIT_CONFIG_NOSYSTEM"
            | "GIT_ATTR_NOSYSTEM"
            | "GIT_CONFIG_PARAMETERS"
            | "GIT_CONFIG_COUNT"
            | "GIT_CONFIG"
    ) || key
        .strip_prefix("GIT_CONFIG_KEY_")
        .or_else(|| key.strip_prefix("GIT_CONFIG_VALUE_"))
        .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct CapturedContext {
    cwd: PathBuf,
    warm: Option<PathBuf>,
    layout: Option<ManagedChild>,
    config: Vec<u8>,
    pub(super) entries: Vec<(String, String)>,
    files: BTreeMap<PathBuf, Option<Vec<u8>>>,
    pub(super) system_attributes: Vec<u8>,
    pub(super) global_attributes: Vec<u8>,
    pub(super) info_attributes: Vec<u8>,
    pub(super) effective_origin: Option<String>,
    pub(super) recorded_origin: Option<String>,
}
impl std::fmt::Debug for CapturedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapturedContext")
            .field("cwd", &self.cwd)
            .finish_non_exhaustive()
    }
}
impl NativeGitAdmission {
    fn metadata(
        &self,
        cwd: &Path,
        args: &[&str],
        git_dir: Option<&Path>,
    ) -> Result<std::process::Output, GitError> {
        let selected = self.selection.as_ref().ok_or_else(|| {
            unsupported("local materialisation requires captured configuration-selection inputs")
        })?;
        let mut command = Command::new(&self.installation.executable);
        command
            .env_clear()
            .envs(&selected.environment)
            // GIT_CONFIG selects a file for `git config` ONLY. It does not
            // replace the config used by remote/check-attr/checkout. This reader
            // observes that operation view, never the config-only override.
            .env_remove("GIT_CONFIG")
            .env("PATH", &self.installation.helpers)
            .env("GIT_EXEC_PATH", &self.installation.helpers)
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ALLOW_PROTOCOL", "")
            .env("GIT_TRACE2", "0")
            .env("GIT_TRACE2_EVENT", "0")
            .env("GIT_TRACE2_PERF", "0")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .current_dir(cwd);
        if let Some(git_dir) = git_dir {
            command.arg("--git-dir").arg(git_dir);
        }
        command.args(args);
        let control = crate::git_backend::transfers::current()
            .progress
            .unwrap_or_default()
            .cancellation()
            .clone();
        let output = crate::git_backend::shell::controlled::output(&mut command, &control)
            .map_err(|error| {
                crate::git_backend::shell::controlled::git_error(
                    error,
                    "captured Git metadata reader".into(),
                )
            })?;
        if output.status.success() || output.status.code() == Some(1) {
            return Ok(output);
        }
        Err(unsupported(
            "selected Git metadata reader failed; context was not captured",
        ))
    }
    pub(super) fn capture_context(&self, warm: Option<&Path>) -> Result<CapturedContext, GitError> {
        self.capture_child_context(warm, None)
    }
    pub(super) fn capture_child_context(
        &self,
        warm: Option<&Path>,
        layout: Option<&ManagedChild>,
    ) -> Result<CapturedContext, GitError> {
        if let Some(warm) = warm {
            validate_selected_directory(warm, layout)?;
        }
        let selected = self.selection.as_ref().ok_or_else(|| {
            unsupported("local materialisation requires captured configuration-selection inputs")
        })?;
        let cwd = warm.unwrap_or(&selected.cwd);
        // Clone does not inherit the invocation repository's local config.
        // An absent owned git-dir suppresses discovery without changing cwd
        // semantics for relative HOME/config selectors.
        let fresh = warm
            .is_none()
            .then(|| self.temporary("vibe-config-read-"))
            .transpose()?;
        let git_dir = fresh.as_ref().map(|root| root.path().join("absent.git"));
        let read = |args: &[&str]| self.metadata(cwd, args, git_dir.as_deref());
        let config = read(&[
            "config",
            "--null",
            "--includes",
            "--show-origin",
            "--show-scope",
            "--list",
        ])?;
        if !config.status.success() {
            return Err(unsupported("configuration view is unavailable"));
        }
        let fields: Vec<_> = config.stdout.split(|b| *b == 0).collect();
        if fields.last() != Some(&b"".as_slice()) || (fields.len() - 1) % 3 != 0 {
            return Err(unsupported("unknown configuration record layout"));
        }
        let mut captured = CapturedContext {
            cwd: cwd.into(),
            warm: warm.map(Path::to_path_buf),
            layout: layout.cloned(),
            config: config.stdout.clone(),
            entries: Vec::new(),
            files: BTreeMap::new(),
            system_attributes: Vec::new(),
            global_attributes: Vec::new(),
            info_attributes: Vec::new(),
            effective_origin: None,
            recorded_origin: None,
        };
        for row in fields[..fields.len() - 1].as_chunks::<3>().0 {
            let origin = std::str::from_utf8(row[1])
                .map_err(|_| unsupported("non-UTF-8 configuration origin"))?;
            if let Some(path) = origin.strip_prefix("file:") {
                captured.read_file(&absolute_from(cwd, path))?;
            } else if origin != "command line:" {
                return Err(unsupported("opaque configuration origin"));
            }
            let value = std::str::from_utf8(row[2])
                .map_err(|_| unsupported("non-UTF-8 configuration value"))?;
            let (key, value) = value.split_once('\n').unwrap_or((value, ""));
            captured.entries.push((key.into(), value.into()));
        }
        captured.validate_layout()?;
        if warm.is_none()
            && captured
                .entries
                .iter()
                .any(|(key, _)| key.starts_with("includeif."))
        {
            return Err(unsupported(
                "fresh conditional includes require a captured final repository context",
            ));
        }
        if warm.is_some() {
            // This is data-only Git URL resolution, not ssh -G or execution
            // of the captured SSH command. An old destination is only observed
            // here; chosen requests separately require exact profile admission.
            let explicit_ssh = if !self.ssh.is_empty()
                || (self.ssh_configuration.is_some() || self.ssh_provider.is_some())
            {
                let effective = read(&["remote", "get-url", "origin"])?;
                let address = text(effective.stdout)?;
                effective.status.success()
                    && (address.starts_with("ssh://")
                        || (!address.contains("://") && address.contains(':')))
            } else {
                false
            };
            captured.validate_warm_transport(explicit_ssh, !self.ssh.is_empty())?;
        }
        for (name, system) in [("GIT_ATTR_SYSTEM", true), ("GIT_ATTR_GLOBAL", false)] {
            let result = read(&["var", name])?;
            let bytes = if result.status.success() {
                let path = text(result.stdout)?;
                if path.contains(['\n', '\r']) {
                    return Err(unsupported("ambiguous attributes path"));
                }
                captured
                    .read_file(&absolute_from(cwd, &path))?
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            if system {
                captured.system_attributes = bytes;
            } else {
                captured.global_attributes = bytes;
            }
        }
        if let Some(warm) = warm {
            let git = selected_git_dir(warm, layout);
            if layout.is_some() {
                captured.read_file(&warm.join(".git"))?;
            }
            captured.info_attributes = captured
                .read_file(&git.join("info/attributes"))?
                .unwrap_or_default();
            let effective = read(&["remote", "get-url", "origin"])?;
            if !effective.status.success() {
                return Err(GitError::OriginMissing { path: warm.into() });
            }
            captured.effective_origin = Some(text(effective.stdout)?);
            captured.recorded_origin = captured
                .entries
                .iter()
                .rev()
                .find(|(k, _)| k == "remote.origin.url")
                .map(|(_, v)| v.clone());
        }
        Ok(captured)
    }
}
fn absolute_from(cwd: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.into()
    } else {
        cwd.join(path)
    }
}
impl CapturedContext {
    pub(super) fn validate_warm_transport(
        &self,
        explicit_ssh: bool,
        explicit_direct: bool,
    ) -> Result<(), GitError> {
        for (key, value) in &self.entries {
            if value.is_empty() {
                continue;
            }
            if explicit_ssh
                && ((explicit_direct && key == "core.sshcommand")
                    || key.starts_with("credential.")
                    || key.starts_with("http.")
                    || (key.starts_with("remote.") && key.ends_with(".proxy")))
            {
                continue;
            }
            if key.starts_with("credential.")
                || key.starts_with("http.")
                || matches!(
                    key.as_str(),
                    "core.askpass" | "core.gitproxy" | "core.sshcommand"
                )
                || (key.starts_with("remote.")
                    && [
                        ".proxy",
                        ".uploadpack",
                        ".receivepack",
                        ".vcs",
                        ".serveroption",
                    ]
                    .iter()
                    .any(|suffix| key.ends_with(suffix)))
            {
                return Err(unsupported(
                    "warm transport settings require a separately admitted proxy, credential, header or TLS context",
                ));
            }
        }
        Ok(())
    }
    fn read_file(&mut self, path: &Path) -> Result<Option<Vec<u8>>, GitError> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(io(error)),
        };
        self.files.insert(path.into(), bytes.clone());
        Ok(bytes)
    }
    fn validate_layout(&self) -> Result<(), GitError> {
        for (key, value) in &self.entries {
            if (key.starts_with("extensions.") && key != "extensions.objectformat")
                || (key == "core.worktree"
                    && !self.layout.as_ref().zip(self.warm.as_ref()).is_some_and(
                        |(layout, warm)| {
                            std::fs::canonicalize(layout.git_dir.join(value)).ok()
                                == std::fs::canonicalize(warm).ok()
                        },
                    ))
                || key == "core.sparsecheckout"
                || key == "core.sparsecheckoutcone"
                || key.ends_with(".promisor")
                || key.ends_with(".partialclonefilter")
                || (key == "core.repositoryformatversion" && value != "0" && value != "1")
            {
                return Err(unsupported(
                    "unsupported repository layout, sparse checkout or promisor context",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn recheck(&self, profile: &NativeGitAdmission) -> Result<(), GitError> {
        if &profile.capture_child_context(self.warm.as_deref(), self.layout.as_ref())? != self {
            return Err(unsupported("captured configuration or attributes changed"));
        }
        Ok(())
    }
}
pub(super) fn validate_selected_directory(
    warm: &Path,
    layout: Option<&ManagedChild>,
) -> Result<(), GitError> {
    if let Some(layout) = layout {
        layout.recheck(warm)?;
    }
    let git = selected_git_dir(warm, layout);
    let m = std::fs::symlink_metadata(&git).map_err(io)?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || git.join("commondir").exists()
        || git.join("objects/info/alternates").exists()
    {
        return Err(unsupported("unsupported warm Git directory layout"));
    }
    for entry in walkdir::WalkDir::new(&git).follow_links(false) {
        let entry = entry.map_err(|_| unsupported("unreadable warm Git metadata"))?;
        if entry.file_type().is_symlink()
            || (!entry.file_type().is_dir() && !entry.file_type().is_file())
            || entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "promisor")
        {
            return Err(unsupported(
                "warm metadata contains unsupported links, special files or promisor objects",
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManagedChild {
    git_dir: PathBuf,
    parent_git: PathBuf,
    name: String,
    locator: Option<materialisation::Tree>,
}
pub(super) fn selected_git_dir(warm: &Path, layout: Option<&ManagedChild>) -> PathBuf {
    layout.map_or_else(|| warm.join(".git"), |layout| layout.git_dir.clone())
}
pub(super) fn managed_child(
    warm: &Path,
    parent_git: &Path,
    name: &str,
) -> Result<Option<ManagedChild>, GitError> {
    if warm.join(".git").is_dir() {
        return Ok(None);
    }
    let layout = ManagedChild {
        git_dir: parent_git.join("modules").join(name),
        parent_git: parent_git.into(),
        name: name.into(),
        locator: materialisation::tree(&warm.join(".git"))?,
    };
    layout.recheck(warm)?;
    Ok(Some(layout))
}
impl ManagedChild {
    fn recheck(&self, warm: &Path) -> Result<(), GitError> {
        validate_child_ancestors(warm)?;
        validate_child_ancestors(&self.git_dir)?;
        if materialisation::tree(&warm.join(".git"))? != self.locator {
            return Err(unsupported("managed child locator changed"));
        }
        let locator = warm.join(".git");
        if !std::fs::symlink_metadata(&locator).map_err(io)?.is_file() {
            return Err(unsupported("managed child locator is not regular"));
        }
        let value = std::fs::read_to_string(&locator).map_err(io)?;
        let target = value
            .strip_prefix("gitdir: ")
            .map(|value| value.trim_end_matches('\n'))
            .filter(|value| !value.contains(['\n', '\r', '\0']))
            .ok_or_else(|| unsupported("managed child locator is malformed"))?;
        if Path::new(target).is_absolute()
            || std::fs::canonicalize(warm.join(target)).map_err(io)?
                != std::fs::canonicalize(&self.git_dir).map_err(io)?
        {
            return Err(unsupported(
                "managed child locator does not name its exact parent module store",
            ));
        }
        Ok(())
    }
}

pub(super) fn validate_child_ancestors(path: &Path) -> Result<(), GitError> {
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(unsupported("managed child metadata crosses a symlink"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io(error)),
        }
    }
    Ok(())
}
