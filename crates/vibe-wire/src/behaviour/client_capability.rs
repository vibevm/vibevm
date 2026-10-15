//! Semantic checks over the separate generated capability family.
use crate::generated::client_capability::e1::{error as e, request as q, result as r};
fn text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
}
fn digest(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
pub fn request(v: &q::Request) -> Result<(), &'static str> {
    if v.schema != 1 || !text(&v.context_id) || !text(&v.target_id) {
        Err("invalid request binding")
    } else {
        Ok(())
    }
}
pub fn result(v: &r::Result) -> Result<(), &'static str> {
    use sha2::{Digest, Sha256};
    if v.schema != 1
        || !text(&v.context_id)
        || !text(&v.target_id)
        || !digest(&v.capture_id)
        || !text(&v.root)
        || v.capabilities.len() > 4096
    {
        return Err("invalid capture binding");
    }
    for (id, c) in &v.capabilities {
        if c.provider
            .parse::<vibe_core::manifest::ProviderPin>()
            .is_err()
            || !digest(&c.descriptor_digest)
            || !digest(&c.manifest_digest)
        {
            return Err("invalid provenance");
        }
        let request: vibe_core::manifest::ClientRequest = serde_json::from_value(serde_json::json!({"schema":1,"client_id":id,"version":c.version,"bindings":c.bindings})).map_err(|_|"invalid descriptor")?;
        request.validate().map_err(|_| "invalid descriptor")?;
        let expected = format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&request).map_err(|_| "descriptor encoding")?)
        );
        if c.descriptor_digest != expected {
            return Err("descriptor digest mismatch");
        }
    }
    Ok(())
}
/// Reject duplicate map keys before the generated reader can collapse them.
pub fn read_result(bytes: &[u8]) -> Result<r::Result, &'static str> {
    let unique: Unique =
        serde_json::from_slice(bytes).map_err(|_| "invalid or duplicate JSON member")?;
    let value: r::Result =
        serde_json::from_value(unique.0).map_err(|_| "invalid generated result")?;
    result(&value)?;
    Ok(value)
}
struct Unique(serde_json::Value);
impl<'de> serde::Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON without duplicate members")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut map = serde_json::Map::new();
                while let Some((k, v)) = a.next_entry::<String, Unique>()? {
                    if map.insert(k, v.0).is_some() {
                        return Err(serde::de::Error::custom("duplicate member"));
                    }
                }
                Ok(Unique(serde_json::Value::Object(map)))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<Unique>()? {
                    v.push(x.0);
                }
                Ok(Unique(v.into()))
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Unique, E> {
                Ok(Unique(s.into()))
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                Ok(Unique(serde_json::json!(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(serde_json::Value::Null))
            }
        }
        d.deserialize_any(Visitor)
    }
}
pub fn publication(
    v: &crate::generated::machine::e1::operation_envelope::OperationEnvelope,
) -> Result<(), &'static str> {
    use crate::generated::machine::e1::operation_envelope as env;
    if v.schema != 1
        || v.operation_id.as_deref() != Some("rust:vibe-orchestrator:client-capabilities")
        || v.domain_owner.as_deref() != Some("rust:vibe-orchestrator")
        || v.source_id.as_deref() != Some("captured-client-capabilities/1")
        || v.evidence_id.is_some()
    {
        return Err("foreign envelope");
    }
    match &v.body {
        env::Body::Result(b) => {
            if b.format_id.as_deref() != Some("client-capability-result")
                || b.effect_state != env::BodyResultEffectState::NoEffects
                || b.completeness != env::BodyResultCompleteness::Complete
            {
                return Err("foreign result semantics");
            }
            let result = read_result(b.payload_json.as_deref().ok_or("payload")?.as_bytes())?;
            if b.provenance_id.as_deref() != Some(result.capture_id.as_str()) {
                return Err("foreign provenance");
            }
        }
        env::Body::Error(b) => {
            if b.format_id.as_deref() != Some("client-capability-error")
                || b.effect_state != env::BodyErrorEffectState::NoEffects
                || b.completeness != env::BodyErrorCompleteness::Unavailable
                || b.provenance_id.is_some()
            {
                return Err("foreign error semantics");
            }
            let value: e::Error = serde_json::from_str(b.payload_json.as_deref().ok_or("payload")?)
                .map_err(|_| "invalid error")?;
            error(&value)?;
        }
        _ => return Err("expected publication"),
    }
    Ok(())
}
pub fn error(v: &e::Error) -> Result<(), &'static str> {
    if v.schema != 1 || !text(&v.message) || !text(&v.context_id) || !text(&v.target_id) {
        Err("invalid error")
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod m_08_c_1_cases {
    use super::*;
    #[test]
    fn map_closed_schema_and_operations_keep_open_qualified_ids() {
        let bytes = include_bytes!("../../../../formats/corpora/client-capability/e1/result.json");
        let v = read_result(bytes).unwrap();
        let legacy = read_result(include_bytes!(
            "../../../../formats/corpora/client-capability/e1/legacy3.json"
        ))
        .unwrap();
        assert_eq!(legacy.capabilities.len(), 3);
        let mut invalid = v.clone();
        invalid.schema = 2;
        assert!(result(&invalid).is_err());
        let c = invalid
            .capabilities
            .get_mut("org.example/new-client")
            .unwrap();
        c.bindings.push(c.bindings[0].clone());
        invalid.schema = 1;
        assert!(result(&invalid).is_err());
        use crate::generated::machine::e1::operation_envelope as env;
        let envelope = env::OperationEnvelope {
            schema: 1,
            operation_id: Some("rust:vibe-orchestrator:client-capabilities".into()),
            domain_owner: Some("rust:vibe-orchestrator".into()),
            source_id: Some("captured-client-capabilities/1".into()),
            evidence_id: None,
            body: env::Body::Result(Box::new(env::BodyResult {
                format_id: Some("client-capability-result".into()),
                payload_json: Some(serde_json::to_string(&v).unwrap()),
                provenance_id: Some(v.capture_id.clone()),
                effect_state: env::BodyResultEffectState::NoEffects,
                completeness: env::BodyResultCompleteness::Complete,
            })),
        };
        publication(&envelope).unwrap();
        let mut wrong = envelope.clone();
        wrong.operation_id = Some("rust:vibe-orchestrator:adapter-plan".into());
        assert!(publication(&wrong).is_err());
        let mut wrong = envelope.clone();
        let env::Body::Result(body) = &mut wrong.body else {
            unreachable!()
        };
        body.format_id = Some("adapter-plan-result".into());
        assert!(publication(&wrong).is_err());
        let mut wrong = envelope;
        let env::Body::Result(body) = &mut wrong.body else {
            unreachable!()
        };
        body.effect_state = env::BodyResultEffectState::Completed;
        assert!(publication(&wrong).is_err());

        assert!(read_result(br#"{"schema":1,"schema":1}"#).is_err());
        let mut value = serde_json::to_value(&v).unwrap();
        value["capabilities"]["org.example/new-client"]["bindings"][0]["operations"] =
            serde_json::json!(["launch"]);
        assert!(serde_json::from_value::<r::Result>(value).is_err());
        assert!(crate::behaviour::adapter_plan::read_result(bytes).is_err());
        let q: q::Request = serde_json::from_slice(include_bytes!(
            "../../../../formats/corpora/client-capability/e1/request.json"
        ))
        .unwrap();
        request(&q).unwrap();
        assert!(
            crate::behaviour::adapter_plan::read_request(&serde_json::to_vec(&q).unwrap()).is_err()
        );
        let e: e::Error = serde_json::from_slice(include_bytes!(
            "../../../../formats/corpora/client-capability/e1/error.json"
        ))
        .unwrap();
        error(&e).unwrap();
    }
    #[test]
    fn legacy_three_wire_roundtrip_and_unrepresentable_key_refusal() {
        use crate::generated::adapter_plan::e1::{error::Error, request::Request, result::Result};
        fn roundtrip<T: serde::de::DeserializeOwned + serde::Serialize>(bytes: &[u8]) {
            let v: T = serde_json::from_slice(bytes).unwrap();
            let emitted = serde_json::to_vec(&v).unwrap();
            let again: T = serde_json::from_slice(&emitted).unwrap();
            assert_eq!(emitted, serde_json::to_vec(&again).unwrap());
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(bytes).unwrap(),
                serde_json::to_value(v).unwrap()
            );
        }
        roundtrip::<Request>(include_bytes!(
            "../../../../formats/corpora/adapter-plan/e1/request.json"
        ));
        roundtrip::<Result>(include_bytes!(
            "../../../../formats/corpora/adapter-plan/e1/plan.json"
        ));
        roundtrip::<Result>(include_bytes!(
            "../../../../formats/corpora/adapter-plan/e1/session.json"
        ));
        roundtrip::<Error>(include_bytes!(
            "../../../../formats/corpora/adapter-plan/e1/error.json"
        ));
        roundtrip::<Error>(include_bytes!(
            "../../../../formats/corpora/adapter-plan/e1/currentness-unavailable.json"
        ));
    }
}
