//! Human/unattended application source-versus-binary selection.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#distribution");

use anyhow::Result;
use dialoguer::Confirm;

use crate::output;

use super::distribution::{DistributionTarget, fetch_index, matching_target, source_differs};
use super::model::ApplicationIdentity;
use super::source::ResolvedApplication;

pub struct SelectedBinary {
    pub target: DistributionTarget,
    pub application: ApplicationIdentity,
}

pub fn select_binary(
    ctx: &output::Context,
    application: &ResolvedApplication,
    from_source: bool,
    offline: bool,
) -> Result<Option<SelectedBinary>> {
    if from_source || offline {
        return Ok(None);
    }
    let Some(locator) = &application.distribution else {
        return Ok(None);
    };
    let Some(index) = fetch_index(locator)? else {
        return Ok(None);
    };
    let Some(target) = matching_target(&index, &application.application)? else {
        return Ok(None);
    };
    if source_differs(target, application.source.as_ref())
        && !ctx.is_json()
        && !ctx.is_unattended()
        && console::user_attended()
    {
        let build_source = Confirm::new()
            .with_prompt(format!(
                "A different source revision is available for `{}`. Build it instead of installing the published {} {} binary?",
                application.application.id, target.os, target.arch
            ))
            .default(false)
            .interact()?;
        if build_source {
            return Ok(None);
        }
    }
    Ok(Some(SelectedBinary {
        target: target.clone(),
        application: index.application,
    }))
}
