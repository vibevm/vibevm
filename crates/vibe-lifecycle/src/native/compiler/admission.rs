//! Payload-free catalog admission and admitted compiler invocation.

use super::*;

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
