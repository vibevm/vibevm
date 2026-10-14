//! Semantic bridge from generated carriers to the authored owner. No proof promotion.
use crate::generated::{
    derivative_binding::DerivativeBinding as WireBinding, derivative_result::DerivativeResult,
};
use vibe_core::manifest::derivation::{BindingError, DerivationBinding};
impl WireBinding {
    pub fn checked_binding(&self) -> Result<DerivationBinding, BindingError> {
        serde_json::from_value(
            serde_json::to_value(self).map_err(|_| BindingError("binding encoding"))?,
        )
        .map_err(|_| BindingError("binding semantics"))
    }
}
impl DerivativeResult {
    /// Checks internal retained provenance, not freshness or source authenticity.
    /// The workspace owner additionally requires the exact current observation.
    pub fn checked_binding(&self) -> Result<DerivationBinding, BindingError> {
        let binding = DerivationBinding::parse(&self.binding_toml)?;
        if self.schema != 1
            || self.base_identity != binding.base_capture()
            || self.local_identity != binding.local().observation_id()?
            || self.provenance_identity != binding.provenance_id()?
            || self.result_identity != binding.result_id()?
            || self.inventory_identity != binding.local().inventory_id()?
        {
            return Err(BindingError("result provenance mismatch"));
        }
        Ok(binding)
    }
}

#[cfg(test)]
mod m_10_b_1_cases {
    use super::*;
    const BINDING: &str = include_str!("../../../../formats/corpora/derivatives/e1/binding.json");
    const RESULT: &str = include_str!("../../../../formats/corpora/derivatives/e1/result.json");
    #[test]
    fn null_selectors_are_absent_and_exactly_one_effective_selector_is_required() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../schemas/derivative_binding.jtd.json"
        ))
        .unwrap();
        for selector in ["file", "directory"] {
            let field = &schema["definitions"]["descriptor"]["optionalProperties"][selector];
            assert_eq!(field["type"], "string");
            assert_eq!(field["nullable"], true);
            assert_eq!(field["metadata"]["x-default"], serde_json::Value::Null);
        }
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../formats/corpora/derivatives/e1/selector-cases.json"
        ))
        .unwrap();
        let original: WireBinding = serde_json::from_str(BINDING).unwrap();
        let expected = original.checked_binding().unwrap();
        for case in cases.as_array().unwrap() {
            // Exercise both uses of the shared descriptor definition independently.
            for source in ["base", "local"] {
                let mut value: serde_json::Value = serde_json::from_str(BINDING).unwrap();
                let descriptor = value[source]["descriptor"].as_object_mut().unwrap();
                descriptor.remove("file");
                descriptor.remove("directory");
                descriptor.extend(case["selectors"].as_object().unwrap().clone());
                let wire: WireBinding = serde_json::from_value(value).unwrap();
                let checked = wire.checked_binding();
                assert_eq!(
                    checked.is_ok(),
                    case["valid"].as_bool().unwrap(),
                    "{}: {source}",
                    case["name"]
                );
                if case["valid"] == true {
                    assert_eq!(checked.unwrap(), expected);
                    assert_eq!(
                        serde_json::to_value(&wire).unwrap(),
                        serde_json::to_value(&original).unwrap()
                    );
                }
            }
        }
    }
    #[test]
    fn authored_and_generated_corpus_round_trip() {
        let wire: WireBinding = serde_json::from_str(BINDING).unwrap();
        let checked = wire.checked_binding().unwrap();
        let result: DerivativeResult = serde_json::from_str(RESULT).unwrap();
        assert_eq!(result.checked_binding().unwrap(), checked);
        assert_eq!(
            serde_json::to_value(&wire).unwrap(),
            serde_json::from_str::<serde_json::Value>(BINDING).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::from_str::<serde_json::Value>(RESULT).unwrap()
        );
    }
    #[test]
    fn strict_carriers_refuse_unknown_fields_and_proof_claims() {
        let mut value: serde_json::Value = serde_json::from_str(BINDING).unwrap();
        value["local"]["cache"] = "output".into();
        assert!(serde_json::from_value::<WireBinding>(value).is_err());
        let mut value: serde_json::Value = serde_json::from_str(RESULT).unwrap();
        value["strength"] = "source-proven".into();
        assert!(serde_json::from_value::<DerivativeResult>(value).is_err());
    }
    #[test]
    fn ambiguous_descriptor_missing_source_and_stale_capture_refuse() {
        let value: serde_json::Value = serde_json::from_str(BINDING).unwrap();
        for variant in 0..4 {
            let mut bad = value.clone();
            match variant {
                0 => bad["base"]["descriptor"]["directory"] = "payload".into(),
                1 => bad["local"]["files"] = serde_json::json!([]),
                2 => bad["base_capture"] = "absent".into(),
                _ => bad["local"]["owner"] = bad["base"]["owner"].clone(),
            }
            let wire: WireBinding = serde_json::from_value(bad).unwrap();
            assert!(wire.checked_binding().is_err());
        }
    }
    #[test]
    fn result_hash_cannot_be_replaced_with_equal_byte_identity() {
        let mut result: DerivativeResult = serde_json::from_str(RESULT).unwrap();
        assert_ne!(result.base_identity, result.local_identity);
        result.result_identity = result.inventory_identity.clone();
        assert!(result.checked_binding().is_err());
    }
}
