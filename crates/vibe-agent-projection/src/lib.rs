//! `vibe-agent-projection` — agent-facing package-skill projection and
//! the agent profiles it targets (R7.4 architecture §5).
//!
//! This is the cycle-breaking lower library extracted unchanged from
//! `vibe-mcp`: [`agents`] owns the fixed set of MCP-capable coding agents
//! and their per-(agent, scope) skill paths; [`pkgskill`] owns the
//! package-declared skill projection, strict ownership receipts, recovery,
//! durable staging and the planning types the lifecycle's package phase
//! consumes — scope-general, `Scope::Project` and `Scope::User` alike;
//! the PROJECT-scope package binding and its receipt reconciliation
//! are why CLI world planning depends on this crate instead of the MCP
//! crate, which would cycle once MCP consumes the orchestrator.
//!
//! `vibe-mcp` keeps compatibility re-exports at `vibe_mcp::{agents,
//! pkgskill}` for one transition; no behavior duplicated, no deployment
//! story added — portable client artifacts and explicit deploy remain R8.
//!
//! ## What does not live here
//!
//! MCP transport, JSON-RPC, tools, the MCP-entry config writers
//! (`agent_config.rs`, `pkg_servers.rs`), the agentic relay, and the
//! SKILL.md template writer all stay in `vibe-mcp`.

#![forbid(unsafe_code)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");

pub mod agents;
pub mod pkgskill;

/// Project a diff outcome (`created` / `updated` / `unchanged`) onto its
/// dry-run preview: a create or update becomes its `would-*` form; anything
/// else is reported as-is. The SKILL.md writer and the package-skill
/// projector (PROP-018 §2.6) share this so the dry-run lifecycle vocabulary
/// lives in one place rather than being re-spelled per writer.
///
/// ```
/// use vibe_agent_projection::preview_status;
/// // A dry run previews a mutation; a real run reports it as-is.
/// assert_eq!(preview_status("created", true), "would-create");
/// assert_eq!(preview_status("updated", true), "would-update");
/// assert_eq!(preview_status("created", false), "created");
/// assert_eq!(preview_status("unchanged", true), "unchanged");
/// ```
#[doc(hidden)]
pub fn preview_status(base: &'static str, dry_run: bool) -> &'static str {
    match (base, dry_run) {
        ("created", true) => "would-create",
        ("updated", true) => "would-update",
        (s, _) => s,
    }
}
