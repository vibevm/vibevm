//! Contract-to-health-plan preparation. No child process is started here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use crate::contract::{
    BaselineMode, CustomProtocol, Healthcheck, MavenRunner, NetworkPolicy, NodeManager, TestsMode,
};
use crate::glob::Glob;
use crate::model::{EntryKind, Inventory};

use super::model::*;
use super::preset::{cargo_commands, maven_commands, npm_commands, python_commands};

const CUSTOM_FILE_CAP: usize = 16 * 1024 * 1024;
const CUSTOM_BUNDLE_CAP: u64 = 64 * 1024 * 1024;

/// Injected executable/test discovery. Implementations may observe and seal
/// identities, but must not execute a shell command or mutate the project.
pub trait HealthResolver {
    fn resolve_asset(&mut self, request: ResolveAssetRequest)
    -> Result<AssetIdentity, HealthError>;

    fn resolve_custom_launch(
        &mut self,
        check_id: &str,
        interpreter: &str,
        source: &str,
    ) -> Result<ResolvedCustomLaunch, HealthError>;

    fn discover_tests(
        &mut self,
        project: &Project,
        inventory: &Inventory,
        request: &TestDiscoveryRequest,
    ) -> Result<TestPresence, HealthError>;
}

pub fn prepare<R: HealthResolver>(
    project: &Project,
    contract: &crate::contract::Contract,
    inventory: &Inventory,
    resolver: &mut R,
) -> Result<PreparedHealth, HealthError> {
    usize::try_from(contract.health.max_stdout_bytes).map_err(|_| {
        HealthError::Preparation("health.max_stdout_bytes exceeds this platform's usize".to_owned())
    })?;
    usize::try_from(contract.health.max_stderr_bytes).map_err(|_| {
        HealthError::Preparation("health.max_stderr_bytes exceeds this platform's usize".to_owned())
    })?;
    usize::try_from(contract.health.max_result_bytes).map_err(|_| {
        HealthError::Preparation("health.max_result_bytes exceeds this platform's usize".to_owned())
    })?;
    let mut rows = contract.healthcheck.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.id().as_bytes().cmp(right.id().as_bytes()));
    let mut checks = Vec::with_capacity(rows.len());
    let mut blockers = Vec::new();
    for row in rows {
        match prepare_check(project, contract, inventory, resolver, row) {
            Ok(check) => checks.push(check),
            Err(error) => blockers.push(blocker_for(row.id(), error)),
        }
    }
    if !checks
        .iter()
        .any(|check| check.applicability == Applicability::Applicable)
    {
        blockers.push(HealthBlocker {
            code: "health-no-applicable-required-check".to_owned(),
            check_id: None,
            message: "the health panel has no applicable required check".to_owned(),
        });
    }
    blockers.sort_by(|left, right| {
        (&left.code, &left.check_id, &left.message).cmp(&(
            &right.code,
            &right.check_id,
            &right.message,
        ))
    });
    let baseline = match contract.health.baseline {
        BaselineMode::Strict => BaselinePolicy::Strict,
        BaselineMode::NoRegression => BaselinePolicy::NoRegression,
    };
    let mut prepared = PreparedHealth {
        plan_id: String::new(),
        baseline,
        max_stdout_bytes: contract.health.max_stdout_bytes,
        max_stderr_bytes: contract.health.max_stderr_bytes,
        max_result_bytes: contract.health.max_result_bytes,
        termination_grace_seconds: contract.health.termination_grace_seconds,
        checks,
        blockers,
    };
    if let Some(blocker) = persistence_capacity_blocker(&prepared, inventory)? {
        prepared.blockers.push(blocker);
        prepared.blockers.sort_by(|left, right| {
            (&left.code, &left.check_id, &left.message).cmp(&(
                &right.code,
                &right.check_id,
                &right.message,
            ))
        });
    }
    prepared.plan_id = health_identity(&prepared)?;
    Ok(prepared)
}

