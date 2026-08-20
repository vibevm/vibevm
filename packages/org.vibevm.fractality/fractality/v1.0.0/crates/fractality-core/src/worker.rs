//! The worker seam: the spec a pod executes, and the backend trait that
//! produces it (plan D2/D3).
//!
//! A [`WorkerSpec`] is a **complete** description of the child process —
//! argv, the *entire* environment, cwd, and an optional stdin payload.
//! The pod spawns with `env_clear()` + exactly this map, so invariant I1
//! (a worker never inherits `ANTHROPIC_*` / `CLAUDE_*` from the parent)
//! holds structurally: there is no inherit-and-override path to forget.
//!
//! The packet — not this trait — is the future-proof seam (D7): backends
//! for other tools consume packets unchanged.

use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::RunId;
use crate::packet::Packet;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Complete child-process description handed to a pod.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerSpec {
    /// `argv[0]` is the program; the rest are its arguments.
    pub argv: Vec<String>,
    /// Working directory of the child.
    pub cwd: Utf8PathBuf,
    /// Payload the pod writes to the child's stdin, then closes the pipe
    /// (`None` = stdin is null). The claude-code backend feeds the prompt
    /// here rather than as a positional argument: Windows command lines
    /// cap at 32 KiB, and `.cmd`-shim spawns forbid newlines in arguments
    /// — both fatal to big one-shot task texts (F14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    /// The whole environment. Nothing else reaches the child (I1).
    /// Declared last: TOML wants scalars before tables.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl WorkerSpec {
    pub fn from_toml_str(text: &str) -> Result<Self, CoreError> {
        let spec: WorkerSpec = toml::from_str(text)?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn to_toml_string(&self) -> Result<String, CoreError> {
        self.validate()?;
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.argv.is_empty() || self.argv[0].trim().is_empty() {
            return Err(CoreError::WorkerSpec {
                message: "argv must name a program (argv[0] empty)".to_owned(),
            });
        }
        Ok(())
    }
}

/// The pod's product-path input (Phase 2): where the run lives and how
/// deep it nests. Deliberately tiny — the packet stays in the run dir's
/// own `packet.toml` (D4) and profiles stay in the home's
/// `profiles.toml`, so nothing here duplicates and nothing here is
/// secret.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSpec {
    /// Spec schema; this build writes and reads `1`.
    pub schema: u32,
    pub run_id: RunId,
    pub run_dir: Utf8PathBuf,
    /// Where the worker works (worktree, scratch dir, or the run dir).
    pub workspace_dir: Utf8PathBuf,
    /// Nesting depth (0 = boss-spawned).
    pub depth: u32,
    pub node_id: String,
}

impl RunSpec {
    pub const FILE_NAME: &'static str = "run-spec.toml";

    pub fn from_toml_str(text: &str) -> Result<Self, CoreError> {
        let spec: RunSpec = toml::from_str(text)?;
        if spec.schema != 1 {
            return Err(CoreError::WorkerSpec {
                message: format!(
                    "run-spec schema {} is not supported (this build speaks 1)",
                    spec.schema
                ),
            });
        }
        Ok(spec)
    }

    pub fn to_toml_string(&self) -> Result<String, CoreError> {
        Ok(toml::to_string_pretty(self)?)
    }
}

/// A worker credential in transit. Wrapped so it cannot leak through
/// `Debug`/`Display` formatting — the one place a token becomes text is
/// the env map the backend constructs (secrets hygiene, workspace
/// contract).
#[derive(Clone)]
pub struct BackendSecrets {
    token: String,
}

impl BackendSecrets {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    /// The raw bearer. Callers write it into the worker env and nowhere
    /// else — never into logs, specs on disk, or error messages.
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl std::fmt::Debug for BackendSecrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BackendSecrets([redacted])")
    }
}

/// Per-run facts a backend needs beyond the packet and the profile.
#[derive(Debug, Clone, PartialEq)]
pub struct RunContext {
    pub run_id: RunId,
    pub run_dir: Utf8PathBuf,
    /// The worker's working directory (worktree / scratch dir / run dir).
    pub workspace_dir: Utf8PathBuf,
    /// Nesting depth (0 = boss-spawned); rides into the worker env as
    /// `FRACTALITY_DEPTH`.
    pub depth: u32,
    pub node_id: String,
}

