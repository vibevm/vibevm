//! Pure projection of the existing INDEX.md manifest. Conditions are data.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#CAPTURED-GRAPH");
use serde::Deserialize;
use vibe_spec::{
    BootGraph, GraphContribution, GraphOccurrence, InclusionParent, ReferenceState, SourceIdentity,
};

#[derive(Debug, Clone, Deserialize)]
pub struct IndexEntry {
    pub path: String,
    pub kind: String,
    #[serde(default)]
    pub when: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DynamicIndex {
    #[serde(rename = "static")]
    pub static_pointer: Option<String>,
    #[serde(default, rename = "entry")]
    pub entries: Vec<IndexEntry>,
}

pub fn read_index(text: &str) -> Result<DynamicIndex, toml::de::Error> {
    toml::from_str(text)
}

#[derive(Debug, Clone)]
pub struct IndexCapture {
    pub capture_id: String,
    /// Complete ordered entry evidence, including unevaluated conditions and
    /// unknown kind spellings. Entry ordinal equals contribution ordinal.
    pub index: DynamicIndex,
    pub graph: BootGraph,
}

pub fn capture_index(capture_id: String, text: &str) -> Result<IndexCapture, toml::de::Error> {
    let index = read_index(text)?;
    let mut graph = BootGraph::default();
    for entry in &index.entries {
        let source = graph.source(
            SourceIdentity::StaticEntry {
                origin: "INDEX.md".into(),
                path: entry.path.clone(),
            },
            None,
        );
        graph.contributions.push(GraphContribution {
            origin: "INDEX.md".into(),
            path: entry.path.clone(),
            kind: entry.kind.clone(),
            seed: Some(source),
            occurrences: vec![GraphOccurrence {
                source,
                requested: None,
                state: ReferenceState::Unknown,
                parent: InclusionParent::Root,
                artifact_lines: None,
            }],
        });
    }
    if let Some(path) = &index.static_pointer {
        graph.source(
            SourceIdentity::StaticEntry {
                origin: "INDEX.md".into(),
                path: path.clone(),
            },
            None,
        );
    }
    graph.gaps.push("INDEX.md entry.path/kind/when records declarations, not condition outcomes, file availability, source spans or provider inclusion chains. The static pointer is retained separately and is not an extra entry occurrence.".into());
    Ok(IndexCapture {
        capture_id,
        index,
        graph,
    })
}
