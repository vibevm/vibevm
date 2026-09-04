use crate::contract::{CargoBuild, CargoProfile};

use super::{hermetic_environment, literal};
use crate::health::model::{
    CommandStep, EnvironmentValue, NetworkMode, PreparedCommand, TestDisposition,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn cargo_commands(
    executable_asset_id: &str,
    build: CargoBuild,
    workspace: bool,
    locked: bool,
    all_targets: bool,
    profile: CargoProfile,
    features: &[String],
    tests: TestDisposition,
    network: NetworkMode,
) -> Vec<PreparedCommand> {
    let mut sorted_features = features.to_vec();
    sorted_features.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    let argv_for = |subcommand: &str| {
        let mut argv = vec![literal(subcommand)];
        if workspace {
            argv.push(literal("--workspace"));
        }
        if all_targets {
            argv.push(literal("--all-targets"));
        }
        if !sorted_features.is_empty() {
            argv.push(literal("--features"));
            argv.push(literal(sorted_features.join(",")));
        }
        argv.push(literal("--profile"));
        argv.push(literal(match profile {
            CargoProfile::Dev => "dev",
            CargoProfile::Release => "release",
        }));
        if locked {
            argv.push(literal("--locked"));
        }
        if network == NetworkMode::ToolOffline {
            argv.push(literal("--offline"));
        }
        argv
    };
    let mut environment = hermetic_environment();
    environment.insert(
        "CARGO_TARGET_DIR".to_owned(),
        EnvironmentValue::ScratchPath("cargo-target".to_owned()),
    );
    environment.insert(
        "CARGO_INCREMENTAL".to_owned(),
        EnvironmentValue::Literal("0".to_owned()),
    );
    environment.insert(
        "CARGO_TERM_COLOR".to_owned(),
        EnvironmentValue::Literal("never".to_owned()),
    );
    let build_subcommand = match build {
        CargoBuild::Check => "check",
        CargoBuild::Build => "build",
    };
    let mut commands = vec![PreparedCommand {
        step: CommandStep::Build,
        executable_asset_id: executable_asset_id.to_owned(),
        argv: argv_for(build_subcommand),
        environment: environment.clone(),
        accepted_exit_codes: vec![0],
    }];
    if tests.runs() {
        commands.push(PreparedCommand {
            step: CommandStep::Test,
            executable_asset_id: executable_asset_id.to_owned(),
            argv: argv_for("test"),
            environment,
            accepted_exit_codes: vec![0],
        });
    }
    commands
}
