//! Passive navigation over a producer's protected same-invocation capture.
use super::*;

impl BootDebuggerRoute {
    pub(super) fn retain_produced(
        &self,
        state: &mut State,
        capture_id: &str,
        produced: hypothetical::ProducedBoot,
    ) -> Answer<String> {
        let (graph, reads, artifact, world) = produced.into_parts();
        let parent = state.snapshots.get(capture_id).ok_or_else(internal)?;
        let id = hash(
            json(&(
                "produced-derivative-boot/1",
                &world,
                &reads,
                &artifact,
                &graph.carrier,
            ))?
            .as_bytes(),
        );
        if state.snapshots.contains_key(&id) {
            return Ok(id);
        }
        let mut resources = vec![];
        let mut retained = BTreeMap::new();
        let mut add = |label: String, bytes: Vec<u8>, basis: r::ResourceBasis| {
            let resource_id = format!("{id}:{}", resources.len());
            resources.push(r::Resource {
                resource_id: resource_id.clone(),
                path: label,
                digest: Some(hash(&bytes)),
                buffer_version: None,
                basis,
                bytes: exact(bytes.len() as u64),
            });
            retained.insert(resource_id.clone(), bytes);
            resource_id
        };
        let artifact_digest = hash(&artifact);
        let artifact_id = add(
            "builtin-emitted-artifact".into(),
            artifact,
            r::ResourceBasis::OriginalBytes,
        );
        let mut sources = vec![];
        let mut records = vec![];
        for (index, source) in graph.sources.iter().enumerate() {
            let identity = match &source.identity {
                vibe_spec::SourceIdentity::Spec(address) => {
                    r::SourceIdentity::Spec(Box::new(r::SourceIdentitySpec {
                        address: address.clone(),
                    }))
                }
                vibe_spec::SourceIdentity::CapturedNode(i) => {
                    r::SourceIdentity::CapturedNode(Box::new(r::SourceIdentityCapturedNode {
                        index: i.to_string(),
                    }))
                }
                vibe_spec::SourceIdentity::StaticEntry { origin, path } => {
                    r::SourceIdentity::StaticEntry(Box::new(r::SourceIdentityStaticEntry {
                        origin: origin.clone(),
                        path: path.clone(),
                    }))
                }
            };
            // Exact compiler-assigned address from that same invocation's
            // worklist read and closure node, never a path/hash inference.
            let read = reads.iter().find(|read| {
                source.identity == vibe_spec::SourceIdentity::Spec(read.address.clone())
            });
            let mut raw_id = None;
            let mut derivative_read = None;
            if let Some(read) = read {
                let binding = read.record.checked_binding().map_err(|_| internal())?;
                let file = binding
                    .local()
                    .files
                    .iter()
                    .find(|f| f.path == read.path)
                    .ok_or_else(internal)?;
                let raw = add(
                    format!("{}:raw:{}", read.record.result_identity, read.path),
                    file.bytes.clone(),
                    r::ResourceBasis::OriginalBytes,
                );
                let projection = add(
                    format!(
                        "{}:selected-markdown:{}",
                        read.record.result_identity, read.address
                    ),
                    read.projection.as_bytes().to_vec(),
                    r::ResourceBasis::CapturedProjection,
                );
                derivative_read = Some(r::DerivativeRead {
                    derivative_result_json: json(&read.record)?,
                    path: read.path.clone(),
                    raw_resource_id: raw.clone(),
                    projection_resource_id: projection,
                    projection_recipe: r::DerivativeReadProjectionRecipe::CapturedVibeSelectionV1,
                    strength: r::DerivativeReadStrength::Observed,
                    current_base: r::DerivativeReadCurrentBase::Unverified,
                });
                raw_id = Some(raw);
                if !records.contains(&read.record) {
                    records.push(read.record.clone());
                }
            }
            sources.push(r::Source {
                source: index as u32,
                identity,
                snapshot_id: id.clone(),
                artifact_resource_id: artifact_id.clone(),
                artifact_digest: artifact_digest.clone(),
                authored_resource_id: raw_id,
                derivative_read,
            });
        }
        let mut nodes = vec![];
        for (c, row) in graph.contributions.iter().enumerate() {
            for (i, node) in row.occurrences.iter().enumerate() {
                nodes.push(r::Node {
                    contribution: c as u32,
                    occurrence: i as u32,
                    source: node.source as u32,
                    path: row.path.clone(),
                    origin: row.origin.clone(),
                    requested: node.requested.clone(),
                    parent: None,
                    root: false,
                    artifact_lines: None,
                });
            }
        }
        let mut gaps = graph.gaps.clone();
        gaps.push("Same-invocation derivative reads are observed retained provenance; current base authentication and raw/projection coordinate equivalence are not asserted.".into());
        let measurement =
            measurement::measure_graph(&id, &graph, &BTreeMap::new(), &BTreeMap::new());
        let snapshot = Snapshot {
            capture: parent.capture.clone(),
            retained,
            resources,
            nodes,
            sources,
            gaps,
            derivatives: (!records.is_empty())
                .then(|| serde_json::to_string(&records).expect("checked records")),
            projection: Some(Projection { graph, measurement }),
        };
        state.snapshots.insert(id.clone(), Arc::new(snapshot));
        Ok(id)
    }
}