pub(crate) fn persistence_capacity_blocker(
    prepared: &PreparedHealth,
    inventory: &Inventory,
) -> Result<Option<HealthBlocker>, HealthError> {
    const JSON_EXPANSION: u128 = 6;
    const FIXED_ENVELOPE: u128 = 4 * 1024 * 1024;

    let commands = prepared
        .checks
        .iter()
        .map(|check| check.commands.len() as u128)
        .sum::<u128>();
    let structured = prepared
        .checks
        .iter()
        .filter(|check| check.protocol == ResultProtocol::VibeHealthJsonV1)
        .count() as u128;
    let streams_per_command = u128::from(prepared.max_stdout_bytes)
        .checked_add(u128::from(prepared.max_stderr_bytes))
        .ok_or_else(|| HealthError::Preparation("health evidence size overflow".to_owned()))?;
    let retained = commands
        .checked_mul(2)
        .and_then(|value| value.checked_mul(streams_per_command))
        .and_then(|value| {
            structured
                .checked_mul(2)
                .and_then(|count| count.checked_mul(u128::from(prepared.max_result_bytes)))
                .and_then(|results| value.checked_add(results))
        })
        .ok_or_else(|| HealthError::Preparation("health evidence size overflow".to_owned()))?;
    let health_plan_bytes = serde_json::to_vec(prepared)
        .map_err(|error| HealthError::Preparation(format!("sizing health plan: {error}")))?
        .len() as u128;
    let tree_overhead = inventory.entries.iter().try_fold(0u128, |total, entry| {
        total
            .checked_add(2048)
            .and_then(|value| value.checked_add((entry.path.len() as u128).saturating_mul(12)))
            .ok_or_else(|| HealthError::Preparation("project evidence size overflow".to_owned()))
    })?;
    let worst_case = retained
        .checked_mul(JSON_EXPANSION)
        .and_then(|value| value.checked_add(health_plan_bytes.saturating_mul(4)))
        .and_then(|value| value.checked_add(tree_overhead))
        .and_then(|value| value.checked_add(FIXED_ENVELOPE))
        .ok_or_else(|| HealthError::Preparation("transaction evidence size overflow".to_owned()))?;
    let capacity = u128::from(
        crate::transaction::MAX_CANONICAL_REPORT_BYTES
            .min(crate::transaction::MAX_TRANSACTION_JOURNAL_BYTES) as u64,
    );
    Ok((worst_case > capacity).then(|| HealthBlocker {
        code: "health-evidence-store-capacity".to_owned(),
        check_id: None,
        message: format!(
            "declared health panel can require {worst_case} encoded bytes, exceeding the {capacity}-byte transaction/report capacity"
        ),
    }))
}

pub fn add_blockers(
    prepared: &mut PreparedHealth,
    blockers: impl IntoIterator<Item = HealthBlocker>,
) -> Result<(), HealthError> {
    prepared.blockers.extend(blockers);
    prepared.blockers.sort_by(|left, right| {
        (&left.code, &left.check_id, &left.message).cmp(&(
            &right.code,
            &right.check_id,
            &right.message,
        ))
    });
    prepared.blockers.dedup();
    prepared.plan_id = health_identity(prepared)?;
    Ok(())
}

