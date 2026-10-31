//! The host facts of step conditions and needs (PROP-065 `#conditions`,
//! `#needs`): PATH and environment lookups made exactly here, at the CLI
//! boundary, and handed to `vibe-app` as functions.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#conditions");

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
