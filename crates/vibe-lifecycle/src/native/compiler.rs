//! Artifact-backed compiler-native invocation over one retained registry epoch.

use std::path::Path;

use specmark::spec;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::ExtensionHandler;
use vibe_native_loader::{NativeCompileInvocation, NativeLoader};
use vibe_spec::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind, compiler_native_implementation_digest,
};
use vibe_wire::behaviour::native_compile;
use vibe_wire::generated::native::e1::compile_request::CompileRequest;
use vibe_wire::generated::shared::{Execution, Io, Project, World};

use crate::ExtensionRegistryRow;
use crate::execution::effective_config;
use crate::process::execution_scratch;

use super::{
    NativeArtifactError, NativeBuildExecution, path::publish_load_image, process_loader,
    resolve_native_artifact,
};

/// One borrowed compiler-native execution epoch backed by retained ARTIFACTs.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-NATIVE-ONLY")]
pub struct ArtifactCompilerNativeInvoker<'a> {
    all_compile_rows: &'a [&'a ExtensionRegistryRow],
    execution: NativeBuildExecution<'a>,
    project: &'a Project,
    world: &'a World,
    run_id: &'a str,
    loader: &'static NativeLoader,
}

impl<'a> ArtifactCompilerNativeInvoker<'a> {
    #[must_use]
    pub fn new(
        all_compile_rows: &'a [&'a ExtensionRegistryRow],
        execution: NativeBuildExecution<'a>,
        project: &'a Project,
        world: &'a World,
        run_id: &'a str,
    ) -> Self {
        Self {
            all_compile_rows,
            execution,
            project,
            world,
            run_id,
            loader: process_loader(),
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
        let order = usize::try_from(call.order()).map_err(|_| {
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
            .execution
            .candidates
            .iter()
            .any(|candidate| std::ptr::eq(*candidate, row))
        {
            return Err(failed(format!(
                "compile row `{}` is not retained by the native execution epoch",
                row.key()
            )));
        }
        let selected_root = self
            .execution
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
        let scratch = execution_scratch(
            self.execution.selected_project_root,
            self.run_id,
            &qualified_key,
        )
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
        let artifact = resolve_native_artifact(&self.execution, prepared.row)
            .map_err(|error| artifact_failure(&prepared.qualified_key, error))?;
        let image = publish_load_image(
            self.execution.selected_project_root,
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

struct PreparedCall<'row> {
    row: &'row ExtensionRegistryRow,
    point: vibe_core::lifecycle::CompilePoint,
    qualified_key: String,
    request: CompileRequest,
}

fn artifact_failure(key: &str, error: NativeArtifactError) -> CompilerNativeInvokerError {
    match error {
        NativeArtifactError::SourceRecordMissing { record } => CompilerNativeInvokerError::new(
            CompilerNativeInvokerErrorKind::BuildableSourceUnavailable,
            format!("compile row `{key}` source record `{record}` is missing"),
        ),
        other => failed(format!("compile row `{key}` artifact: {other}")),
    }
}

fn failed(detail: impl AsRef<str>) -> CompilerNativeInvokerError {
    CompilerNativeInvokerError::new(CompilerNativeInvokerErrorKind::InvocationFailed, detail)
}
