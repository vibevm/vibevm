//! Pure inspection of host-captured bytes. This module has no filesystem port.
use super::{capture_static, dynamic_index::capture_index, measurement, why_included};
use std::collections::BTreeMap;
use vibe_spec::BootGraph;

#[derive(Debug, Clone, Copy)]
pub enum CapturedKind {
    Static,
    Index,
    Compiler,
}

pub struct Projection {
    pub graph: BootGraph,
    pub measurement: measurement::Lane,
}

pub fn project(id: &str, kind: CapturedKind, bytes: &[u8]) -> Result<Projection, String> {
    let graph = match kind {
        CapturedKind::Compiler => {
            vibe_spec::inspect_boot_snapshot(bytes).map_err(|e| e.to_string())?
        }
        CapturedKind::Static => {
            capture_static(
                id.into(),
                std::str::from_utf8(bytes).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?
            .graph
        }
        CapturedKind::Index => {
            capture_index(
                id.into(),
                std::str::from_utf8(bytes).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?
            .graph
        }
    };
    // Captured artifact bytes do not prove which authored source produced them.
    let mut measured = measurement::measure_graph(id, &graph, &BTreeMap::new(), &BTreeMap::new());
    measured.artifact = Some(measurement::measure_reader(bytes).map_err(|e| e.to_string())?);
    Ok(Projection {
        graph,
        measurement: measured,
    })
}

impl Projection {
    pub fn explain(&self, contribution: usize, occurrence: usize) -> Option<Vec<usize>> {
        why_included(self.graph.contributions.get(contribution)?, occurrence)
    }
}

#[cfg(test)]
mod m_06_c_1_cases {
    use super::*;
    #[test]
    fn captured_index_preserves_repetition_and_unavailable_attribution() {
        let bytes = b"[[entry]]\npath='a.md'\nkind='dynamic'\n[[entry]]\npath='a.md'\nkind='dynamic'\nwhen='os:linux'\n";
        let p = project("capture-a", CapturedKind::Index, bytes).unwrap();
        assert_eq!(p.graph.contributions.len(), 2);
        assert_eq!(p.explain(1, 0), Some(vec![0]));
        assert!(matches!(
            p.measurement.totals.runtime_reads,
            measurement::Count::Unavailable { .. }
        ));
        assert!(matches!(
            p.measurement.totals.inclusive_bytes,
            measurement::Count::Unavailable { .. }
        ));
        assert_eq!(
            p.measurement.artifact.as_ref().unwrap().bytes,
            bytes.len() as u64
        );
        assert!(p.explain(2, 0).is_none());
    }
    #[test]
    fn invalid_captured_source_is_not_an_empty_success() {
        assert!(project("capture", CapturedKind::Index, b"[bad").is_err());
        assert!(project("capture", CapturedKind::Static, &[255]).is_err());
        assert!(project("capture", CapturedKind::Compiler, b"{}").is_err());
    }
}
