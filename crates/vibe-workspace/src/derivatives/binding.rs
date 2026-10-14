//! Pure preparation of stored derivative records. No materialization port.
use super::*;
use vibe_core::manifest::derivation::{
    BindingError, DerivationBinding, RetainedFile, RetainedSource, retain_binding_document,
};
use vibe_wire::generated::derivative_result::{
    DerivativeResult as WireResult, DerivativeResultStrength as Strength,
};

impl SourceInput {
    /// Retain observed M01 bytes with the M16 interpretation. This does not
    /// authenticate a capture; production proof remains with slot_verify.
    pub fn retained_source(&self, resource: &str) -> RetainedSource {
        RetainedSource {
            owner: self.coordinate.clone(),
            resource: resource.into(),
            descriptor: self.descriptor.clone(),
            files: self
                .files
                .iter()
                .map(|(path, file)| RetainedFile {
                    path: path.clone(),
                    bytes: file.snapshot().bytes.clone(),
                })
                .collect(),
        }
    }
}

/// A validated result contains the complete independently retained authored
/// document, not a path into disposable output. Decoding a wire record alone
/// grants neither proof nor materialization authority.
#[derive(Debug)]
pub struct BoundDerivative {
    binding: DerivationBinding,
    record: WireResult,
}
impl BoundDerivative {
    pub fn binding(&self) -> &DerivationBinding {
        &self.binding
    }
    pub fn record(&self) -> &WireResult {
        &self.record
    }
    pub fn strength(&self) -> crate::source_query::SourceStrength {
        crate::source_query::SourceStrength::Observed
    }
    pub fn read(
        record: WireResult,
        current_base: Option<&SourceInput>,
    ) -> Result<Self, BindingError> {
        record.checked_binding()?;
        let expected = prepare_binding(&record.binding_toml, current_base)?.value;
        if serde_json::to_value(&record).map_err(|_| BindingError("result encoding"))?
            != serde_json::to_value(&expected.record)
                .map_err(|_| BindingError("result encoding"))?
        {
            return Err(BindingError("stale or altered result/provenance"));
        }
        Ok(expected)
    }
}

/// Passive validation precedes any downstream write. An absent or stale base
/// is an error, even though its previous bytes remain navigable in the record.
pub fn prepare_binding(
    text: &str,
    current_base: Option<&SourceInput>,
) -> Result<Outcome<BoundDerivative>, BindingError> {
    let retained = retain_binding_document(text)?;
    let binding = DerivationBinding::parse(&retained)?;
    let current = current_base.ok_or(BindingError("exact current base absent"))?;
    let observed = current.retained_source(&binding.base().resource);
    if observed.observation_id()? != binding.base_capture() {
        return Err(BindingError("stale exact base"));
    }
    DerivationBinding::validate_set(std::slice::from_ref(&binding))?;
    for source in [binding.base(), binding.local()] {
        if source.descriptor.interpretation() == vibe_core::manifest::SourceInterpretation::VibeSpec
        {
            let mut documents = BTreeSet::new();
            for file in &source.files {
                let document = vibe_spec::canonical_doc_path(&file.path);
                if !documents.insert(vibe_safefs::identity_key(&document)) {
                    return Err(BindingError("ambiguous Vibe document representation"));
                }
            }
        }
    }
    let record = WireResult {
        schema: 1,
        binding_toml: retained,
        base_identity: binding.base_capture().into(),
        local_identity: binding.local().observation_id()?,
        provenance_identity: binding.provenance_id()?,
        result_identity: binding.result_id()?,
        inventory_identity: binding.local().inventory_id()?,
        strength: Strength::Observed,
    };
    Ok(passive(BoundDerivative { binding, record }))
}

#[cfg(test)]
mod m_10_b_1_cases;
