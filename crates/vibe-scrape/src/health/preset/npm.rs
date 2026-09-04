specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use crate::contract::InstallMode;

use super::{hermetic_environment, literal};
use crate::health::model::{
    CommandStep, EnvironmentValue, NetworkMode, PreparedArg, PreparedCommand, TestDisposition,
};

pub(crate) fn npm_commands(
    node_asset_id: &str,
    cli_asset_id: &str,
    install: InstallMode,
    script: &str,
    test_script: Option<&str>,
    tests: TestDisposition,
    network: NetworkMode,
) -> Vec<PreparedCommand> {
    let mut environment = hermetic_environment();
    environment.insert(
        "npm_config_cache".to_owned(),
        EnvironmentValue::ScratchPath("npm-cache".to_owned()),
    );
    for (name, value) in [
        ("npm_config_audit", "false"),
        ("npm_config_fund", "false"),
        ("npm_config_update_notifier", "false"),
    ] {
        environment.insert(name.to_owned(), EnvironmentValue::Literal(value.to_owned()));
    }
    let prefix = || vec![PreparedArg::AssetPath(cli_asset_id.to_owned())];
    let mut commands = Vec::new();
    if install == InstallMode::Ci {
        let mut argv = prefix();
        argv.push(literal("ci"));
        if network == NetworkMode::ToolOffline {
            argv.push(literal("--offline"));
        }
        commands.push(PreparedCommand {
            step: CommandStep::Install,
            executable_asset_id: node_asset_id.to_owned(),
            argv,
            environment: environment.clone(),
            accepted_exit_codes: vec![0],
        });
    }
    let mut build_argv = prefix();
    build_argv.extend([literal("run"), literal(script)]);
    commands.push(PreparedCommand {
        step: CommandStep::Build,
        executable_asset_id: node_asset_id.to_owned(),
        argv: build_argv,
        environment: environment.clone(),
        accepted_exit_codes: vec![0],
    });
    if tests.runs() {
        let mut test_argv = prefix();
        test_argv.extend([
            literal("run"),
            literal(test_script.expect("test disposition was prepared with a script")),
        ]);
        commands.push(PreparedCommand {
            step: CommandStep::Test,
            executable_asset_id: node_asset_id.to_owned(),
            argv: test_argv,
            environment,
            accepted_exit_codes: vec![0],
        });
    }
    commands
}
