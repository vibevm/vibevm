use super::super::backend::{BackendError, BackendId, EmitBackend};
use super::super::ir::{
    ArtifactFrame, LaneContribution, LaneIr, PreEmissionWitness, PreparedEmissionTarget,
};
use super::super::pass::PassName;
use super::framing::{self, CommentSyntax};

pub(crate) struct StaticXmlBackend {
    id: BackendId,
    pass: PassName,
}

impl StaticXmlBackend {
    pub(crate) fn new() -> Self {
        Self {
            id: BackendId::new("static-xml").expect("built-in backend id is valid"),
            pass: PassName::new("emit:static-xml").expect("built-in pass name is valid"),
        }
    }
}

impl EmitBackend for StaticXmlBackend {
    fn id(&self) -> &BackendId {
        &self.id
    }

    fn pass_name(&self) -> &PassName {
        &self.pass
    }

    fn emit(&self, lane: &LaneIr, witness: &PreEmissionWitness) -> Result<Vec<u8>, BackendError> {
        emit_xml(lane, witness).map(String::into_bytes)
    }
}

fn emit_xml(lane: &LaneIr, witness: &PreEmissionWitness) -> Result<String, BackendError> {
    if matches!(lane.context().frame(), ArtifactFrame::CompatibilityFragment) {
        return Err(emit_error(
            "the static-xml backend cannot emit a compatibility fragment",
        ));
    }
    let (generated_path, source_root) = match (
        lane.frame.generated_path.as_deref(),
        lane.frame.source_root.as_deref(),
    ) {
        (Some(generated), Some(source)) => (generated, source),
        _ => return Err(emit_error("final STATIC Lane is missing its frame paths")),
    };
    let syntax = CommentSyntax::Xml;
    let PreparedEmissionTarget::Xml { documents } = &witness.prepared_target else {
        return Err(emit_error(
            "static XML emission is missing prepared documents",
        ));
    };
    if documents.len() != lane.contributions.len() {
        return Err(emit_error(
            "prepared XML documents do not align with Lane contributions",
        ));
    }
    let mut output =
        framing::static_header(syntax, generated_path, witness.transforms_header.as_deref());
    output.push_str(&framing::resolution_preamble(syntax, source_root));
    if !lane.frame.renames.is_empty() {
        output.push_str(&framing::tombstone(syntax, &lane.frame.renames));
    }
    for (index, contribution) in lane.contributions.iter().enumerate() {
        match contribution {
            LaneContribution::Normal { meta, .. } | LaneContribution::Simple { meta, .. } => {
                output.push_str(&framing::static_marker(syntax, meta));
                output.push_str("\n\n");
                let document = documents[index].as_ref().ok_or_else(|| {
                    emit_error(format!(
                        "static contribution `{}` has no prepared XML document",
                        meta.origin
                    ))
                })?;
                let xml = vibe_specdoc::to_xml(document);
                output.push_str(xml.trim_end());
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
    static PIVOT_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_pivot_calls() {
    PIVOT_CALLS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn pivot_calls() -> usize {
    PIVOT_CALLS.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(crate) fn record_pivot_call() {
    PIVOT_CALLS.with(|count| count.set(count.get() + 1));
}

fn emit_error(reason: impl Into<String>) -> BackendError {
    BackendError::Emit {
        backend: "static-xml".to_string(),
        reason: reason.into(),
    }
}
