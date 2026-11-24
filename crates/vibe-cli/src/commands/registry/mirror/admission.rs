//! Resolve and admit a mirror source before the target or transport is opened.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");

use anyhow::{Result, anyhow};
use vibe_core::manifest::{Manifest, RegistrySection};
use vibe_publish::git_publish::redact_credentials;

/// Find the configured source and apply the same effective policy as registry sync.
/// This is an early enumeration check, not a replacement for transport admission.
pub(super) fn resolve(
    requested: &str,
    registries: &[RegistrySection],
    manifest: &Manifest,
) -> Result<RegistrySection> {
    let source = registries
        .iter()
        .find(|registry| registry.name == requested)
        .ok_or_else(|| {
            let requested = shown(requested);
            let configured = if registries.is_empty() {
                "(none)".to_owned()
            } else {
                registries
                    .iter()
                    .map(|registry| shown(&registry.name))
                    .collect::<Vec<_>>()
                    .join(" · ")
            };
            anyhow!(
                "registry_not_found: no registry named {requested} \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
                 fix: `vibe registry add <name> {requested}`): configured: {configured}"
            )
        })?;
    let policy = super::super::effective_source_policy(Some(manifest))?;
    if let Some(selector) = policy.excluded_by(&source.url) {
        let name = shown(&source.name);
        let endpoint = shown(&source.url);
        let selector = shown(&selector.to_string());
        return Err(anyhow!(
            "source_policy_excluded: registry {name} is disabled by the source policy \
             [hosting {selector}] \
             (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY; \
             fix: `vibe registry list`): mirroring it would read {endpoint}, \
             which exclude_hosting forbids"
        ));
    }
    Ok(source.clone())
}

/// Keep authored URLs and names inert in terminal output and error records.
fn shown(value: &str) -> String {
    redact_credentials(value)
        .chars()
        .flat_map(|character| {
            if character.is_control() || character == '`' {
                character.escape_default().collect::<Vec<_>>()
            } else {
                vec![character]
            }
        })
        .collect()
}
