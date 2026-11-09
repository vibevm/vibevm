//! Whether the source registry has an index that answers, and the client
//! that reads it (PROP-005 `#form-factor`).
//!
//! The mirror prefers an index over every other listing: it carries each
//! version's content hash, so a version the target already holds is
//! decided without fetching a byte. This is the one place that decides
//! whether such an index exists; a `None` sends the caller down the
//! `--package` or organisation-API path instead.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor");

use anyhow::{Result, bail};
use vibe_core::manifest::RegistrySection;
use vibe_registry::{IndexAuth, IndexClient, IndexUrlResolution, IndexUrlSource, ProbeOutcome};

/// The source's index, when one answers (PROP-005 `#form-factor` ladder).
/// A configured `index_url` that does not answer is an error — the
/// operator said where the index is — while the host-derived default
/// simply being absent is not.
pub(super) fn probe_index(
    reg: &RegistrySection,
    operation: &vibe_package_source::SourceOperation,
) -> Result<Option<IndexClient>> {
    let IndexUrlResolution::Url { base, source } = vibe_registry::resolve_index_url(reg) else {
        return Ok(None);
    };
    if source == IndexUrlSource::Default
        && vibe_registry::resolve_registry_auth(
            vibe_registry::RegistryAuthInput::SelectedRegistry(reg),
        )?
        .is_ssh_source()
    {
        return Ok(None);
    }
    let probe = match operation.http_admission()? {
        Some(binding) => {
            binding.check_address(&base)?;
            IndexClient::probe_with_admission(&base, IndexAuth::for_registry(reg, &base)?, binding)
                .map_err(refusal_first)?
        }
        None => IndexClient::probe(&base, IndexAuth::for_registry(reg, &base)?),
    };
    match probe {
        ProbeOutcome::Found(client) => Ok(Some(*client)),
        ProbeOutcome::Absent if source == IndexUrlSource::Default => Ok(None),
        ProbeOutcome::Absent => bail!(
            "the index configured for registry `{}` at `{base}` does not answer \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
             fix: correct `index_url`, or set it to `none` and mirror with `--package`)",
            reg.name
        ),
        ProbeOutcome::Refused { reason } => bail!(
            "the index of registry `{}` refused the probe: {reason}",
            reg.name
        ),
    }
}

/// An index error, with a contact refusal surfaced as itself: the refusal
/// is the operator's own source policy speaking and must not be reworded
/// as "the index did not answer".
pub(super) fn refusal_first(error: vibe_registry::IndexError) -> anyhow::Error {
    match error {
        vibe_registry::IndexError::ContactRefused(refusal) => refusal.into(),
        other => other.into(),
    }
}
