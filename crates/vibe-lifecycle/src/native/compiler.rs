//! Artifact-backed compiler-native invocation over one retained registry epoch.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use specmark::spec;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{ExtensionHandler, MechanismRoutes};
use vibe_native_loader::{NativeCompileInvocation, NativeCompiler, NativeLoader};
use vibe_spec::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind, CompilerNativePolicy, compiler_native_implementation_digest,
};
use vibe_wire::behaviour::native_compile;
use vibe_wire::generated::native::e1::compile_request::CompileRequest;
use vibe_wire::generated::shared::{Execution, Io, Project, World};
use vibe_workspace::WorkspaceError;
use vibe_workspace::extension_world::{
    CompilerNativeFactBinding, CompilerNativeFactError, CompilerNativeReplayFactory,
    OwnerNativeCompileBinding, OwnerNativeCompileProvider, OwnerRuntimeId, OwnerRuntimeView,
    PendingBuildFact,
};

use crate::execution::effective_config;
use crate::process::execution_scratch;
use crate::{ExtensionRegistryRow, MechanismRegistry};

use super::{
    NativeArtifactError, NativeBuildExecution, NativePlatform,
    compiler_facts::{CompilerArtifactResolutionError, PendingFactRecorder},
    path::publish_load_image,
    process_loader, resolve_native_artifact_for_compiler,
};

