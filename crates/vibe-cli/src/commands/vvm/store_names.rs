//! Executable names owned by a VVM instance on this platform.

/// The `vibe` binary's file name on this platform.
pub const BINARY_NAME: &str = if cfg!(windows) { "vibe.exe" } else { "vibe" };
/// The companion `vibe-index` binary's file name on this platform.
pub const INDEX_BINARY_NAME: &str = if cfg!(windows) {
    "vibe-index.exe"
} else {
    "vibe-index"
};
/// Private companion for user-application launch points, kept beside `vibe`.
pub const APP_PROXY_BINARY_NAME: &str = if cfg!(windows) {
    "vibe-app-proxy.exe"
} else {
    "vibe-app-proxy"
};
