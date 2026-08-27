//! Agent profiles and detection — the compatibility re-export (R7.4
//! architecture §5). The implementation moved unchanged into the lower
//! `vibe-agent-projection` crate so CLI world planning no longer depends
//! upward on the MCP crate; `vibe_mcp::agents::*` names exactly the same
//! public items as `vibe_agent_projection::agents::*` for one transition,
//! and no behavior is duplicated here.

pub use vibe_agent_projection::agents::*;

#[cfg(test)]
mod compat_tests {
    /// Compiles only when both arguments name the same type — a copied
    /// wrapper or a re-declared struct would not unify.
    fn same_type<T>(_: &T, _: &T) {}

    /// Compiles only when both arguments are the same function *item* —
    /// a forwarding wrapper has its own item type and would not unify.
    fn same_item<T>(_: T, _: T) {}

    /// R7.4 §5: the compatibility surface must be the moved items, not
    /// copies. Representative types, functions and constants from both
    /// moved families are pinned through both crate paths.
    #[test]
    fn reexports_are_the_moved_items_not_copies() {
        same_type(
            &crate::agents::Agent::ClaudeCode,
            &vibe_agent_projection::agents::Agent::ClaudeCode,
        );
        same_type(
            &crate::agents::Scope::Project,
            &vibe_agent_projection::agents::Scope::Project,
        );
        same_type(
            &crate::pkgskill::PackageSkillReport {
                skill: "demo".into(),
                agent: "claude".into(),
                scope: "project",
                path: None,
                status: "skipped",
                note: None,
            },
            &vibe_agent_projection::pkgskill::PackageSkillReport {
                skill: "demo".into(),
                agent: "claude".into(),
                scope: "project",
                path: None,
                status: "skipped",
                note: None,
            },
        );
        same_item(
            crate::agents::detect_agents,
            vibe_agent_projection::agents::detect_agents,
        );
        same_item(
            crate::pkgskill::install_package_skill,
            vibe_agent_projection::pkgskill::install_package_skill,
        );
        same_item(
            crate::pkgskill::prepare_declared_skill_projection,
            vibe_agent_projection::pkgskill::prepare_declared_skill_projection,
        );
        assert_eq!(
            crate::pkgskill::PROJECT_SKILL_PREFIX,
            vibe_agent_projection::pkgskill::PROJECT_SKILL_PREFIX
        );
    }
}