/// Turns a task packet plus profile, credentials, and run context into a
/// concrete worker spec. The first implementation is the `claude-code`
/// backend.
///
/// The `os_env` parameter is a snapshot captured at the caller's
/// composition root — backends stay pure functions over it (no ambient
/// reads), which is also what makes the I1 poisoned-parent test a plain
/// unit test.
///
/// The canonical implementation shape:
///
/// ```
/// use std::collections::BTreeMap;
///
/// use fractality_core::profile::{Profile, ProfilesFile};
/// use fractality_core::worker::{BackendSecrets, RunContext, WorkerBackend, WorkerSpec};
/// use fractality_core::{CoreError, Packet};
///
/// struct EchoBackend;
///
/// impl WorkerBackend for EchoBackend {
///     fn id(&self) -> &'static str {
///         "echo"
///     }
///     fn build_spec(
///         &self,
///         _packet: &Packet,
///         _profile: &Profile,
///         _secrets: &BackendSecrets,
///         _os_env: &BTreeMap<String, String>,
///         ctx: &RunContext,
///     ) -> Result<WorkerSpec, CoreError> {
///         Ok(WorkerSpec {
///             argv: vec!["echo".into(), ctx.run_id.to_string()],
///             cwd: ctx.workspace_dir.clone(),
///             stdin: None,
///             env: Default::default(),
///         })
///     }
/// }
///
/// let profiles = ProfilesFile::from_toml_str(
///     "schema = 1\n[profile.glm]\nbackend = \"claude-code\"\nbase_url = \"http://x\"\ntoken_file = \"t\"\n[profile.glm.models]\nbig = \"m1\"\nsmall = \"m2\"\nhaiku_slot = \"m2\"\n",
/// )
/// .expect("profiles parse");
/// let profile = profiles.get("glm").expect("glm exists");
/// let ctx = RunContext {
///     run_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("fixed ulid"),
///     run_dir: "runs/x".into(),
///     workspace_dir: "runs/x/work".into(),
///     depth: 0,
///     node_id: "node-1".into(),
/// };
/// let packet = Packet::from_toml_str(
///     "schema = 1\n[task]\ntitle = \"t\"\ngoal = \"g\"\n[routing]\nprofile = \"glm\"\n",
/// )
/// .expect("packet parses");
/// let secrets = BackendSecrets::new("bearer".into());
/// let spec = EchoBackend
///     .build_spec(&packet, profile, &secrets, &BTreeMap::new(), &ctx)
///     .expect("spec builds");
/// assert_eq!(spec.argv[0], "echo");
/// assert_eq!(format!("{secrets:?}"), "BackendSecrets([redacted])");
/// ```
pub trait WorkerBackend: Send + Sync {
    /// Stable backend id (`"claude-code"`, …); recorded per run.
    fn id(&self) -> &'static str;

    /// Builds the complete child-process spec for one run.
    fn build_spec(
        &self,
        packet: &Packet,
        profile: &crate::profile::Profile,
        secrets: &BackendSecrets,
        os_env: &BTreeMap<String, String>,
        ctx: &RunContext,
    ) -> Result<WorkerSpec, CoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_spec_round_trips_and_validates() {
        let text = r#"
            argv = ["claude", "--print"]
            cwd = "C:/work/run-1"
            stdin = "multi\nline\nprompt"
            [env]
            PATH = "C:/bin"
        "#;
        let spec = WorkerSpec::from_toml_str(text).expect("parses");
        assert_eq!(spec.argv[0], "claude");
        assert_eq!(spec.stdin.as_deref(), Some("multi\nline\nprompt"));
        assert_eq!(spec.env.get("PATH").map(String::as_str), Some("C:/bin"));
        let back =
            WorkerSpec::from_toml_str(&spec.to_toml_string().expect("renders")).expect("re-parses");
        assert_eq!(spec, back);
    }

    #[test]
    fn stdin_is_optional_and_defaults_to_none() {
        let text = "argv = [\"x\"]\ncwd = \"w\"\n";
        let spec = WorkerSpec::from_toml_str(text).expect("parses");
        assert_eq!(spec.stdin, None);
        assert!(
            !spec.to_toml_string().expect("renders").contains("stdin"),
            "absent stdin never serializes"
        );
    }

    #[test]
    fn empty_argv_is_rejected() {
        let text = "argv = []\ncwd = \"x\"\n";
        assert!(WorkerSpec::from_toml_str(text).is_err());
    }
}
