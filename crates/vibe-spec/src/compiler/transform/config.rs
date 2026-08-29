//! The lossless neutral effective-configuration tree and its canonical digest.
//!
//! TOML datetime and the TOML number tower are not JSON values, so generic
//! JSON never enters plan identity; the workspace lowerer converts effective
//! `ExtensionConfig` rows into this owned tree instead
//! (R4-TRANSFORM-PLAN-ABI §3). Tables are sorted `BTreeMap`s, so table order
//! is semantic-insensitive; array order is semantic and retained. `None` at
//! the plan layer means no effective config was authored, `Some(empty)`
//! means an authored activation cleared the value — that distinction lives in
//! `TransformPlan` (T2), not here: this cell only ever digests a real table.

// `TransformPlan::build` digests through this cell, but the plan family
// stays crate-internal dead code until T4 carries it on `ArtifactPlan`;
// T10's workspace adapter is its real first cross-crate consumer. Until
// then only the transform cells and their tests construct these values.
#![allow(dead_code)]

use std::collections::BTreeMap;

use crate::compiler::digest::StableDigest;

/// The canonical digest domain of one effective-configuration table (epoch 1).
const CONFIG_DIGEST_DOMAIN: &[u8] = b"vibe-transform-config-v1\0epoch=1\0";

/// Closed value tags of the frozen config frame: a closed tag byte, then the
/// exact payload (R4-TRANSFORM-PLAN-ABI §4).
const TAG_STRING: u8 = 0;
const TAG_INTEGER: u8 = 1;
const TAG_FLOAT: u8 = 2;
const TAG_BOOLEAN: u8 = 3;
const TAG_DATETIME: u8 = 4;
const TAG_ARRAY: u8 = 5;
const TAG_TABLE: u8 = 6;

/// A neutral configuration table: sorted by key, so insertion order is not
/// semantic.
pub(crate) type ConfigTable = BTreeMap<String, ConfigValue>;

/// One lossless TOML semantic value: string/i64/canonical-float/bool/
/// field-preserving datetime, ordered array, or sorted table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfigValue {
    String(String),
    Integer(i64),
    Float(ConfigFloat),
    Boolean(bool),
    Datetime(ConfigDatetime),
    Array(Vec<ConfigValue>),
    Table(ConfigTable),
}

/// A float held as its canonical f64 bit key.
///
/// Every NaN spelling collapses to one key (the `EqTomlTable` law:
/// `f64::NAN.to_bits()`), while signed zero stays distinct — `+0.0` and
/// `-0.0` differ in identity exactly as their bits differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigFloat(u64);

impl ConfigFloat {
    /// Wrap an `f64` into its canonical bit key.
    pub(crate) fn new(value: f64) -> Self {
        Self(canonical_float_bits(value))
    }

    /// The canonical bit key: two floats are the same config value iff their
    /// keys are equal.
    pub(crate) fn bits(self) -> u64 {
        self.0
    }

    /// The `f64` carried by the canonical key.
    pub(crate) fn value(self) -> f64 {
        f64::from_bits(self.0)
    }
}

/// The `EqTomlTable` canonicalisation: every NaN is one key, all other
/// values keep their exact bits (so signed zero stays distinct).
fn canonical_float_bits(value: f64) -> u64 {
    if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

/// One calendar date, mirroring `toml_datetime::Date` field-for-field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigDate {
    year: u16,
    month: u8,
    day: u8,
}

impl ConfigDate {
    /// Checked against the same laws `toml_datetime`'s parser enforces:
    /// `date-fullyear` is exactly four digits (0000–9999), month 01–12,
    /// day 01–(the month's length, leap-year aware). A five-digit year is
    /// unrepresentable: it can never parse into a TOML value, so it never
    /// becomes a config identity here either.
    pub(crate) fn new(year: u16, month: u8, day: u8) -> Result<Self, ConfigDatetimeError> {
        if year > 9999 {
            return Err(ConfigDatetimeError::Component {
                component: "date.year",
                value: i64::from(year),
                law: "date-fullyear is exactly four digits (0000–9999)".to_string(),
            });
        }
        if !(1..=12).contains(&month) {
            return Err(ConfigDatetimeError::Component {
                component: "date.month",
                value: i64::from(month),
                law: "month between 01 and 12".to_string(),
            });
        }
        let max_day = days_in_month(year, month);
        if day < 1 || day > max_day {
            return Err(ConfigDatetimeError::Component {
                component: "date.day",
                value: i64::from(day),
                law: format!("day between 01 and {max_day:02}"),
            });
        }
        Ok(Self { year, month, day })
    }

