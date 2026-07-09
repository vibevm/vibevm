//! The D5 clean-slate environment constructor (invariant I1).
//!
//! A worker env is **built from scratch**: an OS whitelist copied out of
//! a caller-provided snapshot, then the provider injections, then the
//! profile's extra env, then the fractality context. There is no
//! inherit-and-override path — one forgotten variable cannot silently
//! route a swarm to the boss's subscription.
//!
//! Pure function over the snapshot: the composition root (the pod's
//! `main`) captures `std::env::vars()` once and passes it in, which is
//! also exactly what makes the poisoned-parent test below a plain unit
//! test (CI-grade, not optional — D5).

use std::collections::BTreeMap;

use camino::Utf8Path;
use fractality_core::profile::Profile;
use fractality_core::worker::{BackendSecrets, RunContext};

use crate::env::{OS_WHITELIST_POSIX, OS_WHITELIST_WINDOWS, fractality, provider};

specmark::scope!("spec://fractality/PROP-001#invariants");

/// Builds the complete worker environment.
///
/// Layering (later layers may override earlier ones, deliberately):
/// OS whitelist → provider config → `profile.env` extras → fractality
/// context. The profile extras layer lets a profile tune provider knobs
/// (`API_TIMEOUT_MS`, telemetry switches) without code changes (D6).
///
/// ```
/// use std::collections::BTreeMap;
///
/// use fractality_backend_claude_code::envbuild::build_worker_env;
/// use fractality_core::profile::ProfilesFile;
/// use fractality_core::worker::{BackendSecrets, RunContext};
///
/// let profiles = ProfilesFile::from_toml_str(
///     "schema = 1\n[profile.glm]\nbackend = \"claude-code\"\nbase_url = \"http://gw\"\ntoken_file = \"t\"\n[profile.glm.models]\nbig = \"m-big\"\nsmall = \"m-small\"\nhaiku_slot = \"m-small\"\n",
/// )
/// .expect("profiles parse");
/// let profile = profiles.get("glm").expect("glm");
/// let ctx = RunContext {
///     run_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("ulid"),
///     run_dir: "runs/x".into(),
///     workspace_dir: "runs/x/work".into(),
///     depth: 0,
///     node_id: "node-1".into(),
/// };
/// let mut os_env = BTreeMap::new();
/// os_env.insert("PATH".to_owned(), "C:/bin".to_owned());
/// os_env.insert("ANTHROPIC_API_KEY".to_owned(), "parent-poison".to_owned());
///
/// let env = build_worker_env(
///     &os_env,
///     profile,
///     &BackendSecrets::new("worker-bearer".into()),
///     camino::Utf8Path::new("runs/x/cc-config"),
///     &ctx,
/// );
/// assert_eq!(env.get("PATH").map(String::as_str), Some("C:/bin"));
/// assert_eq!(env.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str), Some("worker-bearer"));
/// assert_eq!(env.get("ANTHROPIC_API_KEY"), None, "parent poison never crosses");
/// ```
pub fn build_worker_env(
    os_env: &BTreeMap<String, String>,
    profile: &Profile,
    secrets: &BackendSecrets,
    config_dir: &Utf8Path,
    ctx: &RunContext,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();

    let whitelist: &[&str] = if cfg!(windows) {
        OS_WHITELIST_WINDOWS
    } else {
        OS_WHITELIST_POSIX
    };
    for name in whitelist {
        // Windows env names are case-insensitive and the OS block carries
        // mixed casings (`Path`, `ComSpec` on a stock install) — match
        // accordingly and copy under the whitelist's canonical name, or a
        // PowerShell-launched pod silently hands its worker no PATH (F14).
        // POSIX names are case-sensitive identifiers; match exactly.
        let value = if cfg!(windows) {
            os_env
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v)
        } else {
            os_env.get(*name)
        };
        if let Some(value) = value {
            env.insert((*name).to_owned(), value.clone());
        }
    }

    env.insert(provider::BASE_URL.to_owned(), profile.base_url.clone());
    env.insert(provider::AUTH_TOKEN.to_owned(), secrets.token().to_owned());
    env.insert(
        provider::DEFAULT_OPUS_MODEL.to_owned(),
        profile.models.big.clone(),
    );
    env.insert(
        provider::DEFAULT_SONNET_MODEL.to_owned(),
        profile.models.big.clone(),
    );
    env.insert(
        provider::DEFAULT_HAIKU_MODEL.to_owned(),
        profile.models.haiku_slot.clone(),
    );
    env.insert(provider::CONFIG_DIR.to_owned(), config_dir.to_string());

    for (key, value) in &profile.env {
        env.insert(key.clone(), value.clone());
    }

    env.insert(fractality::RUN_ID.to_owned(), ctx.run_id.to_string());
    env.insert(fractality::DEPTH.to_owned(), ctx.depth.to_string());
    env.insert(fractality::NODE_ID.to_owned(), ctx.node_id.clone());

    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::POISON_PREFIXES;

    fn fixture_profile() -> fractality_core::profile::ProfilesFile {
        fractality_core::profile::ProfilesFile::from_toml_str(
            r#"
                schema = 1
                [profile.glm]
                backend = "claude-code"
                base_url = "https://api.z.ai/api/anthropic"
                token_file = "~/.vibevm/zai.api.token"
                [profile.glm.models]
                big = "glm-5.2[1m]"
                small = "glm-5-turbo"
                haiku_slot = "glm-5-turbo"
                [profile.glm.env]
                API_TIMEOUT_MS = "3000000"
            "#,
        )
        .expect("fixture profiles parse")
    }

    fn fixture_ctx() -> RunContext {
        RunContext {
            run_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("ulid"),
            run_dir: "runs/x".into(),
            workspace_dir: "runs/x/work".into(),
            depth: 2,
            node_id: "node-1".into(),
        }
    }

    /// The D5 poisoned-parent test: a parent environment carrying every
    /// poison shape must not leak a single variable into the worker env.
    #[test]
    fn poisoned_parent_env_never_reaches_the_worker() {
        let profiles = fixture_profile();
        let profile = profiles.get("glm").expect("glm");
        let mut os_env = BTreeMap::new();
        os_env.insert("PATH".to_owned(), "C:/bin".to_owned());
        os_env.insert("TEMP".to_owned(), "C:/t".to_owned());
        os_env.insert("ANTHROPIC_API_KEY".to_owned(), "poison-key".to_owned());
        os_env.insert("ANTHROPIC_BASE_URL".to_owned(), "poison-url".to_owned());
        os_env.insert("ANTHROPIC_AUTH_TOKEN".to_owned(), "poison-token".to_owned());
        os_env.insert("CLAUDE_CONFIG_DIR".to_owned(), "poison-dir".to_owned());
        os_env.insert("CLAUDECODE_FLAG".to_owned(), "poison-flag".to_owned());
        os_env.insert("SOME_RANDOM_VAR".to_owned(), "not-whitelisted".to_owned());

        let env = build_worker_env(
            &os_env,
            profile,
            &BackendSecrets::new("real-worker-token".into()),
            Utf8Path::new("runs/x/cc-config"),
            &fixture_ctx(),
        );

        // Whitelisted OS vars pass; non-whitelisted never do.
        assert_eq!(env.get("PATH").map(String::as_str), Some("C:/bin"));
        assert_eq!(env.get("SOME_RANDOM_VAR"), None);

        // Every provider-shaped value comes from the profile/secrets,
        // never from the parent.
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://api.z.ai/api/anthropic")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str),
            Some("real-worker-token")
        );
        assert_eq!(
            env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
            Some("runs/x/cc-config")
        );
        assert_eq!(env.get("ANTHROPIC_API_KEY"), None);
        assert_eq!(env.get("CLAUDECODE_FLAG"), None);

        // No poisoned parent VALUE survives anywhere in the map.
        for value in env.values() {
            assert!(!value.starts_with("poison"), "leaked parent value: {value}");
        }
        // Poison-prefixed NAMES exist only where we deliberately inject.
        for name in env.keys() {
            let injected = name.starts_with("ANTHROPIC_DEFAULT_")
                || name == "ANTHROPIC_BASE_URL"
                || name == "ANTHROPIC_AUTH_TOKEN"
                || name == "CLAUDE_CONFIG_DIR";
            if POISON_PREFIXES.iter().any(|p| name.starts_with(p)) {
                assert!(injected, "unexpected poison-prefixed name: {name}");
            }
        }
    }

    /// F14 regression pin: a stock Windows environment spells the two
    /// load-bearing names `Path` and `ComSpec`; the whitelist copy must
    /// match case-insensitively and canonicalize, or the worker loses its
    /// PATH whenever the pod is launched outside bash.
    #[cfg(windows)]
    #[test]
    fn windows_env_casing_is_matched_and_canonicalized() {
        let profiles = fixture_profile();
        let profile = profiles.get("glm").expect("glm");
        let mut os_env = BTreeMap::new();
        os_env.insert("Path".to_owned(), "C:/real/bin".to_owned());
        os_env.insert(
            "ComSpec".to_owned(),
            "C:/Windows/system32/cmd.exe".to_owned(),
        );

        let env = build_worker_env(
            &os_env,
            profile,
            &BackendSecrets::new("t".into()),
            Utf8Path::new("cc"),
            &fixture_ctx(),
        );
        assert_eq!(env.get("PATH").map(String::as_str), Some("C:/real/bin"));
        assert_eq!(
            env.get("COMSPEC").map(String::as_str),
            Some("C:/Windows/system32/cmd.exe")
        );
        assert_eq!(env.get("Path"), None, "canonical name only — no duplicates");
    }

    #[test]
    fn layering_provider_then_profile_extras_then_context() {
        let profiles = fixture_profile();
        let profile = profiles.get("glm").expect("glm");
        let env = build_worker_env(
            &BTreeMap::new(),
            profile,
            &BackendSecrets::new("t".into()),
            Utf8Path::new("cc"),
            &fixture_ctx(),
        );
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_OPUS_MODEL").map(String::as_str),
            Some("glm-5.2[1m]")
        );
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL").map(String::as_str),
            Some("glm-5-turbo")
        );
        assert_eq!(
            env.get("API_TIMEOUT_MS").map(String::as_str),
            Some("3000000"),
            "profile extras land verbatim"
        );
        assert_eq!(env.get("FRACTALITY_DEPTH").map(String::as_str), Some("2"));
        assert_eq!(
            env.get("FRACTALITY_RUN_ID").map(String::as_str),
            Some("01ARZ3NDEKTSV4RRFFQ69G5FAV")
        );
    }
}
