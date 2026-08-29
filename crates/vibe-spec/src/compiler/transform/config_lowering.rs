//! Effective configuration → the neutral plan tree (R4-TRANSFORM-PLAN-ABI
//! §3), the half of the T10B lowering that owns config presence.
//!
//! The plan layer distinguishes three states, and this cell is where the
//! distinction is decided once:
//!
//! | effective config on the row | lowered |
//! |---|---|
//! | absent | `None` — no effective config was authored |
//! | present and empty | `Some(empty)` — an authored activation cleared it |
//! | present and non-empty | a typed refusal, today (see below) |
//!
//! `None` and `Some(empty)` are different plan identities and stay different
//! through the digest; fusing them would make an authored clearing
//! indistinguishable from silence.
//!
//! **The non-empty gap is a crate-edge fact, not a design choice.** The
//! neutral tree ([`super::config::ConfigValue`]) is lossless precisely
//! because TOML datetime and the TOML number tower are not JSON values, so
//! lowering a non-empty table means reading `toml::Value` variants — and
//! `toml` is a DEV dependency of `vibe-spec`, never a runtime one (the
//! dependency-DAG fence in `plan_fence_tests` states the exact runtime set).
//! Every route around that edge is already ruled out: lowering in the
//! workspace over a widened `ConfigTable` is the ABI §5.3 rejected
//! alternative ("makes two crates own one canonical form"), and a
//! render/parse round trip is forbidden by §3 outright. A value tower cannot
//! be read through inherent accessors alone either — `toml_datetime::Offset`
//! discriminates `Z` from a signed minute offset only by naming the enum,
//! and §3 keeps those two identities distinct.
//!
//! So the honest interim is a REFUSAL, never a silent `None`: a row that
//! authored real configuration would otherwise lower into a plan whose
//! digest asserts "no config was authored", and the symptom would be a
//! transform that runs unconfigured while its identity claims it was.
//!
//! <!-- REVIEW: close this gap by giving `vibe-spec` the `toml` runtime edge
//! its own ABI §5.3 lowering authority requires (one line in
//! `crates/vibe-spec/Cargo.toml`, plus the runtime-set assertion in
//! `plan_fence_tests::the_dependency_dag_gains_exactly_the_two_intended_lower_edges`),
//! then replace [`lower_effective_config`]'s refusal arm with the value-tower
//! walk. The manifest is outside this atom's write perimeter, so the gap is
//! named rather than crossed. -->

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use vibe_core::manifest::ExtensionConfig;

use super::config::ConfigTable;
use super::plan::TransformConfig;

/// Lower one row's EFFECTIVE configuration into plan identity.
///
/// Effective, never authored: the registry has already applied whole-value
/// host activation replacement, and the plan digests what will actually be
/// delivered.
pub(super) fn lower_effective_config(
    config: Option<&ExtensionConfig>,
) -> Result<Option<TransformConfig>, ConfigLoweringGap> {
    let Some(config) = config else {
        // Absence is absence: no effective config was authored.
        return Ok(None);
    };
    if !config.is_empty() {
        return Err(ConfigLoweringGap::ValueTower);
    }
    // An authored activation cleared the value. A real, empty table — not
    // absence — so it digests, and it digests differently from `None`.
    Ok(Some(TransformConfig::new(ConfigTable::new())))
}

/// Why one row's effective configuration could not become plan identity.
///
/// One arm, named for the seam that closes it rather than for a symptom, so
/// the refusal reads as "this is not implemented yet" and never as "your
/// configuration is invalid".
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ConfigLoweringGap {
    #[error(
        "lowering a non-empty effective configuration into plan identity awaits the lossless TOML value tower; an empty (cleared) configuration and an absent one already lower exactly"
    )]
    ValueTower,
}
