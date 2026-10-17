//! Normal workspace manifest -> original B2 consumer intent.
use crate::InstallSource;
use vibe_core::manifest::Manifest;
use vibe_resolver::{
    SolveError,
    resolvo_engine::request::{ConfigurationInput, NormalizedRequest},
};
use vibe_workspace::Workspace;

/// Read every consumer declaration, never installed state or resolved choices.
pub fn authored_configuration(
    workspace: &Workspace,
    request: &mut NormalizedRequest,
) -> Result<bool, SolveError> {
    let mut edges = Vec::new();
    let mut add = |manifest: &Manifest, consumer: &str| -> Result<(), SolveError> {
        for row in &manifest.consumer_configuration {
            if manifest.consumer_node().is_none() {
                return Err(failure("configuration requires a consumer node"));
            }
            let packages: Vec<_> = manifest
                .requires
                .packages
                .iter()
                .filter(|p| p.qualified_name() == row.dependency)
                .collect();
            if packages.len() != 1 {
                return Err(failure(
                    "configuration requires one original registry dependency",
                ));
            }
            let selections: serde_json::Map<String, serde_json::Value> = row.selections.iter().map(|(group, selection)| {
                let mut value = serde_json::json!({"intent":selection.intent, "provenance":format!("manifest:{consumer}:{}", row.dependency)});
                if let Some(members) = &selection.members { value["members"] = serde_json::json!(members); }
                (group.clone(), value)
            }).collect();
            edges.push(serde_json::json!({
                "consumer": consumer, "dependency":row.dependency, "role":"runtime",
                "domain": { "definition":{"reference":row.definition,"producer":row.definition_owner},
                    "target":{"reference":row.target,"producer":row.target_owner},
                    "target_role":"produced_tool_runtime", "identity":row.domain },
                "instance":row.instance, "configuration":row.configuration,
                "package":{"kind":"vibe", "reference":packages[0].to_string()},
                "policy":"require_selection", "selections":selections,
                "features":[], "all_features":false
            }));
        }
        Ok(())
    };
    for (relative, manifest) in workspace.iter_nodes() {
        let path = workspace.node_abs_path(relative);
        let consumer = path
            .to_str()
            .ok_or_else(|| failure("consumer node path must be UTF-8"))?;
        add(manifest, consumer)?;
    }
    if edges.is_empty() {
        return Ok(false);
    }
    let wire = serde_json::from_value(serde_json::json!({"epoch":"1","edges":edges}))
        .map_err(|e| failure(&e.to_string()))?;
    vibe_wire::behaviour::consumer_configuration::admit_request(&wire)
        .map_err(|e| failure(&e.to_string()))?;
    if let Some(existing) = &request.configuration {
        if existing.request != wire {
            return Err(failure(
                "source-bound intent differs from current authored manifest",
            ));
        }
    } else {
        request.configuration = Some(ConfigurationInput {
            request: wire,
            definitions: vec![],
            defaults: vec![],
        });
    }
    Ok(true)
}

fn failure(message: &str) -> SolveError {
    SolveError::UnsupportedRoute(message.into())
}

/// Standard explicit binding for reconfiguration/update/prepared replay callers.
/// The normal install planner invokes the same reader automatically.
pub fn bind_authored_request<'a, S: InstallSource + ?Sized>(
    source: &'a S,
    workspace: &Workspace,
    features: &vibe_resolver::FeatureRequest,
) -> Result<Box<dyn InstallSource + 'a>, SolveError> {
    // Reuse the planner's complete root/declaration owner. This entry is for
    // workspace intent, not CLI root replacement or a lock-derived snapshot.
    let empty = vibe_core::manifest::Lockfile::empty("authored workspace", "");
    let (roots, _, origins) =
        crate::plan::effective_root_constraints(workspace, &workspace.root, &[], &empty)
            .map_err(|e| failure(&e.to_string()))?;
    let mut request = NormalizedRequest::legacy(&roots, features);
    request.root_origins = origins;
    authored_configuration(workspace, &mut request)?;
    source
        .begin_normalized_transaction(&request)?
        .ok_or_else(|| failure("authored configuration requires a normalized source transaction"))
}