/// One borrowed compiler-native execution epoch backed by retained ARTIFACTs.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-NATIVE-ONLY")]
pub struct ArtifactCompilerNativeInvoker<'a> {
    all_compile_rows: Box<[&'a ExtensionRegistryRow]>,
    native_candidates: Box<[&'a ExtensionRegistryRow]>,
    selected_project_root: &'a Path,
    mechanisms: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
    platform: NativePlatform,
    offline: bool,
    created_at: &'a str,
    project: &'a Project,
    world: &'a World,
    run_id: &'a str,
    loader: &'static NativeLoader,
    facts: PendingFactRecorder,
    admitted_frontends: Mutex<BTreeMap<u32, AdmittedFrontend>>,
}

impl<'a> ArtifactCompilerNativeInvoker<'a> {
    #[must_use]
    pub fn new(
        all_compile_rows: &[&'a ExtensionRegistryRow],
        execution: NativeBuildExecution<'a>,
        project: &'a Project,
        world: &'a World,
        run_id: &'a str,
    ) -> Self {
        Self::from_parts(
            all_compile_rows,
            execution.candidates,
            execution.selected_project_root,
            execution.registry,
            execution.routes,
            execution.platform,
            execution.offline,
            execution.created_at,
            project,
            world,
            run_id,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        all_compile_rows: &[&'a ExtensionRegistryRow],
        native_candidates: &[&'a ExtensionRegistryRow],
        selected_project_root: &'a Path,
        mechanisms: &'a MechanismRegistry,
        routes: &'a MechanismRoutes,
        platform: NativePlatform,
        offline: bool,
        created_at: &'a str,
        project: &'a Project,
        world: &'a World,
        run_id: &'a str,
    ) -> Self {
        Self {
            all_compile_rows: all_compile_rows.to_vec().into_boxed_slice(),
            native_candidates: native_candidates.to_vec().into_boxed_slice(),
            selected_project_root,
            mechanisms,
            routes,
            platform,
            offline,
            created_at,
            project,
            world,
            run_id,
            loader: process_loader(),
            facts: PendingFactRecorder::new(),
            admitted_frontends: Mutex::new(BTreeMap::new()),
        }
    }

    fn execution(&self) -> NativeBuildExecution<'_> {
        NativeBuildExecution {
            candidates: &self.native_candidates,
            selected_project_root: self.selected_project_root,
            registry: self.mechanisms,
            routes: self.routes,
            platform: self.platform,
            offline: self.offline,
            created_at: self.created_at,
        }
    }

    #[cfg(test)]
    pub(super) const fn loader(&self) -> &'static NativeLoader {
        self.loader
    }

    #[cfg(test)]
    pub(super) fn request_for_test(
        &self,
        call: CompilerNativeCall<'_>,
    ) -> Result<CompileRequest, CompilerNativeInvokerError> {
        self.prepare(call).map(|prepared| prepared.request)
    }

    fn prepare<'row>(
        &'row self,
        call: CompilerNativeCall<'_>,
    ) -> Result<PreparedCall<'row>, CompilerNativeInvokerError> {
        let prepared = self.prepare_row(
            call.key(),
            call.point(),
            call.order(),
            call.config(),
            call.implementation(),
        )?;
        let manager_config = call.config().clone();
        let frontend_physical_stem = call.frontend_physical_stem().map(str::to_owned);
        let scratch = execution_scratch(
            self.selected_project_root,
            self.run_id,
            &prepared.qualified_key,
        )
        .map_err(|error| {
            failed(format!(
                "compile row `{}` scratch: {error}",
                prepared.qualified_key
            ))
        })?;
        let request = CompileRequest {
            envelope: 1,
            execution: Execution {
                config: manager_config,
                id: prepared.row.declaration().id.clone(),
                package: prepared.row.provider().to_string(),
            },
            io: Io {
                scratch: scratch.display().to_string().replace('\\', "/"),
            },
            payload: call.into_payload(),
            point: prepared.point.to_string(),
            project: self.project.clone(),
            world: self.world.clone(),
            frontend_physical_stem,
        };
        native_compile::validate_request(&request).map_err(|error| {
            failed(format!(
                "compile row `{}` generated request: {error}",
                prepared.qualified_key
            ))
        })?;
        Ok(PreparedCall {
            row: prepared.row,
            order: prepared.order,
            point: prepared.point,
            qualified_key: prepared.qualified_key,
            request,
        })
    }

    fn prepare_row<'row>(
        &'row self,
        key: &vibe_core::manifest::ExtensionKey,
        point: vibe_core::lifecycle::CompilePoint,
        manager_order: u32,
        config: &BTreeMap<String, Option<serde_json::Value>>,
        expected_implementation: vibe_spec::CompilerNativeImplementationDigest,
    ) -> Result<PreparedRow<'row>, CompilerNativeInvokerError> {
        let order = usize::try_from(manager_order)
            .map_err(|_| failed(format!("compile row order {manager_order} is not an index")))?;
        let row = self.all_compile_rows.get(order).copied().ok_or_else(|| {
            failed(format!(
                "compile row order {manager_order} is outside retained epoch {}",
                self.all_compile_rows.len()
            ))
        })?;
        if row.key() != key {
            return Err(failed(format!(
                "compile row order {} names `{}`, not `{}`",
                manager_order,
                row.key(),
                key
            )));
        }
        if !row.is_enabled() {
            return Err(failed(format!("compile row `{}` is disabled", row.key())));
        }
        if !matches!(row.declaration().handler, ExtensionHandler::Native { .. }) {
            return Err(failed(format!(
                "compile row `{}` is not a native handler",
                row.key()
            )));
        }
        if row.declaration().point != ExtensionPoint::Compile(point) {
            return Err(failed(format!(
                "compile row `{}` does not declare point `{}`",
                row.key(),
                point
            )));
        }
        let projected = effective_config(row).map_err(|error| {
            failed(format!(
                "compile row `{}` effective config is unavailable: {error}",
                row.key()
            ))
        })?;
        if &projected != config {
            return Err(failed(format!(
                "compile row `{}` effective config differs from manager call",
                row.key()
            )));
        }
        let implementation = compiler_native_implementation_digest(row).map_err(|error| {
            failed(format!(
                "compile row `{}` implementation is unavailable: {error}",
                row.key()
            ))
        })?;
        if implementation != expected_implementation {
            return Err(failed(format!(
                "compile row `{}` implementation differs from manager call",
                row.key()
            )));
        }
        if !self
            .native_candidates
            .iter()
            .any(|candidate| std::ptr::eq(*candidate, row))
        {
            return Err(failed(format!(
                "compile row `{}` is not retained by the native execution epoch",
                row.key()
            )));
        }
        let selected_root = self
            .selected_project_root
            .canonicalize()
            .map_err(|_| failed("selected project root cannot be canonicalized"))?;
        let injected_root = Path::new(&self.project.root)
            .canonicalize()
            .map_err(|_| failed("injected project root cannot be canonicalized"))?;
        if selected_root != injected_root {
            return Err(failed(
                "injected project root differs from selected project root",
            ));
        }

        Ok(PreparedRow {
            row,
            order: manager_order,
            point,
            qualified_key: key.to_string(),
        })
    }
}

