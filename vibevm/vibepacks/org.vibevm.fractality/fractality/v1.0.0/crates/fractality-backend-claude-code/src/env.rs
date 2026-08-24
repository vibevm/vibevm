//! The environment surface of a headless Claude Code worker.
//!
//! Facts pinned by Phase 0 (plan F2/F3): which variables a clean-slate
//! worker env must carry on this platform, and which variables inject the
//! provider. The D5 constructor (Phase 2) builds envs **from** these
//! whitelists — never by inheriting the parent and subtracting.

specmark::scope!("spec://fractality/PROP-001#invariants");

/// OS-level variables a worker env carries on Windows (F2: omitting
/// `APPDATA`/`LOCALAPPDATA` was the one gap the nested-spawn spike
/// surfaced — Claude Code needs both).
pub const OS_WHITELIST_WINDOWS: &[&str] = &[
    "PATH",
    "HOME",
    "USERPROFILE",
    "TEMP",
    "TMP",
    "SystemRoot",
    "COMSPEC",
    "APPDATA",
    "LOCALAPPDATA",
];

/// OS-level variables a worker env carries on POSIX systems (written
/// portable; this campaign validates Windows only — plan §10).
pub const OS_WHITELIST_POSIX: &[&str] = &["PATH", "HOME", "TMPDIR", "SHELL", "LANG", "LC_ALL"];

/// Provider-config variables the backend injects (F3: the
/// `ANTHROPIC_DEFAULT_*` triple is the model-mapping surface for
/// Anthropic-compatible gateways on Claude Code 2.1.x; the legacy
/// `ANTHROPIC_MODEL`/`ANTHROPIC_SMALL_FAST_MODEL` pair is the wrong
/// surface for this provider).
pub mod provider {
    /// Anthropic-compatible gateway base URL.
    pub const BASE_URL: &str = "ANTHROPIC_BASE_URL";
    /// Bearer the worker authenticates with (read from the profile's
    /// `token_file` at spawn; never logged, never echoed).
    pub const AUTH_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";
    /// Model mapped into the `opus` slot.
    pub const DEFAULT_OPUS_MODEL: &str = "ANTHROPIC_DEFAULT_OPUS_MODEL";
    /// Model mapped into the `sonnet` slot.
    pub const DEFAULT_SONNET_MODEL: &str = "ANTHROPIC_DEFAULT_SONNET_MODEL";
    /// Model mapped into the `haiku` slot (CC-internal small-model traffic).
    pub const DEFAULT_HAIKU_MODEL: &str = "ANTHROPIC_DEFAULT_HAIKU_MODEL";
    /// Isolated per-worker config dir; a fresh one onboards headless with
    /// no interactive step (F4 resolved R5).
    pub const CONFIG_DIR: &str = "CLAUDE_CONFIG_DIR";
}

/// Fractality-context variables every worker receives (D5).
pub mod fractality {
    pub const RUN_ID: &str = "FRACTALITY_RUN_ID";
    pub const DEPTH: &str = "FRACTALITY_DEPTH";
    pub const NODE_ID: &str = "FRACTALITY_NODE_ID";
}

/// Prefixes that must **never** pass from the parent into a worker env
/// (invariant I1). The D5 constructor builds from whitelists, so these
/// exist for the poisoned-parent *assertion*, not for subtraction.
pub const POISON_PREFIXES: &[&str] = &["ANTHROPIC_", "CLAUDE_", "CLAUDECODE"];

#[cfg(test)]
mod tests {
    use super::*;

    /// F2 regression pin: the Windows whitelist must keep the two
    /// variables whose absence broke the nested-spawn spike.
    #[test]
    fn windows_whitelist_carries_appdata_pair() {
        assert!(OS_WHITELIST_WINDOWS.contains(&"APPDATA"));
        assert!(OS_WHITELIST_WINDOWS.contains(&"LOCALAPPDATA"));
    }

    /// The whitelists and the poison list must never intersect — a
    /// variable cannot be both required and forbidden.
    #[test]
    fn whitelists_are_disjoint_from_poison() {
        for name in OS_WHITELIST_WINDOWS.iter().chain(OS_WHITELIST_POSIX) {
            for poison in POISON_PREFIXES {
                assert!(
                    !name.starts_with(poison),
                    "{name} collides with poison prefix {poison}"
                );
            }
        }
    }
}
