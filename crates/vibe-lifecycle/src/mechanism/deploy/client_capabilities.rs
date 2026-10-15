//! Explicit legacy3 executable adapter; descriptors never supply executables.
use super::model::{ClientExecutable, ClientExecutables};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientExecutableMap(BTreeMap<String, ClientExecutable>);
impl ClientExecutableMap {
    pub fn from_legacy3(old: &ClientExecutables) -> Self {
        Self(BTreeMap::from([
            ("com.anthropic/claude".into(), old.claude.clone()),
            ("com.openai/codex".into(), old.codex.clone()),
            ("ai.opencode/opencode".into(), old.opencode.clone()),
        ]))
    }
    pub fn legacy3(&self) -> Result<ClientExecutables, String> {
        if self.0.len() != 3 {
            return Err("map is not exactly legacy3".into());
        }
        let get = |key| {
            self.0
                .get(key)
                .cloned()
                .ok_or_else(|| format!("missing legacy binding {key}"))
        };
        Ok(ClientExecutables {
            claude: get("com.anthropic/claude")?,
            codex: get("com.openai/codex")?,
            opencode: get("ai.opencode/opencode")?,
        })
    }
    /// Trusted surface observations only. No PATH lookup or process permission.
    pub fn observed(rows: BTreeMap<String, ClientExecutable>) -> Result<Self, String> {
        for (id, executable) in &rows {
            let (group, name) = id
                .split_once('/')
                .ok_or("qualified client identity required")?;
            group
                .parse::<vibe_core::Group>()
                .map_err(|e| e.to_string())?;
            name.parse::<vibe_core::PackageName>()
                .map_err(|e| e.to_string())?;
            if executable
                .resolved_path()
                .is_some_and(|path| !path.is_absolute())
            {
                return Err("absolute observed executable required".into());
            }
        }
        Ok(Self(rows))
    }
    pub fn native_legacy3(
        &self,
    ) -> Result<vibe_wire::generated::native::e1::deploy_request::DeployClients, String> {
        use vibe_wire::generated::native::e1::deploy_request as n;
        let old = self.legacy3()?;
        let convert = |value: ClientExecutable| match value {
            ClientExecutable::Missing { .. } => {
                n::DeployClient::Missing(Box::new(n::DeployClientMissing {}))
            }
            ClientExecutable::Resolved { command, path } => {
                n::DeployClient::Resolved(Box::new(n::DeployClientResolved {
                    command,
                    path: path.to_string_lossy().replace('\\', "/"),
                }))
            }
        };
        Ok(n::DeployClients {
            claude: convert(old.claude),
            codex: convert(old.codex),
            opencode: convert(old.opencode),
        })
    }
    pub fn rows(&self) -> &BTreeMap<String, ClientExecutable> {
        &self.0
    }
}
#[cfg(test)]
mod m_08_c_1_cases {
    use super::*;
    #[test]
    fn legacy_three_profiles_and_executable_states_are_explicit() {
        let old = ClientExecutables {
            claude: ClientExecutable::Resolved {
                command: "claude".into(),
                path: "/fixture/claude".into(),
            },
            codex: ClientExecutable::Missing {
                command: "codex".into(),
            },
            opencode: ClientExecutable::Missing {
                command: "opencode".into(),
            },
        };
        let map = ClientExecutableMap::from_legacy3(&old);
        assert_eq!(map.legacy3().unwrap(), old);
        let bytes = serde_json::to_string(&map.native_legacy3().unwrap()).unwrap();
        assert_eq!(
            bytes,
            r#"{"claude":{"state":"resolved","command":"claude","path":"/fixture/claude"},"codex":{"state":"missing"},"opencode":{"state":"missing"}}"#
        );
        let wire: vibe_wire::generated::native::e1::deploy_request::DeployClients =
            serde_json::from_str(&bytes).unwrap();
        assert_eq!(serde_json::to_string(&wire).unwrap(), bytes);
        let mut rows = map.rows().clone();
        rows.insert(
            "org.example/new-client".into(),
            ClientExecutable::Missing {
                command: "new".into(),
            },
        );
        let map = ClientExecutableMap::observed(rows).unwrap();
        assert!(map.legacy3().is_err());
        assert!(
            map.rows()["org.example/new-client"]
                .resolved_path()
                .is_none()
        );
    }
}