    pub(crate) const fn year(self) -> u16 {
        self.year
    }

    pub(crate) const fn month(self) -> u8 {
        self.month
    }

    pub(crate) const fn day(self) -> u8 {
        self.day
    }
}

/// The TOML date law `toml_datetime`'s parser applies: February honors the
/// Gregorian leap-year rule.
fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// One wall-clock time, mirroring `toml_datetime::Time` field-for-field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigTime {
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
}

impl ConfigTime {
    /// Checked against the same laws `toml_datetime`'s parser enforces:
    /// hour 00–23, minute 00–59, second 00–60 (leap second), nanosecond
    /// within 999_999_999.
    pub(crate) fn new(
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> Result<Self, ConfigDatetimeError> {
        if hour > 23 {
            return Err(component_error("time.hour", i64::from(hour), 23));
        }
        if minute > 59 {
            return Err(component_error("time.minute", i64::from(minute), 59));
        }
        if second > 60 {
            return Err(component_error("time.second", i64::from(second), 60));
        }
        if nanosecond > 999_999_999 {
            return Err(ConfigDatetimeError::Component {
                component: "time.nanosecond",
                value: i64::from(nanosecond),
                law: "nanosecond within 999_999_999".to_string(),
            });
        }
        Ok(Self {
            hour,
            minute,
            second,
            nanosecond,
        })
    }

    pub(crate) const fn hour(self) -> u8 {
        self.hour
    }

    pub(crate) const fn minute(self) -> u8 {
        self.minute
    }

    pub(crate) const fn second(self) -> u8 {
        self.second
    }

    pub(crate) const fn nanosecond(self) -> u32 {
        self.nanosecond
    }
}

fn component_error(component: &'static str, value: i64, max: i64) -> ConfigDatetimeError {
    ConfigDatetimeError::Component {
        component,
        value,
        law: format!("value between 00 and {max:02}"),
    }
}

/// One UTC offset, mirroring `toml_datetime::Offset`: `Z` and a signed
/// custom minute offset are distinct identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigOffset {
    Z,
    Custom { minutes: i16 },
}

impl ConfigOffset {
    /// Checked against the parser's law: ±HH:MM with HH 00–23 and MM 00–59,
    /// i.e. minutes within ±(23*60+59).
    pub(crate) fn custom(minutes: i16) -> Result<Self, ConfigDatetimeError> {
        if !(-1439..=1439).contains(&minutes) {
            return Err(ConfigDatetimeError::Component {
                component: "offset.minutes",
                value: i64::from(minutes),
                law: "minutes between -1439 and 1439".to_string(),
            });
        }
        Ok(Self::Custom { minutes })
    }

    /// The signed offset minutes; `None` for `Z`.
    pub(crate) fn minutes(self) -> Option<i16> {
        match self {
            Self::Z => None,
            Self::Custom { minutes } => Some(minutes),
        }
    }
}

/// One TOML datetime, mirroring `toml_datetime::Datetime` field-for-field
/// with optional date/time/offset; no render/parse round trip enters identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigDatetime {
    date: Option<ConfigDate>,
    time: Option<ConfigTime>,
    offset: Option<ConfigOffset>,
}

impl ConfigDatetime {
    /// The shape law `toml_datetime` itself enforces: local date, local
    /// time, local date-time, and offset date-time are the four legal
    /// shapes — an offset requires both date and time, and a datetime with
    /// neither component is unrepresentable.
    pub(crate) fn new(
        date: Option<ConfigDate>,
        time: Option<ConfigTime>,
        offset: Option<ConfigOffset>,
    ) -> Result<Self, ConfigDatetimeError> {
        if date.is_none() && time.is_none() {
            return Err(ConfigDatetimeError::Shape {
                date: false,
                time: false,
                offset: offset.is_some(),
            });
        }
        if offset.is_some() && (date.is_none() || time.is_none()) {
            return Err(ConfigDatetimeError::Shape {
                date: date.is_some(),
                time: time.is_some(),
                offset: true,
            });
        }
        Ok(Self { date, time, offset })
    }

