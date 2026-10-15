//! Canonical open client map. Availability and authority are not inferred.
use std::collections::BTreeMap;
use vibe_core::manifest::ClientRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCapability {
    pub provider: String,
    pub descriptor_digest: String,
    pub request: ClientRequest,
    pub disabled: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientCapabilities(BTreeMap<String, ClientCapability>);
impl ClientCapabilities {
    pub fn admit(rows: impl IntoIterator<Item = ClientCapability>) -> Result<Self, String> {
        let mut map = BTreeMap::new();
        for row in rows {
            row.request.validate()?;
            if map.len() >= 4096 {
                return Err("client map count bound".into());
            }
            if map.insert(row.request.client_id.clone(), row).is_some() {
                return Err("duplicate client binding".into());
            }
        }
        Ok(Self(map))
    }
    pub fn rows(&self) -> &BTreeMap<String, ClientCapability> {
        &self.0
    }
    /// Existing profile readers must explicitly refuse an unrepresentable ID.
    pub fn legacy_profiles(&self) -> Result<Vec<crate::agents::Agent>, String> {
        self.0
            .keys()
            .map(|id| match id.as_str() {
                "com.anthropic/claude" => Ok(crate::agents::Agent::ClaudeCode),
                "com.anthropic/claude-desktop" => Ok(crate::agents::Agent::ClaudeCodeDesktop),
                "com.cursor/cursor" => Ok(crate::agents::Agent::Cursor),
                "ai.qwen/qwen-code" => Ok(crate::agents::Agent::QwenCode),
                "com.openai/codex" => Ok(crate::agents::Agent::Codex),
                "ai.opencode/opencode" => Ok(crate::agents::Agent::OpenCode),
                _ => Err(format!(
                    "client {id} is not representable by legacy profiles"
                )),
            })
            .collect()
    }
}
#[cfg(test)]
mod m_08_c_1_cases {
    use super::*;
    use vibe_core::manifest::*;
    #[test]
    fn new_id_destination_and_key_are_map_data() {
        let r = ClientRequest {
            schema: 1,
            client_id: "org.example/new-client".into(),
            version: "*".into(),
            bindings: vec![ClientBinding {
                scope: ClientScope::User,
                shape: ClientShape::JsonMember,
                destination: ".new/settings.json".into(),
                key: vec!["new.key".into()],
                operations: vec![ClientOperation::ReconcileJsonMember],
            }],
        };
        let row = ClientCapability {
            provider: "org.example/package#client".into(),
            descriptor_digest: "a".repeat(64),
            request: r,
            disabled: false,
        };
        let map = ClientCapabilities::admit([row.clone()]).unwrap();
        assert_eq!(
            map.rows()["org.example/new-client"].request.bindings[0].key,
            ["new.key"]
        );
        assert!(map.legacy_profiles().is_err());
        for (id, expected) in [
            ("com.anthropic/claude", crate::agents::Agent::ClaudeCode),
            ("com.openai/codex", crate::agents::Agent::Codex),
            ("ai.opencode/opencode", crate::agents::Agent::OpenCode),
            (
                "com.anthropic/claude-desktop",
                crate::agents::Agent::ClaudeCodeDesktop,
            ),
            ("com.cursor/cursor", crate::agents::Agent::Cursor),
            ("ai.qwen/qwen-code", crate::agents::Agent::QwenCode),
        ] {
            let mut legacy = row.clone();
            legacy.request.client_id = id.into();
            assert_eq!(
                ClientCapabilities::admit([legacy])
                    .unwrap()
                    .legacy_profiles()
                    .unwrap(),
                vec![expected]
            );
        }
        assert!(ClientCapabilities::admit([row.clone(), row]).is_err());
    }
}
