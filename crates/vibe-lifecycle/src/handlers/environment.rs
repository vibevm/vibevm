//! Script handler environment assembled from the selected provider and context.

use std::path::Path;

use vibe_core::lifecycle::{ExtensionPoint, SlotPoint};
use vibe_wire::generated::lifecycle::e1::context::Context;

use crate::process::minimal_environment;
use crate::{ExtensionProvider, ExtensionRegistryRow, SlotTarget};

pub(super) fn script_env(
    row: &ExtensionRegistryRow,
    target: Option<&SlotTarget>,
    context: &Context,
    context_path: &Path,
    reply: &Path,
) -> std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString> {
    let executable = std::env::current_exe()
        .map(|path| machine_path(&path))
        .unwrap_or_else(|_| "vibe".to_string());
    let (group, name, version, kind, dir) = if let Some(target) = target {
        (
            target.group.clone(),
            target.name.clone(),
            target.version.clone(),
            target.kind.clone(),
            target.root.clone(),
        )
    } else {
        match row.provider() {
            ExtensionProvider::Dependency(provider) => (
                provider.id.group().to_string(),
                provider.id.name().to_string(),
                provider.version.clone(),
                provider.kind.to_string(),
                machine_path(&provider.root),
            ),
            ExtensionProvider::Host(provider) => (
                "__host__".into(),
                provider.identity.to_string(),
                provider.version.clone(),
                provider
                    .kind
                    .map(|kind| kind.to_string())
                    .unwrap_or_else(|| "project".into()),
                machine_path(&provider.root),
            ),
        }
    };
    let hook_phase = match row.declaration().point {
        ExtensionPoint::Slot(SlotPoint::PreInstall) => "pre-install",
        ExtensionPoint::Slot(SlotPoint::PostInstall) => "post-install",
        _ => "phase",
    };
    minimal_environment([
        ("VIBE_PACKAGE_GROUP".into(), group),
        ("VIBE_PACKAGE_NAME".into(), name),
        ("VIBE_PACKAGE_VERSION".into(), version),
        ("VIBE_PACKAGE_KIND".into(), kind),
        ("VIBE_PACKAGE_DIR".into(), dir),
        ("VIBE_HOOK_PHASE".into(), hook_phase.into()),
        ("VIBE_PROJECT_ROOT".into(), context.project.root.clone()),
        ("VIBE_EXECUTABLE".into(), executable),
        ("VIBE_EXTENSION_PROVIDER".into(), row.provider().to_string()),
        ("VIBE_CONTEXT".into(), machine_path(context_path)),
        ("VIBE_REPLY".into(), machine_path(reply)),
    ])
}

fn machine_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
