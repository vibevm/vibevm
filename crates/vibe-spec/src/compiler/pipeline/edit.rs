//! Transactional placement around the immutable built-in pass snapshot.

use std::collections::BTreeMap;

use super::{
    CompilerPipeline, CompilerPipelineError, preserved_segment_endpoints,
    validate_segment_endpoints,
};
use crate::compiler::ir::IrCardinality;
use crate::compiler::pass::{
    DynPass, Pass, PassDescriptor, PassName, PassSegment, PassSegmentError, erase_pass,
};
use crate::compiler::pass_tier::frontend::DocumentPipeline;

enum Placement {
    Before(PassName),
    After(PassName),
    Replace(PassName),
}

/// One already-typed edit. Runtime registration will construct these later.
pub(crate) struct PipelineEdit<'pass> {
    placement: Placement,
    pass: Box<dyn DynPass + 'pass>,
}

impl<'pass> PipelineEdit<'pass> {
    pub(crate) fn before<P: Pass + 'pass>(anchor: PassName, pass: P) -> Self {
        Self {
            placement: Placement::Before(anchor),
            pass: erase_pass(pass),
        }
    }

    pub(crate) fn after<P: Pass + 'pass>(anchor: PassName, pass: P) -> Self {
        Self {
            placement: Placement::After(anchor),
            pass: erase_pass(pass),
        }
    }

    pub(crate) fn replace<P: Pass + 'pass>(anchor: PassName, pass: P) -> Self {
        Self {
            placement: Placement::Replace(anchor),
            pass: erase_pass(pass),
        }
    }

    fn anchor(&self) -> &PassName {
        match &self.placement {
            Placement::Before(anchor) | Placement::After(anchor) | Placement::Replace(anchor) => {
                anchor
            }
        }
    }
}

#[derive(Default)]
struct AnchorEdits<'pass> {
    before: Vec<Box<dyn DynPass + 'pass>>,
    replacement: Option<Box<dyn DynPass + 'pass>>,
    after: Vec<Box<dyn DynPass + 'pass>>,
}

impl<'pass> CompilerPipeline<'pass> {
    /// Validate one complete edit batch, then rebuild both segments once.
    pub(crate) fn apply_builtin_edits(
        &mut self,
        edits: Vec<PipelineEdit<'pass>>,
    ) -> Result<(), CompilerPipelineError> {
        if self.edits_applied {
            return Err(CompilerPipelineError::EditsAlreadyApplied);
        }
        let document = self.document.descriptors().collect::<Vec<_>>();
        let artifact = self.artifact.descriptors().collect::<Vec<_>>();
        let base = document.iter().chain(&artifact).collect::<Vec<_>>();
        let mut names = self.pass_names.clone();
        let mut grouped = BTreeMap::<PassName, AnchorEdits<'pass>>::new();

        for edit in edits {
            let anchor = edit.anchor().clone();
            if !self.builtin_names.contains(&anchor) {
                return Err(CompilerPipelineError::AnchorNotBuiltin { anchor });
            }
            let target = base
                .iter()
                .find(|descriptor| descriptor.name == anchor)
                .expect("every recorded builtin name remains in the base schedule");
            let descriptor = edit.pass.descriptor();
            if !names.insert(descriptor.name.clone()) {
                return Err(CompilerPipelineError::DuplicateName {
                    pass: descriptor.name,
                });
            }
            let slot = grouped.entry(anchor.clone()).or_default();
            match edit.placement {
                Placement::Before(_) => slot.before.push(edit.pass),
                Placement::After(_) => slot.after.push(edit.pass),
                Placement::Replace(_) => {
                    if slot.replacement.is_some() {
                        return Err(CompilerPipelineError::DuplicateReplacement { anchor });
                    }
                    if descriptor.input != target.input || descriptor.output != target.output {
                        return Err(CompilerPipelineError::ReplacementShape {
                            anchor,
                            replacement: descriptor.name,
                            expected_input: target.input,
                            expected_output: target.output,
                            actual_input: descriptor.input,
                            actual_output: descriptor.output,
                        });
                    }
                    slot.replacement = Some(edit.pass);
                }
            }
        }

        let planned_document = planned(&document, &grouped);
        let planned_artifact = planned(&artifact, &grouped);
        validate_segment(
            "document",
            IrCardinality::Document,
            preserved_segment_endpoints("document", &document),
            &planned_document,
        )?;
        validate_segment(
            "artifact",
            IrCardinality::Artifact,
            preserved_segment_endpoints("artifact", &artifact),
            &planned_artifact,
        )?;

        let document = rebuild(self.document.take_passes(), &mut grouped);
        let artifact = rebuild(
            std::mem::take(&mut self.artifact).into_passes(),
            &mut grouped,
        );
        debug_assert!(grouped.is_empty());
        let document = DocumentPipeline::from_passes(document)?;
        let artifact = PassSegment::from_passes(artifact)?;
        self.document = document;
        self.artifact = artifact;
        self.pass_names = names;
        self.edits_applied = true;
        Ok(())
    }
}

fn planned(
    base: &[PassDescriptor],
    edits: &BTreeMap<PassName, AnchorEdits<'_>>,
) -> Vec<PassDescriptor> {
    let mut result = Vec::new();
    for pass in base {
        if let Some(edit) = edits.get(&pass.name) {
            result.extend(edit.before.iter().map(|pass| pass.descriptor()));
            result.push(
                edit.replacement
                    .as_ref()
                    .map_or_else(|| pass.clone(), |pass| pass.descriptor()),
            );
            result.extend(edit.after.iter().map(|pass| pass.descriptor()));
        } else {
            result.push(pass.clone());
        }
    }
    result
}

fn validate_segment(
    segment: &'static str,
    cardinality: IrCardinality,
    endpoints: Option<super::SegmentEndpoints>,
    passes: &[PassDescriptor],
) -> Result<(), CompilerPipelineError> {
    for pass in passes {
        if pass.input.cardinality != cardinality || pass.output.cardinality != cardinality {
            return Err(CompilerPipelineError::WrongSegmentCardinality {
                segment,
                pass: pass.name.clone(),
                expected: cardinality,
                input: pass.input,
                output: pass.output,
            });
        }
    }
    for pair in passes.windows(2) {
        if pair[0].output != pair[1].input {
            return Err(PassSegmentError::BrokenChain {
                previous: pair[0].name.clone(),
                previous_output: pair[0].output,
                next: pair[1].name.clone(),
                next_input: pair[1].input,
            }
            .into());
        }
    }
    match endpoints {
        Some(endpoints) => validate_segment_endpoints(
            endpoints,
            passes.first().map(|pass| pass.input),
            passes.last().map(|pass| pass.output),
        ),
        None => Ok(()),
    }
}

fn rebuild<'pass>(
    base: Vec<Box<dyn DynPass + 'pass>>,
    edits: &mut BTreeMap<PassName, AnchorEdits<'pass>>,
) -> Vec<Box<dyn DynPass + 'pass>> {
    let mut result = Vec::new();
    for pass in base {
        let name = pass.descriptor().name;
        if let Some(mut edit) = edits.remove(&name) {
            result.append(&mut edit.before);
            result.push(edit.replacement.take().unwrap_or(pass));
            result.append(&mut edit.after);
        } else {
            result.push(pass);
        }
    }
    result
}
