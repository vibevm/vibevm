use super::super::backend::{BackendError, BackendId, EmitBackend};
use super::super::ir::{
    ArtifactFrame, LaneChunk, LaneContribution, LaneIr, LaneNode, PreEmissionWitness,
};
use super::super::pass::PassName;
use super::framing::{self, CommentSyntax};

pub(crate) struct StaticMarkdownBackend {
    id: BackendId,
    pass: PassName,
}

impl StaticMarkdownBackend {
    pub(crate) fn new() -> Self {
        Self {
            id: BackendId::new("static-md").expect("built-in backend id is valid"),
            pass: PassName::new("emit:static-md").expect("built-in pass name is valid"),
        }
    }
}

impl EmitBackend for StaticMarkdownBackend {
    fn id(&self) -> &BackendId {
        &self.id
    }

    fn pass_name(&self) -> &PassName {
        &self.pass
    }

    fn emit(&self, lane: &LaneIr, witness: &PreEmissionWitness) -> Result<Vec<u8>, BackendError> {
        emit_markdown(lane, witness.transforms_header.as_deref()).map(String::into_bytes)
    }
}

fn emit_markdown(lane: &LaneIr, transforms: Option<&str>) -> Result<String, BackendError> {
    #[cfg(test)]
    RENDER_CALLS.with(|count| count.set(count.get() + 1));
    if matches!(lane.context().frame(), ArtifactFrame::CompatibilityFragment) {
        let mut output = String::new();
        for contribution in &lane.contributions {
            match contribution {
                LaneContribution::Normal { chunks, .. }
                | LaneContribution::Simple { chunks, .. } => {
                    output.push_str(&flatten_markdown(chunks));
                }
                LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => {
                    return Err(emit_error(
                        "compatibility fragments cannot contain elided or hoisted inputs",
                    ));
                }
            }
        }
        return Ok(output);
    }

    let (generated_path, source_root) = lane_paths(lane)?;
    let syntax = CommentSyntax::Markdown;
    let mut output = framing::static_header(syntax, generated_path, transforms);
    output.push_str(&framing::resolution_preamble(syntax, source_root));
    if !lane.frame.renames.is_empty() {
        output.push_str(&framing::tombstone(syntax, &lane.frame.renames));
    }
    for contribution in &lane.contributions {
        match contribution {
            LaneContribution::Normal { meta, chunks, .. }
            | LaneContribution::Simple { meta, chunks, .. } => {
                output.push_str(&framing::static_marker(syntax, meta));
                output.push_str("\n\n");
                let body = flatten_markdown(chunks);
                output.push_str(body.trim_end());
                output.push_str("\n\n");
            }
            LaneContribution::Elided { meta } => {
                output.push_str(&framing::elided_marker(syntax, meta));
                output.push_str("\n\n");
            }
            LaneContribution::Hoisted { meta, target } => {
                output.push_str(&framing::hoisted_marker(syntax, &meta.origin));
                let coordinate = hoisted_coordinate(target)?;
                output.push_str(&format!("\n#use spec://{coordinate}\n\n"));
            }
        }
    }
    Ok(output)
}

fn hoisted_coordinate(target: &crate::SpecAddress) -> Result<String, BackendError> {
    match &target.authority {
        crate::Authority::Package {
            group,
            name,
            version: None,
        } => Ok(format!("{group}/{name}")),
        _ => Err(emit_error(
            "hoisted target is not an unversioned package document",
        )),
    }
}

#[cfg(test)]
std::thread_local! {
    static RENDER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_render_calls() {
    RENDER_CALLS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn render_calls() -> usize {
    RENDER_CALLS.with(std::cell::Cell::get)
}

/// Render one contribution's chunk stream to its Markdown body — the one
/// spelling the backend writes and the analyzer's attribution accounting
/// counts, so a contribution's content bytes and the backend's rendering
/// of it cannot drift apart.
pub(crate) fn flatten_markdown(chunks: &[LaneChunk]) -> String {
    let mut output = String::new();
    for chunk in chunks {
        match chunk {
            LaneChunk::NormalOpen { marker, .. } => {
                output.push_str(&crate::markers::open(marker.as_str()));
                output.push('\n');
            }
            LaneChunk::NormalClose { marker, .. } => {
                output.push_str(&crate::markers::close(marker.as_str()));
                output.push('\n');
            }
            LaneChunk::Node(node) => match node.as_ref() {
                LaneNode::Normal { body, .. } | LaneNode::Simple { body, .. } => {
                    output.push_str(body)
                }
            },
            LaneChunk::ForcedNewline { .. } => output.push('\n'),
        }
    }
    output
}

fn lane_paths(lane: &LaneIr) -> Result<(&str, &str), BackendError> {
    match (&lane.frame.generated_path, &lane.frame.source_root) {
        (Some(generated), Some(source)) => Ok((generated, source)),
        _ => Err(emit_error("final STATIC Lane is missing its frame paths")),
    }
}

fn emit_error(reason: impl Into<String>) -> BackendError {
    BackendError::Emit {
        backend: "static-md".to_string(),
        reason: reason.into(),
    }
}
