//! Owner-declared client URI tokens over opaque resource bytes. No path open,
//! URI authority or source-byte authentication follows from this table.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION");

use std::collections::{BTreeMap, BTreeSet};

use vibe_core::machine_view::{OpaqueResourceId, OpaqueText};

/// An owner supplies a token for one exact opaque resource and revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUriDeclaration {
    pub resource: OpaqueResourceId,
    pub captured_revision: OpaqueText,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceUriMapError {
    InvalidToken(String),
    DuplicateResourceRevision,
    DuplicateToken(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUriLookupError {
    WrongScope,
    WrongRevision,
    AbsentResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUriStatus {
    OwnerDeclaredUnverifiedSourceBytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientResourceUri {
    uri: String,
    resource: OpaqueResourceId,
    captured_revision: OpaqueText,
    authorized_scope: OpaqueText,
    status: ResourceUriStatus,
}
impl ClientResourceUri {
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn resource(&self) -> &OpaqueResourceId {
        &self.resource
    }
    pub fn captured_revision(&self) -> &OpaqueText {
        &self.captured_revision
    }
    pub fn authorized_scope(&self) -> &OpaqueText {
        &self.authorized_scope
    }
    pub fn status(&self) -> ResourceUriStatus {
        self.status
    }
}

/// A fixed-scope in-memory declaration map. Raw bytes never enter the URI;
/// exact source authentication and token resolution belong to a later owner.
#[derive(Debug, Clone)]
pub struct OwnerResourceUriMap {
    authorized_scope: OpaqueText,
    tokens: BTreeMap<(Vec<u8>, String), String>,
}

fn safe_token(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_')
}

impl OwnerResourceUriMap {
    /// The owner supplies every token; this constructor only checks spelling
    /// and duplicate logical bindings, never physical source or scope grants.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
        r = 1
    )]
    pub fn new(
        authorized_scope: OpaqueText,
        declarations: Vec<ResourceUriDeclaration>,
    ) -> Result<Self, ResourceUriMapError> {
        let mut tokens = BTreeMap::new();
        let mut used = BTreeSet::new();
        for declaration in declarations {
            if !safe_token(&declaration.token) {
                return Err(ResourceUriMapError::InvalidToken(declaration.token));
            }
            if !used.insert(declaration.token.clone()) {
                return Err(ResourceUriMapError::DuplicateToken(declaration.token));
            }
            let key = (
                declaration.resource.bytes().to_vec(),
                declaration.captured_revision.as_str().to_owned(),
            );
            if tokens.insert(key, declaration.token).is_some() {
                return Err(ResourceUriMapError::DuplicateResourceRevision);
            }
        }
        Ok(Self {
            authorized_scope,
            tokens,
        })
    }

    /// Return only a declared client navigation URI for this exact scope,
    /// resource octet sequence and captured revision. No read is attempted.
    pub fn lookup(
        &self,
        authorized_scope: &OpaqueText,
        resource: &OpaqueResourceId,
        captured_revision: &OpaqueText,
    ) -> Result<ClientResourceUri, ResourceUriLookupError> {
        if authorized_scope != &self.authorized_scope {
            return Err(ResourceUriLookupError::WrongScope);
        }
        let Some(token) = self.tokens.get(&(
            resource.bytes().to_vec(),
            captured_revision.as_str().to_owned(),
        )) else {
            if self
                .tokens
                .keys()
                .any(|(bytes, _)| bytes == resource.bytes())
            {
                return Err(ResourceUriLookupError::WrongRevision);
            }
            return Err(ResourceUriLookupError::AbsentResource);
        };
        Ok(ClientResourceUri {
            uri: format!("vibevm-resource:{token}"),
            resource: resource.clone(),
            captured_revision: captured_revision.clone(),
            authorized_scope: authorized_scope.clone(),
            status: ResourceUriStatus::OwnerDeclaredUnverifiedSourceBytes,
        })
    }
}

#[cfg(test)]
#[path = "resource_uri/tests.rs"]
mod tests;