fn prepare_check<R: HealthResolver>(
    project: &Project,
    contract: &crate::contract::Contract,
    inventory: &Inventory,
    resolver: &mut R,
    row: &Healthcheck,
) -> Result<PreparedHealthcheck, HealthError> {
    let (root, timeout, when, override_network, kind) = match row {
        Healthcheck::Cargo {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (
            root,
            timeout_seconds,
            when,
            *network,
            HealthcheckKind::Cargo,
        ),
        Healthcheck::Npm {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (root, timeout_seconds, when, *network, HealthcheckKind::Npm),
        Healthcheck::Maven {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (
            root,
            timeout_seconds,
            when,
            *network,
            HealthcheckKind::Maven,
        ),
        Healthcheck::PythonPip {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (
            root,
            timeout_seconds,
            when,
            *network,
            HealthcheckKind::PythonPip,
        ),
        Healthcheck::Custom {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (
            root,
            timeout_seconds,
            when,
            Some(*network),
            HealthcheckKind::Custom,
        ),
    };
    let network = network_mode(override_network.unwrap_or(contract.health.network));
    #[cfg(windows)]
    if let Healthcheck::Custom {
        protocol,
        reads,
        writes,
        spawn,
        network: custom_network,
        ..
    } = row
        && (*protocol != CustomProtocol::ExitCode
            || *custom_network != NetworkPolicy::Inherit
            || reads.as_slice() != ["**"]
            || !writes.is_empty()
            || !*spawn)
    {
        return Err(HealthError::Unsupported(
            "Windows epoch-1 custom health supports only protocol=exit-code, network=inherit, reads=[\"**\"], writes=[], spawn=true"
                .to_owned(),
        ));
    }
    let applicability = applicability(root, when.as_ref(), inventory);
    let custom_effects = matches!(row, Healthcheck::Custom { .. });
    let (mut reads, mut writes, spawn) = match row {
        Healthcheck::Custom {
            reads,
            writes,
            spawn,
            ..
        } => (reads.clone(), writes.clone(), *spawn),
        _ => (vec!["**".to_owned()], vec!["**".to_owned()], true),
    };
    reads.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    writes.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let unrestricted_reads = reads == ["**"];
    let effects = EffectPlan {
        reads,
        writes,
        spawn,
    };
    let mut sandbox = SandboxRequirement::for_check(network, custom_effects, spawn);
    if custom_effects && unrestricted_reads {
        // A complete read universe needs no narrower read-policy sandbox.
        sandbox.read_policy_enforcement = false;
    }
    if let Applicability::SkippedWhenMissing { .. } = &applicability {
        return Ok(PreparedHealthcheck {
            id: row.id().to_owned(),
            kind,
            root: root.clone(),
            applicability,
            tests: row.tests().map(|_| TestDisposition::SkippedNotPresent),
            network,
            assets: Vec::new(),
            commands: Vec::new(),
            effects,
            sandbox,
            protocol: protocol_for(row),
            custom_bundle: None,
            assurance_reductions: vec!["healthcheck-not-applicable".to_owned()],
            timeout_seconds: *timeout,
        });
    }
    ensure_root_exists(root, inventory)?;

    let (tests, mut assets, commands, protocol, custom_bundle) = match row {
        Healthcheck::Cargo {
            id,
            build,
            workspace,
            locked,
            all_targets,
            tests,
            profile,
            features,
            ..
        } => {
            require_file(inventory, &rooted(root, "Cargo.toml"), id, "Cargo manifest")?;
            let tests = prepare_tests(
                project,
                inventory,
                resolver,
                id,
                kind,
                root,
                *tests,
                None,
                *workspace,
                *all_targets,
                features.clone(),
            )?;
            let cargo = resolve(
                resolver,
                ResolveAssetRequest {
                    id: format!("{id}/cargo"),
                    role: AssetRole::Cargo,
                    selector: "cargo".to_owned(),
                },
            )?;
            let rustc = resolve(
                resolver,
                ResolveAssetRequest {
                    id: format!("{id}/rustc"),
                    role: AssetRole::Rustc,
                    selector: "rustc".to_owned(),
                },
            )?;
            let rustdoc = resolve(
                resolver,
                ResolveAssetRequest {
                    id: format!("{id}/rustdoc"),
                    role: AssetRole::Rustdoc,
                    selector: "rustdoc".to_owned(),
                },
            )?;
            let commands = cargo_commands(
                &cargo.id,
                &rustc.id,
                &rustdoc.id,
                *build,
                *workspace,
                *locked,
                *all_targets,
                *profile,
                features,
                tests,
                network,
            );
            (
                Some(tests),
                vec![cargo, rustc, rustdoc],
                commands,
                ResultProtocol::BuiltIn,
                None,
            )
        }
        Healthcheck::Npm {
            id,
            manager,
            lockfile,
            install,
            build_script,
            typecheck_script,
            tests,
            test_script,
            ..
        } => {
            if *manager != NodeManager::Npm {
                return Err(HealthError::Unsupported(format!(
                    "healthcheck `{id}` selects {manager:?}; schema-1 core currently has exact argv only for npm"
                )));
            }
            let package_json = rooted(root, "package.json");
            require_file(inventory, &package_json, id, "npm manifest")?;
            require_file(inventory, &rooted(root, lockfile), id, "npm lockfile")?;
            validate_npm_script(
                project,
                &package_json,
                build_script
                    .as_deref()
                    .or(typecheck_script.as_deref())
                    .expect("contract validation requires exactly one build or typecheck script"),
                id,
            )?;
            let tests = prepare_tests(
                project,
                inventory,
                resolver,
                id,
                kind,
                root,
                *tests,
                test_script.clone(),
                false,
                false,
                Vec::new(),
            )?;
            let node = resolve(
                resolver,
                ResolveAssetRequest {
                    id: format!("{id}/node"),
                    role: AssetRole::Node,
                    selector: "node".to_owned(),
                },
            )?;
            let cli = resolve(
                resolver,
                ResolveAssetRequest {
                    id: format!("{id}/npm-cli"),
                    role: AssetRole::NpmCli,
                    selector: "npm-cli.js".to_owned(),
                },
            )?;
            let script = build_script
                .as_deref()
                .or(typecheck_script.as_deref())
                .expect("contract validation requires exactly one script");
            let commands = npm_commands(
                &node.id,
                &cli.id,
                *install,
                script,
                test_script.as_deref(),
                tests,
                network,
            );
            (
                Some(tests),
                vec![node, cli],
                commands,
                ResultProtocol::BuiltIn,
                None,
            )
        }
        Healthcheck::Maven {
            id,
            runner,
            goal,
            offline,
            tests,
            ..
        } => {
            require_file(inventory, &rooted(root, "pom.xml"), id, "Maven manifest")?;
            let tests = prepare_tests(
                project,
                inventory,
                resolver,
                id,
                kind,
                root,
                *tests,
                None,
                false,
                false,
                Vec::new(),
            )?;
            let launcher = resolve(
                resolver,
                ResolveAssetRequest {
                    id: format!("{id}/maven-launcher"),
                    role: AssetRole::MavenLauncher,
                    selector: match runner {
                        MavenRunner::WrapperFirst => "maven-wrapper-first",
                        MavenRunner::Explicit => "maven-explicit",
                    }
                    .to_owned(),
                },
            )?;
            let commands = maven_commands(&launcher.id, goal, *offline, tests, network);
            (
                Some(tests),
                vec![launcher],
                commands,
                ResultProtocol::BuiltIn,
                None,
            )
        }
        Healthcheck::PythonPip {
            id,
            interpreter,
            source_roots,
            dependency_check,
            build,
            tests,
            test_runner,
            ..
        } => {
            for source_root in source_roots {
                let path = rooted(root, source_root);
                if !inventory.entries.iter().any(|entry| {
                    entry.path == path || entry.path.starts_with(&(path.clone() + "/"))
                }) {
                    return Err(HealthError::Preparation(format!(
                        "healthcheck `{id}` Python source root `{path}` is absent"
                    )));
                }
            }
            let tests = prepare_tests(
                project,
                inventory,
                resolver,
                id,
                kind,
                root,
                *tests,
                test_runner.clone(),
                false,
                false,
                Vec::new(),
            )?;
            let python = resolve(
                resolver,
                ResolveAssetRequest {
                    id: format!("{id}/python"),
                    role: AssetRole::Python,
                    selector: interpreter.clone(),
                },
            )?;
            let package_shaped = ["pyproject.toml", "setup.cfg", "setup.py"]
                .iter()
                .any(|path| inventory_has(inventory, &rooted(root, path)));
            let commands = python_commands(
                &python.id,
                source_roots,
                *dependency_check,
                *build,
                package_shaped,
                test_runner.as_deref(),
                tests,
                network,
            );
            (
                Some(tests),
                vec![python],
                commands,
                ResultProtocol::BuiltIn,
                None,
            )
        }
        Healthcheck::Custom {
            id,
            source,
            snapshot,
            interpreter,
            argv,
            protocol,
            ..
        } => {
            let bundle = prepare_bundle(project, inventory, source, snapshot)?;
            let launch = resolver.resolve_custom_launch(id, interpreter, source)?;
            if launch.style == CustomLaunchStyle::Direct {
                return Err(HealthError::Unsupported(
                    "direct bundled custom executables are not supported by the epoch-1 local backend"
                        .to_owned(),
                ));
            }
            validate_asset(&launch.asset)?;
            let expected_id = format!("{id}/custom-launch");
            let expected_role = match launch.style {
                CustomLaunchStyle::Interpreter => AssetRole::CustomInterpreter,
                CustomLaunchStyle::Direct => AssetRole::CustomNative,
            };
            if launch.asset.id != expected_id || launch.asset.role != expected_role {
                return Err(HealthError::Preparation(format!(
                    "resolver returned the wrong custom launch identity for `{id}`"
                )));
            }
            let executable_id = launch.asset.id.clone();
            let mut prepared_argv = Vec::with_capacity(argv.len() + 1);
            if launch.style == CustomLaunchStyle::Interpreter {
                prepared_argv.push(PreparedArg::BundlePath(source.clone()));
            }
            prepared_argv.extend(argv.iter().map(|arg| custom_arg(arg)));
            let command = PreparedCommand {
                step: CommandStep::Verify,
                executable_asset_id: executable_id,
                argv: prepared_argv,
                environment: super::preset::hermetic_environment(),
                accepted_exit_codes: vec![0],
            };
            (
                None,
                vec![launch.asset],
                vec![command],
                match protocol {
                    CustomProtocol::ExitCode => ResultProtocol::ExitCode,
                    CustomProtocol::VibeHealthJsonV1 => ResultProtocol::VibeHealthJsonV1,
                },
                Some(bundle),
            )
        }
    };
    sandbox.atomic_result = protocol == ResultProtocol::VibeHealthJsonV1;
    assets.sort_by(|left, right| left.id.as_bytes().cmp(right.id.as_bytes()));
    let mut assurance_reductions = Vec::new();
    if network != NetworkMode::Deny {
        assurance_reductions.push(
            match network {
                NetworkMode::ToolOffline => "network-tool-offline-unverified",
                NetworkMode::Inherit => "network-inherited",
                NetworkMode::Deny => unreachable!(),
            }
            .to_owned(),
        );
    }
    if tests.is_some_and(TestDisposition::reduces_assurance) {
        assurance_reductions.push("tests-skipped".to_owned());
    }
    if let Healthcheck::PythonPip { build: false, .. } = row {
        let package_shaped = ["pyproject.toml", "setup.cfg", "setup.py"]
            .iter()
            .any(|path| inventory_has(inventory, &rooted(root, path)));
        if package_shaped {
            assurance_reductions.push("python-package-build-omitted".to_owned());
        }
    }
    Ok(PreparedHealthcheck {
        id: row.id().to_owned(),
        kind,
        root: root.clone(),
        applicability,
        tests,
        network,
        assets,
        commands,
        effects,
        sandbox,
        protocol,
        custom_bundle,
        assurance_reductions,
        timeout_seconds: *timeout,
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_tests<R: HealthResolver>(
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

fn prepare_bundle(
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

fn custom_arg(value: &str) -> PreparedArg {
    match value {
        "{root}" => PreparedArg::Root,
        "{phase}" => PreparedArg::Phase,
        "{scratch}" => PreparedArg::Scratch,
        "{result}" => PreparedArg::Result,
        literal => PreparedArg::Literal(literal.to_owned()),
    }
}

fn resolve<R: HealthResolver>(
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

fn validate_asset(asset: &AssetIdentity) -> Result<(), HealthError> {
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

fn applicability(
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

fn ensure_root_exists(root: &str, inventory: &Inventory) -> Result<(), HealthError> {
    if root == "." || inventory.entries.iter().any(|entry| entry.path == root) {
        Ok(())
    } else {
        Err(HealthError::Preparation(format!(
            "health root `{root}` is absent"
        )))
    }
}

fn inventory_has(inventory: &Inventory, path: &str) -> bool {
    inventory.entries.iter().any(|entry| entry.path == path)
}

fn require_file(
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

fn validate_npm_script(
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

fn rooted(root: &str, path: &str) -> String {
    if root == "." {
        path.to_owned()
    } else {
        format!("{root}/{path}")
    }
}

fn network_mode(value: NetworkPolicy) -> NetworkMode {
    match value {
        NetworkPolicy::Deny => NetworkMode::Deny,
        NetworkPolicy::ToolOffline => NetworkMode::ToolOffline,
        NetworkPolicy::Inherit => NetworkMode::Inherit,
    }
}

fn protocol_for(row: &Healthcheck) -> ResultProtocol {
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

fn health_identity(value: &PreparedHealth) -> Result<String, HealthError> {
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

fn blocker_for(check_id: &str, error: HealthError) -> HealthBlocker {
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
