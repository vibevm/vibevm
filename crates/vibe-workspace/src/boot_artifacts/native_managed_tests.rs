use std::collections::BTreeMap;
use std::fs;
use std::sync::{Arc, Mutex};

use tempfile::TempDir;
use vibe_core::manifest::{LinkType, SpecFormat};
use vibe_spec::{
    CompileObserver, CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind, CompilerNativePolicy, CompilerPendingArtifact,
    CompilerPendingFinalizeError, EmissionEvent, StageDeltaEvent, TransformPlan,
    finalize_compiler_pending_artifact,
};
use vibe_wire::generated::native::e1::compile_reply::{CompileReply, CompileReplySkip};
use vibe_wire::generated::shared::Timestamp;

use crate::boot::{BootBand, BootEntry, BootProvenance, EffectiveBoot};
use crate::compile_trace::{ScopeAcquisition, TraceLimits, TraceRun};
use crate::extension_world::{
    CompilerNativeFactBinding, CompilerNativeFactError, ExtensionWorldEpoch, LoweredOwnerRuntimes,
    OwnerNativeCompileBinding, OwnerNativeCompileProvider, OwnerRuntimeEpoch, OwnerRuntimeId,
    OwnerRuntimeLowering, OwnerRuntimeRunFacts, OwnerRuntimeView, PendingBuildFact,
    PendingBuildProviderDigest, PendingHandlerConfigWitness, PendingPlatformKey,
    PendingSourceWitness, lower_owner_runtimes,
};
use crate::{Workspace, WorkspaceError};

use super::SelfCoordinate;
use super::native_managed::{
    OwnerNativeCompileMode, OwnerNativeCompileStatus, compile_static_owner_managed,
    write_boot_artifacts_owner_managed,
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
}

impl CompilerNativeInvoker for FakeBinding {
    fn invoke(&self, _call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
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
    fail_facts: bool,
    fail_ready: bool,
    pub(crate) owners: Vec<OwnerRuntimeId>,
    ready_finishes: Arc<Mutex<usize>>,
}

impl FakeProvider {
    pub(crate) fn new(reply: Reply) -> Self {
        Self {
            reply,
            fail_facts: false,
            fail_ready: false,
            owners: Vec::new(),
            ready_finishes: Arc::new(Mutex::new(0)),
        }
    }
}

impl OwnerNativeCompileProvider for FakeProvider {
    type Binding<'owner> = FakeBinding;

    fn bind<'owner>(
        &mut self,
        owner: OwnerRuntimeView<'owner>,
    ) -> Result<OwnerNativeCompileBinding<Self::Binding<'owner>>, WorkspaceError> {
        self.owners.push(owner.runtime().id().clone());
        Ok(OwnerNativeCompileBinding::new(
            FakeBinding {
                reply: self.reply,
                fail_facts: self.fail_facts,
                fail_ready: self.fail_ready,
                ready_finishes: Arc::clone(&self.ready_finishes),
            },
            CompilerNativePolicy::collect(),
        ))
    }
}

#[derive(Default)]
struct Observer {
    emissions: Mutex<Vec<(usize, usize, usize)>>,
    deltas: Mutex<usize>,
}

impl CompileObserver for Observer {
    fn emission(&self, event: &EmissionEvent) {
        self.emissions.lock().expect("observer").push((
            event.total_bytes(),
            event.frame_bytes(),
            event.contributions().iter().map(|part| part.bytes()).sum(),
        ));
    }

    fn stage_delta(&self, _event: &StageDeltaEvent) {
        *self.deltas.lock().expect("observer") += 1;
    }
}

struct Fixture {
    _root: TempDir,
    workspace: Workspace,
    epoch: OwnerRuntimeEpoch,
    boot: EffectiveBoot,
    self_coord: SelfCoordinate,
}

