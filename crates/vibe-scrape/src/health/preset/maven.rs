use super::{hermetic_environment, literal};
use crate::health::model::{CommandStep, NetworkMode, PreparedCommand, TestDisposition};

pub(crate) fn maven_commands(
    executable_asset_id: &str,
    goal: &str,
    offline: bool,
    tests: TestDisposition,
    network: NetworkMode,
) -> Vec<PreparedCommand> {
    let mut argv = vec![literal("--batch-mode"), literal("--no-transfer-progress")];
    if offline || network == NetworkMode::ToolOffline {
        argv.push(literal("--offline"));
    }
    if !tests.runs() {
        argv.push(literal("-DskipTests"));
    }
    argv.push(literal(goal));
    vec![PreparedCommand {
        step: CommandStep::Verify,
        executable_asset_id: executable_asset_id.to_owned(),
        argv,
        environment: hermetic_environment(),
        accepted_exit_codes: vec![0],
    }]
}
