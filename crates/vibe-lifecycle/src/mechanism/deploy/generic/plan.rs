use super::super::state::{DeployState, DeploymentHome};
use super::*;
use vibe_core::manifest::{ClientOperation, ClientShape};
use vibe_safefs::StableFileSnapshot;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

const CAP: usize = 4 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GenericResource {
    pub scope: ClientScope,
    pub destination: String,
    /// Literal key segments, never a filesystem path.
    pub key: Vec<String>,
    pub operation: ClientOperation,
    pub desired_digest: String,
}
impl GenericResource {
    pub(crate) fn owned(&self, host: &GenericHost) -> String {
        // JSON framing distinguishes literal slashes/dots and segment boundaries.
        format!(
            "generic:{}",
            serde_json::to_string(&(
                host.root(self.scope)
                    .root_path()
                    .components()
                    .collect::<std::path::PathBuf>()
                    .to_string_lossy(),
                &self.destination,
                &self.key
            ))
            .expect("string tuple")
        )
    }
    // Incumbent resource spelling also used by legacy receipt ownership.
    pub(crate) fn lock(&self, host: &GenericHost) -> String {
        match self.scope {
            ClientScope::User => format!("home:{}", self.destination),
            ClientScope::Project => host
                .project
                .root_path()
                .join(&self.destination)
                .to_string_lossy()
                .into_owned(),
        }
    }
    pub(crate) fn locks(&self, host: &GenericHost) -> Vec<String> {
        // Scope and root boundaries are host composition choices, not physical
        // document identities. Project accepts component-normalized absolute
        // roots (no parent traversal or links); collect components so accepted
        // `.`/separator spellings share the same absolute destination key.
        // DeployState owns Unicode/case normalization and ordered acquisition.
        let absolute = host
            .root(self.scope)
            .root_path()
            .join(&self.destination)
            .components()
            .collect::<std::path::PathBuf>()
            .to_string_lossy()
            .into_owned();
        // Retain the incumbent spelling too: User home:relative, or the
        // original Project absolute spelling. The existing owner deduplicates
        // and acquires both in its total order, never replacing their lease.
        vec![absolute, self.lock(host)]
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Document {
    pub scope: ClientScope,
    pub destination: String,
    pub before: Option<StableFileSnapshot>,
    pub after: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct GenericPlan {
    pub(super) admitted: AdmittedClient,
    pub(super) control: ControlObservation,
    pub(super) roots: [String; 3],
    pub(super) protected: ProtectedControls,
    pub(super) resources: Vec<GenericResource>,
    pub(super) documents: Vec<Document>,
    pub(super) receipts: Vec<(String, DeployReceipt)>,
    pub(super) target: String,
    pub(super) state_identity: Option<String>,
}
impl GenericPlan {
    pub fn resources(&self) -> &[GenericResource] {
        &self.resources
    }
    pub fn payloads(&self) -> &[GenericPayload] {
        &self.admitted.payloads
    }
    /// Full resulting document content, including active MCP launch settings.
    pub fn documents(&self) -> impl Iterator<Item = (ClientScope, &str, &[u8])> {
        self.documents
            .iter()
            .map(|d| (d.scope, d.destination.as_str(), d.after.as_slice()))
    }
    /// Ephemeral typed host plan identity, not a persistent grant format.
    pub fn identity(&self) -> String {
        digest(format!("generic-host-plan/1:{self:?}").as_bytes())
    }
    pub(super) fn locks(&self, host: &GenericHost) -> Vec<String> {
        self.resources
            .iter()
            .flat_map(|r| r.locks(host))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub(super) fn deploy_plan(&self, host: &GenericHost) -> super::super::protocol::DeployPlan {
        use super::super::protocol::{DeployPlan, PlannedDeployResource};
        DeployPlan {
            resources: self
                .resources
                .iter()
                .map(|r| PlannedDeployResource {
                    resource: r.owned(host),
                    desired_digest: r.desired_digest.clone(),
                })
                .collect(),
            lock_resources: self.locks(host),
            config_digest: self.identity(),
            reversible: false,
            summary: "explicitly admitted generic file/config projection; no process execution"
                .into(),
        }
    }
}

pub(super) fn validate_payloads(
    request: &ClientRequest,
    payloads: &[GenericPayload],
) -> Result<(), GenericError> {
    if request.bindings.len() != payloads.len() {
        return Err(refuse(
            "one separately admitted payload per binding required",
        ));
    }
    let mut total = 0usize;
    for (binding, payload) in request.bindings.iter().zip(payloads) {
        match (binding.shape, payload) {
            (ClientShape::File, GenericPayload::File(bytes)) => {
                total = total.saturating_add(bytes.len())
            }
            (ClientShape::Tree, GenericPayload::Tree(files))
                if !files.is_empty() && files.len() <= 4096 =>
            {
                for (path, bytes) in files {
                    vibe_safefs::split_relative(path).map_err(refuse)?;
                    total = total.saturating_add(bytes.len());
                }
            }
            (ClientShape::JsonMember, GenericPayload::JsonMember(v)) => {
                total = total.saturating_add(encoded(v)?.len())
            }
            (ClientShape::TomlMember, GenericPayload::TomlMember(v)) => {
                total = total.saturating_add(encoded(v)?.len())
            }
            _ => return Err(refuse("payload/closed-operation shape mismatch")),
        }
    }
    if total > CAP {
        return Err(refuse("admitted payload exceeds bounded host budget"));
    }
    Ok(())
}

impl GenericHost {
    pub(super) fn home(&self, target: &str) -> DeploymentHome {
        DeploymentHome::new(
            &self
                .settings
                .root_path()
                .join(super::super::model::DEPLOY_STATE_DIR),
            &self.roots[0],
            None,
            target,
        )
    }
    pub(super) fn receipts(&self) -> Result<Vec<(String, DeployReceipt)>, GenericError> {
        match DeployState::observe_bound(&self.settings).map_err(refuse)? {
            Some(state) => state.receipts_bound().map_err(refuse),
            None => Ok(vec![]),
        }
    }
    pub(super) fn state_identity(&self) -> Result<Option<String>, GenericError> {
        DeployState::observe_bound(&self.settings)
            .map_err(refuse)?
            .map(|s| s.generic_root_identity().map_err(refuse))
            .transpose()
    }
    pub fn plan(&self, admitted: &AdmittedClient) -> Result<GenericPlan, GenericError> {
        self.current(&admitted.capture)?;
        let request = &admitted.capture.observation.request;
        request.validate().map_err(refuse)?;
        validate_payloads(request, &admitted.payloads)?;
        let control = self.authority.current_control().map_err(refuse)?;
        if control.principal.is_empty() || control.policy_generation.is_empty() {
            return Err(refuse("current principal and policy generation required"));
        }
        let target = format!("generic-{}", digest(request.client_id.as_bytes()));
        let state_identity = self.state_identity()?;
        let receipts = self.receipts()?;
        let home = self.home(&target);
        let prior = receipts
            .iter()
            .find(|(id, _)| id == home.id())
            .map(|(_, r)| r);
        let mut resources = Vec::new();
        let mut values = Vec::new();
        // First judge authored binding footprints, including tree containers.
        for (i, a) in request.bindings.iter().enumerate() {
            vibe_safefs::split_relative(&a.destination).map_err(refuse)?;
            self.protect(a.scope, &a.destination)?;
            for b in &request.bindings[..i] {
                if a.scope != b.scope {
                    continue;
                }
                if overlap(&a.destination, &b.destination) {
                    let same = a.destination == b.destination;
                    if !same
                        || a.key.is_empty()
                        || b.key.is_empty()
                        || a.shape != b.shape
                        || key_overlap(&a.key, &b.key)
                    {
                        return Err(refuse(
                            "duplicate, alias, ancestor or whole-document/member conflict",
                        ));
                    }
                }
            }
        }
        for (b, p) in request.bindings.iter().zip(&admitted.payloads) {
            let rows: Vec<(String, GenericPayload)> = match p {
                GenericPayload::Tree(files) => files
                    .iter()
                    .map(|(path, bytes)| {
                        (
                            format!("{}/{path}", b.destination),
                            GenericPayload::File(bytes.clone()),
                        )
                    })
                    .collect(),
                _ => vec![(b.destination.clone(), p.clone())],
            };
            for (destination, value) in rows {
                self.protect(b.scope, &destination)?;
                let bytes = value_bytes(&value)?;
                resources.push(GenericResource {
                    scope: b.scope,
                    destination,
                    key: b.key.clone(),
                    operation: b.operations[0],
                    desired_digest: digest(&bytes),
                });
                values.push(value);
            }
        }
        for (i, a) in resources.iter().enumerate() {
            for b in &resources[..i] {
                if a.scope == b.scope
                    && overlap(&a.destination, &b.destination)
                    && (a.destination != b.destination
                        || a.key.is_empty()
                        || b.key.is_empty()
                        || key_overlap(&a.key, &b.key))
                {
                    return Err(refuse("expanded footprint aliases or overlaps"));
                }
            }
        }
        for scope in [ClientScope::Project, ClientScope::User] {
            let paths = resources
                .iter()
                .filter(|r| r.scope == scope)
                .map(|r| r.destination.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let protected = self
                .protected
                .rows()
                .filter(|r| r.scope == scope)
                .map(|r| r.relative.clone())
                .collect::<Vec<_>>();
            self.root(scope)
                .preflight_set_against(&paths, &protected)
                .map_err(refuse)?;
        }
        super::super::ownership::refuse_generic_foreign(&receipts, home.id(), self, &resources)
            .map_err(refuse)?;
        // C3 owns resource-set migration. Existing exact sets may reconcile.
        if let Some(prior) = prior {
            let old = prior
                .resources
                .iter()
                .map(|r| r.resource.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let new = resources
                .iter()
                .map(|r| r.owned(self))
                .collect::<std::collections::BTreeSet<_>>();
            if old != new {
                return Err(refuse("generic resource-set upgrades require C3 admission"));
            }
        }
        let mut documents: Vec<Document> = vec![];
        for (r, v) in resources.iter().zip(&values) {
            let index = if let Some(index) = documents
                .iter()
                .position(|d| d.scope == r.scope && d.destination == r.destination)
            {
                index
            } else {
                let before = self
                    .root(r.scope)
                    .read_file_snapshot_bounded(&r.destination, CAP)
                    .map_err(refuse)?;
                documents.push(Document {
                    scope: r.scope,
                    destination: r.destination.clone(),
                    after: before.as_ref().map_or_else(Vec::new, |b| b.bytes.clone()),
                    before,
                });
                documents.len() - 1
            };
            let doc = &mut documents[index];
            let found = observe_value(doc.before.as_ref().map(|b| b.bytes.as_slice()), r)?;
            let owned = prior.and_then(|receipt| {
                receipt
                    .resources
                    .iter()
                    .find(|o| o.resource == r.owned(self))
            });
            match (found, owned) {
                (None, None) => {}
                (Some(found), Some(owned)) if digest(&found) == owned.post_digest => {}
                _ => return Err(refuse("foreign occupant or stale owned value")),
            }
            doc.after = reconcile(&doc.after, r, v)?;
            if doc.after.len() > CAP {
                return Err(refuse("resulting document exceeds bounded host budget"));
            }
        }
        self.current(&admitted.capture)?;
        if self.authority.current_control().map_err(refuse)? != control {
            return Err(refuse("policy changed during plan"));
        }
        if self.state_identity()? != state_identity {
            return Err(refuse("state home changed during plan"));
        }
        Ok(GenericPlan {
            state_identity,
            admitted: admitted.clone(),
            control,
            roots: self.roots.clone(),
            protected: self.protected.clone(),
            resources,
            documents,
            receipts,
            target,
        })
    }
    fn protect(&self, scope: ClientScope, destination: &str) -> Result<(), GenericError> {
        for p in self.protected.rows().filter(|p| p.scope == scope) {
            if overlap(destination, &p.relative) {
                return Err(refuse("protected host control resource or ancestor"));
            }
        }
        Ok(())
    }
}
pub(super) fn overlap(a: &str, b: &str) -> bool {
    vibe_safefs::paths_overlap(std::path::Path::new(a), std::path::Path::new(b))
}
pub(super) fn key_overlap(a: &[String], b: &[String]) -> bool {
    a.starts_with(b) || b.starts_with(a)
}
pub(super) fn value_bytes(p: &GenericPayload) -> Result<Vec<u8>, GenericError> {
    match p {
        GenericPayload::File(b) => Ok(b.clone()),
        GenericPayload::JsonMember(v) => encoded(v),
        GenericPayload::TomlMember(v) => toml_value_bytes(v),
        GenericPayload::Tree(_) => Err(refuse("unexpanded tree")),
    }
}
fn toml_value_bytes(value: &toml::Value) -> Result<Vec<u8>, GenericError> {
    // TOML datetime is not a JSON object; preserve the typed distinction.
    fn finite(v: &toml::Value) -> bool {
        match v {
            toml::Value::Float(f) => f.is_finite(),
            toml::Value::Array(a) => a.iter().all(finite),
            toml::Value::Table(t) => t.values().all(finite),
            _ => true,
        }
    }
    if !finite(value) {
        return Err(refuse(
            "nonfinite TOML values are outside admitted closed payload semantics",
        ));
    }
    let table = toml::Value::Table(toml::map::Map::from_iter([(
        "value".to_owned(),
        value.clone(),
    )]));
    Ok(toml::to_string(&table).map_err(refuse)?.into_bytes())
}

// Strict dynamic JSON decoding uses serde's incumbent parser, rejecting
// duplicate members instead of losing a foreign value during reconciliation.
struct UniqueJson(serde_json::Value);
impl<'de> serde::Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueJson(n.into()))
                    .ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(serde_json::Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = a.next_element::<UniqueJson>()? {
                    values.push(v.0);
                }
                Ok(UniqueJson(values.into()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = a.next_entry::<String, UniqueJson>()? {
                    if values.insert(key, value.0).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON member"));
                    }
                }
                Ok(UniqueJson(values.into()))
            }
        }
        d.deserialize_any(Visitor)
    }
}

fn json_doc(bytes: &[u8]) -> Result<serde_json::Value, GenericError> {
    if bytes.is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_slice::<UniqueJson>(bytes)
            .map(|v| v.0)
            .map_err(refuse)
    }
}
fn toml_doc(bytes: &[u8]) -> Result<toml::Value, GenericError> {
    if bytes.is_empty() {
        Ok(toml::Value::Table(Default::default()))
    } else {
        toml::from_str(std::str::from_utf8(bytes).map_err(refuse)?).map_err(refuse)
    }
}
pub(super) fn observe_value(
    bytes: Option<&[u8]>,
    r: &GenericResource,
) -> Result<Option<Vec<u8>>, GenericError> {
    let Some(bytes) = bytes else { return Ok(None) };
    if r.key.is_empty() {
        return Ok(Some(bytes.to_vec()));
    }
    if bytes.is_empty() && r.operation == ClientOperation::ReconcileJsonMember {
        return Err(refuse("existing empty JSON document is invalid"));
    }
    match r.operation {
        ClientOperation::ReconcileJsonMember => {
            let doc = json_doc(bytes)?;
            let mut at = &doc;
            for key in &r.key {
                let map = at
                    .as_object()
                    .ok_or_else(|| refuse("scalar JSON ancestor"))?;
                let Some(next) = map.get(key) else {
                    return Ok(None);
                };
                at = next;
            }
            Ok(Some(encoded(at)?))
        }
        ClientOperation::ReconcileTomlMember => {
            let doc = toml_doc(bytes)?;
            let mut at = &doc;
            for key in &r.key {
                let map = at
                    .as_table()
                    .ok_or_else(|| refuse("scalar TOML ancestor"))?;
                let Some(next) = map.get(key) else {
                    return Ok(None);
                };
                at = next;
            }
            Ok(Some(toml_value_bytes(at)?))
        }
        _ => Err(refuse("member operation mismatch")),
    }
}
fn reconcile(
    bytes: &[u8],
    r: &GenericResource,
    p: &GenericPayload,
) -> Result<Vec<u8>, GenericError> {
    match p {
        GenericPayload::File(b) => Ok(b.clone()),
        GenericPayload::JsonMember(v) => {
            let mut doc = json_doc(bytes)?;
            let mut at = &mut doc;
            for key in &r.key[..r.key.len() - 1] {
                at = at
                    .as_object_mut()
                    .ok_or_else(|| refuse("scalar JSON ancestor"))?
                    .entry(key.clone())
                    .or_insert_with(|| serde_json::json!({}));
            }
            at.as_object_mut()
                .ok_or_else(|| refuse("scalar JSON ancestor"))?
                .insert(r.key.last().unwrap().clone(), v.clone());
            serde_json::to_vec_pretty(&doc).map_err(refuse)
        }
        GenericPayload::TomlMember(v) => {
            let mut doc = toml_doc(bytes)?;
            let mut at = &mut doc;
            for key in &r.key[..r.key.len() - 1] {
                at = at
                    .as_table_mut()
                    .ok_or_else(|| refuse("scalar TOML ancestor"))?
                    .entry(key.clone())
                    .or_insert_with(|| toml::Value::Table(Default::default()));
            }
            at.as_table_mut()
                .ok_or_else(|| refuse("scalar TOML ancestor"))?
                .insert(r.key.last().unwrap().clone(), v.clone());
            Ok(toml::to_string_pretty(&doc).map_err(refuse)?.into_bytes())
        }
        _ => Err(refuse("unexpanded tree")),
    }
}
