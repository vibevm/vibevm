//! The task packet — the universal seam (plan D7, `schema = 1`).
//!
//! A packet is a versioned TOML document describing one delegated task:
//! what to do, in which workspace, under what budget, routed to which
//! profile/model slot. The packet — not the backend trait — is the
//! future-proof boundary: any future backend (Codex, VibeVM Pixel)
//! consumes packets unchanged.
//!
//! Unknown fields are rejected loudly (`deny_unknown_fields`): packets are
//! human-authored, and a typo that silently drops a budget line is worse
//! than a parse error. Forward compatibility is carried by the explicit
//! `schema` field, not by leniency.

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::RunId;

specmark::scope!("spec://fractality/PROP-001#model");

/// One delegated task, as authored (TOML) and as carried on the bus (JSON).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Packet {
    /// Schema version; this build speaks exactly `1` (D7 golden law).
    pub schema: u32,
    pub task: TaskSpec,
    #[serde(default)]
    pub context: ContextSpec,
    #[serde(default)]
    pub workspace: WorkspaceSpec,
    #[serde(default)]
    pub output: OutputSpec,
    #[serde(default)]
    pub budget: BudgetSpec,
    pub routing: RoutingSpec,
}

/// What the worker is asked to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    /// Short human-facing name; shows up in `fractality ps`.
    pub title: String,
    /// Full, self-contained task text (markdown). The worker sees nothing else.
    pub goal: String,
    /// Commands run in the workspace after the worker finishes; pass/fail is recorded.
    #[serde(default)]
    pub acceptance: Vec<String>,
}

/// Optional context handed to the worker.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSpec {
    /// Files made visible to the worker (via the worktree or copied in).
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    /// D-C3-2 access-list: run-ids whose RESULT files become readable
    /// FileRefs for this child. Default `[]` — a child sees a prior
    /// result **only** when explicitly granted here (the
    /// anti-orchestration-collapse contract, FD-2/FD-3). There is
    /// deliberately no field for a parent's or sibling's transcript:
    /// only named results ever cross the seam (the fold law, RD-5).
    #[serde(default)]
    pub context_from: Vec<RunId>,
}

/// Where the worker works (D8: worktree by default).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSpec {
    #[serde(default)]
    pub mode: WorkspaceMode,
    /// Repository the worktree is cut from (worktree mode).
    #[serde(default = "default_repo")]
    pub repo: String,
    /// Base ref for the worktree branch.
    #[serde(default = "default_base")]
    pub base: String,
}

impl Default for WorkspaceSpec {
    fn default() -> Self {
        Self {
            mode: WorkspaceMode::default(),
            repo: default_repo(),
            base: default_base(),
        }
    }
}

/// Workspace provisioning mode (D8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMode {
    /// `git worktree add` on a fresh branch; the branch is the deliverable.
    #[default]
    Worktree,
    /// A scratch directory; file artifacts are the deliverable.
    Dir,
    /// No provisioned workspace (pure-analysis tasks).
    None,
}

impl WorkspaceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkspaceMode::Worktree => "worktree",
            WorkspaceMode::Dir => "dir",
            WorkspaceMode::None => "none",
        }
    }
}

/// The output contract (D4/D8): what the worker must leave behind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputSpec {
    /// The worker-authored final report, relative to the workspace.
    #[serde(default = "default_result")]
    pub result: String,
    /// Deliverable branch name (worktree mode); defaults to
    /// `fractality/<run-id>` at spawn time when absent.
    #[serde(default)]
    pub branch: Option<String>,
    /// D-C3-2: an optional JSON Schema (raw JSON text) the worker's
    /// structured result is validated against at the collection seam,
    /// with one retry-on-violation (Ф1.2b enforces it). Absent = no
    /// schema gate. Kept as a string so dep-light core needs no JSON
    /// parser; the pod — which already has the runtime — parses and
    /// validates (jsonschema 0.47.0, Ф0 s1). The seam validates
    /// format first, quality second (FD-15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<String>,
}

impl Default for OutputSpec {
    fn default() -> Self {
        Self {
            result: default_result(),
            branch: None,
            output_schema: None,
        }
    }
}

/// Hard budget; exceeding it kills the run with `killed(budget)` —
/// the wall clock and token cap are enforced by mission-control's
/// watchdog (Phase 4), `max_turns` rides the worker invocation. A
/// value of 0 on any axis means "unlimited" for that axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetSpec {
    #[serde(default = "default_wall_secs")]
    pub wall_secs: u64,
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    /// Cumulative output tokens across the run.
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u64,
}

impl Default for BudgetSpec {
    fn default() -> Self {
        Self {
            wall_secs: default_wall_secs(),
            max_turns: default_max_turns(),
            max_output_tokens: default_max_output_tokens(),
        }
    }
}

/// Where the run goes: a profile (D6) and a model slot inside it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingSpec {
    /// Profile name from `~/.fractality/profiles.toml` (D6).
    pub profile: String,
    /// Model slot within the profile: `big`, `small`, … (resolved by the profile).
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_repo() -> String {
    ".".to_owned()
}
fn default_base() -> String {
    "main".to_owned()
}
fn default_result() -> String {
    "result.md".to_owned()
}
fn default_wall_secs() -> u64 {
    1800
}
fn default_max_turns() -> u32 {
    40
}
fn default_max_output_tokens() -> u64 {
    200_000
}
fn default_model() -> String {
    "big".to_owned()
}

