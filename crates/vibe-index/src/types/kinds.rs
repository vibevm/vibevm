//! Package-kind and naming-convention vocabularies — re-exports of
//! the generated wire types. The shapes live in
//! `vibe_wire::generated::shared` (JTD is the source of truth, PROP-000
//! §16) and their behaviour — `as_str`, `known`, `FromStr`,
//! `repo_name` — in `vibe_wire::behaviour`, beside the form it
//! describes. PROP-005 §3.2's deliberate duplicate is retired: the
//! parity test that watched the two copies for divergence watched a
//! distance that no longer exists.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#deps");

pub use vibe_wire::generated::shared::{NamingConvention, PackageKind};
