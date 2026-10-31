//! The host facts of step conditions and needs (PROP-065 `#conditions`,
//! `#needs`): PATH and environment lookups made exactly here, at the CLI
//! boundary, and handed to `vibe-app` as functions; the external agent
//! registration (PROP-064) is read here for the same reason.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#conditions");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#external-registration");

/// Whether `name` is a file on PATH (with the usual Windows extensions).
pub(super) fn command_on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    let names: Vec<String> = if cfg!(windows) {
        ["", ".exe", ".cmd", ".bat", ".com"]
            .iter()
            .map(|extension| format!("{name}{extension}"))
            .collect()
    } else {
        vec![name.to_owned()]
    };
    std::env::split_paths(&path).any(|dir| names.iter().any(|name| dir.join(name).is_file()))
}

/// Whether the environment variable `name` is set.
pub(super) fn env_set(name: &str) -> bool {
    std::env::var_os(name).is_some()
}

/// A `path` parameter value made absolute against the working directory
/// (PROP-065 `#parameters`).
pub(super) fn absolute_path(value: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(value);
    if path.is_absolute() {
        return path;
    }
    std::env::current_dir().map_or(path.clone(), |cwd| cwd.join(path))
}

/// The external agent registration of the invoking program
/// (PROP-064 `#external-registration`): `VIBE_AGENT_COMMAND` and the JSON
/// array `VIBE_AGENT_ARGS`, read exactly here.
pub(crate) fn external_agent() -> Result<Option<vibe_app::ExternalAgent>, vibe_app::AgentError> {
    let read = |name: &str| std::env::var(name).ok();
    vibe_app::ExternalAgent::from_values(
        read("VIBE_AGENT_COMMAND").as_deref(),
        read("VIBE_AGENT_ARGS").as_deref(),
    )
}
