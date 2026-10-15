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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GenericFootprint {
    pub scope: ClientScope,
    pub destination: String,
    pub key: Vec<String>,
    pub operation: ClientOperation,
}
pub(super) trait ResourceShape {
    fn footprint(&self) -> GenericFootprint;
}
impl ResourceShape for GenericResource {
    fn footprint(&self) -> GenericFootprint {
        GenericFootprint {
            scope: self.scope,
            destination: self.destination.clone(),
            key: self.key.clone(),
            operation: self.operation,
        }
    }
}
impl ResourceShape for GenericFootprint {
    fn footprint(&self) -> GenericFootprint {
        self.clone()
    }
}
impl GenericResource {
    pub(crate) fn owned(&self, host: &GenericHost) -> String {
        self.footprint().owned(host)
    }
    pub(crate) fn lock(&self, host: &GenericHost) -> String {
        self.footprint().lock(host)
    }
    pub(crate) fn locks(&self, host: &GenericHost) -> Vec<String> {
        self.footprint().locks(host)
    }
}
impl GenericFootprint {
    fn with_digest(self, digest: String) -> GenericResource {
        GenericResource {
            scope: self.scope,
            destination: self.destination,
            key: self.key,
            operation: self.operation,
            desired_digest: digest,
        }
    }

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
    pub after: Option<Vec<u8>>,
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
    pub(super) coordination_identity: String,
    pub(super) migration: bool,
    pub(super) transfer: Option<adoption::TransferPlan>,
    pub(super) removed: Vec<GenericResource>,
    pub(super) retained_state:
        Option<vibe_wire::generated::deploy_generic_state::DeployGenericState>,
}
impl GenericPlan {
    pub fn target(&self) -> &str {
        &self.target
    }
    pub fn resources(&self) -> &[GenericResource] {
        &self.resources
    }
    /// Explicit removal footprint, included in the exact upgrade grant.
    pub fn removed_resources(&self) -> &[GenericResource] {
        &self.removed
    }
    /// Complete document effects, including whole-file absence.
    pub fn desired_documents(&self) -> impl Iterator<Item = (ClientScope, &str, Option<&[u8]>)> {
        self.documents
            .iter()
            .map(|d| (d.scope, d.destination.as_str(), d.after.as_deref()))
    }
    pub fn payloads(&self) -> &[GenericPayload] {
        &self.admitted.payloads
    }
    /// Full resulting document content, including active MCP launch settings.
    pub fn documents(&self) -> impl Iterator<Item = (ClientScope, &str, &[u8])> {
        self.documents.iter().filter_map(|d| {
            d.after
                .as_deref()
                .map(|bytes| (d.scope, d.destination.as_str(), bytes))
        })
    }
    /// Canonical durable subject. Process-local capture IDs and filesystem
    /// handles remain freshness checks, never durable identity material.
    pub fn identity(&self) -> String {
        let documents = self
            .documents
            .iter()
            .map(|d| {
                (
                    d.scope,
                    &d.destination,
                    d.before.as_ref().map(|b| (&b.sha256, b.size, b.unix_mode)),
                    d.after.as_ref().map(|b| digest(b)),
                )
            })
            .collect::<Vec<_>>();
        digest(
            &encoded(&(
                "generic-host-plan/2",
                &self.admitted.capture.observation.identity,
                &self.admitted.capture.observation.request,
                &self.control,
                &self.roots,
                &self.protected,
                &self.resources,
                documents,
                &self.receipts,
                &self.target,
                &self.coordination_identity,
                self.migration,
                self.transfer.as_ref().map(|t| {
                    (
                        &t.family,
                        &t.donor_id,
                        &t.donor_root_identity,
                        digest(&t.receipt),
                        &t.resources,
                    )
                }),
                &self.removed,
                &self.retained_state,
            ))
            .expect("serializable closed plan"),
        )
    }
    pub(super) fn locks(&self, host: &GenericHost) -> Vec<String> {
        self.resources
            .iter()
            .chain(&self.removed)
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
        self.plan_mode(admitted, false, None)
    }
    /// Explicit migration of C2 ownership with independently supplied closed
    /// semantics. Equal unowned bytes never qualify; original before-state is
    /// unavailable, even though current owned bytes can be retained.
    pub fn prepare_migration(
        &self,
        admitted: &AdmittedClient,
    ) -> Result<GenericPlan, GenericError> {
        self.plan_mode(admitted, true, None)
    }
    pub(super) fn plan_mode(
        &self,
        admitted: &AdmittedClient,
        migration: bool,
        transfer: Option<adoption::TransferPlan>,
    ) -> Result<GenericPlan, GenericError> {
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
        let mut retained = false;
        let mut retained_state = None;
        if let Some(state) = DeployState::observe_bound(&self.settings).map_err(refuse)? {
            if let Some(record) = state.read_generic(&home).map_err(refuse)? {
                retained = true;
                retained_state = Some(record.clone());
                if record.pending.is_some() {
                    return Err(refuse("pending generic operation requires recovery"));
                }
                if let Some(c) = record.committed {
                    if c.revoked {
                        return Err(refuse("receipt revoked"));
                    }
                    if c.coordination_identity != self.arena_identity()? {
                        return Err(refuse("coordination arena identity mismatch"));
                    }
                }
            }
        }
        let prior = receipts
            .iter()
            .find(|(id, _)| id == home.id())
            .map(|(_, r)| r);
        if prior.is_some_and(|r| {
            r.status == vibe_wire::generated::deploy_receipt::ReceiptStatus::RolledBack
        }) {
            return Err(refuse("inactive receipt proves no generic ownership"));
        }
        if migration && transfer.is_none() && (retained || prior.is_none()) {
            return Err(refuse(
                "migration requires an exact legacy receipt without retained state",
            ));
        }
        if !migration && prior.is_some() && !retained {
            return Err(refuse(
                "legacy generic receipt requires explicit closed-semantics migration",
            ));
        }
        if transfer.is_some() && (prior.is_some() || retained) {
            return Err(refuse("recipient already owns state"));
        }
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
        super::super::ownership::refuse_generic_foreign(&receipts.iter().filter(|(id,_)| !transfer.as_ref().is_some_and(|t|t.family == vibe_wire::generated::deploy_generic_state::TransferFamily::Deployment && t.donor_id == *id)).cloned().collect::<Vec<_>>(), home.id(), self, &resources)
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
            if migration && old != new {
                return Err(refuse("generic resource-set upgrades require C3 admission"));
            }
        }
        if let Some(t) = &transfer {
            if resources.len() != t.resources.len()
                || resources.iter().any(|r| {
                    !t.resources
                        .iter()
                        .any(|o| same_closed_resource(o, r) && o.desired_digest == r.desired_digest)
                })
            {
                return Err(refuse(
                    "closed semantics do not match exact donor ownership",
                ));
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
                    after: before.as_ref().map(|b| b.bytes.clone()),
                    before,
                });
                documents.len() - 1
            };
            let doc = &mut documents[index];
            let found = observe_value(doc.before.as_ref().map(|b| b.bytes.as_slice()), r)?;
            let donor_owned = transfer
                .as_ref()
                .and_then(|t| t.resources.iter().find(|o| same_closed_resource(o, r)))
                .map(|o| vibe_wire::generated::deploy_receipt::OwnedResource {
                    resource: r.owned(self),
                    post_digest: o.desired_digest.clone(),
                });
            let owned = prior
                .and_then(|receipt| {
                    receipt
                        .resources
                        .iter()
                        .find(|o| o.resource == r.owned(self))
                })
                .or(donor_owned.as_ref());
            if migration && owned.is_none_or(|o| o.post_digest != r.desired_digest) {
                return Err(refuse("migration semantics do not match old ownership"));
            }
            match (found, owned) {
                (None, None) => {}
                (Some(found), Some(owned)) if digest(&found) == owned.post_digest => {}
                _ => return Err(refuse("foreign occupant or stale owned value")),
            }
            doc.after = Some(reconcile(doc.after.as_deref().unwrap_or_default(), r, v)?);
            if doc.after.as_ref().is_some_and(|b| b.len() > CAP) {
                return Err(refuse("resulting document exceeds bounded host budget"));
            }
        }
        let mut removed = Vec::new();
        if let Some(record) = &retained_state {
            let c = record
                .committed
                .as_ref()
                .ok_or_else(|| refuse("missing committed semantics"))?;
            let receipt = prior.ok_or_else(|| refuse("missing joined receipt"))?;
            if c.receipt_digest != Some(digest(&encoded(receipt)?))
                || c.generation != receipt.generation
            {
                return Err(refuse("changed receipt join"));
            }
            if c.transfer.as_ref().is_some_and(|t| {
                t.phase == vibe_wire::generated::deploy_generic_state::TransferPhase::Restored
            }) {
                return Err(refuse("recipient relinquished to donor"));
            }
            for row in &c.resources {
                let r = inverse::resource(row);
                if !receipt
                    .resources
                    .iter()
                    .any(|o| o.resource == r.owned(self))
                {
                    continue;
                }
                if resources.iter().any(|n| n.owned(self) == r.owned(self)) {
                    continue;
                }
                self.protect(r.scope, &r.destination)?;
                if resources.iter().any(|n| {
                    n.scope == r.scope
                        && overlap(&n.destination, &r.destination)
                        && (n.destination != r.destination || key_overlap(&n.key, &r.key))
                }) {
                    return Err(refuse("upgrade old/new footprints overlap"));
                }
                let index = if let Some(i) = documents
                    .iter()
                    .position(|d| d.scope == r.scope && d.destination == r.destination)
                {
                    i
                } else {
                    let before = self
                        .root(r.scope)
                        .read_file_snapshot_bounded(&r.destination, CAP)
                        .map_err(refuse)?;
                    documents.push(Document {
                        scope: r.scope,
                        destination: r.destination.clone(),
                        after: before.as_ref().map(|b| b.bytes.clone()),
                        before,
                    });
                    documents.len() - 1
                };
                let doc = &mut documents[index];
                let found = observe_value(doc.before.as_ref().map(|b| b.bytes.as_slice()), &r)?;
                let owned = receipt
                    .resources
                    .iter()
                    .find(|o| o.resource == r.owned(self))
                    .unwrap();
                if found.as_ref().map(|b| digest(b)).as_ref() != Some(&owned.post_digest) {
                    return Err(refuse("removed resource drift"));
                }
                doc.after = restore_value(doc.after.as_deref(), &r, None)?;
                removed.push(r.with_digest(owned.post_digest.clone()));
            }
        }
        self.current(&admitted.capture)?;
        if migration {
            for document in &mut documents {
                document.after = Some(
                    document
                        .before
                        .as_ref()
                        .ok_or_else(|| refuse("migration destination missing"))?
                        .bytes
                        .clone(),
                );
            }
        }
        if self.authority.current_control().map_err(refuse)? != control {
            return Err(refuse("policy changed during plan"));
        }
        if self.state_identity()? != state_identity {
            return Err(refuse("state home changed during plan"));
        }
        Ok(GenericPlan {
            migration,
            transfer,
            removed,
            retained_state,
            coordination_identity: self.arena_identity()?,
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
    pub(super) fn protect(
        &self,
        scope: ClientScope,
        destination: &str,
    ) -> Result<(), GenericError> {
        if overlap(destination, ".vibe")
            || destination
                .split('/')
                .any(|part| part.ends_with(".vibe-skill-receipt.toml"))
        {
            return Err(refuse("protected donor ownership state"));
        }
        if vibe_safefs::paths_overlap(
            &self.root(scope).root_path().join(destination),
            &self.arena().root_path().join(".vibe"),
        ) {
            return Err(refuse("protected coordination arena lock store"));
        }
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
    r: &impl ResourceShape,
) -> Result<Option<Vec<u8>>, GenericError> {
    let r = r.footprint();
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
    r: &impl ResourceShape,
    p: &GenericPayload,
) -> Result<Vec<u8>, GenericError> {
    let r = r.footprint();
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

/// Restore/remove exactly one literal member while retaining current foreign
/// neighbors. Whole-file absence is represented by None, never a digest.
pub(super) fn restore_value(
    bytes: Option<&[u8]>,
    r: &impl ResourceShape,
    desired: Option<&[u8]>,
) -> Result<Option<Vec<u8>>, GenericError> {
    let r = r.footprint();
    if r.key.is_empty() {
        return Ok(desired.map(<[u8]>::to_vec));
    }
    let bytes = bytes.unwrap_or_default();
    if let Some(value) = desired {
        let payload = match r.operation {
            ClientOperation::ReconcileJsonMember => GenericPayload::JsonMember(json_doc(value)?),
            ClientOperation::ReconcileTomlMember => GenericPayload::TomlMember(
                toml_doc(value)?
                    .get("value")
                    .cloned()
                    .ok_or_else(|| refuse("retained TOML value missing"))?,
            ),
            _ => return Err(refuse("unsupported retained operation")),
        };
        return reconcile(bytes, &r, &payload).map(Some);
    }
    match r.operation {
        ClientOperation::ReconcileJsonMember => {
            let mut doc = json_doc(bytes)?;
            let mut at = &mut doc;
            for key in &r.key[..r.key.len() - 1] {
                let map = at
                    .as_object_mut()
                    .ok_or_else(|| refuse("scalar JSON ancestor"))?;
                let Some(next) = map.get_mut(key) else {
                    return Ok(Some(bytes.to_vec()));
                };
                at = next;
            }
            at.as_object_mut()
                .ok_or_else(|| refuse("scalar JSON ancestor"))?
                .remove(r.key.last().unwrap());
            Ok(Some(serde_json::to_vec_pretty(&doc).map_err(refuse)?))
        }
        ClientOperation::ReconcileTomlMember => {
            let mut doc = toml_doc(bytes)?;
            let mut at = &mut doc;
            for key in &r.key[..r.key.len() - 1] {
                let map = at
                    .as_table_mut()
                    .ok_or_else(|| refuse("scalar TOML ancestor"))?;
                let Some(next) = map.get_mut(key) else {
                    return Ok(Some(bytes.to_vec()));
                };
                at = next;
            }
            at.as_table_mut()
                .ok_or_else(|| refuse("scalar TOML ancestor"))?
                .remove(r.key.last().unwrap());
            Ok(Some(
                toml::to_string_pretty(&doc).map_err(refuse)?.into_bytes(),
            ))
        }
        _ => Err(refuse("unsupported retained operation")),
    }
}

fn same_closed_resource(a: &GenericResource, b: &GenericResource) -> bool {
    let file = |op| {
        matches!(
            op,
            ClientOperation::ProjectFile | ClientOperation::ProjectTree
        )
    };
    a.scope == b.scope
        && a.destination == b.destination
        && a.key == b.key
        && (a.operation == b.operation || (file(a.operation) && file(b.operation)))
}
