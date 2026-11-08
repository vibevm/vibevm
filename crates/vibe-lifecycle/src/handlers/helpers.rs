//! Existing provider path, scratch, environment and stream projections.
use super::*;

pub(super) fn provider_root(row: &ExtensionRegistryRow) -> &Path {
    match row.provider() {
        ExtensionProvider::Dependency(provider) => &provider.root,
        ExtensionProvider::Host(provider) => &provider.root,
    }
}

pub(super) fn verified_scratch(context: &Context, key: &str) -> Result<PathBuf, HandlerError> {
    let advertised = Path::new(&context.io.scratch);
    let run_id = advertised
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(|| HandlerError::Scratch {
            key: key.into(),
            reason: "scratch has no run component".into(),
        })?;
    let actual = execution_scratch(Path::new(&context.project.root), run_id, key)
        .map_err(|error| scratch_error(key, error))?;
    let expected = advertised
        .canonicalize()
        .map_err(|error| HandlerError::Scratch {
            key: key.into(),
            reason: error.to_string(),
        })?;
    if actual != expected {
        return Err(HandlerError::Scratch {
            key: key.into(),
            reason: "advertised scratch does not match key digest".into(),
        });
    }
    Ok(actual)
}

pub(super) fn contained_existing(
    root: &Path,
    path: &Path,
    key: &str,
) -> Result<PathBuf, HandlerError> {
    let root = root
        .canonicalize()
        .map_err(|error| HandlerError::UnsafePath {
            key: key.into(),
            path: root.display().to_string(),
            reason: error.to_string(),
        })?;
    let path = path
        .canonicalize()
        .map_err(|error| HandlerError::UnsafePath {
            key: key.into(),
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
    if !path.starts_with(&root) {
        return Err(HandlerError::UnsafePath {
            key: key.into(),
            path: path.display().to_string(),
            reason: "canonical path escapes provider root".into(),
        });
    }
    Ok(path)
}

pub(super) fn script_env(
    row: &ExtensionRegistryRow,
    target: Option<&SlotTarget>,
    context: &Context,
    context_path: &Path,
    reply: &Path,
) -> std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString> {
    let executable = std::env::current_exe()
        .map(|path| machine_path(&path))
        .unwrap_or_else(|_| "vibe".to_string());
    let (group, name, version, kind, dir) = if let Some(target) = target {
        (
            target.group.clone(),
            target.name.clone(),
            target.version.clone(),
            target.kind.clone(),
            target.root.clone(),
        )
    } else {
        match row.provider() {
            ExtensionProvider::Dependency(provider) => (
                provider.id.group().to_string(),
                provider.id.name().to_string(),
                provider.version.clone(),
                provider.kind.to_string(),
                machine_path(&provider.root),
            ),
            ExtensionProvider::Host(provider) => (
                "__host__".into(),
                provider.identity.to_string(),
                provider.version.clone(),
                provider
                    .kind
                    .map(|kind| kind.to_string())
                    .unwrap_or_else(|| "project".into()),
                machine_path(&provider.root),
            ),
        }
    };
    let hook_phase = match row.declaration().point {
        ExtensionPoint::Slot(SlotPoint::PreInstall) => "pre-install",
        ExtensionPoint::Slot(SlotPoint::PostInstall) => "post-install",
        _ => "phase",
    };
    minimal_environment([
        ("VIBE_PACKAGE_GROUP".into(), group),
        ("VIBE_PACKAGE_NAME".into(), name),
        ("VIBE_PACKAGE_VERSION".into(), version),
        ("VIBE_PACKAGE_KIND".into(), kind),
        ("VIBE_PACKAGE_DIR".into(), dir),
        ("VIBE_HOOK_PHASE".into(), hook_phase.into()),
        ("VIBE_PROJECT_ROOT".into(), context.project.root.clone()),
        ("VIBE_EXECUTABLE".into(), executable),
        ("VIBE_EXTENSION_PROVIDER".into(), row.provider().to_string()),
        ("VIBE_CONTEXT".into(), machine_path(context_path)),
        ("VIBE_REPLY".into(), machine_path(reply)),
    ])
}

pub(super) fn streams(output: ProcessOutput) -> HandlerStreams {
    HandlerStreams {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        stdout_truncated: output.stdout_truncated,
        stderr_truncated: output.stderr_truncated,
    }
}

pub(super) fn scratch_error(key: &str, error: ScratchError) -> HandlerError {
    HandlerError::Scratch {
        key: key.into(),
        reason: error.to_string(),
    }
}

pub(super) fn machine_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
