//! Effective configuration → the neutral plan tree (R4-TRANSFORM-PLAN-ABI
//! §3), the half of the T10B lowering that owns config presence and — since
//! R4.2 — the lossless value tower itself.
//!
//! The plan layer distinguishes three states, and this cell is where the
//! distinction is decided once:
//!
//! | effective config on the row | lowered |
//! |---|---|
//! | absent | `None` — no effective config was authored |
//! | present and empty | `Some(empty)` — an authored activation cleared it |
//! | present and non-empty | `Some(the lossless tree)` |
//!
//! `None` and `Some(empty)` are different plan identities and stay different
//! through the digest; fusing them would make an authored clearing
//! indistinguishable from silence. `Some(empty)` and `Some(non-empty)` differ
//! for the ordinary reason: the table digests its own contents.
//!
//! **Why the walk is a walk and not a round trip.** §3 forbids generic JSON
//! and forbids a render/parse round trip entering identity, because TOML
//! datetime and the TOML number tower are not JSON values. So every arm is
//! mapped component for component: a datetime's date/time/offset are copied
//! field by field through [`super::config`]'s checked constructors (`Z` and a
//! signed minute offset stay two identities, exactly as `toml_datetime`
//! spells them), a float becomes its canonical bit key, table order is
//! semantic-insensitive and sorted by the neutral tree's own `BTreeMap`,
//! and array order is semantic and retained.
//!
//! **This is the one cell in `vibe-spec` that names `toml`.** T10B could not
//! write it: `toml` was a DEV dependency of this crate, and the crate-edge
//! fact was recorded as an interim refusal rather than crossed. R4.2 adds the
//! one runtime edge the ABI §5.3 lowering authority requires, and the
//! dependency-DAG fences state the new runtime set with that reason. Nothing
//! else in the transform cells may name `toml`; the fence families still ban
//! it everywhere but here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use vibe_core::manifest::ExtensionConfig;

use super::config::{
    ConfigDate, ConfigDatetime, ConfigDatetimeError, ConfigFloat, ConfigOffset, ConfigTable,
    ConfigTime, ConfigValue,
};
use super::plan::TransformConfig;

/// Lower one row's EFFECTIVE configuration into plan identity.
///
/// Effective, never authored: the registry has already applied whole-value
/// host activation replacement, and the plan digests what will actually be
/// delivered.
pub(super) fn lower_effective_config(
    config: Option<&ExtensionConfig>,
) -> Result<Option<TransformConfig>, ConfigLoweringError> {
    let Some(config) = config else {
        // Absence is absence: no effective config was authored.
        return Ok(None);
    };
    // A present table lowers whatever it holds — including nothing. An
    // authored activation that cleared the value is a real, empty table, so
    // it digests, and it digests differently from `None`.
    Ok(Some(TransformConfig::new(lower_table(config.as_table())?)))
}

/// One TOML table, key for key, into the sorted neutral table.
fn lower_table(table: &toml::Table) -> Result<ConfigTable, ConfigLoweringError> {
    let mut lowered = ConfigTable::new();
    for (key, value) in table {
        lowered.insert(key.clone(), lower_value(value)?);
    }
    Ok(lowered)
}

/// One TOML value into its neutral counterpart — exhaustive by construction,
/// so a widened `toml::Value` stops compiling here rather than lowering into
/// a silently lossy identity.
fn lower_value(value: &toml::Value) -> Result<ConfigValue, ConfigLoweringError> {
    Ok(match value {
        toml::Value::String(text) => ConfigValue::String(text.clone()),
        toml::Value::Integer(number) => ConfigValue::Integer(*number),
        toml::Value::Float(number) => ConfigValue::Float(ConfigFloat::new(*number)),
        toml::Value::Boolean(flag) => ConfigValue::Boolean(*flag),
        toml::Value::Datetime(datetime) => ConfigValue::Datetime(lower_datetime(datetime)?),
        toml::Value::Array(items) => ConfigValue::Array(
            // Array order is SEMANTIC and retained (§3); only tables sort.
            items
                .iter()
                .map(lower_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        toml::Value::Table(table) => ConfigValue::Table(lower_table(table)?),
    })
}

/// One TOML datetime, component for component.
///
/// No render/parse round trip enters identity (§3): the three optional
/// members are copied straight across, and the offset's two spellings stay
/// two variants rather than collapsing into a minute count that could not
/// tell `Z` from `+00:00`.
fn lower_datetime(datetime: &toml::value::Datetime) -> Result<ConfigDatetime, ConfigLoweringError> {
    let date = datetime
        .date
        .map(|date| ConfigDate::new(date.year, date.month, date.day))
        .transpose()?;
    let time = datetime
        .time
        .map(|time| ConfigTime::new(time.hour, time.minute, time.second, time.nanosecond))
        .transpose()?;
    let offset = datetime
        .offset
        .map(|offset| match offset {
            toml::value::Offset::Z => Ok(ConfigOffset::Z),
            toml::value::Offset::Custom { minutes } => ConfigOffset::custom(minutes),
        })
        .transpose()?;
    Ok(ConfigDatetime::new(date, time, offset)?)
}

/// Why one row's effective configuration could not become plan identity.
///
/// One arm — and it is a value refusal, not a capability gap. T10B's
/// `ConfigLoweringGap::ValueTower` was named for the seam that would close
/// it; R4.2 closed that seam, so the family was renamed rather than left
/// asserting "not implemented yet" about a check that rejects a genuinely
/// illegal value. The neutral tree's constructors enforce the same laws
/// `toml_datetime`'s parser does, so a parsed manifest never reaches this
/// arm — but a `toml::value::Datetime` is a struct of public fields, and a
/// value built around the parser must refuse typed rather than panic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ConfigLoweringError {
    #[error("an effective configuration value is not a legal TOML datetime: {source}")]
    Datetime {
        #[source]
        source: ConfigDatetimeError,
    },
}

impl From<ConfigDatetimeError> for ConfigLoweringError {
    fn from(source: ConfigDatetimeError) -> Self {
        Self::Datetime { source }
    }
}
