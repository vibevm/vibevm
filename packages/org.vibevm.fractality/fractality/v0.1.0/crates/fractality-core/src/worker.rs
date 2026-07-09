//! The worker seam: the spec a pod executes, and the backend trait that
//! produces it (plan D2/D3).
//!
//! A [`WorkerSpec`] is a **complete** description of the child process —
//! argv, the *entire* environment, cwd. The pod spawns with
//! `env_clear()` + exactly this map, so invariant I1 (a worker never
//! inherits `ANTHROPIC_*` / `CLAUDE_*` from the parent) holds
//! structurally: there is no inherit-and-override path to forget.
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

/// Per-run facts a backend needs beyond the packet. Phase 2 extends this
/// with the resolved profile (D6); the shape is additive by design.
#[derive(Debug, Clone, PartialEq)]
pub struct RunContext {
    pub run_id: RunId,
    pub run_dir: Utf8PathBuf,
    /// Nesting depth (0 = boss-spawned); rides into the worker env as
    /// `FRACTALITY_DEPTH` (Phase 4).
    pub depth: u32,
    pub node_id: String,
}

/// Turns a task packet plus run context into a concrete worker spec.
/// The first implementation is the `claude-code` backend (Phase 2).
///
/// The canonical implementation shape:
///
/// ```
/// use fractality_core::worker::{RunContext, WorkerBackend, WorkerSpec};
/// use fractality_core::{CoreError, Packet};
///
/// struct EchoBackend;
///
/// impl WorkerBackend for EchoBackend {
///     fn id(&self) -> &'static str {
///         "echo"
///     }
///     fn build_spec(&self, _packet: &Packet, ctx: &RunContext) -> Result<WorkerSpec, CoreError> {
///         Ok(WorkerSpec {
///             argv: vec!["echo".into(), ctx.run_id.to_string()],
///             cwd: ctx.run_dir.clone(),
///             env: Default::default(),
///         })
///     }
/// }
///
/// let ctx = RunContext {
///     run_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("fixed ulid"),
///     run_dir: "runs/x".into(),
///     depth: 0,
///     node_id: "node-1".into(),
/// };
/// let packet = Packet::from_toml_str(
///     "schema = 1\n[task]\ntitle = \"t\"\ngoal = \"g\"\n[routing]\nprofile = \"glm\"\n",
/// )
/// .expect("packet parses");
/// let spec = EchoBackend.build_spec(&packet, &ctx).expect("spec builds");
/// assert_eq!(spec.argv[0], "echo");
/// ```
pub trait WorkerBackend: Send + Sync {
    /// Stable backend id (`"claude-code"`, …); recorded per run.
    fn id(&self) -> &'static str;

    /// Builds the complete child-process spec for one run.
    fn build_spec(&self, packet: &Packet, ctx: &RunContext) -> Result<WorkerSpec, CoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_spec_round_trips_and_validates() {
        let text = r#"
            argv = ["claude", "-p", "hello"]
            cwd = "C:/work/run-1"
            [env]
            PATH = "C:/bin"
        "#;
        let spec = WorkerSpec::from_toml_str(text).expect("parses");
        assert_eq!(spec.argv[0], "claude");
        assert_eq!(spec.env.get("PATH").map(String::as_str), Some("C:/bin"));
        let back =
            WorkerSpec::from_toml_str(&spec.to_toml_string().expect("renders")).expect("re-parses");
        assert_eq!(spec, back);
    }

    #[test]
    fn empty_argv_is_rejected() {
        let text = "argv = []\ncwd = \"x\"\n";
        assert!(WorkerSpec::from_toml_str(text).is_err());
    }
}
