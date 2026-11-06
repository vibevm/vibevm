use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use vibe_spec::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind, CompilerNativePolicy,
};
use vibe_wire::generated::native::e2::compile_reply::{
    CompileReply, CompileReplyOk, CompileReplySkip,
};

use crate::WorkspaceError;
use crate::extension_world::{
    CompilerNativeFactBinding, CompilerNativeFactError, CompilerNativeReplayFactory,
    OwnerNativeCompileBinding, OwnerNativeCompileProvider, OwnerRuntimeId, OwnerRuntimeView,
    PendingBuildFact, PendingBuildProviderDigest, PendingHandlerConfigWitness, PendingPlatformKey,
    PendingSourceWitness,
};

#[derive(Clone, Copy)]
pub(crate) enum Reply {
    Skip,
    AppendEmitted(&'static str),
    Missing,
    Hard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FakePolicyKind {
    Fail,
    Collect,
    Resolve,
}

impl FakePolicyKind {
    fn of(policy: &CompilerNativePolicy) -> Self {
        let rendered = format!("{policy:?}");
        if rendered.contains("Fail") {
            Self::Fail
        } else if rendered.contains("Collect") {
            Self::Collect
        } else if rendered.contains("Resolve") {
            Self::Resolve
        } else {
            panic!("unknown compiler native policy status: {rendered}")
        }
    }
}

pub(crate) struct FakeBinding {
    reply: Reply,
    fail_facts: bool,
    fail_ready: bool,
    ready_finishes: Arc<Mutex<usize>>,
    invocations: Arc<Mutex<usize>>,
    fact_drains: Arc<Mutex<usize>>,
}

impl CompilerNativeInvoker for FakeBinding {
    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        let mut invocations = self.invocations.lock().map_err(|_| {
            CompilerNativeInvokerError::new(
                CompilerNativeInvokerErrorKind::InvocationFailed,
                "fixture invocation counter poisoned",
            )
        })?;
        *invocations += 1;
        drop(invocations);
        match self.reply {
            Reply::AppendEmitted(marker) => append_emitted_reply(call, marker),
            Reply::Skip => serde_json::to_vec(&CompileReply::Skip(Box::new(CompileReplySkip {
                envelope: 2,
                message: Some("fixture skip".to_owned()),
            })))
            .map_err(|error| {
                CompilerNativeInvokerError::new(
                    CompilerNativeInvokerErrorKind::InvocationFailed,
                    error.to_string(),
                )
            }),
            Reply::Missing => Err(CompilerNativeInvokerError::new(
                CompilerNativeInvokerErrorKind::BuildableSourceUnavailable,
                "fixture source record missing",
            )),
            Reply::Hard => Err(CompilerNativeInvokerError::new(
                CompilerNativeInvokerErrorKind::InvocationFailed,
                "fixture hard failure",
            )),
        }
    }
}

/// Use the existing canonical byte codec; only the test's emitted bytes move.
fn append_emitted_reply(
    call: CompilerNativeCall<'_>,
    marker: &str,
) -> Result<Vec<u8>, CompilerNativeInvokerError> {
    use vibe_wire::behaviour::native_backend::{self, AdmittedBackendReply};
    use vibe_wire::generated::native::e1::backend_reply::{BackendReply, BackendReplyOk};
    use vibe_wire::generated::shared::IrV2;
    let error = |detail: &str| {
        CompilerNativeInvokerError::new(CompilerNativeInvokerErrorKind::InvocationFailed, detail)
    };
    let pass = format!("transform:emitted:{}", call.key());
    let mut payload = call.into_payload();
    let IrV2::EmittedArtifact(artifact) = &mut payload else {
        return Err(error("marker fixture requires emitted IR"));
    };
    let encoded = BackendReply::Ok(Box::new(BackendReplyOk {
        envelope: 1,
        bytes_b64: artifact.emitted.bytes_b64.clone(),
        message: None,
    }));
    let AdmittedBackendReply::Ok { mut bytes, .. } =
        native_backend::validate_reply(&encoded).map_err(|failure| error(&failure.to_string()))?
    else {
        return Err(error("marker fixture byte decode returned failure"));
    };
    if fixture_emitted_digest(&bytes) != artifact.emitted.provenance.bytes_digest {
        return Err(error(
            "marker fixture digest framing disagrees with manager input",
        ));
    }
    bytes.extend_from_slice(marker.as_bytes());
    artifact.emitted.provenance.bytes_digest = fixture_emitted_digest(&bytes);
    artifact.emitted.provenance.emitted_transforms.push(pass);
    let BackendReply::Ok(encoded) =
        native_backend::ok_reply(&bytes, None).map_err(|failure| error(&failure.to_string()))?
    else {
        return Err(error("marker fixture byte encode returned failure"));
    };
    artifact.emitted.bytes_b64 = encoded.bytes_b64;
    serde_json::to_vec(&CompileReply::Ok(Box::new(CompileReplyOk {
        envelope: 2,
        payload,
        message: None,
    })))
    .map_err(|failure| error(&failure.to_string()))
}

/// Test-only external-producer spelling of the current emitted-bytes/v1
/// protocol. Canonical owners: vibe-spec/compiler/emit/digest.rs::bytes_digest
/// and compiler/digest.rs::StableDigest. Checked against every actual input
/// before constructing the changed candidate; production keeps its one owner.
fn fixture_emitted_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new();
    for field in [b"vibe-spec/emitted-bytes/v1".as_slice(), bytes] {
        digest.update((field.len() as u64).to_le_bytes());
        digest.update(field);
    }
    format!("{:x}", digest.finalize())
}

