//! Explicit domain/generated conversion for the deploy lock sidecar.

use vibe_wire::generated::deploy_lock_resources as generated;

use super::{LockBinding, LockResources};
use crate::mechanism::deploy::protocol::NativeProviderBinding;

pub(in crate::mechanism::deploy) fn encode(value: &LockResources) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(&to_wire(value)).map_err(|error| error.to_string())
}

pub(in crate::mechanism::deploy) fn decode(bytes: &[u8]) -> Result<LockResources, String> {
    serde_json::from_slice(bytes)
        .map(from_wire)
        .map_err(|error| error.to_string())
}

fn to_wire(value: &LockResources) -> generated::DeployLockResources {
    generated::DeployLockResources {
        schema: value.schema,
        committed: value.committed.as_ref().map(binding_to_wire),
        pending: value.pending.as_ref().map(binding_to_wire),
    }
}

fn from_wire(value: generated::DeployLockResources) -> LockResources {
    LockResources {
        schema: value.schema,
        committed: value.committed.map(binding_from_wire),
        pending: value.pending.map(binding_from_wire),
    }
}

fn binding_to_wire(value: &LockBinding) -> generated::LockBinding {
    generated::LockBinding {
        generation: value.generation,
        plan_hash: value.plan_hash.clone(),
        resources: value.resources.clone(),
        native: value.native.as_ref().map(native_to_wire),
    }
}

fn binding_from_wire(value: generated::LockBinding) -> LockBinding {
    LockBinding {
        generation: value.generation,
        plan_hash: value.plan_hash,
        resources: value.resources,
        native: value.native.map(native_from_wire),
    }
}

fn native_to_wire(value: &NativeProviderBinding) -> generated::NativeProviderBinding {
    generated::NativeProviderBinding {
        target: value.target.clone(),
        mechanism: value.mechanism.clone(),
        pin: value.pin.clone(),
        descriptor_id: value.descriptor_id.clone(),
        protocol: value.protocol,
        provider_version: value.provider_version.clone(),
        provider_hash: value.provider_hash.clone(),
        provider_root: value.provider_root.clone(),
        platform: value.platform.clone(),
        record_root: value.record_root.clone(),
        origin: value.origin.clone(),
        record_id: value.record_id.clone(),
        record_path: value.record_path.clone(),
        image: value.image.clone(),
        image_digest: value.image_digest.clone(),
        image_bytes: value.image_bytes,
    }
}

