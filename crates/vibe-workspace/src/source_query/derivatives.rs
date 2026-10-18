//! Passive provenance retained independently of current selected-source proof.
//! The strict result and core binding owners validate identity; no hash grammar,
//! resolver, provider or filesystem port is introduced here.
use crate::derivatives::{DerivativeInspection, FileInspection};
use vibe_core::manifest::{
    SourceInterpretation,
    derivation::{BindingError, DerivationBinding, RetainedSource},
};
use vibe_wire::generated::derivative_result::DerivativeResult;

#[derive(Debug, Clone, Default)]
pub struct DerivativeCatalog {
    records: Vec<DerivativeResult>,
}
impl DerivativeCatalog {
    pub fn new(records: Vec<DerivativeResult>) -> Result<Self, BindingError> {
        if records.len() > 128
            || records
                .iter()
                .try_fold(0usize, |n, r| n.checked_add(r.binding_toml.len()))
                .is_none_or(|n| n > 64 * 1024 * 1024)
        {
            return Err(BindingError("retained derivative catalogue bound"));
        }
        let bindings = records
            .iter()
            .map(DerivativeResult::checked_binding)
            .collect::<Result<Vec<_>, _>>()?;
        DerivationBinding::validate_set(&bindings)?;
        for binding in &bindings {
            for source in [binding.base(), binding.local()] {
                if source.descriptor.interpretation() == SourceInterpretation::VibeSpec {
                    let mut documents = std::collections::BTreeSet::new();
                    for file in &source.files {
                        if !documents.insert(vibe_safefs::identity_key(
                            &vibe_spec::canonical_doc_path(&file.path),
                        )) {
                            return Err(BindingError("ambiguous Vibe document representation"));
                        }
                    }
                }
            }
        }
        Ok(Self { records })
    }
    /// Explicit host-selected relative records, using the incumbent bounded
    /// no-follow reader. No discovery, repair, base recapture or provider call.
    pub fn load(root: &std::path::Path, paths: &[String]) -> Result<Self, String> {
        if paths.len() > 128 {
            return Err("retained derivative catalogue count bound".into());
        }
        if paths.is_empty() {
            return Ok(Self::default());
        }
        let project = vibe_safefs::Project::open(root).map_err(|e| e.to_string())?;
        let mut records = Vec::new();
        let mut remaining = 64 * 1024 * 1024;
        for path in paths {
            let captured = project
                .read_file_snapshot_bounded(path, remaining)
                .map_err(|e| e.to_string())?
                .ok_or("retained derivative record absent")?;
            remaining -= captured.bytes.len();
            records.push(serde_json::from_slice(&captured.bytes).map_err(|e| e.to_string())?);
        }
        Self::new(records).map_err(|e| e.to_string())
    }
    /// Explicit producer capture through the incumbent full-inventory owner.
    /// Only the selected pinned local package may contribute these bytes.
    pub fn capture_selected(
        &self,
        project: &vibe_safefs::Project,
        package: &vibe_core::manifest::PackageMeta,
    ) -> Result<std::collections::BTreeMap<String, Vec<u8>>, String> {
        let mut files = std::collections::BTreeMap::new();
        for record in &self.records {
            let binding = record.checked_binding().map_err(|e| e.to_string())?;
            let local = binding.local();
            if local.owner.group.as_ref() != Some(&package.group)
                || local.owner.name.as_str() != package.name
                || local.owner.kind.as_ref() != Some(&package.kind)
                || local.owner.version.to_string() != format!("={}", package.version)
            {
                continue;
            }
            for file in &local.files {
                if !matches!(package.select_source(&file.path).map_err(|e| format!("{e:?}"))?, vibe_core::manifest::SelectedSource::Descriptor { descriptor, .. } if descriptor == &local.descriptor)
                {
                    return Err("selected derivative descriptor differs".into());
                }
            }
            crate::derivatives::capture_retained(project, local).map_err(|e| e.to_string())?;
            files.extend(
                local
                    .files
                    .iter()
                    .map(|file| (file.path.clone(), file.bytes.clone())),
            );
        }
        Ok(files)
    }
    pub fn records(&self) -> &[DerivativeResult] {
        &self.records
    }
    /// A retained record stays inspectable after its live providers disappear.
    /// This returns Observed history; it never checks or asserts current bases.
    pub fn inspect(&self, result: &str) -> Result<DerivativeInspection, BindingError> {
        let record = self
            .records
            .iter()
            .find(|r| r.result_identity == result)
            .ok_or(BindingError("unknown retained derivative result"))?;
        let binding = record.checked_binding()?;
        Ok(DerivativeInspection {
            base_identity: record.base_identity.clone(),
            local_identity: record.local_identity.clone(),
            provenance_identity: record.provenance_identity.clone(),
            result_identity: record.result_identity.clone(),
            inventory_identity: record.inventory_identity.clone(),
            base: inventory(binding.base()),
            local: inventory(binding.local()),
        })
    }
}
fn inventory(source: &RetainedSource) -> Vec<FileInspection> {
    crate::derivatives::inspection_inventory(source)
}
