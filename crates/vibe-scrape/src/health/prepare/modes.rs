use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_tests<R: HealthResolver>(
    project: &Project,
    inventory: &Inventory,
    resolver: &mut R,
    check_id: &str,
    kind: HealthcheckKind,
    root: &str,
    mode: TestsMode,
    selector: Option<String>,
    workspace: bool,
    all_targets: bool,
    features: Vec<String>,
) -> Result<TestDisposition, HealthError> {
    if mode == TestsMode::Skip {
        return Ok(TestDisposition::SkippedByContract);
    }
    let request = TestDiscoveryRequest {
        check_id: check_id.to_owned(),
        kind,
        root: root.to_owned(),
        selector,
        workspace,
        all_targets,
        features,
    };
    match (mode, resolver.discover_tests(project, inventory, &request)?) {
        (TestsMode::IfPresent, TestPresence::Present) => Ok(TestDisposition::RunIfPresent),
        (TestsMode::IfPresent, TestPresence::Absent) => Ok(TestDisposition::SkippedNotPresent),
        (TestsMode::Required, TestPresence::Present) => Ok(TestDisposition::RunRequired),
        (TestsMode::Required, TestPresence::Absent) => Err(HealthError::Preparation(format!(
            "healthcheck `{check_id}` requires tests but no test target is discoverable"
        ))),
        (_, TestPresence::Indeterminate) => Err(HealthError::Preparation(format!(
            "healthcheck `{check_id}` test presence is indeterminate"
        ))),
        (TestsMode::Skip, _) => unreachable!(),
    }
}

pub(super) fn prepare_bundle(
    project: &Project,
    inventory: &Inventory,
    source: &str,
    patterns: &[String],
) -> Result<CustomBundle, HealthError> {
    let globs = patterns
        .iter()
        .map(|pattern| {
            Glob::parse(pattern).map_err(|error| HealthError::Preparation(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut entries = Vec::new();
    let mut total = 0_u64;
    for entry in &inventory.entries {
        if !globs.iter().any(|glob| glob.matches(&entry.path)) {
            continue;
        }
        match entry.kind {
            EntryKind::Directory => entries.push(BundleEntry {
                path: entry.path.clone(),
                kind: BundleEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: entry.unix_mode,
                content: None,
            }),
            EntryKind::File => {
                let snapshot = project
                    .read_file_snapshot_bounded(&entry.path, CUSTOM_FILE_CAP)
                    .map_err(|error| {
                        HealthError::Preparation(format!(
                            "snapshotting custom verifier `{}`: {error:#}",
                            entry.path
                        ))
                    })?
                    .ok_or_else(|| {
                        HealthError::Preparation(format!(
                            "custom verifier member `{}` disappeared",
                            entry.path
                        ))
                    })?;
                let digest = format!("sha256:{}", snapshot.sha256);
                if entry.sha256.as_deref() != Some(digest.as_str())
                    || entry.bytes != Some(snapshot.size)
                    || entry.unix_mode != snapshot.unix_mode
                    || entry.identity != Some(snapshot.identity)
                {
                    return Err(HealthError::Preparation(format!(
                        "custom verifier member `{}` changed since inventory",
                        entry.path
                    )));
                }
                total = total.checked_add(snapshot.size).ok_or_else(|| {
                    HealthError::Preparation("custom verifier bundle size overflow".to_owned())
                })?;
                if total > CUSTOM_BUNDLE_CAP {
                    return Err(HealthError::Preparation(format!(
                        "custom verifier bundle exceeds the {CUSTOM_BUNDLE_CAP}-byte cap"
                    )));
                }
                entries.push(BundleEntry {
                    path: entry.path.clone(),
                    kind: BundleEntryKind::File,
                    sha256: Some(digest),
                    bytes: Some(snapshot.size),
                    mode: snapshot.unix_mode,
                    content: Some(snapshot.bytes),
                });
            }
        }
    }
    entries.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    if !entries
        .iter()
        .any(|entry| entry.path == source && entry.kind == BundleEntryKind::File)
    {
        return Err(HealthError::Preparation(format!(
            "custom verifier source `{source}` is not one regular snapshot member"
        )));
    }
    let encoded = serde_json::to_vec(&entries).map_err(|error| {
        HealthError::Preparation(format!("encoding custom verifier manifest: {error}"))
    })?;
    Ok(CustomBundle {
        sha256: format!("sha256:{:x}", Sha256::digest(encoded)),
        source: source.to_owned(),
        entries,
    })
}

pub(super) fn custom_arg(value: &str) -> PreparedArg {
    match value {
        "{root}" => PreparedArg::Root,
        "{phase}" => PreparedArg::Phase,
        "{scratch}" => PreparedArg::Scratch,
        "{result}" => PreparedArg::Result,
        literal => PreparedArg::Literal(literal.to_owned()),
    }
}

pub(super) fn resolve<R: HealthResolver>(
    resolver: &mut R,
    request: ResolveAssetRequest,
) -> Result<AssetIdentity, HealthError> {
    let expected_id = request.id.clone();
    let expected_role = request.role.clone();
    let asset = resolver.resolve_asset(request)?;
    if asset.id != expected_id || asset.role != expected_role {
        return Err(HealthError::Preparation(format!(
            "resolver returned the wrong identity for `{expected_id}`"
        )));
    }
    validate_asset(&asset)?;
    Ok(asset)
}

pub(super) fn validate_asset(asset: &AssetIdentity) -> Result<(), HealthError> {
    if asset.id.is_empty()
        || asset.display_path.is_empty()
        || asset.platform_identity.is_empty()
        || asset.version.is_empty()
    {
        return Err(HealthError::Preparation(format!(
            "asset `{}` has incomplete sealed identity",
            asset.id
        )));
    }
    let valid_digest = asset.sha256.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    });
    if !valid_digest {
        return Err(HealthError::Preparation(format!(
            "asset `{}` has an invalid SHA-256 identity",
            asset.id
        )));
    }
    Ok(())
}

pub(super) fn applicability(
    root: &str,
    when: Option<&crate::contract::When>,
    inventory: &Inventory,
) -> Applicability {
    let Some(when) = when else {
        return Applicability::Applicable;
    };
    let path = rooted(root, &when.path_exists);
    if inventory_has(inventory, &path) {
        Applicability::Applicable
    } else {
        Applicability::SkippedWhenMissing { path }
    }
}

pub(super) fn ensure_root_exists(root: &str, inventory: &Inventory) -> Result<(), HealthError> {
    if root == "." || inventory.entries.iter().any(|entry| entry.path == root) {
        Ok(())
    } else {
        Err(HealthError::Preparation(format!(
            "health root `{root}` is absent"
        )))
    }
}

pub(super) fn inventory_has(inventory: &Inventory, path: &str) -> bool {
    inventory.entries.iter().any(|entry| entry.path == path)
}

pub(super) fn require_file(
    inventory: &Inventory,
    path: &str,
    check_id: &str,
    label: &str,
) -> Result<(), HealthError> {
    if inventory
        .entries
        .iter()
        .any(|entry| entry.path == path && entry.kind == EntryKind::File)
    {
        Ok(())
    } else {
        Err(HealthError::Preparation(format!(
            "healthcheck `{check_id}` {label} `{path}` is absent or not a regular file"
        )))
    }
}

pub(super) fn validate_npm_script(
    project: &Project,
    package_json: &str,
    script: &str,
    check_id: &str,
) -> Result<(), HealthError> {
    let bytes = project
        .read_file_bounded(package_json, 4 * 1024 * 1024)
        .map_err(|error| {
            HealthError::Preparation(format!("reading npm manifest `{package_json}`: {error:#}"))
        })?
        .ok_or_else(|| {
            HealthError::Preparation(format!("npm manifest `{package_json}` disappeared"))
        })?;
    super::protocol::reject_duplicate_keys(&bytes)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        HealthError::Preparation(format!("invalid npm manifest `{package_json}`: {error}"))
    })?;
    if value
        .get("scripts")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|scripts| {
            scripts
                .get(script)
                .is_some_and(serde_json::Value::is_string)
        })
    {
        Ok(())
    } else {
        Err(HealthError::Preparation(format!(
            "healthcheck `{check_id}` requires missing npm script `{script}`"
        )))
    }
}

