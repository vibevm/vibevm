//! Identity and digest REDs of the neutral effective-configuration tree
//! (PROP-054 `#TRANSFORM-PLAN-IDENTITY`).

use sha2::{Digest, Sha256};
use specmark::verifies;

use super::config::{
    ConfigDate, ConfigDatetime, ConfigDatetimeError, ConfigFloat, ConfigOffset, ConfigTable,
    ConfigTime, ConfigValue, config_digest,
};

fn table(pairs: &[(&str, ConfigValue)]) -> ConfigTable {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

fn datetime(
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    nano: u32,
) -> ConfigValue {
    ConfigValue::Datetime(
        ConfigDatetime::new(
            Some(ConfigDate::new(year, month, day).unwrap()),
            Some(ConfigTime::new(hour, minute, second, nano).unwrap()),
            None,
        )
        .unwrap(),
    )
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn table_key_insertion_order_is_not_semantic_at_any_depth() {
    let mut forward_nested = ConfigTable::new();
    forward_nested.insert("x".to_string(), ConfigValue::Integer(1));
    forward_nested.insert("a".to_string(), ConfigValue::Integer(2));
    let mut forward = ConfigTable::new();
    forward.insert("beta".to_string(), ConfigValue::Boolean(true));
    forward.insert("alpha".to_string(), ConfigValue::Table(forward_nested));

    let mut reverse_nested = ConfigTable::new();
    reverse_nested.insert("a".to_string(), ConfigValue::Integer(2));
    reverse_nested.insert("x".to_string(), ConfigValue::Integer(1));
    let mut reverse = ConfigTable::new();
    reverse.insert("alpha".to_string(), ConfigValue::Table(reverse_nested));
    reverse.insert("beta".to_string(), ConfigValue::Boolean(true));

    assert_eq!(forward, reverse);
    assert_eq!(config_digest(&forward), config_digest(&reverse));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn array_order_is_semantic_and_moves_the_digest() {
    let ascending = table(&[(
        "items",
        ConfigValue::Array(vec![ConfigValue::Integer(1), ConfigValue::Integer(2)]),
    )]);
    let descending = table(&[(
        "items",
        ConfigValue::Array(vec![ConfigValue::Integer(2), ConfigValue::Integer(1)]),
    )]);
    assert_ne!(ascending, descending);
    assert_ne!(config_digest(&ascending), config_digest(&descending));
}

#[test]
fn integer_one_float_one_and_boolean_true_are_three_identities() {
    let integer = table(&[("v", ConfigValue::Integer(1))]);
    let float = table(&[("v", ConfigValue::Float(ConfigFloat::new(1.0)))]);
    let boolean = table(&[("v", ConfigValue::Boolean(true))]);
    assert_ne!(config_digest(&integer), config_digest(&float));
    assert_ne!(config_digest(&integer), config_digest(&boolean));
    assert_ne!(config_digest(&float), config_digest(&boolean));
    // A string spelling collides with none of them either.
    let string = table(&[("v", ConfigValue::String("1".to_string()))]);
    assert_ne!(config_digest(&integer), config_digest(&string));
}

#[test]
fn signed_zeros_stay_distinct() {
    let positive = table(&[("v", ConfigValue::Float(ConfigFloat::new(0.0)))]);
    let negative = table(&[("v", ConfigValue::Float(ConfigFloat::new(-0.0)))]);
    assert_eq!(ConfigFloat::new(0.0).bits(), 0);
    assert_eq!(ConfigFloat::new(-0.0).bits(), 1 << 63);
    assert_ne!(positive, negative);
    assert_ne!(config_digest(&positive), config_digest(&negative));
}

#[test]
fn every_nan_spelling_is_one_canonical_identity() {
    let quiet = ConfigFloat::new(f64::NAN);
    let payload = ConfigFloat::new(f64::from_bits(0x7ff8_0000_0000_0001));
    let negative_payload = ConfigFloat::new(f64::from_bits(0xfff0_0000_0000_0001));
    assert_eq!(quiet, payload);
    assert_eq!(quiet, negative_payload);
    assert_eq!(quiet.bits(), f64::NAN.to_bits());
    let one = table(&[("v", ConfigValue::Float(quiet))]);
    let other = table(&[("v", ConfigValue::Float(negative_payload))]);
    assert_eq!(config_digest(&one), config_digest(&other));
    // The canonical NaN is its own key, not any finite neighbor's.
    assert_ne!(
        config_digest(&one),
        config_digest(&table(&[("v", ConfigValue::Float(ConfigFloat::new(0.0)))]))
    );
}

#[test]
fn local_z_and_signed_offsets_are_distinct_datetime_identities() {
    let mut digests = Vec::new();
    for offset in [
        None,
        Some(ConfigOffset::Z),
        Some(ConfigOffset::custom(120).unwrap()),
    ] {
        let value = ConfigValue::Datetime(
            ConfigDatetime::new(
                Some(ConfigDate::new(2026, 8, 29).unwrap()),
                Some(ConfigTime::new(12, 34, 56, 789).unwrap()),
                offset,
            )
            .unwrap(),
        );
        digests.push(config_digest(&table(&[("when", value)])));
    }
    let minus = ConfigValue::Datetime(
        ConfigDatetime::new(
            Some(ConfigDate::new(2026, 8, 29).unwrap()),
            Some(ConfigTime::new(12, 34, 56, 789).unwrap()),
            Some(ConfigOffset::custom(-120).unwrap()),
        )
        .unwrap(),
    );
    digests.push(config_digest(&table(&[("when", minus)])));
    for left in 0..digests.len() {
        for right in (left + 1)..digests.len() {
            assert_ne!(
                digests[left], digests[right],
                "offsets {left}/{right} collided"
            );
        }
    }
}

#[test]
fn every_datetime_component_binds_the_digest() {
    let base = datetime(2026, 8, 29, 23, 59, 60, 999_999_999);
    let base_digest = config_digest(&table(&[("when", base)]));
    for changed in [
        datetime(2025, 8, 29, 23, 59, 60, 999_999_999),
        datetime(2026, 7, 29, 23, 59, 60, 999_999_999),
        datetime(2026, 8, 28, 23, 59, 60, 999_999_999),
        datetime(2026, 8, 29, 22, 59, 60, 999_999_999),
        datetime(2026, 8, 29, 23, 58, 60, 999_999_999),
        datetime(2026, 8, 29, 23, 59, 59, 999_999_999),
        datetime(2026, 8, 29, 23, 59, 60, 999_999_998),
    ] {
        assert_ne!(
            base_digest,
            config_digest(&table(&[("when", changed)])),
            "a datetime component change did not move the digest"
        );
    }
}

#[test]
fn the_empty_table_is_a_real_stable_digest_and_absence_is_not_one() {
    let empty = ConfigTable::new();
    let first = config_digest(&empty);
    assert_eq!(first, config_digest(&ConfigTable::new()));
    assert_ne!(*first.as_bytes(), [0; 32]);
    assert_ne!(
        first,
        config_digest(&table(&[("v", ConfigValue::Boolean(false))]))
    );
    // Absence of authored config is the plan layer's `Option` (T2); this
    // cell's only input is a real table, so no digest here can spell absence.
}

/// A longhand golden assembled byte-by-byte from the frozen ABI, independent
/// of the streaming production writer: buffer the exact frame, then hash
/// once. A shared framing bug has nothing to hide behind.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_longhand_vector_independent_of_the_production_writer_agrees() {
    let mut subject = ConfigTable::new();
    subject.insert("alpha".to_string(), ConfigValue::String("one".to_string()));
    subject.insert("beta".to_string(), ConfigValue::Integer(-2));
    subject.insert(
        "gamma".to_string(),
        ConfigValue::Float(ConfigFloat::new(1.5)),
    );
    subject.insert("delta".to_string(), ConfigValue::Boolean(true));
    subject.insert(
        "epsilon".to_string(),
        ConfigValue::Datetime(
            ConfigDatetime::new(
                Some(ConfigDate::new(2026, 8, 29).unwrap()),
                Some(ConfigTime::new(12, 34, 56, 789).unwrap()),
                Some(ConfigOffset::custom(-75).unwrap()),
            )
            .unwrap(),
        ),
    );
    subject.insert(
        "zeta".to_string(),
        ConfigValue::Array(vec![ConfigValue::Integer(1), ConfigValue::Boolean(false)]),
    );
    let mut nested = ConfigTable::new();
    nested.insert("inner".to_string(), ConfigValue::String("v".to_string()));
    subject.insert("eta".to_string(), ConfigValue::Table(nested));

    // Entries iterate sorted: alpha, beta, delta, epsilon, eta, gamma, zeta.
    let mut frame: Vec<u8> = Vec::new();
    let domain = b"vibe-transform-config-v1\0epoch=1\0";
    frame.extend_from_slice(&(domain.len() as u64).to_le_bytes());
    frame.extend_from_slice(domain);
    frame.push(6); // table tag
    frame.extend_from_slice(&7u64.to_le_bytes()); // seven entries

    // alpha: String("one")
    frame.extend_from_slice(&5u64.to_le_bytes());
    frame.extend_from_slice(b"alpha");
    frame.push(0);
    frame.extend_from_slice(&3u64.to_le_bytes());
    frame.extend_from_slice(b"one");
    // beta: Integer(-2)
    frame.extend_from_slice(&4u64.to_le_bytes());
    frame.extend_from_slice(b"beta");
    frame.push(1);
    frame.extend_from_slice(&(-2i64).to_le_bytes());
    // delta: Boolean(true)
    frame.extend_from_slice(&5u64.to_le_bytes());
    frame.extend_from_slice(b"delta");
    frame.push(3);
    frame.push(1);
    // epsilon: Datetime 2026-08-29T12:34:56.789-01:15
    frame.extend_from_slice(&7u64.to_le_bytes());
    frame.extend_from_slice(b"epsilon");
    frame.push(4);
    frame.push(1); // date present
    frame.extend_from_slice(&2026u32.to_le_bytes());
    frame.push(8);
    frame.push(29);
    frame.push(1); // time present
    frame.push(12);
    frame.push(34);
    frame.push(56);
    frame.extend_from_slice(&789u32.to_le_bytes());
    frame.push(1); // offset present
    frame.push(1); // custom variant
    frame.extend_from_slice(&(-75i32 as u32).to_le_bytes());
    // eta: Table { inner: String("v") }
    frame.extend_from_slice(&3u64.to_le_bytes());
    frame.extend_from_slice(b"eta");
    frame.push(6);
    frame.extend_from_slice(&1u64.to_le_bytes());
    frame.extend_from_slice(&5u64.to_le_bytes());
    frame.extend_from_slice(b"inner");
    frame.push(0);
    frame.extend_from_slice(&1u64.to_le_bytes());
    frame.extend_from_slice(b"v");
    // gamma: Float(1.5)
    frame.extend_from_slice(&5u64.to_le_bytes());
    frame.extend_from_slice(b"gamma");
    frame.push(2);
    frame.extend_from_slice(&1.5f64.to_bits().to_le_bytes());
    // zeta: Array [Integer(1), Boolean(false)]
    frame.extend_from_slice(&4u64.to_le_bytes());
    frame.extend_from_slice(b"zeta");
    frame.push(5);
    frame.extend_from_slice(&2u64.to_le_bytes());
    frame.push(1);
    frame.extend_from_slice(&1i64.to_le_bytes());
    frame.push(3);
    frame.push(0);

    let mut hand = Sha256::new();
    hand.update(&frame);
    let expected: [u8; 32] = hand.finalize().into();
    assert_eq!(config_digest(&subject).as_bytes(), &expected);
}

#[test]
fn checked_datetime_constructors_reject_lawless_components() {
    assert!(ConfigDate::new(2026, 0, 10).is_err());
    assert!(ConfigDate::new(2026, 13, 10).is_err());
    assert!(ConfigDate::new(2026, 1, 0).is_err());
    assert!(ConfigDate::new(2026, 4, 31).is_err());
    assert!(ConfigDate::new(2025, 2, 29).is_err());
    assert!(ConfigDate::new(2024, 2, 29).is_ok());
    assert!(ConfigTime::new(24, 0, 0, 0).is_err());
    assert!(ConfigTime::new(23, 60, 0, 0).is_err());
    assert!(ConfigTime::new(23, 59, 61, 0).is_err());
    assert!(ConfigTime::new(23, 59, 60, 0).is_ok());
    assert!(ConfigTime::new(23, 59, 59, 1_000_000_000).is_err());
    assert!(ConfigOffset::custom(1440).is_err());
    assert!(ConfigOffset::custom(-1440).is_err());
    assert!(ConfigOffset::custom(1439).is_ok());
    assert!(ConfigOffset::custom(-1439).is_ok());
}

/// `toml_datetime` parses `date-fullyear` as exactly four digits
/// (`date-fullyear = 4DIGIT`), so the neutral tree mirrors that law at its
/// checked constructor: the boundary year is a real identity, and a
/// five-digit year can never become a `ConfigDate` value.
#[test]
fn the_year_law_is_exactly_four_digits_at_both_boundaries() {
    let boundary = ConfigDate::new(9999, 12, 31).unwrap();
    assert_eq!(
        (boundary.year(), boundary.month(), boundary.day()),
        (9999, 12, 31)
    );
    // The accepted boundary is a first-class identity: it digests stably.
    let value = ConfigValue::Datetime(ConfigDatetime::new(Some(boundary), None, None).unwrap());
    assert_eq!(
        config_digest(&table(&[("when", value)])),
        config_digest(&table(&[(
            "when",
            ConfigValue::Datetime(ConfigDatetime::new(Some(boundary), None, None).unwrap())
        )]))
    );
    // A five-digit year is refused — at the boundary and at the type's own
    // ceiling — as a `date.year` component violation, never silently kept.
    for unlawful in [10000, u16::MAX] {
        match ConfigDate::new(unlawful, 1, 1) {
            Err(ConfigDatetimeError::Component {
                component: "date.year",
                ..
            }) => {}
            other => panic!("expected a date.year refusal for {unlawful}, got {other:?}"),
        }
    }
}

#[test]
fn the_datetime_shape_law_matches_toml_datetime() {
    let date = ConfigDate::new(2026, 8, 29).unwrap();
    let time = ConfigTime::new(12, 34, 56, 789).unwrap();
    assert!(ConfigDatetime::new(None, None, None).is_err());
    assert!(ConfigDatetime::new(None, Some(time), Some(ConfigOffset::Z)).is_err());
    assert!(ConfigDatetime::new(Some(date), None, Some(ConfigOffset::Z)).is_err());
    assert!(ConfigDatetime::new(Some(date), None, None).is_ok());
    assert!(ConfigDatetime::new(None, Some(time), None).is_ok());
    assert!(ConfigDatetime::new(Some(date), Some(time), None).is_ok());
    assert!(ConfigDatetime::new(Some(date), Some(time), Some(ConfigOffset::Z)).is_ok());
}

#[test]
fn the_digest_is_deterministic_and_length_framing_separates_shapes() {
    let subject = table(&[("ab", ConfigValue::Boolean(false))]);
    assert_eq!(
        config_digest(&subject),
        config_digest(&subject.clone()),
        "digesting a clone changed the digest"
    );
    // Same key prefix, different length and shape: the u64 length frame and
    // the tag byte keep a flat key from impersonating a nested one.
    let nested = table(&[(
        "a",
        ConfigValue::Table(table(&[("b", ConfigValue::Boolean(false))])),
    )]);
    assert_ne!(config_digest(&subject), config_digest(&nested));
    // Empty array, empty table, and empty string are three different empties.
    let empty_array = table(&[("v", ConfigValue::Array(Vec::new()))]);
    let empty_table = table(&[("v", ConfigValue::Table(ConfigTable::new()))]);
    let empty_string = table(&[("v", ConfigValue::String(String::new()))]);
    assert_ne!(config_digest(&empty_array), config_digest(&empty_table));
    assert_ne!(config_digest(&empty_array), config_digest(&empty_string));
    assert_ne!(config_digest(&empty_table), config_digest(&empty_string));
}

#[test]
fn accessors_expose_the_frozen_components_without_new_identity() {
    let date = ConfigDate::new(2026, 8, 29).unwrap();
    let time = ConfigTime::new(12, 34, 56, 789).unwrap();
    let datetime = ConfigDatetime::new(
        Some(date),
        Some(time),
        Some(ConfigOffset::custom(-75).unwrap()),
    )
    .unwrap();
    assert_eq!(datetime.date(), Some(date));
    assert_eq!(datetime.time(), Some(time));
    assert_eq!(datetime.offset().and_then(ConfigOffset::minutes), Some(-75));
    assert_eq!(ConfigOffset::Z.minutes(), None);
    assert_eq!((date.year(), date.month(), date.day()), (2026, 8, 29));
    assert_eq!(
        (time.hour(), time.minute(), time.second(), time.nanosecond()),
        (12, 34, 56, 789)
    );
    assert_eq!(ConfigFloat::new(2.5).value(), 2.5);
    assert!(ConfigFloat::new(f64::NAN).value().is_nan());
}
