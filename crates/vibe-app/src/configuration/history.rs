//! Historical meaning consumed by evaluation, independent of source readiness.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");
use vibe_wire::generated::shared as w;

/// Borrowed prior intent and meaning. No target controls or executable result
/// are needed to compare a transported history with a current definition.
#[derive(Debug, Clone)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
pub struct ConfigurationHistory<'a> {
    pub definition: &'a w::ConfigurationDefinition,
    pub fingerprint: &'a str,
    pub input: &'a w::ConfigurationInput,
    pub reliance: &'a [String],
    pub parameters: Vec<HistoricalParameter<'a>>,
}

/// Only the historical facts a parameter consumer needs. A reusable local
/// realization is already admitted for this host; a transported source-host
/// path is diagnostic data and must never populate that field.
#[derive(Debug, Clone, Copy)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
pub enum HistoricalParameter<'a> {
    Public {
        id: &'a str,
        kind: &'a w::ConfigurationParameterType,
        origin: &'a w::ConfigurationParameterOrigin,
        source: Option<&'a w::ConfigurationValue>,
        reusable_local_realization: Option<&'a w::ConfigurationValue>,
    },
    Secret {
        id: &'a str,
        origin: &'a w::ConfigurationParameterOrigin,
        provided: bool,
    },
}
impl<'a> HistoricalParameter<'a> {
    pub(crate) fn id(&self) -> &'a str {
        match self {
            Self::Public { id, .. } | Self::Secret { id, .. } => id,
        }
    }
}
impl<'a> From<&'a w::ConfigurationRecord> for ConfigurationHistory<'a> {
    fn from(record: &'a w::ConfigurationRecord) -> Self {
        Self {
            definition: &record.definition,
            fingerprint: &record.evaluation.definition,
            input: &record.evaluation.input,
            reliance: &record.evaluation.reliance,
            parameters: record
                .evaluation
                .parameters
                .iter()
                .map(|p| match p {
                    w::ConfigurationParameterResult::Public(p) => HistoricalParameter::Public {
                        id: &p.id,
                        kind: &p.type_,
                        origin: &p.origin,
                        source: p.source.as_ref(),
                        reusable_local_realization: p.value.as_ref(),
                    },
                    w::ConfigurationParameterResult::Secret(p) => HistoricalParameter::Secret {
                        id: &p.id,
                        origin: &p.origin,
                        provided: p.provided,
                    },
                })
                .collect(),
        }
    }
}

/// Local maintenance may reuse an admitted local realization. A portable
/// transfer must instead bind every present public Path explicitly on this host.
#[derive(Debug, Clone, Copy)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
pub enum HistoryUse<'a> {
    Local,
    Transfer {
        path_bindings: &'a std::collections::BTreeMap<String, String>,
        required_secrets: &'a [String],
    },
}
