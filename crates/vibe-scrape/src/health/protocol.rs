//! Strict custom-result decoding and relational semantic checks.

use std::collections::BTreeSet;
use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

use super::model::{Finding, HealthError, HealthStatus, Severity, StructuredVerdict};

pub const DEFAULT_RESULT_CAP: usize = 1024 * 1024;

pub fn parse_health_result(bytes: &[u8], cap: usize) -> Result<StructuredVerdict, HealthError> {
    if bytes.len() > cap {
        return Err(HealthError::Protocol(format!(
            "custom result is {} bytes, over the {cap}-byte cap",
            bytes.len()
        )));
    }
    std::str::from_utf8(bytes)
        .map_err(|error| HealthError::Protocol(format!("custom result is not UTF-8: {error}")))?;
    reject_duplicate_keys(bytes)?;
    let wire: vibe_wire::generated::scrape::e1::health_result::HealthResult =
        serde_json::from_slice(bytes)
            .map_err(|error| HealthError::Protocol(format!("invalid health JSON: {error}")))?;
    if wire.protocol != 1 {
        return Err(HealthError::Protocol(format!(
            "custom result protocol must equal 1, got {}",
            wire.protocol
        )));
    }
    let mut ids = BTreeSet::new();
    let mut has_warning = false;
    let mut has_error = false;
    let findings = wire
        .findings
        .into_iter()
        .map(|finding| {
            if finding.id.is_empty() {
                return Err(HealthError::Protocol(
                    "custom finding id must be nonempty".to_owned(),
                ));
            }
            if !ids.insert(finding.id.clone()) {
                return Err(HealthError::Protocol(format!(
                    "duplicate custom finding id `{}`",
                    finding.id
                )));
            }
            let severity = match finding.severity {
                vibe_wire::generated::scrape::e1::health_result::Severity::Info => Severity::Info,
                vibe_wire::generated::scrape::e1::health_result::Severity::Warning => {
                    has_warning = true;
                    Severity::Warning
                }
                vibe_wire::generated::scrape::e1::health_result::Severity::Error => {
                    has_error = true;
                    Severity::Error
                }
            };
            Ok(Finding {
                id: finding.id,
                severity,
                message: finding.message,
                evidence: finding.evidence,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let status = match wire.status {
        vibe_wire::generated::scrape::e1::health_result::HealthStatus::Pass => {
            if has_warning || has_error {
                return Err(HealthError::Protocol(
                    "pass result carries a warning/error finding".to_owned(),
                ));
            }
            HealthStatus::Pass
        }
        vibe_wire::generated::scrape::e1::health_result::HealthStatus::Warn => {
            if !has_warning || has_error {
                return Err(HealthError::Protocol(
                    "warn result requires a warning and forbids errors".to_owned(),
                ));
            }
            HealthStatus::Warn
        }
        vibe_wire::generated::scrape::e1::health_result::HealthStatus::Fail => {
            if !has_error {
                return Err(HealthError::Protocol(
                    "fail result requires an error finding".to_owned(),
                ));
            }
            HealthStatus::Fail
        }
    };
    Ok(StructuredVerdict {
        status,
        summary: wire.summary,
        findings,
        metrics: wire.metrics,
    })
}

pub(crate) fn reject_duplicate_keys(bytes: &[u8]) -> Result<(), HealthError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    NoDuplicateKeys
        .deserialize(&mut deserializer)
        .map_err(|error| {
            HealthError::Protocol(format!("invalid or duplicate-key JSON: {error}"))
        })?;
    deserializer
        .end()
        .map_err(|error| HealthError::Protocol(format!("trailing JSON input: {error}")))
}

#[derive(Clone, Copy)]
struct NoDuplicateKeys;

impl<'de> DeserializeSeed<'de> for NoDuplicateKeys {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for NoDuplicateKeys {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_string<E>(self, _: String) -> Result<(), E> {
        Ok(())
    }
    fn visit_none<E>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_some<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.deserialize(deserializer)
    }
    fn visit_seq<A>(self, mut sequence: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element_seed(self)?.is_some() {}
        Ok(())
    }
    fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!("duplicate object key `{key}`")));
            }
            map.next_value_seed(self)?;
        }
        Ok(())
    }
}
