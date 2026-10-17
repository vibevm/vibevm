//! Pure projection of caller-frozen manifest resources. No preparation authority.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003-dep-evolution#M05E1-DISCOVERY"
);

use serde::Serialize;
use vibe_core::manifest::Manifest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoveryCounters {
    pub provider_calls: usize,
    pub solves: usize,
    pub fetches: usize,
    pub authored_writes: usize,
    pub cache_writes: usize,
    pub manifests_parsed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigurationDiscovery {
    pub capture_identity: Option<String>,
    pub status: &'static str,
    pub candidates: Vec<CachedDefinition>,
    pub unavailable: Vec<String>,
    pub counters: DiscoveryCounters,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CachedDefinition {
    pub resource: String,
    /// Digest of exact manifest bytes, distinct from the C1 receipt.
    pub manifest_digest: String,
    pub receipt: serde_json::Value,
    pub coordinate: String,
    pub version: String,
    /// Lossless D1 projection of the admitted normalized B2 definition,
    /// including domain, declarations, guards, typed effects and defaults.
    pub definition: serde_json::Value,
}

/// Each tuple is (resource identity, exact bytes). The digest is calculated
/// from those bytes, never accepted independently from a caller assertion.
pub fn discover<'a>(
    resources: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> ConfigurationDiscovery {
    let mut result = ConfigurationDiscovery {
        capture_identity: None,
        status: "unavailable",
        candidates: vec![],
        unavailable: vec![],
        counters: DiscoveryCounters {
            provider_calls: 0,
            solves: 0,
            fetches: 0,
            authored_writes: 0,
            cache_writes: 0,
            manifests_parsed: 0,
        },
    };
    for (resource, bytes) in resources {
        result.counters.manifests_parsed += 1;
        let parsed = std::str::from_utf8(bytes)
            .map_err(|e| e.to_string())
            .and_then(|text| Manifest::parse_str(text).map_err(|e| e.to_string()));
        let manifest = match parsed {
            Ok(manifest) => manifest,
            Err(error) => {
                result.unavailable.push(format!("{resource}: {error}"));
                continue;
            }
        };
        let Some(package) = &manifest.package else {
            continue;
        };
        result.unavailable.push(format!(
            "{resource}: {}/{}@{}: normalized configuration authority unavailable",
            package.group, package.name, package.version
        ));
    }
    if !result.candidates.is_empty() {
        result.status = "needs_selection";
    }
    result
}

/// Assemble already validated presentation rows. Admission remains with the
/// typed M16 bridge, never this serializable view or a manifest path.
pub fn project_admitted(
    capture_identity: String,
    candidates: Vec<CachedDefinition>,
    unavailable: Vec<String>,
) -> ConfigurationDiscovery {
    let mut view = discover([]);
    view.capture_identity = Some(capture_identity);
    view.status = if candidates.is_empty() {
        "unavailable"
    } else {
        "needs_selection"
    };
    view.candidates = candidates;
    view.unavailable = unavailable;
    view
}

#[cfg(test)]
mod m_05_e_1_cases {
    use super::*;
    const MANIFEST: &[u8] =
        b"[package]\ngroup='org.demo'\nname='demo'\nkind='flow'\nversion='1.0.0'\n";
    #[test]
    fn frozen_definition_effect_default_identity() {
        // Raw bytes cannot establish a normalized definition or a C1 receipt.
        let view = discover([("cache/vibe.toml", MANIFEST)]);
        assert!(view.candidates.is_empty());
        assert!(view.unavailable[0].contains("authority unavailable"));
    }
    #[test]
    fn empty_cache_unavailable() {
        let view = discover([]);
        assert_eq!(view.status, "unavailable");
        assert!(view.candidates.is_empty());
        assert_eq!(view.counters.provider_calls, 0);
    }
    #[test]
    fn query_preserves_manifest_lock_and_cache() {
        let bytes = MANIFEST.to_vec();
        let before = bytes.clone();
        let view = discover([("cache/vibe.toml", bytes.as_slice())]);
        assert_eq!(bytes, before);
        assert_eq!(
            (
                view.counters.authored_writes,
                view.counters.cache_writes,
                view.counters.fetches,
                view.counters.solves
            ),
            (0, 0, 0, 0)
        );
    }
    #[test]
    fn same_unresolved_basis_returns_needs_selection() {
        // Aggregation preserves unresolved state; typed admission is tested at M16.
        let row = CachedDefinition {
            resource: "retained-resource".into(),
            manifest_digest: "retained-byte-witness".into(),
            receipt: serde_json::json!({"source":"retained-source","content":"retained-content","definition":"retained-definition"}),
            coordinate: "org.demo/demo".into(),
            version: "1.0.0".into(),
            definition: serde_json::json!({"constraints":{"groups":{"mode":{"cardinality":"exactly-one","members":["red","blue"],"guards":[],"defaults":["red"]}}}}),
        };
        let view = project_admitted("retained-capture".into(), vec![row.clone()], vec![]);
        assert_eq!(
            view,
            project_admitted("retained-capture".into(), vec![row], vec![])
        );
        assert_eq!(view.status, "needs_selection");
    }
}
