//! Source policy precedes metadata, lifecycle and deferred in-place publication.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY");
use crate::{PlannedInstall, error::Result};
use vibe_core::manifest::SpecFormat;
pub(crate) fn planned(planned: &PlannedInstall, format: SpecFormat) -> Result<()> {
    vibe_workspace::install::validate_source_admission(
        &planned.workspace,
        &planned.resolution,
        format,
    )?;
    for fetched in &planned.fetched {
        if let Some(prepared) = &fetched.in_place {
            vibe_workspace::install::validate_declared_source_operation(
                prepared.manifest(),
                prepared.slot(),
                format,
                true,
            )?;
        }
    }
    Ok(())
}
