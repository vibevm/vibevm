//! The composition root's own red — the offline posture the version
//! manager is handed.
//!
//! Its own cell because the composition root is the assembly of ambient
//! input, and a policy's proofs are a second responsibility from the
//! policy itself. The `[env]` allowlist's proofs sit beside the
//! allowlist, in `env_allowlist`. (It moved here with `vvm_offline` when
//! `main.rs` reached its line budget; the resolver, the variable and the
//! reason below are unchanged.)
use super::vvm_offline;
use rust_ai_native_env_audit::EnvGuard;

/// The version manager is handed a RESOLVED posture, not a raw flag.
///
/// Both lower rungs are ambient reads the vvm domain is forbidden, so if
/// this function did not do them nobody would: the flag alone reached the
/// domain and `vibe --offline self update` went to the network anyway. All
/// three rungs are pinned here, against a user config the test owns, so the
/// answer cannot come from the operator's real machine.
#[test]
fn the_version_manager_is_handed_the_resolved_offline_posture() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config.toml");
    let mut env = EnvGuard::lock();
    env.set("VIBEVM_USER_CONFIG", &config.display().to_string());

    // No rung says offline: online, which stays the default.
    env.unset("VIBE_OFFLINE");
    std::fs::write(&config, "").unwrap();
    assert!(!vvm_offline(false));

    // The flag rung — the one B-160 was about.
    assert!(vvm_offline(true));

    // The environment rung, with no flag.
    env.set("VIBE_OFFLINE", "1");
    assert!(vvm_offline(false));

    // The config rung, with neither flag nor environment.
    env.unset("VIBE_OFFLINE");
    std::fs::write(&config, "[net]\noffline = true\n").unwrap();
    assert!(vvm_offline(false));

    // A config that cannot be parsed does not fail the command here; it
    // loses its rung and the two above it still decide.
    std::fs::write(&config, "[net\n").unwrap();
    assert!(!vvm_offline(false));
    assert!(vvm_offline(true));
}
