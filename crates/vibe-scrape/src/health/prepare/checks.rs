use super::modes::*;
use super::*;

pub(super) fn prepare_check<R: HealthResolver>(
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