impl Packet {
    /// Parses and validates a packet from its authored TOML form.
    pub fn from_toml_str(text: &str) -> Result<Self, CoreError> {
        let packet: Packet = toml::from_str(text)?;
        packet.validate()?;
        Ok(packet)
    }

    /// Renders the packet back to TOML (the run-dir `packet.toml` copy, D4).
    pub fn to_toml_string(&self) -> Result<String, CoreError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Structural validation beyond serde: schema pin and non-empty anchors.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema != 1 {
            return Err(CoreError::PacketSchema { found: self.schema });
        }
        if self.task.title.trim().is_empty() {
            return Err(CoreError::PacketField {
                field: "task.title",
                hint: "give the run a short human-facing name",
            });
        }
        if self.task.goal.trim().is_empty() {
            return Err(CoreError::PacketField {
                field: "task.goal",
                hint: "the goal is the entire task text the worker sees",
            });
        }
        if self.routing.profile.trim().is_empty() {
            return Err(CoreError::PacketField {
                field: "routing.profile",
                hint: "name a profile from profiles.toml (D6)",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_packet_toml() -> &'static str {
        r#"
            schema = 1
            [task]
            title = "t"
            goal = "g"
            [routing]
            profile = "glm"
        "#
    }

    #[test]
    fn minimal_packet_gets_documented_defaults() {
        let p = Packet::from_toml_str(minimal_packet_toml()).expect("parses");
        assert_eq!(p.workspace.mode, WorkspaceMode::Worktree);
        assert_eq!(p.workspace.repo, ".");
        assert_eq!(p.workspace.base, "main");
        assert_eq!(p.output.result, "result.md");
        assert_eq!(p.output.branch, None);
        assert_eq!(p.budget.wall_secs, 1800);
        assert_eq!(p.budget.max_turns, 40);
        assert_eq!(p.budget.max_output_tokens, 200_000);
        assert_eq!(p.routing.model, "big");
        assert!(p.context.files.is_empty());
    }

    #[test]
    fn schema_other_than_one_is_rejected() {
        let text = minimal_packet_toml().replace("schema = 1", "schema = 2");
        let err = Packet::from_toml_str(&text).expect_err("schema 2 must fail");
        assert!(matches!(err, CoreError::PacketSchema { found: 2 }));
    }

    #[test]
    fn unknown_fields_are_rejected_loudly() {
        let text = format!("{}\nbudgett = 3\n", minimal_packet_toml());
        assert!(Packet::from_toml_str(&text).is_err());
    }

    #[test]
    fn empty_goal_is_rejected() {
        let text = minimal_packet_toml().replace("goal = \"g\"", "goal = \"  \"");
        let err = Packet::from_toml_str(&text).expect_err("blank goal must fail");
        assert!(matches!(
            err,
            CoreError::PacketField {
                field: "task.goal",
                ..
            }
        ));
    }

    #[test]
    fn packet_round_trips_through_toml() {
        let p = Packet::from_toml_str(minimal_packet_toml()).expect("parses");
        let rendered = p.to_toml_string().expect("renders");
        let back = Packet::from_toml_str(&rendered).expect("re-parses");
        assert_eq!(p, back);
    }

    /// The example packet shipped in spec/examples is the D7 golden fixture:
    /// if this test breaks, either the schema or the example drifted — fix
    /// whichever one lied.
    #[test]
    fn hello_glm_example_is_a_valid_schema_1_packet() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../spec/examples/hello-glm.toml"
        );
        let text = std::fs::read_to_string(path).expect("example packet exists");
        let p = Packet::from_toml_str(&text).expect("example packet parses");
        insta::assert_debug_snapshot!(p);
    }

    /// D-C3-2 access-list: absent means empty (a child sees no prior
    /// result unless explicitly granted); an explicit list of run-ids
    /// parses into `context_from`.
    #[test]
    fn context_from_access_list_parses_and_defaults_empty() {
        let p = Packet::from_toml_str(minimal_packet_toml()).expect("parses");
        assert!(
            p.context.context_from.is_empty(),
            "no access-list by default — isolation is the default"
        );

        let text = format!(
            "{}\n[context]\ncontext_from = [\"01ARZ3NDEKTSV4RRFFQ69G5FAV\"]\n",
            minimal_packet_toml()
        );
        let p = Packet::from_toml_str(&text).expect("parses with context_from");
        assert_eq!(p.context.context_from.len(), 1);
        assert_eq!(
            p.context.context_from[0].to_string(),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV"
        );
    }

    /// D-C3-2: output_schema is optional (absent by default) and carries
    /// raw JSON Schema text when present — the pod validates against it at
    /// the collection seam (Ф1.2b).
    #[test]
    fn output_schema_is_optional_and_carries_raw_json() {
        let p = Packet::from_toml_str(minimal_packet_toml()).expect("parses");
        assert!(
            p.output.output_schema.is_none(),
            "no schema gate by default"
        );

        let text = format!(
            "{}\n[output]\noutput_schema = '{{\"type\":\"object\"}}'\n",
            minimal_packet_toml()
        );
        let p = Packet::from_toml_str(&text).expect("parses with output_schema");
        assert_eq!(
            p.output.output_schema.as_deref(),
            Some("{\"type\":\"object\"}")
        );
    }
}
