//! `vibe why` — explain one package's place in the root's effective world
//! (PROP-050 ##VIBE-WHY): the admitting chain with its rule when the
//! package is present, the classified blocked edges when it is not. The
//! debugging surface without which a visibility system rots into folklore.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#verification");

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use vibe_core::visibility::{BlockReason, ProvenanceRule, WhyVerdict, load_installed_world, why};

use crate::output;

/// `vibe why <group>/<name>` — is this package in the project's effective
/// world, and through which chain (or: which declared edge blocked it)?
#[derive(Debug, Args)]
pub struct WhyArgs {
    /// The package coordinate to explain, in `<group>/<name>` form.
    pub coordinate: String,

    /// Project root with `vibe.toml`. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

/// Run `vibe why`: load the installed world, analyse it, and print the
/// verdict for one coordinate. The exit code stays zero for every verdict
/// — a blocked or unknown coordinate is the answer, not a failure; only
/// input/world-read errors take the ordinary `Result` error path.
pub fn run(_ctx: &output::Context, args: WhyArgs) -> Result<()> {
    let root = super::resolve_project_root(&args.path)?;
    let world = load_installed_world(&root).map_err(anyhow::Error::msg)?;
    match why(&world, &args.coordinate) {
        WhyVerdict::Present(provenance) => {
            let rule = rule_word(provenance.rule);
            let chain = provenance.path.join(" -> ");
            let via = provenance.via_override.as_deref().unwrap_or("none");
            _ctx.summary(&format!(
                "{} — present: {rule} via {chain} (override: {via})",
                args.coordinate
            ));
        }
        WhyVerdict::Absent { blocked } => {
            if blocked.is_empty() {
                _ctx.summary(&format!(
                    "{} — absent: no declared edge reaches it in this world",
                    args.coordinate
                ));
            } else {
                _ctx.summary(&format!("{} — absent:", args.coordinate));
                for edge in &blocked {
                    _ctx.summary(&format!(
                        "  blocked at {} -> {}: {}",
                        edge.from,
                        args.coordinate,
                        reason_text(&edge.reason)
                    ));
                }
            }
        }
        WhyVerdict::UnknownCoordinate => {
            _ctx.summary(&format!(
                "{} — unknown coordinate: nothing in this world declares it; \
                 try `vibe tree` for the installed inventory",
                args.coordinate
            ));
        }
    }
    Ok(())
}

/// The provenance rule as a single hyphenated word for the printed chain.
fn rule_word(rule: ProvenanceRule) -> &'static str {
    match rule {
        ProvenanceRule::RootEdge => "root-edge",
        ProvenanceRule::PublicChain => "public-chain",
        ProvenanceRule::FriendsChain => "friends-chain",
    }
}

/// One blocked edge's reason as a printable clause.
fn reason_text(reason: &BlockReason) -> String {
    match reason {
        BlockReason::Private => "the edge is private".to_string(),
        BlockReason::NotAFriend => "the declarant is not in the root's friend closure".to_string(),
        BlockReason::Unfriended => "the declarant unfriended the target".to_string(),
        BlockReason::Excluded => "an edge exclusion kills it in this subtree".to_string(),
        BlockReason::OverrideKilled => {
            "an override entry killed the edge (exclude = true)".to_string()
        }
        BlockReason::AllowFriendsRejected => {
            "the grant is rejected by the target's allow-friends".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WhyArgs, reason_text, rule_word};
    use vibe_core::visibility::{BlockReason, ProvenanceRule};

    #[test]
    fn args_carry_coordinate_and_path() {
        let args = WhyArgs {
            coordinate: "org.x/wal".into(),
            path: "some/root".into(),
        };
        assert_eq!(args.coordinate, "org.x/wal");
        assert_eq!(args.path.to_string_lossy(), "some/root");
    }

    #[test]
    fn rules_render_as_hyphenated_words() {
        assert_eq!(rule_word(ProvenanceRule::RootEdge), "root-edge");
        assert_eq!(rule_word(ProvenanceRule::PublicChain), "public-chain");
        assert_eq!(rule_word(ProvenanceRule::FriendsChain), "friends-chain");
    }

    #[test]
    fn every_block_reason_renders_a_clause() {
        let reasons = [
            BlockReason::Private,
            BlockReason::NotAFriend,
            BlockReason::Unfriended,
            BlockReason::Excluded,
            BlockReason::OverrideKilled,
            BlockReason::AllowFriendsRejected,
        ];
        for reason in reasons {
            assert!(!reason_text(&reason).is_empty());
        }
    }
}
