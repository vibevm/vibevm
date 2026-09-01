//! Artifact-backed compiler-native invocation over one retained registry epoch.

use std::collections::BTreeMap;
use std::path::Path;

use specmark::spec;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{ExtensionHandler, MechanismRoutes};
use vibe_native_loader::{NativeCompileInvocation, NativeLoader};
use vibe_spec::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind, CompilerNativePolicy, compiler_native_implementation_digest,
};
use vibe_wire::behaviour::native_compile;
use vibe_wire::generated::native::e1::compile_request::CompileRequest;
use vibe_wire::generated::shared::{Execution, Io, Project, World};
use vibe_workspace::WorkspaceError;
use vibe_workspace::extension_world::{
    CompilerNativeFactBinding, CompilerNativeFactError, OwnerNativeCompileBinding,
    OwnerNativeCompileProvider, OwnerRuntimeId, OwnerRuntimeView, PendingBuildFact,
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
        let manager_order = call.order();
        let order = usize::try_from(manager_order).map_err(|_| {
            failed(format!(
                "compile row order {} is not an index",
                call.order()
            ))
        })?;
        let row = self.all_compile_rows.get(order).copied().ok_or_else(|| {
            failed(format!(
                "compile row order {} is outside retained epoch {}",
                call.order(),
                self.all_compile_rows.len()
            ))
        })?;
        if row.key() != call.key() {
            return Err(failed(format!(
                "compile row order {} names `{}`, not `{}`",
                call.order(),
                row.key(),
                call.key()
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
        if row.declaration().point != ExtensionPoint::Compile(call.point()) {
            return Err(failed(format!(
                "compile row `{}` does not declare point `{}`",
                row.key(),
                call.point()
            )));
        }
        let projected = effective_config(row).map_err(|error| {
            failed(format!(
                "compile row `{}` effective config is unavailable: {error}",
                row.key()
            ))
        })?;
        if &projected != call.config() {
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
        if implementation != call.implementation() {
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

        let qualified_key = call.key().to_string();
        let point = call.point();
        let manager_config = call.config().clone();
        let scratch = execution_scratch(self.selected_project_root, self.run_id, &qualified_key)
            .map_err(|error| failed(format!("compile row `{qualified_key}` scratch: {error}")))?;
        let request = CompileRequest {
            envelope: 1,
            execution: Execution {
                config: manager_config,
                id: row.declaration().id.clone(),
                package: row.provider().to_string(),
            },
            io: Io {
                scratch: scratch.display().to_string().replace('\\', "/"),
            },
            payload: call.into_payload(),
            point: point.to_string(),
            project: self.project.clone(),
            world: self.world.clone(),
        };
        native_compile::validate_request(&request).map_err(|error| {
            failed(format!(
                "compile row `{qualified_key}` generated request: {error}"
            ))
        })?;
        Ok(PreparedCall {
            row,
            order: manager_order,
            point,
            qualified_key,
            request,
        })
    }
}

impl CompilerNativeInvoker for ArtifactCompilerNativeInvoker<'_> {
    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        let prepared = self.prepare(call)?;
        let encoded = serde_json::to_vec(&prepared.request).map_err(|error| {
            failed(format!(
                "compile row `{}` request serialization: {error}",
                prepared.qualified_key
            ))
        })?;
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

fn artifact_failure(key: &str, error: NativeArtifactError) -> CompilerNativeInvokerError {
    failed(format!("compile row `{key}` artifact: {error}"))
}

fn failed(detail: impl AsRef<str>) -> CompilerNativeInvokerError {
    CompilerNativeInvokerError::new(CompilerNativeInvokerErrorKind::InvocationFailed, detail)
}
