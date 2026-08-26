//! Strict serde intermediates for the authored `[[mechanism]]` table and
//! `[mechanisms]` route map.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::PathBuf;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{MechanismDecl, MechanismFreshness, MechanismKey, MechanismRole, ProviderPin};
use crate::manifest::extension::ExtensionHandlerWire;

const ONE_MACHINE: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MechanismDeclWire {
    id: String,
    role: MechanismRoleWire,
    name: String,
    handler: ExtensionHandlerWire,
    protocol: u32,
    config_schema: PathBuf,
    freshness: MechanismFreshnessWire,
}

/// The `[mechanisms]` route table — logical key to exact provider identity,
/// kept in authored order. A free-form key map, so `deny_unknown_fields` has
/// nothing to deny here: every key is data, and both halves are validated by
/// the domain conversion below.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct MechanismRoutesWire(IndexMap<String, String>);

impl MechanismRoutesWire {
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MechanismRoleWire {
    Build,
    Package,
    Deploy,
    Acquire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MechanismFreshnessWire {
    Engine,
    Provider,
}

impl TryFrom<MechanismDeclWire> for MechanismDecl {
    type Error = String;

    fn try_from(wire: MechanismDeclWire) -> Result<Self, Self::Error> {
        Ok(Self {
            id: wire.id,
            role: wire.role.into(),
            name: wire.name,
            handler: wire.handler.into(),
            protocol: wire.protocol,
            config_schema: wire.config_schema,
            freshness: wire.freshness.into(),
        })
    }
}

impl TryFrom<MechanismDecl> for MechanismDeclWire {
    type Error = String;

    fn try_from(declaration: MechanismDecl) -> Result<Self, Self::Error> {
        declaration.validate()?;
        Ok(Self {
            id: declaration.id,
            role: declaration.role.into(),
            name: declaration.name,
            handler: declaration.handler.into(),
            protocol: declaration.protocol,
            config_schema: declaration.config_schema,
            freshness: declaration.freshness.into(),
        })
    }
}

impl TryFrom<MechanismRoutesWire> for super::MechanismRoutes {
    type Error = String;

    fn try_from(wire: MechanismRoutesWire) -> Result<Self, Self::Error> {
        let mut routes = Self::default();
        for (key, value) in wire.0 {
            let key = key
                .parse::<MechanismKey>()
                .map_err(|error| format!("[mechanisms] route key `{key}` is invalid: {error}"))?;
            let pin = parse_route_value(&key, &value)?;
            routes.insert(key, pin);
        }
        Ok(routes)
    }
}

impl TryFrom<super::MechanismRoutes> for MechanismRoutesWire {
    type Error = String;

    fn try_from(routes: super::MechanismRoutes) -> Result<Self, Self::Error> {
        Ok(Self(
            routes
                .iter()
                .map(|(key, pin)| (key.to_string(), pin.to_string()))
                .collect(),
        ))
    }
}

fn parse_route_value(key: &MechanismKey, value: &str) -> Result<ProviderPin, String> {
    ProviderPin::parse(value).map_err(|error| {
        format!(
            "[mechanisms] route `{key}` value is invalid: {error}; an exact route pins one installed provider, never a short id ({ONE_MACHINE})"
        )
    })
}

impl From<MechanismRoleWire> for MechanismRole {
    fn from(wire: MechanismRoleWire) -> Self {
        match wire {
            MechanismRoleWire::Build => Self::Build,
            MechanismRoleWire::Package => Self::Package,
            MechanismRoleWire::Deploy => Self::Deploy,
            MechanismRoleWire::Acquire => Self::Acquire,
        }
    }
}

impl From<MechanismRole> for MechanismRoleWire {
    fn from(role: MechanismRole) -> Self {
        match role {
            MechanismRole::Build => Self::Build,
            MechanismRole::Package => Self::Package,
            MechanismRole::Deploy => Self::Deploy,
            MechanismRole::Acquire => Self::Acquire,
        }
    }
}

impl From<MechanismFreshnessWire> for MechanismFreshness {
    fn from(wire: MechanismFreshnessWire) -> Self {
        match wire {
            MechanismFreshnessWire::Engine => Self::Engine,
            MechanismFreshnessWire::Provider => Self::Provider,
        }
    }
}

impl From<MechanismFreshness> for MechanismFreshnessWire {
    fn from(freshness: MechanismFreshness) -> Self {
        match freshness {
            MechanismFreshness::Engine => Self::Engine,
            MechanismFreshness::Provider => Self::Provider,
        }
    }
}