fn native_from_wire(value: generated::NativeProviderBinding) -> NativeProviderBinding {
    NativeProviderBinding {
        target: value.target,
        mechanism: value.mechanism,
        pin: value.pin,
        descriptor_id: value.descriptor_id,
        protocol: value.protocol,
        provider_version: value.provider_version,
        provider_hash: value.provider_hash,
        provider_root: value.provider_root,
        platform: value.platform,
        record_root: value.record_root,
        origin: value.origin,
        record_id: value.record_id,
        record_path: value.record_path,
        image: value.image,
        image_digest: value.image_digest,
        image_bytes: value.image_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native() -> NativeProviderBinding {
        NativeProviderBinding {
            target: "tool".to_owned(),
            mechanism: "deploy:vibe-bin".to_owned(),
            pin: "org.example/providers#native".to_owned(),
            descriptor_id: "native".to_owned(),
            protocol: 1,
            provider_version: "1.2.3".to_owned(),
            provider_hash: Some(format!("sha256:{}", "c".repeat(64))),
            provider_root: "slots/provider".to_owned(),
            platform: "windows-x86_64".to_owned(),
            record_root: "slot".to_owned(),
            origin: "source-record".to_owned(),
            record_id: Some("d".repeat(64)),
            record_path: Some(format!(".vibe/state/artifacts/{}.json", "d".repeat(64))),
            image: ".vibe/native/images/provider.dll".to_owned(),
            image_digest: "e".repeat(64),
            image_bytes: 4_294_967_297,
        }
    }

    #[test]
    fn full_domain_binding_round_trips_and_keeps_interleaved_wire_order() {
        let value = LockResources {
            schema: 1,
            committed: None,
            pending: Some(LockBinding {
                generation: 3,
                plan_hash: "b".repeat(64),
                resources: vec!["config/tool.json".to_owned()],
                native: Some(native()),
            }),
        };
        value.validate().expect("the domain fixture is valid");
        let encoded = encode(&value).expect("generated writer accepts the conversion");
        let decoded = decode(&encoded).expect("generated reader accepts its own bytes");
        decoded
            .validate()
            .expect("the decoded domain value is valid");
        assert_eq!(decoded, value);

        let text = String::from_utf8(encoded).unwrap();
        for pair in [
            ("\"provider_version\"", "\"provider_hash\""),
            ("\"provider_hash\"", "\"provider_root\""),
            ("\"origin\"", "\"record_id\""),
            ("\"record_path\"", "\"image\""),
            ("\"image_digest\"", "\"image_bytes\""),
        ] {
            assert!(
                text.find(pair.0).unwrap() < text.find(pair.1).unwrap(),
                "the existing domain declaration order is durable: {text}"
            );
        }
    }

    #[test]
    fn strict_generated_reader_refuses_unknown_nested_state() {
        let raw = br#"{"schema":1,"pending":{"generation":3,"plan_hash":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","resources":[],"repair":true}}"#;
        let error = decode(raw).expect_err("unknown state is not additive");
        assert!(error.contains("unknown field"), "{error}");
        assert!(error.contains("repair"), "{error}");
    }

    #[test]
    fn absent_native_facts_keep_required_null_bytes_and_missing_keys_refuse() {
        let mut binding = native();
        binding.provider_hash = None;
        binding.record_id = None;
        binding.record_path = None;
        binding.origin = "prebuilt".to_owned();
        let value = LockResources {
            schema: 1,
            committed: None,
            pending: Some(LockBinding {
                generation: 3,
                plan_hash: "b".repeat(64),
                resources: vec!["config/tool.json".to_owned()],
                native: Some(binding),
            }),
        };
        let bytes = encode(&value).unwrap();
        assert_eq!(
            bytes,
            format!(
                "{{\n  \"schema\": 1,\n  \"pending\": {{\n    \"generation\": 3,\n    \"plan_hash\": \"{}\",\n    \"resources\": [\n      \"config/tool.json\"\n    ],\n    \"native\": {{\n      \"target\": \"tool\",\n      \"mechanism\": \"deploy:vibe-bin\",\n      \"pin\": \"org.example/providers#native\",\n      \"descriptor_id\": \"native\",\n      \"protocol\": 1,\n      \"provider_version\": \"1.2.3\",\n      \"provider_hash\": null,\n      \"provider_root\": \"slots/provider\",\n      \"platform\": \"windows-x86_64\",\n      \"record_root\": \"slot\",\n      \"origin\": \"prebuilt\",\n      \"record_id\": null,\n      \"record_path\": null,\n      \"image\": \".vibe/native/images/provider.dll\",\n      \"image_digest\": \"{}\",\n      \"image_bytes\": 4294967297\n    }}\n  }}\n}}",
                "b".repeat(64),
                "e".repeat(64)
            )
            .as_bytes(),
            "the pre-schema domain writer emitted explicit null at these positions"
        );
        assert_eq!(decode(&bytes).unwrap(), value);

        for member in ["provider_hash", "record_id", "record_path"] {
            let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            json["pending"]["native"]
                .as_object_mut()
                .unwrap()
                .remove(member);
            let error = decode(&serde_json::to_vec(&json).unwrap())
                .expect_err("required-nullable members may be null but never absent");
            assert!(error.contains("missing field"), "{member}: {error}");
            assert!(error.contains(member), "{member}: {error}");
        }
    }
}
