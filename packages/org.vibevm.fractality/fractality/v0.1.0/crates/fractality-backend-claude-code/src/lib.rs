//! The Claude Code worker backend (plan D2).
//!
//! Three layers, all pure functions over caller-provided inputs:
//! the provider-facing **facts** as constants ([`env`], pinned by the
//! Phase 0 spikes F2/F3/F4), the D5 clean-slate environment constructor
//! with the poisoned-parent test ([`envbuild`], invariant I1), and the
//! headless invocation builder ([`invocation`]). [`ClaudeCodeBackend`]
//! composes them into the [`fractality_core::WorkerBackend`] seam.
//!
//! Purity is load-bearing: no filesystem, no ambient environment, no
//! clock. The pod (the composition root) reads the token file, snapshots
//! the OS env, and creates the config dir; this crate only computes.

pub mod env;
pub mod envbuild;
pub mod invocation;

use std::collections::BTreeMap;

use fractality_core::profile::Profile;
use fractality_core::worker::{BackendSecrets, RunContext, WorkerBackend, WorkerSpec};
use fractality_core::{CoreError, Packet};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The claude-code backend.
///
/// ```
/// use fractality_backend_claude_code::ClaudeCodeBackend;
/// use fractality_core::worker::WorkerBackend;
///
/// assert_eq!(ClaudeCodeBackend.id(), "claude-code");
/// assert_eq!(ClaudeCodeBackend::ID, "claude-code");
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudeCodeBackend;

impl ClaudeCodeBackend {
    /// Stable backend id, recorded per run.
    pub const ID: &'static str = "claude-code";

    /// Where the isolated `CLAUDE_CONFIG_DIR` lives when the profile says
    /// `auto`: inside the run dir, so it dies with the run's artifacts
    /// and never collides across runs (F4/R5: a fresh dir onboards
    /// headless with no interactive step).
    pub fn config_dir(profile: &Profile, ctx: &RunContext) -> camino::Utf8PathBuf {
        if profile.config_dir == "auto" {
            ctx.run_dir.join("cc-config")
        } else {
            camino::Utf8PathBuf::from(&profile.config_dir)
        }
    }
}

impl WorkerBackend for ClaudeCodeBackend {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn build_spec(
        &self,
        packet: &Packet,
        profile: &Profile,
        secrets: &BackendSecrets,
        os_env: &BTreeMap<String, String>,
        ctx: &RunContext,
    ) -> Result<WorkerSpec, CoreError> {
        let model_id = profile.resolve_model(&packet.routing.model)?;
        let config_dir = Self::config_dir(profile, ctx);
        let env = envbuild::build_worker_env(os_env, profile, secrets, &config_dir, ctx);
        let argv = invocation::build_argv(packet, profile, model_id);
        Ok(WorkerSpec {
            argv,
            cwd: ctx.workspace_dir.clone(),
            stdin: Some(invocation::build_prompt(packet)),
            env,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_spec_composes_env_argv_and_cwd() {
        let profiles = fractality_core::profile::ProfilesFile::from_toml_str(
            r#"
                schema = 1
                [profile.glm]
                backend = "claude-code"
                base_url = "http://gw"
                token_file = "t"
                [profile.glm.models]
                big = "m-big"
                small = "m-small"
                haiku_slot = "m-small"
            "#,
        )
        .expect("profiles parse");
        let profile = profiles.get("glm").expect("glm");
        let packet = Packet::from_toml_str(
            "schema = 1\n[task]\ntitle = \"t\"\ngoal = \"g\"\n[routing]\nprofile = \"glm\"\nmodel = \"small\"\n",
        )
        .expect("packet parses");
        let ctx = RunContext {
            run_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("ulid"),
            run_dir: "runs/x".into(),
            workspace_dir: "runs/x/work".into(),
            depth: 0,
            node_id: "n".into(),
        };
        let spec = ClaudeCodeBackend
            .build_spec(
                &packet,
                profile,
                &BackendSecrets::new("tok".into()),
                &BTreeMap::new(),
                &ctx,
            )
            .expect("spec builds");
        assert_eq!(spec.cwd, camino::Utf8PathBuf::from("runs/x/work"));
        assert!(spec.argv.contains(&"m-small".to_owned()));
        let stdin = spec.stdin.as_deref().expect("prompt rides stdin (F14)");
        assert!(stdin.starts_with('g'), "the goal opens the prompt");
        assert!(stdin.contains("Output contract"));
        assert!(
            !spec.argv.iter().any(|a| a.contains("Output contract")),
            "no argv copy of the prompt"
        );
        // Compare via join: the separator is the platform's business.
        assert_eq!(
            spec.env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
            Some(ctx.run_dir.join("cc-config").as_str()),
            "auto config dir lands inside the run dir"
        );
        assert_eq!(
            spec.env.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str),
            Some("tok")
        );
    }

    #[test]
    fn wrong_model_slot_is_refused_loudly() {
        let profiles = fractality_core::profile::ProfilesFile::from_toml_str(
            r#"
                schema = 1
                [profile.p]
                backend = "claude-code"
                base_url = "http://gw"
                token_file = "t"
                [profile.p.models]
                big = "a"
                small = "b"
                haiku_slot = "b"
            "#,
        )
        .expect("profiles parse");
        let profile = profiles.get("p").expect("p");
        let packet = Packet::from_toml_str(
            "schema = 1\n[task]\ntitle = \"t\"\ngoal = \"g\"\n[routing]\nprofile = \"p\"\nmodel = \"turbo\"\n",
        )
        .expect("packet parses");
        let ctx = RunContext {
            run_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("ulid"),
            run_dir: "r".into(),
            workspace_dir: "w".into(),
            depth: 0,
            node_id: "n".into(),
        };
        let err = ClaudeCodeBackend
            .build_spec(
                &packet,
                profile,
                &BackendSecrets::new("tok".into()),
                &BTreeMap::new(),
                &ctx,
            )
            .expect_err("slot `turbo` must refuse");
        assert!(err.to_string().contains("model slot"));
    }
}