fn fixture(native: bool, with_static: bool) -> Fixture {
    let root = tempfile::tempdir().expect("workspace");
    let extension = if native {
        "\n[[extension]]\nid='native'\npoint='compile:emitted'\nhandler={kind='native',crate_dir='native'}\n"
    } else {
        ""
    };
    fs::write(
        root.path().join("vibe.toml"),
        format!("[project]\ngroup='org.demo'\nname='demo'\nversion='0.1.0'\n{extension}"),
    )
    .expect("manifest");
    fs::create_dir_all(root.path().join("boot")).expect("boot dir");
    fs::write(
        root.path().join("boot/input.md"),
        "# Input {#root}\n\nbody\n",
    )
    .expect("boot source");
    let workspace = Workspace::load(root.path()).expect("workspace");
    let lowered: LoweredOwnerRuntimes = lower_owner_runtimes(
        &workspace,
        &ExtensionWorldEpoch::empty(),
        OwnerRuntimeLowering::new(".", BTreeMap::new()),
    )
    .expect("runtimes");
    let epoch = lowered.bind_run(OwnerRuntimeRunFacts {
        run_id: "0123456789abcdef0123456789abcdef".to_owned(),
        state_root: root.path().join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        created_at: "2026-09-01T00:00:00Z".to_owned(),
    });
    let boot = EffectiveBoot {
        entries: with_static
            .then(|| BootEntry {
                path: "boot/input.md".to_owned(),
                band: BootBand::Dependency,
                link: LinkType::Static,
                when: None,
                origin: ".".to_owned(),
                provenance: BootProvenance::Node,
                use_ref: false,
                format: Default::default(),
                unit_substituted: false,
                elided: false,
            })
            .into_iter()
            .collect(),
    };
    Fixture {
        _root: root,
        workspace,
        epoch,
        boot,
        self_coord: SelfCoordinate::new(Some("org.demo".to_owned()), "demo".to_owned()),
    }
}

fn compile(
    fixture: &Fixture,
    provider: Option<&mut FakeProvider>,
    mode: OwnerNativeCompileMode<'_>,
) -> Result<Option<super::native_managed::OwnerManagedStaticCompile>, WorkspaceError> {
    compile_static_owner_managed(
        &fixture.boot,
        &fixture.workspace.root,
        &fixture.self_coord,
        SpecFormat::Mixed,
        fixture.epoch.node(".")?,
        mode,
        provider,
    )
}

#[test]
fn no_static_and_builtin_only_never_request_a_provider() {
    let empty = fixture(true, false);
    let mut provider = FakeProvider::new(Reply::Missing);
    assert!(
        compile(&empty, Some(&mut provider), OwnerNativeCompileMode::Plain)
            .expect("compile")
            .is_none()
    );
    assert!(provider.owners.is_empty());

    let builtin = fixture(false, true);
    let compiled = compile(&builtin, Some(&mut provider), OwnerNativeCompileMode::Plain)
        .expect("compile")
        .expect("artifact");
    assert!(compiled.native().is_none());
    assert!(provider.owners.is_empty());
}

#[test]
fn ready_and_pending_plain_observed_paths_agree_on_final_bytes_and_evidence() {
    for (reply, run_id) in [
        (Reply::Skip, "2123456789abcdef0123456789abcdef"),
        (Reply::Missing, "3123456789abcdef0123456789abcdef"),
    ] {
        let plain_fixture = fixture(true, true);
        let observed_fixture = fixture(true, true);
        let traced_fixture = fixture(true, true);
        let mut plain_provider = FakeProvider::new(reply);
        let mut observed_provider = FakeProvider::new(reply);
        let mut traced_provider = FakeProvider::new(reply);
        let plain = compile(
            &plain_fixture,
            Some(&mut plain_provider),
            OwnerNativeCompileMode::Plain,
        )
        .expect("plain")
        .expect("artifact");
        let observer = Arc::new(Observer::default());
        let observed = compile(
            &observed_fixture,
            Some(&mut observed_provider),
            OwnerNativeCompileMode::Observed(observer.clone()),
        )
        .expect("observed")
        .expect("artifact");
        let run = TraceRun::open_with_limits(
            &traced_fixture.workspace.root,
            run_id,
            Timestamp::from_timestamp(900, 0).expect("timestamp"),
            TraceLimits::for_test(u64::MAX, 9),
        )
        .expect("trace run");
        let acquisition = ScopeAcquisition::node(&run, ".", SpecFormat::Mixed);
        let traced = compile(
            &traced_fixture,
            Some(&mut traced_provider),
            OwnerNativeCompileMode::Traced(&acquisition),
        )
        .expect("traced")
        .expect("artifact");
        assert_eq!(plain.artifact().bytes(), observed.artifact().bytes());
        assert_eq!(plain.artifact().bytes(), traced.artifact().bytes());
        assert_eq!(
            plain.native().map(|outcome| outcome.status()),
            observed.native().map(|outcome| outcome.status())
        );
        assert_eq!(
            plain.native().map(|outcome| outcome.status()),
            traced.native().map(|outcome| outcome.status())
        );
        let emissions = observer.emissions.lock().expect("observer");
        assert_eq!(emissions[0].0, observed.artifact().bytes().len());
        assert_eq!(emissions[0].1, emissions[0].0 - emissions[0].2);
        if matches!(reply, Reply::Missing) {
            let plain_pending = plain.native().expect("native").pending().expect("pending");
            let observed_pending = observed
                .native()
                .expect("native")
                .pending()
                .expect("pending");
            assert_eq!(plain_pending.0, observed_pending.0);
            assert_eq!(
                plain_pending.0,
                traced
                    .native()
                    .expect("native")
                    .pending()
                    .expect("pending")
                    .0
            );
            assert!(
                !std::str::from_utf8(plain.artifact().bytes())
                    .expect("utf8")
                    .contains("<!-- vibe:transforms __host__/demo#native -->")
            );
            assert_eq!(*observer.deltas.lock().expect("observer"), 0);
        } else {
            assert_eq!(
                plain.native().expect("native").status(),
                OwnerNativeCompileStatus::Ready
            );
            assert_eq!(*plain_provider.ready_finishes.lock().expect("finish"), 1);
        }
    }
}

