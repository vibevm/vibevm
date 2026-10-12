//! Selected package members are source inputs, not extra in-place node outputs.

use vibe_core::manifest::Manifest;

use super::super::ResolvedDep;

/// Root and ordinary member entrypoints remain published. A non-root authored
/// package member with the selected typed package identity is protected even
/// when its authored version has drifted from the durable selected slot.
pub(super) fn publish_node_boot(
    rel: &str,
    manifest: &Manifest,
    resolution: &[ResolvedDep],
) -> bool {
    if rel == "." {
        return true;
    }
    let Some(package) = manifest.package.as_ref() else {
        return true;
    };
    !resolution.iter().any(|selected| {
        selected.kind == package.kind
            && selected.group == package.group
            && selected.name == package.name.as_str()
    })
}