    pub(crate) const fn date(&self) -> Option<ConfigDate> {
        self.date
    }

    pub(crate) const fn time(&self) -> Option<ConfigTime> {
        self.time
    }

    pub(crate) const fn offset(&self) -> Option<ConfigOffset> {
        self.offset
    }
}

/// A datetime component or shape violates the TOML datetime laws mirrored
/// from `toml_datetime`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ConfigDatetimeError {
    #[error(
        "datetime component `{component}` value {value} is not a legal TOML datetime value ({law})"
    )]
    Component {
        component: &'static str,
        value: i64,
        law: String,
    },
    #[error(
        "a datetime with date={date} time={time} offset={offset} is not a legal TOML datetime shape; one of date/time must be present, and an offset requires both"
    )]
    Shape {
        date: bool,
        time: bool,
        offset: bool,
    },
}

/// The canonical digest of one effective-configuration table under the frozen
/// `vibe-transform-config-v1` epoch-1 domain.
///
/// Absence of authored config is NOT a config digest: presence is the plan
/// layer's `Option`, never a value here. An authored empty table is a real
/// table and digests stably.
pub(crate) fn config_digest(table: &ConfigTable) -> ConfigDigest {
    let mut digest = StableDigest::new(CONFIG_DIGEST_DOMAIN);
    digest.byte(TAG_TABLE);
    frame_table(&mut digest, table);
    ConfigDigest(digest.finish())
}

/// The 32-byte canonical digest of one effective-configuration table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigDigest([u8; 32]);

impl ConfigDigest {
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Frame one value: a closed tag byte, then the exact payload.
fn frame_value(digest: &mut StableDigest, value: &ConfigValue) {
    match value {
        ConfigValue::String(text) => {
            digest.byte(TAG_STRING);
            digest.field(text.as_bytes());
        }
        ConfigValue::Integer(value) => {
            digest.byte(TAG_INTEGER);
            // i64 little-endian: the two's-complement bits in the u64 width.
            digest.u64(*value as u64);
        }
        ConfigValue::Float(float) => {
            digest.byte(TAG_FLOAT);
            digest.u64(float.bits());
        }
        ConfigValue::Boolean(value) => {
            digest.byte(TAG_BOOLEAN);
            digest.byte(u8::from(*value));
        }
        ConfigValue::Datetime(datetime) => {
            digest.byte(TAG_DATETIME);
            frame_datetime(digest, datetime);
        }
        ConfigValue::Array(items) => {
            digest.byte(TAG_ARRAY);
            digest.usize(items.len());
            for item in items {
                frame_value(digest, item);
            }
        }
        ConfigValue::Table(table) => {
            digest.byte(TAG_TABLE);
            frame_table(digest, table);
        }
    }
}

/// Frame a table: the entry count, then sorted key/value pairs (`BTreeMap`
/// iteration is the sorted order).
fn frame_table(digest: &mut StableDigest, table: &ConfigTable) {
    digest.usize(table.len());
    for (key, value) in table {
        digest.field(key.as_bytes());
        frame_value(digest, value);
    }
}

/// Frame a datetime: explicit presence bytes, then the present numeric
/// components; `Z` and a signed minute offset are distinct variants.
fn frame_datetime(digest: &mut StableDigest, datetime: &ConfigDatetime) {
    match datetime.date() {
        Some(date) => {
            digest.byte(1);
            digest.u32(u32::from(date.year()));
            digest.byte(date.month());
            digest.byte(date.day());
        }
        None => digest.byte(0),
    }
    match datetime.time() {
        Some(time) => {
            digest.byte(1);
            digest.byte(time.hour());
            digest.byte(time.minute());
            digest.byte(time.second());
            digest.u32(time.nanosecond());
        }
        None => digest.byte(0),
    }
    match datetime.offset() {
        Some(ConfigOffset::Z) => {
            digest.byte(1);
            digest.byte(0);
        }
        Some(ConfigOffset::Custom { minutes }) => {
            digest.byte(1);
            digest.byte(1);
            // Signed minutes in the u32 width: two's-complement bits.
            digest.u32(minutes as u32);
        }
        None => digest.byte(0),
    }
}