#[test]
fn native_without_provider_and_post_compile_fact_failure_are_typed() {
    let fixture = fixture(true, true);
    let error = compile(&fixture, None, OwnerNativeCompileMode::Plain).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::NativeCompileProvider { .. }
    ));

    let mut provider = FakeProvider::new(Reply::Missing);
    provider.fail_facts = true;
    let error = compile(&fixture, Some(&mut provider), OwnerNativeCompileMode::Plain).unwrap_err();
    assert!(matches!(error, WorkspaceError::NativeCompileFacts { .. }));

    let mut hard = FakeProvider::new(Reply::Hard);
    let error = compile(&fixture, Some(&mut hard), OwnerNativeCompileMode::Plain).unwrap_err();
    assert!(matches!(error, WorkspaceError::NativeCompile { .. }));

    let mut false_ready = FakeProvider::new(Reply::Skip);
    false_ready.fail_ready = true;
    let error = compile(
        &fixture,
        Some(&mut false_ready),
        OwnerNativeCompileMode::Plain,
    )
    .unwrap_err();
    assert!(matches!(error, WorkspaceError::NativeCompileFacts { .. }));
}

#[test]
fn traced_pending_scope_completes_with_the_finalized_output_fingerprint() {
    const RUN: &str = "0123456789abcdef0123456789abcdef";
    let traced_fixture = fixture(true, true);
    let run = TraceRun::open_with_limits(
        &traced_fixture.workspace.root,
        RUN,
        Timestamp::from_timestamp(1_000, 0).expect("timestamp"),
        TraceLimits::for_test(u64::MAX, 9),
    )
    .expect("trace run");
    let acquisition = ScopeAcquisition::node(&run, ".", SpecFormat::Mixed);
    let mut pending_provider = FakeProvider::new(Reply::Missing);
    let pending = compile(
        &traced_fixture,
        Some(&mut pending_provider),
        OwnerNativeCompileMode::Traced(&acquisition),
    )
    .expect("traced compile")
    .expect("artifact");
    let fingerprint = pending.artifact().output_fingerprint();

    let ready_fixture = fixture(true, true);
    let mut ready_provider = FakeProvider::new(Reply::Skip);
    let ready = compile(
        &ready_fixture,
        Some(&mut ready_provider),
        OwnerNativeCompileMode::Plain,
    )
    .expect("ready compile")
    .expect("artifact");
    assert_ne!(fingerprint, ready.artifact().output_fingerprint());

    let index_path = traced_fixture
        .workspace
        .root
        .join(".vibe/trace")
        .join(RUN)
        .join("index.json");
    let bytes = fs::read(index_path).expect("trace index");
    let index: vibe_wire::generated::compiler_trace_index::e1::index::CompilerTraceIndex =
        serde_json::from_slice(&bytes).expect("generated trace index");
    let value = serde_json::to_value(index).expect("trace value");
    assert!(value.to_string().contains(&fingerprint));
}

#[test]
fn traced_post_compile_fact_failure_terminally_fails_the_scope() {
    const RUN: &str = "1123456789abcdef0123456789abcdef";
    let fixture = fixture(true, true);
    let run = TraceRun::open_with_limits(
        &fixture.workspace.root,
        RUN,
        Timestamp::from_timestamp(1_001, 0).expect("timestamp"),
        TraceLimits::for_test(u64::MAX, 9),
    )
    .expect("trace run");
    let acquisition = ScopeAcquisition::node(&run, ".", SpecFormat::Mixed);
    let mut provider = FakeProvider::new(Reply::Missing);
    provider.fail_facts = true;
    assert!(
        compile(
            &fixture,
            Some(&mut provider),
            OwnerNativeCompileMode::Traced(&acquisition),
        )
        .is_err()
    );
    let index_path = fixture
        .workspace
        .root
        .join(".vibe/trace")
        .join(RUN)
        .join("index.json");
    let bytes = fs::read(index_path).expect("trace index");
    let _: vibe_wire::generated::compiler_trace_index::e1::index::CompilerTraceIndex =
        serde_json::from_slice(&bytes).expect("generated trace index");
    let text = String::from_utf8(bytes).expect("trace utf8");
    assert!(text.contains("failed"));
    assert!(!text.contains("\"status\":\"pending\""));
}

