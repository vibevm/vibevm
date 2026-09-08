//! Payload-free catalog admission and admitted compiler invocation.

use super::*;
use crate::process::execution_scratch;

#[derive(Clone)]
pub(super) enum ArtifactAccess {
    Published,
    Existing { scratch: PathBuf },
}

impl ArtifactAccess {
    pub(super) fn existing(scratch: &Path) -> Result<Self, String> {
        let metadata = std::fs::symlink_metadata(scratch).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
            return Err("existing compiler scratch is a link or not a directory".to_owned());
        }
        let scratch = scratch.canonicalize().map_err(|error| error.to_string())?;
        Ok(Self::Existing { scratch })
    }

    pub(super) fn scratch(
        &self,
        root: &Path,
        run_id: &str,
        key: &str,
    ) -> Result<PathBuf, CompilerNativeInvokerError> {
        match self {
            Self::Published => execution_scratch(root, run_id, key)
                .map_err(|error| failed(format!("compile row `{key}` scratch: {error}"))),
            Self::Existing { scratch } => Ok(scratch.clone()),
        }
    }

    pub(super) fn image(
        &self,
        root: &Path,
        artifact: ResolvedNativeArtifact,
        key: &str,
    ) -> Result<PathBuf, CompilerNativeInvokerError> {
        let source = Path::new(&artifact.path_absolute);
        let image = match self {
            Self::Published => publish_load_image(root, source, &artifact.digest, artifact.bytes),
            Self::Existing { .. } => {
                existing_load_image(root, source, &artifact.digest, artifact.bytes)
            }
        };
        image.map_err(|error| failed(format!("compile row `{key}` image: {error}")))
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
        let admitted = self.admit_compiler(key, order, config, implementation)?;
        self.admitted_frontends
            .lock()
            .map_err(|_| failed("compiler frontend admission cache is unavailable"))?
            .insert(
                order,
                AdmittedFrontend {
                    key: admitted.key,
                    compiler: admitted.compiler,
                },
            );
        Ok(())
    }

    fn admit_backend(
        &self,
        key: &vibe_core::manifest::ExtensionKey,
        order: u32,
        config: &BTreeMap<String, Option<serde_json::Value>>,
        implementation: vibe_spec::CompilerNativeImplementationDigest,
        backend: &str,
    ) -> Result<(), CompilerNativeInvokerError> {
        let admitted = self.admit_compiler(key, order, config, implementation)?;
        let declared = admitted
            .row
            .declaration()
            .pass
            .as_ref()
            .and_then(|pass| pass.artifact.as_deref());
        if declared != Some(backend) {
            return Err(failed(format!(
                "compile row `{}` backend differs from selected `{backend}`",
                admitted.key
            )));
        }
        self.admitted_backends
            .lock()
            .map_err(|_| failed("compiler backend admission cache is unavailable"))?
            .insert(
                order,
                AdmittedBackend {
                    key: admitted.key,
                    backend: backend.to_owned(),
                    compiler: admitted.compiler,
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
        if let Some(backend) = &prepared.backend {
            let admitted = self
                .admitted_backends
                .lock()
                .map_err(|_| failed("compiler backend admission cache is unavailable"))?;
            let admitted = admitted
                .get(&prepared.order)
                .ok_or_else(|| failed("compiler backend was not pre-admitted"))?;
            if admitted.key != prepared.qualified_key
                || admitted.backend != *backend
                || admitted.compiler.extension_id() != prepared.row.declaration().id
                || admitted.compiler.point() != prepared.point
            {
                return Err(failed(
                    "compiler backend admission differs from retained row",
                ));
            }
            return invoke_admitted(admitted, &encoded, &prepared.qualified_key);
        }
        if prepared.request.frontend_physical_stem.is_some() {
            let admitted = self
                .admitted_frontends
                .lock()
                .map_err(|_| failed("compiler frontend admission cache is unavailable"))?;
            let admitted = admitted
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
            return invoke_admitted(admitted, &encoded, &prepared.qualified_key);
        }
        self.invoke_unadmitted(prepared, &encoded)
    }
}

trait Admitted {
    fn compiler(&self) -> &NativeCompiler;
}

impl Admitted for AdmittedFrontend {
    fn compiler(&self) -> &NativeCompiler {
        &self.compiler
    }
}

impl Admitted for AdmittedBackend {
    fn compiler(&self) -> &NativeCompiler {
        &self.compiler
    }
}

fn invoke_admitted(
    admitted: &impl Admitted,
    request: &[u8],
    key: &str,
) -> Result<Vec<u8>, CompilerNativeInvokerError> {
    admitted
        .compiler()
        .invoke(request)
        .map_err(|error| failed(format!("compile row `{key}` loader: {error}")))
}

impl ArtifactCompilerNativeInvoker<'_> {
    fn invoke_unadmitted(
        &self,
        prepared: PreparedCall<'_>,
        encoded: &[u8],
    ) -> Result<Vec<u8>, CompilerNativeInvokerError> {
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
        let image = self.access.image(
            self.selected_project_root,
            artifact,
            &prepared.qualified_key,
        )?;
        self.loader
            .invoke_compile(NativeCompileInvocation {
                library: &image,
                extension_id: &prepared.row.declaration().id,
                point: prepared.point,
                request: encoded,
            })
            .map_err(|error| {
                failed(format!(
                    "compile row `{}` loader: {error}",
                    prepared.qualified_key
                ))
            })
    }
}
