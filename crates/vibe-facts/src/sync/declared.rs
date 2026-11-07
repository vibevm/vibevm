//! All cited native sources are captured and checked before registry publication.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-SYNC",
    r = 1
);
use super::*;
use vibe_spec::source_policy::PackagePolicyObservation;

pub(super) fn load(
    policy: PackagePolicyObservation,
    needed: &BTreeSet<String>,
    registry: &Registry,
) -> Result<SpecSnapshot, RegistryError> {
    let consumer = policy
        .manifest()
        .and_then(|manifest| manifest.consumer_node())
        .ok_or_else(|| policy.unsupported("host sync without consumer identity"))?;
    let group = consumer
        .group
        .ok_or_else(|| policy.unsupported("host sync without self coordinate group"))?;
    let coordinate = format!("{group}/{}", consumer.name);
    for entry in registry
        .entries()
        .filter(|entry| entry.origin == FactOrigin::Spec)
    {
        if !entry.address.starts_with(&format!("spec://{coordinate}/")) {
            return Err(policy
                .unsupported("host sync citation names another coordinate")
                .into());
        }
    }
    let captured = needed
        .iter()
        .map(|document| policy.capture_cited_native(&coordinate, document))
        .collect::<Result<Vec<_>, _>>()?;
    let mut snapshot = BTreeMap::new();
    for source in &captured {
        let text = vibe_spec::project_declared_native(
            source.observation.bytes(),
            &source.media,
            vibe_core::manifest::SourceInterpretation::VibeSpec,
        )
        .map_err(|message| RegistryError::SpecParse {
            path: source.relative.clone().into(),
            line: 0,
            message,
        })?;
        collect_statuses(&coordinate, &source.relative, &text, &mut snapshot)?;
    }
    Ok(SpecSnapshot {
        facts: snapshot,
        policy,
        captured,
    })
}