impl CompilerNativeFactBinding for FakeBinding {
    fn invoker(&self) -> &dyn CompilerNativeInvoker {
        self
    }

    fn take_pending_build_facts(
        &self,
        pending: &vibe_spec::CompilerPendingSet,
    ) -> Result<Vec<PendingBuildFact>, CompilerNativeFactError> {
        let mut drains = self
            .fact_drains
            .lock()
            .map_err(|_| CompilerNativeFactError::poisoned())?;
        *drains += 1;
        drop(drains);
        if self.fail_facts {
            return Err(CompilerNativeFactError::missing(0));
        }
        pending
            .iter()
            .map(|reference| {
                PendingBuildFact::from_pending(
                    reference,
                    PendingPlatformKey::new("linux-x86_64")
                        .map_err(|_| CompilerNativeFactError::construction(reference.order()))?,
                    PendingSourceWitness::new([1; 32]),
                    PendingHandlerConfigWitness::new([2; 32]),
                    "build:cargo"
                        .parse()
                        .map_err(|_| CompilerNativeFactError::construction(reference.order()))?,
                    PendingBuildProviderDigest::new([3; 32]),
                )
                .map_err(|_| CompilerNativeFactError::construction(reference.order()))
            })
            .collect()
    }

    fn finish_ready(&self) -> Result<(), CompilerNativeFactError> {
        if self.fail_ready {
            return Err(CompilerNativeFactError::extra(0));
        }
        let mut count = self
            .ready_finishes
            .lock()
            .map_err(|_| CompilerNativeFactError::poisoned())?;
        *count += 1;
        Ok(())
    }
}

pub(crate) struct FakeProvider {
    reply: Reply,
    replies: BTreeMap<OwnerRuntimeId, Reply>,
    pub(crate) fail_facts: bool,
    pub(crate) fail_ready: bool,
    pub(crate) owners: Vec<OwnerRuntimeId>,
    policies: BTreeMap<OwnerRuntimeId, CompilerNativePolicy>,
    pub(crate) ready_finishes: Arc<Mutex<usize>>,
    pub(crate) invocations: Arc<Mutex<usize>>,
    pub(crate) fact_drains: Arc<Mutex<usize>>,
    strict_policies: bool,
}

impl FakeProvider {
    pub(crate) fn new(reply: Reply) -> Self {
        Self {
            reply,
            replies: BTreeMap::new(),
            fail_facts: false,
            fail_ready: false,
            owners: Vec::new(),
            policies: BTreeMap::new(),
            ready_finishes: Arc::new(Mutex::new(0)),
            invocations: Arc::new(Mutex::new(0)),
            fact_drains: Arc::new(Mutex::new(0)),
            strict_policies: false,
        }
    }