pub(super) fn rooted(root: &str, path: &str) -> String {
    if root == "." {
        path.to_owned()
    } else {
        format!("{root}/{path}")
    }
}

pub(super) fn network_mode(value: NetworkPolicy) -> NetworkMode {
    match value {
        NetworkPolicy::Deny => NetworkMode::Deny,
        NetworkPolicy::ToolOffline => NetworkMode::ToolOffline,
        NetworkPolicy::Inherit => NetworkMode::Inherit,
    }
}

pub(super) fn protocol_for(row: &Healthcheck) -> ResultProtocol {
    match row {
        Healthcheck::Custom {
            protocol: CustomProtocol::ExitCode,
            ..
        } => ResultProtocol::ExitCode,
        Healthcheck::Custom {
            protocol: CustomProtocol::VibeHealthJsonV1,
            ..
        } => ResultProtocol::VibeHealthJsonV1,
        _ => ResultProtocol::BuiltIn,
    }
}

pub(super) fn health_identity(value: &PreparedHealth) -> Result<String, HealthError> {
    let mut projection = serde_json::to_value(value)
        .map_err(|error| HealthError::Preparation(format!("encoding health identity: {error}")))?;
    scrub_display_identity(&mut projection);
    if let Some(object) = projection.as_object_mut() {
        object.insert(
            "plan_id".to_owned(),
            serde_json::Value::String(String::new()),
        );
    }
    let encoded = serde_json::to_vec(&projection)
        .map_err(|error| HealthError::Preparation(format!("encoding health identity: {error}")))?;
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-health-e1\0");
    hash.update(encoded);
    Ok(format!("sha256:{:x}", hash.finalize()))
}

pub(super) fn blocker_for(check_id: &str, error: HealthError) -> HealthBlocker {
    let code = match error {
        HealthError::Preparation(_) => "health-preparation-failed",
        HealthError::Protocol(_) => "health-protocol-preparation-failed",
        HealthError::CheckProtocolFailed { .. } => "health-protocol-preparation-failed",
        HealthError::Execution(_) => "health-execution-preparation-failed",
        HealthError::CommandFailed { .. } => "health-command-preparation-failed",
        HealthError::CommandChangedTree { .. } => "health-tree-changed-during-preparation",
        HealthError::Unsupported(ref message)
            if message.starts_with("Windows epoch-1 custom health") =>
        {
            "health-custom-profile-unsupported"
        }
        HealthError::Unsupported(_) => "health-unsupported",
        HealthError::Tree(_) => "health-tree-preparation-failed",
        HealthError::Cancelled { .. } => "health-cancelled-during-preparation",
        HealthError::TimedOut { .. } => "health-timed-out-during-preparation",
    };
    HealthBlocker {
        code: code.to_owned(),
        check_id: Some(check_id.to_owned()),
        message: error.to_string(),
    }
}

fn scrub_display_identity(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                scrub_display_identity(value);
            }
        }
        serde_json::Value::Object(values) => {
            values.remove("display_path");
            values.remove("platform_identity");
            for value in values.values_mut() {
                scrub_display_identity(value);
            }
        }
        _ => {}
    }
}
