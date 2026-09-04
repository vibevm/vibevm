specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use super::{hermetic_environment, literal};
use crate::health::model::{
    CommandStep, EnvironmentValue, NetworkMode, PreparedArg, PreparedCommand, TestDisposition,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn python_commands(
    executable_asset_id: &str,
    source_roots: &[String],
    dependency_check: bool,
    build: bool,
    package_shaped: bool,
    test_runner: Option<&str>,
    tests: TestDisposition,
    network: NetworkMode,
) -> Vec<PreparedCommand> {
    let mut source_roots = source_roots.to_vec();
    source_roots.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let mut environment = hermetic_environment();
    environment.insert(
        "PYTHONNOUSERSITE".to_owned(),
        EnvironmentValue::Literal("1".to_owned()),
    );
    environment.insert(
        "PYTHONHASHSEED".to_owned(),
        EnvironmentValue::Literal("0".to_owned()),
    );
    environment.insert(
        "PYTHONPYCACHEPREFIX".to_owned(),
        EnvironmentValue::ScratchPath("pycache".to_owned()),
    );
    if network == NetworkMode::ToolOffline {
        environment.insert(
            "PIP_NO_INDEX".to_owned(),
            EnvironmentValue::Literal("1".to_owned()),
        );
        environment.insert(
            "PIP_DISABLE_PIP_VERSION_CHECK".to_owned(),
            EnvironmentValue::Literal("1".to_owned()),
        );
    }
    let command = |step, argv| PreparedCommand {
        step,
        executable_asset_id: executable_asset_id.to_owned(),
        argv,
        environment: environment.clone(),
        accepted_exit_codes: vec![0],
    };
    let mut commands = Vec::new();
    if dependency_check {
        commands.push(command(
            CommandStep::Verify,
            vec![
                literal("-s"),
                literal("-m"),
                literal("pip"),
                literal("--disable-pip-version-check"),
                literal("check"),
            ],
        ));
    }
    let mut compile = vec![
        literal("-s"),
        literal("-m"),
        literal("compileall"),
        literal("--quiet"),
        literal("--force"),
        literal("--invalidation-mode"),
        literal("checked-hash"),
    ];
    compile.extend(source_roots.into_iter().map(literal));
    commands.push(command(CommandStep::Build, compile));
    if build && package_shaped {
        commands.push(command(
            CommandStep::Build,
            vec![
                literal("-s"),
                literal("-m"),
                literal("build"),
                literal("--no-isolation"),
                literal("--outdir"),
                PreparedArg::Scratch,
            ],
        ));
    }
    if tests.runs() {
        commands.push(command(
            CommandStep::Test,
            vec![
                literal("-s"),
                literal("-m"),
                literal(test_runner.expect("test disposition was prepared with a runner")),
            ],
        ));
    }
    commands
}