    pub(crate) fn with_policy(
        mut self,
        owner: OwnerRuntimeId,
        policy: CompilerNativePolicy,
    ) -> Self {
        self.policies.insert(owner, policy);
        self
    }

    pub(crate) fn with_reply(mut self, owner: OwnerRuntimeId, reply: Reply) -> Self {
        self.replies.insert(owner, reply);
        self
    }
}

impl OwnerNativeCompileProvider for FakeProvider {
    type Binding<'owner> = FakeBinding;

    fn bind<'owner>(
        &mut self,
        owner: OwnerRuntimeView<'owner>,
    ) -> Result<OwnerNativeCompileBinding<Self::Binding<'owner>>, WorkspaceError> {
        let owner_id = owner.runtime().id().clone();
        self.owners.push(owner_id.clone());
        let reply = self.replies.remove(&owner_id).unwrap_or(self.reply);
        let policy = match self.policies.remove(&owner_id) {
            Some(policy) => policy,
            None if !self.strict_policies => CompilerNativePolicy::collect(),
            None => {
                return Err(WorkspaceError::NativeCompileProvider {
                    owner: owner_id.to_string(),
                    reason: "replay provider has no exact owner policy".to_owned(),
                });
            }
        };
        Ok(OwnerNativeCompileBinding::new(
            FakeBinding {
                reply,
                fail_facts: self.fail_facts,
                fail_ready: self.fail_ready,
                ready_finishes: Arc::clone(&self.ready_finishes),
                invocations: Arc::clone(&self.invocations),
                fact_drains: Arc::clone(&self.fact_drains),
            },
            policy,
        ))
    }
}

pub(crate) struct FakeReplayFactory {
    reply: Reply,
    mutate: Option<fn(&mut BTreeMap<OwnerRuntimeId, CompilerNativePolicy>)>,
    pub(crate) creates: usize,
    pub(crate) finishes: usize,
    replies: BTreeMap<OwnerRuntimeId, Reply>,
    pub(crate) policy_kinds: BTreeMap<OwnerRuntimeId, FakePolicyKind>,
    pub(crate) visits: Vec<OwnerRuntimeId>,
}

impl FakeReplayFactory {
    pub(crate) const fn new(reply: Reply) -> Self {
        Self {
            reply,
            mutate: None,
            creates: 0,
            finishes: 0,
            replies: BTreeMap::new(),
            policy_kinds: BTreeMap::new(),
            visits: Vec::new(),
        }
    }

    pub(crate) fn with_reply(mut self, owner: OwnerRuntimeId, reply: Reply) -> Self {
        self.replies.insert(owner, reply);
        self
    }

    pub(crate) const fn with_mutation(
        mut self,
        mutate: fn(&mut BTreeMap<OwnerRuntimeId, CompilerNativePolicy>),
    ) -> Self {
        self.mutate = Some(mutate);
        self
    }
}

impl CompilerNativeReplayFactory for FakeReplayFactory {
    type Provider = FakeProvider;

    fn create(
        &mut self,
        mut policies: BTreeMap<OwnerRuntimeId, CompilerNativePolicy>,
    ) -> Result<Self::Provider, WorkspaceError> {
        self.creates += 1;
        self.policy_kinds = policies
            .iter()
            .map(|(owner, policy)| (owner.clone(), FakePolicyKind::of(policy)))
            .collect();
        if let Some(mutate) = self.mutate {
            mutate(&mut policies);
        }
        let mut provider = FakeProvider::new(self.reply);
        provider.policies = policies;
        provider.strict_policies = true;
        provider.replies = std::mem::take(&mut self.replies);
        Ok(provider)
    }

    fn finish(&mut self, provider: Self::Provider) -> Result<(), WorkspaceError> {
        self.finishes += 1;
        self.visits = provider.owners.clone();
        if provider.policies.is_empty() {
            Ok(())
        } else {
            Err(WorkspaceError::NativeCompileProvider {
                owner: "<fake-replay>".to_owned(),
                reason: "unused replay policy".to_owned(),
            })
        }
    }
}