impl CompilerNativeInvoker for ArtifactCompilerNativeInvoker<'_> {
    fn admit_frontend(
        &self,
        key: &vibe_core::manifest::ExtensionKey,
        order: u32,
        config: &BTreeMap<String, Option<serde_json::Value>>,
        implementation: vibe_spec::CompilerNativeImplementationDigest,
    ) -> Result<(), CompilerNativeInvokerError> {
        let prepared = self.prepare_row(
            key,
            vibe_core::lifecycle::CompilePoint::Pass,
            order,
            config,
            implementation,
        )?;
        let artifact = match resolve_native_artifact_for_compiler(
            &self.execution(),
            prepared.row,
            prepared.order,
        ) {
            Ok(artifact) => artifact,
            Err(CompilerArtifactResolutionError::Missing { record, .. }) => {
                return Err(CompilerNativeInvokerError::new(
                    CompilerNativeInvokerErrorKind::BuildableSourceUnavailable,
                    format!(
                        "compile row `{}` source record `{record}` is missing",
                        prepared.qualified_key
                    ),
                ));
            }
            Err(CompilerArtifactResolutionError::Artifact(error)) => {
                return Err(artifact_failure(&prepared.qualified_key, error));
            }
            Err(CompilerArtifactResolutionError::Fact(reason)) => {
                return Err(failed(format!(
                    "compile row `{}` pending facts: {reason}",
                    prepared.qualified_key
                )));
            }
        };
        let image = publish_load_image(
            self.selected_project_root,
            Path::new(&artifact.path_absolute),
            &artifact.digest,
            artifact.bytes,
        )
        .map_err(|error| {
            failed(format!(
                "compile row `{}` image: {error}",
                prepared.qualified_key
            ))
        })?;
        let compiler = self
            .loader
            .admit_compile(&image, &prepared.row.declaration().id, prepared.point)
            .map_err(|error| {
                failed(format!(
                    "compile row `{}` loader: {error}",
                    prepared.qualified_key
                ))
            })?;
        self.admitted_frontends
            .lock()
            .map_err(|_| failed("compiler frontend admission cache is unavailable"))?
            .insert(
                prepared.order,
                AdmittedFrontend {
                    key: prepared.qualified_key,
                    compiler,
                },
            );
        Ok(())
    }

    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        let prepared = self.prepare(call)?;
        let encoded = serde_json::to_vec(&prepared.request).map_err(|error| {
            failed(format!(
                "compile row `{}` request serialization: {error}",
                prepared.qualified_key
            ))
        })?;
        if prepared.request.frontend_physical_stem.is_some() {
            let admitted_frontends = self
                .admitted_frontends
                .lock()
                .map_err(|_| failed("compiler frontend admission cache is unavailable"))?;
            let admitted = admitted_frontends
                .get(&prepared.order)
                .ok_or_else(|| failed("compiler frontend was not pre-admitted"))?;
            if admitted.key != prepared.qualified_key
                || admitted.compiler.extension_id() != prepared.row.declaration().id
                || admitted.compiler.point() != prepared.point
            {
                return Err(failed(
                    "compiler frontend admission differs from retained row",
                ));
            }
            return admitted.compiler.invoke(&encoded).map_err(|error| {
                failed(format!(
                    "compile row `{}` loader: {error}",
                    prepared.qualified_key
                ))
            });
        }
        let execution = self.execution();
        let artifact =
            match resolve_native_artifact_for_compiler(&execution, prepared.row, prepared.order) {
                Ok(artifact) => artifact,
                Err(CompilerArtifactResolutionError::Missing { record, fact }) => {
                    self.facts.record(*fact).map_err(|error| {
                        failed(format!(
                            "compile row `{}` pending fact recorder: {error}",
                            prepared.qualified_key
                        ))
                    })?;
                    return Err(CompilerNativeInvokerError::new(
                        CompilerNativeInvokerErrorKind::BuildableSourceUnavailable,
                        format!(
                            "compile row `{}` source record `{record}` is missing",
                            prepared.qualified_key
                        ),
                    ));
                }
                Err(CompilerArtifactResolutionError::Artifact(error)) => {
                    return Err(artifact_failure(&prepared.qualified_key, error));
                }
                Err(CompilerArtifactResolutionError::Fact(reason)) => {
                    return Err(failed(format!(
                        "compile row `{}` pending facts: {reason}",
                        prepared.qualified_key
                    )));
                }
            };
        let image = publish_load_image(
            self.selected_project_root,
            Path::new(&artifact.path_absolute),
            &artifact.digest,
            artifact.bytes,
        )
        .map_err(|error| {
            failed(format!(
                "compile row `{}` image: {error}",
                prepared.qualified_key
            ))
        })?;
        self.loader
            .invoke_compile(NativeCompileInvocation {
                library: &image,
                extension_id: &prepared.row.declaration().id,
                point: prepared.point,
                request: &encoded,
            })
            .map_err(|error| {
                failed(format!(
                    "compile row `{}` loader: {error}",
                    prepared.qualified_key
                ))
            })
    }
}

