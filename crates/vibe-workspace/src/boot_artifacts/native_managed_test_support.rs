use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use vibe_spec::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind, CompilerNativePolicy,
};
use vibe_wire::generated::native::e1::compile_reply::{CompileReply, CompileReplySkip};

use crate::WorkspaceError;
use crate::extension_world::{
    CompilerNativeFactBinding, CompilerNativeFactError, OwnerNativeCompileBinding,
    OwnerNativeCompileProvider, OwnerRuntimeId, OwnerRuntimeView, PendingBuildFact,
    PendingBuildProviderDigest, PendingHandlerConfigWitness, PendingPlatformKey,
    PendingSourceWitness,
};

#[derive(Clone, Copy)]
pub(crate) enum Reply {
    Skip,
    Missing,
    Hard,
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
    fn invoke(&self, _call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        let mut invocations = self.invocations.lock().map_err(|_| {
            CompilerNativeInvokerError::new(
                CompilerNativeInvokerErrorKind::InvocationFailed,
                "fixture invocation counter poisoned",
            )
        })?;
        *invocations += 1;
        drop(invocations);
        match self.reply {
            Reply::Skip => serde_json::to_vec(&CompileReply::Skip(Box::new(CompileReplySkip {
                envelope: 1,
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
    pub(crate) fail_facts: bool,
    pub(crate) fail_ready: bool,
    pub(crate) owners: Vec<OwnerRuntimeId>,
    policies: BTreeMap<OwnerRuntimeId, CompilerNativePolicy>,
    pub(crate) ready_finishes: Arc<Mutex<usize>>,
    pub(crate) invocations: Arc<Mutex<usize>>,
    pub(crate) fact_drains: Arc<Mutex<usize>>,
}

impl FakeProvider {
    pub(crate) fn new(reply: Reply) -> Self {
        Self {
            reply,
            fail_facts: false,
            fail_ready: false,
            owners: Vec::new(),
            policies: BTreeMap::new(),
            ready_finishes: Arc::new(Mutex::new(0)),
            invocations: Arc::new(Mutex::new(0)),
            fact_drains: Arc::new(Mutex::new(0)),
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
}

impl OwnerNativeCompileProvider for FakeProvider {
    type Binding<'owner> = FakeBinding;

    fn bind<'owner>(
        &mut self,
        owner: OwnerRuntimeView<'owner>,
    ) -> Result<OwnerNativeCompileBinding<Self::Binding<'owner>>, WorkspaceError> {
        let owner_id = owner.runtime().id().clone();
        self.owners.push(owner_id.clone());
        let policy = self
            .policies
            .remove(&owner_id)
            .unwrap_or_else(CompilerNativePolicy::collect);
        Ok(OwnerNativeCompileBinding::new(
            FakeBinding {
                reply: self.reply,
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
