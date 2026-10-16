//! Generated D1 payload adapters over the incumbent core lock envelope.
use crate::generated::lock_configuration::e1::{requested::Requested, resolved::Resolved};
use vibe_core::manifest::LockedConfiguration;

pub fn encode(requested: Requested, mut resolved: Resolved) -> Result<LockedConfiguration, String> {
    canonicalize(&mut resolved);
    let requested = serde_json::to_string(&requested).map_err(|e| e.to_string())?;
    let request_digest = LockedConfiguration::digest(&requested);
    resolved.request_digest = request_digest.clone();
    for edge in &mut resolved.edges {
        edge.request_digest = request_digest.clone();
    }
    let record = LockedConfiguration {
        epoch: LockedConfiguration::EPOCH,
        requested,
        request_digest,
        resolved: serde_json::to_string(&resolved).map_err(|e| e.to_string())?,
    };
    decode(&record)?;
    Ok(record)
}

pub fn decode(record: &LockedConfiguration) -> Result<(Requested, Resolved), String> {
    record.validate().map_err(|e| e.to_string())?;
    let requested: Requested =
        serde_json::from_str(&record.requested).map_err(|e| e.to_string())?;
    let resolved: Resolved = serde_json::from_str(&record.resolved).map_err(|e| e.to_string())?;
    let mut canonical = resolved.clone();
    canonicalize(&mut canonical);
    if canonical != resolved {
        return Err("noncanonical resolved ordering".into());
    }
    if serde_json::to_string(&requested).map_err(|e| e.to_string())? != record.requested
        || serde_json::to_string(&resolved).map_err(|e| e.to_string())? != record.resolved
        || resolved.request_digest != record.request_digest
    {
        return Err("noncanonical or mismatched configuration lock request".into());
    }
    if let Some(configuration) = &requested.configuration {
        super::consumer_configuration::decode_request(configuration.as_bytes())?;
        if requested.definitions.is_empty() {
            return Err("configured lock has no authored definitions".into());
        }
    } else if !requested.definitions.is_empty()
        || !requested.defaults.is_empty()
        || !resolved.configurations.is_empty()
    {
        return Err("derived configuration cannot supply authored authority".into());
    }
    for definition in &requested.definitions {
        if definition.identity.source.is_empty()
            || definition.identity.content.is_empty()
            || definition.identity.definition.is_empty()
            || !serde_json::from_str::<serde_json::Value>(&definition.definition)
                .map_err(|e| e.to_string())?
                .is_object()
        {
            return Err("missing exact owner definition identity".into());
        }
    }
    for receipt in &requested.defaults {
        super::consumer_configuration::decode_request(receipt.request.as_bytes())?;
        if requested.configuration.as_ref() != Some(&receipt.request) {
            return Err("default receipt belongs to another request".into());
        }
    }
    for configuration in &resolved.configurations {
        let result = super::consumer_configuration::decode_result(configuration.result.as_bytes())?;
        let request = super::consumer_configuration::decode_request(
            requested
                .configuration
                .as_ref()
                .ok_or("missing authored configuration")?
                .as_bytes(),
        )?;
        let result_edges = serde_json::to_value(&result.edges).map_err(|e| e.to_string())?;
        let request_edges = serde_json::to_value(&request.edges).map_err(|e| e.to_string())?;
        // Each selected package evaluates the original edges of its instance.
        for edge in result_edges.as_array().ok_or("invalid result edges")? {
            if !request_edges
                .as_array()
                .ok_or("invalid request edges")?
                .contains(edge)
            {
                return Err("resolved outcome contains unauthored consumer intent".into());
            }
        }
    }
    for edge in &resolved.edges {
        if edge.request_digest != record.request_digest
            || edge.declaration.is_empty()
            || edge.owner_identity.source.is_empty()
            || edge.owner_identity.content.is_empty()
            || edge.owner_identity.definition.is_empty()
            || edge.provider_identity.source.is_empty()
            || edge.provider_identity.content.is_empty()
            || edge.provider_identity.definition.is_empty()
        {
            return Err("selected edge lost exact declaration/provider/request identity".into());
        }
    }
    Ok((requested, resolved))
}

fn canonicalize(resolved: &mut Resolved) {
    resolved.packages.sort_by(|a, b| a.package.cmp(&b.package));
    resolved.edges.sort_by(|a, b| {
        (&a.parent, &a.declaration, a.alternative, &a.provider).cmp(&(
            &b.parent,
            &b.declaration,
            b.alternative,
            &b.provider,
        ))
    });
    resolved
        .configurations
        .sort_by(|a, b| a.package.cmp(&b.package));
    for p in &mut resolved.packages {
        p.feature_closure.sort();
    }
    for c in &mut resolved.configurations {
        c.dormant
            .sort_by(|a, b| (a.edge, &a.group).cmp(&(b.edge, &b.group)));
        for d in &mut c.dormant {
            d.members.sort();
        }
    }
}