impl CompilerNativeFactBinding for ArtifactCompilerNativeInvoker<'_> {
    fn invoker(&self) -> &dyn CompilerNativeInvoker {
        self
    }

    fn take_pending_build_facts(
        &self,
        pending: &vibe_spec::CompilerPendingSet,
    ) -> Result<Vec<PendingBuildFact>, CompilerNativeFactError> {
        self.facts.take(pending)
    }

    fn finish_ready(&self) -> Result<(), CompilerNativeFactError> {
        self.facts.finish_ready()
    }
}

/// Lifecycle implementation of workspace's lazy owner-binding port.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER")]
pub struct ArtifactCompilerNativeProvider {
    platform: NativePlatform,
    policies: BTreeMap<OwnerRuntimeId, CompilerNativePolicy>,
}

impl ArtifactCompilerNativeProvider {
    #[must_use]
    pub fn new(
        platform: NativePlatform,
        policies: BTreeMap<OwnerRuntimeId, CompilerNativePolicy>,
    ) -> Self {
        Self { platform, policies }
    }

    fn finish(self) -> Result<(), WorkspaceError> {
        if self.policies.is_empty() {
            return Ok(());
        }
        Err(WorkspaceError::NativeCompileProvider {
            owner: "<replay-provider>".to_owned(),
            reason: format!(
                "{} compiler-native replay policies were not consumed",
                self.policies.len()
            ),
        })
    }
}

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER")]
pub struct ArtifactCompilerNativeReplayFactory {
    platform: NativePlatform,
}

impl ArtifactCompilerNativeReplayFactory {
    #[must_use]
    pub const fn new(platform: NativePlatform) -> Self {
        Self { platform }
    }
}

impl CompilerNativeReplayFactory for ArtifactCompilerNativeReplayFactory {
    type Provider = ArtifactCompilerNativeProvider;

    fn create(
        &mut self,
        policies: BTreeMap<OwnerRuntimeId, CompilerNativePolicy>,
    ) -> Result<Self::Provider, WorkspaceError> {
        Ok(ArtifactCompilerNativeProvider::new(self.platform, policies))
    }

    fn finish(&mut self, provider: Self::Provider) -> Result<(), WorkspaceError> {
        provider.finish()
    }
}

impl OwnerNativeCompileProvider for ArtifactCompilerNativeProvider {
    type Binding<'owner> = ArtifactCompilerNativeInvoker<'owner>;

    fn bind<'owner>(
        &mut self,
        owner: OwnerRuntimeView<'owner>,
    ) -> Result<OwnerNativeCompileBinding<Self::Binding<'owner>>, WorkspaceError> {
        let runtime = owner.runtime();
        let run = owner.run();
        if run.platform != self.platform.key() {
            return Err(WorkspaceError::NativeCompileProvider {
                owner: runtime.id().to_string(),
                reason: format!(
                    "run platform `{}` differs from selected native platform `{}`",
                    run.platform,
                    self.platform.key()
                ),
            });
        }
        let rows = runtime.rows()?;
        let policy = self.policies.remove(runtime.id()).ok_or_else(|| {
            WorkspaceError::NativeCompileProvider {
                owner: runtime.id().to_string(),
                reason: "no injected compiler-native policy exists for this owner".to_owned(),
            }
        })?;
        let invoker = ArtifactCompilerNativeInvoker::from_parts(
            rows.compile(),
            rows.native(),
            owner.selected_root(),
            runtime.mechanisms(),
            runtime.routes(),
            self.platform,
            run.offline,
            &run.created_at,
            owner.project(),
            owner.world(),
            &run.run_id,
        );
        Ok(OwnerNativeCompileBinding::new(invoker, policy))
    }
}

struct PreparedCall<'row> {
    row: &'row ExtensionRegistryRow,
    order: u32,
    point: vibe_core::lifecycle::CompilePoint,
    qualified_key: String,
    request: CompileRequest,
}

struct PreparedRow<'row> {
    row: &'row ExtensionRegistryRow,
    order: u32,
    point: vibe_core::lifecycle::CompilePoint,
    qualified_key: String,
}

struct AdmittedFrontend {
    key: String,
    compiler: NativeCompiler,
}

fn artifact_failure(key: &str, error: NativeArtifactError) -> CompilerNativeInvokerError {
    failed(format!("compile row `{key}` artifact: {error}"))
}

fn failed(detail: impl AsRef<str>) -> CompilerNativeInvokerError {
    CompilerNativeInvokerError::new(CompilerNativeInvokerErrorKind::InvocationFailed, detail)
}
