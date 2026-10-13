//! Prepared visible-source query authority is composed beside the workspace
//! owner view. The root selection below is inert: it is not a read grant.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#PREPARED-VISIBLE-SOURCE-AUTHORITY");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vibe_core::manifest::Manifest;
use vibe_core::visibility::NodeId;
use vibe_extension_registry::HostExtensionSource;
use vibe_spec::SelectedPackage;

use crate::extension_world::{ExtensionWorldEpoch, ExtensionWorldError};

mod capture;
mod visibility;
pub use capture::{
    BoundSourceInputs, CanonicalNodeIdentity, CapturedSourceFile, CommonMarkExtraction,
    ExtractedSource, ExtractionError, ExtractionProvenance, MaterializedVibeMedia,
    PreparedDocInstance, PreparedDocument, PreparedOwnerCapture, PreparedWorldWitness,
    RecordedOutput, RepresentationWitness, RetainedOriginCapture, RetainedOriginDocument,
    RetainedOriginFact, RetainedSourceInventory, SelectedDescriptorWitness, SourceEncoding,
    SourceMembershipIndex, SourceQueryResponse, SourceQueryService, SourceStrength, SpanBasis,
    SpecExtraction, SpecOutline, StructuredView, WitnessedSourceWorld, canonical_node_of_output,
    captured_manifest_closure, native_xml_range_of_output,
};

#[derive(Debug, thiserror::Error)]
pub enum SourceQueryUnavailable {
    #[error("the prepared owner world is unavailable: {0}")]
    Owner(#[from] ExtensionWorldError),
    #[error("visible source `{0}` is absent from this owner's selected closure")]
    VisibleOutsideOwner(String),
    #[error("prepared source capture is unavailable for `{0}`")]
    MissingPreparedCapture(String),
    #[error("prepared source capture is invalid for `{owner}`: {reason}")]
    InvalidCapture { owner: String, reason: String },
    #[error("prepared visibility is unavailable: {0}")]
    Visibility(String),
    #[error("selected workspace node `{0}` is absent")]
    UnknownNode(String),
    #[error("prepared source world is stale at {0}")]
    StaleWorld(&'static str),
    #[error("prepared source world is unavailable at {0}")]
    WorldUnavailable(&'static str),
    #[error("original source proof is unavailable at {0}")]
    SourceProofUnavailable(&'static str),
    #[error("original source capture differs from the independently selected package hash")]
    SourceHashMismatch,
    #[error("builtin source derivation is unavailable at {0}")]
    DerivationUnavailable(&'static str),
}

/// Exact selected roots from one node's own world. This stays crate-private
/// until captured bytes and descriptor identities make a usable authority.
#[derive(Debug)]
pub(crate) struct OwnerRoots {
    pub(crate) host: HostExtensionSource,
    pub(crate) selected: BTreeMap<(String, String), SelectedPackage>,
}

#[cfg(test)]
#[path = "m_01_b_1_cases.rs"]
mod prepared_cases;

#[cfg(test)]
#[path = "m_01_c_3_cases.rs"]
mod m_01_c_3_cases;

pub(crate) fn select_owner_roots(
    epoch: &ExtensionWorldEpoch,
    node_root: &Path,
    node_manifest: &Manifest,
    visible: &BTreeSet<NodeId>,
) -> Result<OwnerRoots, SourceQueryUnavailable> {
    let owner = epoch.node_owner_view(node_root, node_manifest)?;
    let mut selected = BTreeMap::new();
    let mut reached = BTreeSet::new();
    for source in &owner.installed {
        let provider = &source.provider;
        let id = provider.id.to_string();
        if !visible.contains(&id) {
            continue;
        }
        reached.insert(id);
        selected.insert(
            (
                provider.id.group().as_str().to_owned(),
                provider.id.name().as_str().to_owned(),
            ),
            SelectedPackage::new(&provider.version, PathBuf::from(&provider.root)),
        );
    }
    if let Some(stray) = visible.difference(&reached).next() {
        return Err(SourceQueryUnavailable::VisibleOutsideOwner(stray.clone()));
    }
    Ok(OwnerRoots {
        host: owner.host,
        selected,
    })
}

#[cfg(test)]
mod m_01_b_1_cases {
    use super::*;

    #[test]
    fn member_self_is_its_own_host_in_an_empty_selected_world() {
        let root = tempfile::tempdir().unwrap();
        let member = root.path().join("member");
        let manifest = Manifest::parse_str(
            "[package]\ngroup='org.demo'\nname='member'\nkind='flow'\nversion='1.0.0'\n",
        )
        .unwrap();
        let roots = select_owner_roots(
            &ExtensionWorldEpoch::empty(),
            &member,
            &manifest,
            &BTreeSet::new(),
        )
        .unwrap();
        assert_eq!(roots.host.provider.root, member);
        assert!(roots.selected.is_empty());
    }

    #[test]
    fn a_visible_id_outside_this_owner_cannot_be_selected() {
        let root = tempfile::tempdir().unwrap();
        let manifest = Manifest::parse_str("[project]\nname='root'\nversion='1.0.0'\n").unwrap();
        let visible = BTreeSet::from(["org.demo/stray".to_owned()]);
        assert!(matches!(
            select_owner_roots(
                &ExtensionWorldEpoch::empty(),
                root.path(),
                &manifest,
                &visible
            ),
            Err(SourceQueryUnavailable::VisibleOutsideOwner(_))
        ));
    }
}
