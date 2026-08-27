//! Project package-declared skills into coding agents — the compatibility
//! re-export (R7.4 architecture §5). The projection, receipt, recovery and
//! planning implementation moved unchanged into the lower
//! `vibe-agent-projection` crate; `vibe_mcp::pkgskill::*` names exactly the
//! same public items as `vibe_agent_projection::pkgskill::*` for one
//! transition, and no behavior is duplicated here. The identity of both
//! re-export families is pinned by the compatibility test in
//! [`crate::agents`].

pub use vibe_agent_projection::pkgskill::*;
