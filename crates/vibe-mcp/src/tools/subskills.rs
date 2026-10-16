//! MCP delivery of workspace-owned effective captured subskills.
use super::*;
use vibe_core::manifest::{LockedPackage, Lockfile};
use vibe_workspace::install::effective_subskills::{CapturedSubskill, from_locked_package};

pub(super) fn selected(
    ctx: &ServerContext,
    lock: &Lockfile,
    package: &LockedPackage,
) -> Result<Vec<CapturedSubskill>, ToolError> {
    from_locked_package(&ctx.project_root, lock, package).map_err(ToolError::Internal)
}

pub(super) fn visible(sub: &CapturedSubskill) -> Value {
    let meta = &sub.declaration.subskill;
    json!({"path": meta.path, "delivery": meta.delivery, "description": meta.description,
        "describes": meta.describes, "definition_path": sub.definition_path,
        "definition_digest": sub.definition_digest,
        "content_paths": sub.content.keys().collect::<Vec<_>>(),
        "read_tool": "read_subskill"})
}

pub(super) fn read(package: &str, sub: &CapturedSubskill) -> ToolOutput {
    let mut content = String::new();
    for (path, bytes) in &sub.content {
        content.push_str(&format!(
            "--- {path}\n{}\n\n",
            String::from_utf8_lossy(bytes)
        ));
    }
    ToolOutput::ok(
        json!({"package": package, "subskill_path": sub.declaration.subskill.path,
        "delivery": sub.declaration.subskill.delivery, "description": sub.declaration.subskill.description,
        "definition_digest": sub.definition_digest, "definition_path": sub.definition_path,
        "paths": sub.content.keys().collect::<Vec<_>>(), "content": content}),
    )
}