fn mismatched_finalizer(
    pending: CompilerPendingArtifact,
    _plan: &TransformPlan,
    fingerprint: &[u8; 32],
) -> Result<vibe_spec::CompilerFinalizedPendingArtifact, CompilerPendingFinalizeError> {
    let empty = TransformPlan::from_effective_rows(&[]).expect("empty plan");
    finalize_compiler_pending_artifact(pending, &empty, fingerprint)
}

#[test]
fn finalizer_refusal_fails_trace_and_preserves_prior_node_artifacts() {
    let fixture = fixture(true, true);
    let mut provider = FakeProvider::new(Reply::Missing);
    let (written, continuation) = write_boot_artifacts_owner_managed(
        &fixture.workspace.root,
        ".",
        &fixture.workspace.root,
        &fixture.self_coord,
        &fixture.boot,
        SpecFormat::Mixed,
        None,
        fixture.epoch.node(".").expect("owner"),
        Some(&mut provider),
    )
    .expect("pending publication");
    let static_path = written.static_lane.expect("static lane");
    let published = fs::read_to_string(&static_path).expect("published static");
    assert!(published.contains("vibe:transforms-pending"));
    assert!(matches!(
        continuation,
        Some(super::OwnerNativeCompileContinuation::Pending { .. })
    ));

    let prior = b"prior static bytes";
    fs::write(&static_path, prior).expect("prior bytes");
    const RUN: &str = "4123456789abcdef0123456789abcdef";
    let run = TraceRun::open_with_limits(
        &fixture.workspace.root,
        RUN,
        Timestamp::from_timestamp(1_002, 0).expect("timestamp"),
        TraceLimits::for_test(u64::MAX, 9),
    )
    .expect("trace run");
    let mut failing = FakeProvider::new(Reply::Missing);
    let refused = super::native_managed::write_boot_artifacts_owner_managed_with_finalizer(
        &fixture.workspace.root,
        ".",
        &fixture.workspace.root,
        &fixture.self_coord,
        &fixture.boot,
        SpecFormat::Mixed,
        Some(&run),
        fixture.epoch.node(".").expect("owner"),
        Some(&mut failing),
        mismatched_finalizer,
    );
    let error = match refused {
        Err(error) => error,
        Ok(_) => panic!("mismatched finalizer must refuse"),
    };
    assert!(matches!(
        error,
        WorkspaceError::NativePendingFinalize { .. }
    ));
    assert_eq!(fs::read(&static_path).expect("prior survives"), prior);
    let trace = fs::read_to_string(
        fixture
            .workspace
            .root
            .join(".vibe/trace")
            .join(RUN)
            .join("index.json"),
    )
    .expect("trace index");
    assert!(trace.contains("failed"));
    assert!(!trace.contains("\"status\":\"pending\""));
}

#[test]
fn node_unit_and_analyzer_adapters_use_one_core_and_exact_owner_views() {
    let core = include_str!("native_managed.rs");
    let unit = include_str!("../install/bootgen/hybrid_emit/native_managed.rs");
    let bound = include_str!("../install/bootgen/native_managed.rs");
    let seam = "fn compile_static_owner_managed_using<";
    assert_eq!(core.matches(seam).count(), 1);
    assert!(unit.contains("epoch.unit(&owner)"));
    assert!(bound.contains("BTreeMap<OwnerRuntimeId"));
}

#[test]
fn bound_analyzer_returns_final_pending_evidence_and_writes_nothing() {
    let fixture = fixture(true, true);
    let output_dir = fixture
        .workspace
        .root
        .join(vibe_core::layout::current_boot_dir());
    assert!(!output_dir.exists());
    let observer = Arc::new(Observer::default());
    let mut provider = FakeProvider::new(Reply::Missing);
    let analyzed = crate::install::analyze_effective_bound_native(
        &fixture.boot,
        &fixture.workspace.root,
        &fixture.self_coord,
        SpecFormat::Mixed,
        &fixture.epoch,
        ".",
        Some(&mut provider),
        Some(observer.clone()),
    )
    .expect("analyzer")
    .expect("artifact");
    assert!(
        std::str::from_utf8(analyzed.artifact.bytes())
            .expect("utf8")
            .contains("vibe:transforms-pending")
    );
    assert!(matches!(
        analyzed.native,
        Some(super::OwnerNativeCompileContinuation::Pending { .. })
    ));
    assert_eq!(observer.emissions.lock().expect("observer").len(), 1);
    assert_eq!(*observer.deltas.lock().expect("observer"), 0);
    assert!(!output_dir.exists(), "analyzer owns no publication path");
}
