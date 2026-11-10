//! Weak edges — the one place a `[recommends]` table becomes visibility input.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#effective-set");

use super::{EdgeDecl, NodeId};
use crate::manifest::Manifest;

/// Append one weak edge per accepted recommendation of `manifest`.
///
/// A recommendation is **accepted** when the recommended coordinate is a
/// member of the world being analysed, which `present` answers: the solver's
/// greedy weak expansion (PROP-003 §2.3.3) is what decides acceptance, and a
/// recommendation it declined — unsatisfiable, conflicting, carried by no
/// registry — simply has no member and therefore no edge, exactly as
/// `--no-recommends` has none (PROP-077 `#PROJECT-RECOMMENDS`).
///
/// A reference the node already declares in `[requires]` contributes nothing:
/// the strong edge decides, so one coordinate never arrives as both.
///
/// ```
/// use vibe_core::manifest::Manifest;
/// use vibe_core::visibility::{EdgeDecl, NodeId, append_recommended_edges};
///
/// let manifest = Manifest::parse_str(
///     "[package]\ngroup = \"org.x\"\nname = \"recommender\"\nkind = \"flow\"\n\
///      version = \"1.0.0\"\n[recommends]\npackages = [\"org.x/companion\"]\n",
/// )
/// .unwrap();
/// let mut edges: Vec<EdgeDecl> = Vec::new();
/// // Nothing in the world yet: the recommendation was not accepted.
/// append_recommended_edges(&manifest, &mut edges, &|_: &NodeId| false);
/// assert!(edges.is_empty());
/// // Accepted: one weak edge, with the ordinary public/no-friendship defaults.
/// append_recommended_edges(&manifest, &mut edges, &|id: &NodeId| id == "org.x/companion");
/// assert_eq!(edges.len(), 1);
/// assert!(edges[0].weak);
/// assert_eq!(edges[0].to, "org.x/companion");
/// ```
pub fn append_recommended_edges(
    manifest: &Manifest,
    edges: &mut Vec<EdgeDecl>,
    present: &dyn Fn(&NodeId) -> bool,
) {
    for reference in &manifest.recommends.packages {
        // An unqualified reference has no `<group>/<name>` coordinate; a
        // validated manifest never produces one, so this is defensive only.
        let Some(group) = reference.group.as_ref() else {
            continue;
        };
        let to: NodeId = format!("{group}/{}", reference.name);
        if edges.iter().any(|edge| edge.to == to) || !present(&to) {
            continue;
        }
        edges.push(EdgeDecl {
            to,
            weak: true,
            ..EdgeDecl::default()
        });
    }
}
