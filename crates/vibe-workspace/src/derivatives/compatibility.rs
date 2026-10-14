//! Passive consumer-interface checks over the retained RESULT, never the base.
use super::BoundDerivative;
use vibe_core::manifest::SourceInterpretation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportInterface {
    /// A byte inventory contract. Applies equally to all five source families;
    /// it makes no assertion about an executable ABI or semantic XML schema.
    Files(Vec<String>),
    CommonmarkHeading {
        path: String,
        heading: String,
    },
    /// Preserve a consumer contract for which this owner has no inspector.
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportTarget {
    pub name: String,
    pub interface: ExportInterface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compatibility {
    Compatible,
    Incompatible(String),
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationState {
    Created,
}

/// Inspection cannot consume or invalidate retained authored ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub creation: CreationState,
    pub result_identity: String,
    pub target: Option<ExportTarget>,
    pub compatibility: Compatibility,
}

impl BoundDerivative {
    pub fn compatibility(&self, target: Option<&ExportTarget>) -> CompatibilityReport {
        let local = self.binding().local();
        let verdict = match target.map(|target| &target.interface) {
            None => Compatibility::Unknown("no export target selected".into()),
            Some(ExportInterface::Unsupported(contract)) => {
                Compatibility::Unknown(format!("no inspector for {contract}"))
            }
            Some(ExportInterface::Files(paths)) => {
                if paths.is_empty() {
                    Compatibility::Unknown("empty consumer interface".into())
                } else if let Some(missing) = paths
                    .iter()
                    .find(|path| !local.files.iter().any(|file| &file.path == *path))
                {
                    Compatibility::Incompatible(format!("result lacks required file {missing}"))
                } else {
                    Compatibility::Compatible
                }
            }
            Some(ExportInterface::CommonmarkHeading { path, heading }) => {
                if local.descriptor.interpretation() != SourceInterpretation::Commonmark {
                    Compatibility::Incompatible("result is not declared CommonMark".into())
                } else if let Some(file) = local.files.iter().find(|file| file.path == *path) {
                    match std::str::from_utf8(&file.bytes) {
                        Err(_) => Compatibility::Incompatible("result is not UTF-8".into()),
                        Ok(text) => match vibe_specdoc::commonmark::CommonMarkDoc::parse(text)
                            .heading(heading)
                        {
                            Ok(_) => Compatibility::Compatible,
                            Err(error) => {
                                Compatibility::Incompatible(format!("result heading: {error:?}"))
                            }
                        },
                    }
                } else {
                    Compatibility::Incompatible(format!("result lacks {path}"))
                }
            }
        };
        CompatibilityReport {
            creation: CreationState::Created,
            result_identity: self.record().result_identity.clone(),
            target: target.cloned(),
            compatibility: verdict,
        }
    }
}

#[cfg(test)]
mod m_10_b_2_cases;
