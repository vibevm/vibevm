//! Inert package requests. None of these values is execution authority.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientRequest {
    pub schema: u32,
    pub client_id: String,
    pub version: String,
    pub bindings: Vec<ClientBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientBinding {
    pub scope: ClientScope,
    pub shape: ClientShape,
    pub destination: String,
    pub key: Vec<String>,
    pub operations: Vec<ClientOperation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClientScope {
    Project,
    User,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClientShape {
    File,
    Tree,
    JsonMember,
    TomlMember,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClientOperation {
    ProjectFile,
    ProjectTree,
    ReconcileJsonMember,
    ReconcileTomlMember,
}

impl ClientRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err("unsupported client request epoch".into());
        }
        let Some((group, name)) = self.client_id.split_once('/') else {
            return Err("qualified client identity required".into());
        };
        group
            .parse::<crate::Group>()
            .map_err(|_| "invalid client group")?;
        name.parse::<crate::PackageName>()
            .map_err(|_| "invalid client name")?;
        if self.client_id.len() > 256
            || self.version.len() > 128
            || self.version.parse::<semver::VersionReq>().is_err()
        {
            return Err("invalid client identity/version applicability".into());
        }
        if self.bindings.is_empty() || self.bindings.len() > 64 {
            return Err("client binding count".into());
        }
        let mut seen = BTreeSet::new();
        for b in &self.bindings {
            if !relative(&b.destination)
                || b.key.len() > 32
                || b.key
                    .iter()
                    .any(|s| s.is_empty() || s.len() > 256 || s.chars().any(char::is_control))
            {
                return Err("invalid relative destination or structured key".into());
            }
            let expected = match b.shape {
                ClientShape::File => ClientOperation::ProjectFile,
                ClientShape::Tree => ClientOperation::ProjectTree,
                ClientShape::JsonMember => ClientOperation::ReconcileJsonMember,
                ClientShape::TomlMember => ClientOperation::ReconcileTomlMember,
            };
            let member = matches!(b.shape, ClientShape::JsonMember | ClientShape::TomlMember);
            if member == b.key.is_empty() || b.operations != [expected] {
                return Err("scope/shape/operation matrix mismatch".into());
            }
            if !seen.insert((b.scope, &b.destination, &b.key)) {
                return Err("duplicate client binding".into());
            }
        }
        Ok(())
    }
}
fn relative(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 1024
        && !s.contains(['\\', ':'])
        && !s.chars().any(char::is_control)
        && s.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

#[cfg(test)]
mod m_08_c_1_cases {
    use super::*;
    fn request() -> ClientRequest {
        ClientRequest {
            schema: 1,
            client_id: "org.example/new-client".into(),
            version: ">=1.0.0".into(),
            bindings: vec![ClientBinding {
                scope: ClientScope::Project,
                shape: ClientShape::JsonMember,
                destination: ".new/settings.json".into(),
                key: vec!["mcpServers".into(), "new.key".into()],
                operations: vec![ClientOperation::ReconcileJsonMember],
            }],
        }
    }
    #[test]
    fn strict_descriptor_identity_fields_and_absent_compatibility() {
        let r = request();
        r.validate().unwrap();
        let text = toml::to_string(&r).unwrap();
        assert_eq!(toml::from_str::<ClientRequest>(&text).unwrap(), r);
        assert!(toml::from_str::<ClientRequest>(&(text + "\nunknown = true\n")).is_err());
    }
    #[test]
    fn unknown_epoch_operation_and_duplicate_client_refuse() {
        let mut r = request();
        r.schema = 2;
        assert!(r.validate().is_err());
        let text = toml::to_string(&request())
            .unwrap()
            .replace("reconcile-json-member", "launch");
        assert!(toml::from_str::<ClientRequest>(&text).is_err());
        r = request();
        r.bindings.push(r.bindings[0].clone());
        assert!(r.validate().is_err());
        for path in ["../escape", "/absolute", "a//b", "a\\b", "C:relative"] {
            let mut r = request();
            r.bindings[0].destination = path.into();
            assert!(r.validate().is_err());
        }
    }
}
