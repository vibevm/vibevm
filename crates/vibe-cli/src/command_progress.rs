//! Closed command-level progress policy for the CLI composition root.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#COMMAND-COVERAGE");

use crate::cli::{
    AgenticSubcommand, AiuiSubcommand, BinCmd, CacheSubcommand, Command, DocCommand,
    DocShellCommand, ExtensionsCommand, McpSubcommand, PrefsSubcommand, ProgressSubcommand,
    RegistryRemoveTarget, RegistrySubcommand, ScrapeCommand, ScrapeContractCommand, ShowSubcommand,
    SkillSubcommand, VvmSubcommand, WorkspaceSubcommand,
};
use crate::commands::facts::FactsSubcommand;
use crate::commands::refactor::RefactorCommand;
use crate::output::{Context, ProgressMode};
use vibe_core::progress::{Progress, ProgressTask};

/// Every top-level Clap command, used by the policy-inventory regression.
#[cfg(test)]
pub(crate) const TOP_LEVEL_COMMANDS: &[&str] = &[
    "agentic",
    "aiui",
    "bin",
    "build",
    "cache",
    "check",
    "clean",
    "command",
    "create",
    "deploy",
    "deployments",
    "doc",
    "explain",
    "extensions",
    "facts",
    "frame",
    "friends",
    "generate",
    "init",
    "install",
    "list",
    "mcp",
    "outdated",
    "package",
    "prefs",
    "progress",
    "query",
    "refactor",
    "registry",
    "reinstall",
    "requirements",
    "scrape",
    "search",
    "select",
    "self",
    "show",
    "skill",
    "specmap",
    "term",
    "test",
    "tools",
    "trace",
    "tree",
    "undeploy",
    "uninstall",
    "update",
    "validate",
    "vars",
    "verify",
    "version",
    "why",
    "workspace",
];

/// How the composition root may observe one typed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandPolicy {
    /// Constant metadata such as the binary version needs no observer.
    SilentMetadata,
    /// Bounded metadata whose normal output is sufficient.
    FastMetadata,
    /// The command already owns a truthful overall progress task.
    ExistingRoot,
    /// A finite invocation gets one plain stderr liveness fallback.
    FiniteFallback(&'static str),
    /// A TUI or other interface owns the terminal until it exits.
    Interactive,
    /// A long-lived server or stdio protocol owns its streams.
    PersistentProtocol,
    /// A child process owns inherited terminal streams and exit status.
    TerminalPassthrough,
}

impl CommandPolicy {
    /// Selects the renderer without consulting ambient process state.
    pub(crate) fn progress_mode(
        self,
        stderr_interactive: bool,
        explicitly_disabled: bool,
    ) -> ProgressMode {
        if explicitly_disabled {
            return ProgressMode::Disabled;
        }
        match self {
            Self::ExistingRoot if stderr_interactive => ProgressMode::Interactive,
            Self::ExistingRoot
            | Self::FiniteFallback(_)
            | Self::Interactive
            | Self::PersistentProtocol
            | Self::TerminalPassthrough => ProgressMode::Plain,
            Self::FastMetadata | Self::SilentMetadata => ProgressMode::Disabled,
        }
    }

    fn fallback_label(self) -> Option<&'static str> {
        match self {
            Self::FiniteFallback(label) => Some(label),
            _ => None,
        }
    }
}

/// Select a safe policy from typed command variants only.
///
/// `terminal_attached` is supplied by the composition root so the default
/// `tree` route can distinguish its TUI from its redirected plain rendering.
pub(crate) fn policy(command: &Command, terminal_attached: bool) -> CommandPolicy {
    use CommandPolicy::{
        ExistingRoot, FastMetadata, FiniteFallback, Interactive, PersistentProtocol,
        SilentMetadata, TerminalPassthrough,
    };

    match command {
        Command::Init(_) => FiniteFallback("Command init"),
        Command::List(_) => FiniteFallback("Command list"),
        Command::Extensions(args) => match &args.command {
            None => FastMetadata,
            Some(ExtensionsCommand::Compile(_)) => FiniteFallback("Command extensions compile"),
            Some(ExtensionsCommand::Analyze(_)) => FiniteFallback("Command extensions analyze"),
        },
        Command::Validate(_) => FiniteFallback("Command validate"),
        Command::Install(args) if !args.global => ExistingRoot,
        Command::Install(_) => FiniteFallback("Command global install"),
        Command::Generate(_) => FiniteFallback("Command generate"),
        Command::Build(_) => FiniteFallback("Command build"),
        Command::Test(_) => FiniteFallback("Command test"),
        Command::Create(_) => FiniteFallback("Command create"),
        Command::Verify(_) => FiniteFallback("Command verify"),
        Command::Package(_) => FiniteFallback("Command package"),
        Command::Deploy(_) => FiniteFallback("Command deploy"),
        Command::Undeploy(_) => FiniteFallback("Command undeploy"),
        Command::Deployments => FiniteFallback("Command deployments"),
        Command::Clean(_) => FiniteFallback("Command clean"),
        Command::Scrape(args) => match &args.command {
            None => FiniteFallback("Command scrape"),
            Some(ScrapeCommand::Contract(contract)) => match &contract.command {
                ScrapeContractCommand::Init(_) => FiniteFallback("Command scrape contract init"),
                ScrapeContractCommand::Check(_) => FiniteFallback("Command scrape contract check"),
            },
        },
        Command::Outdated(_) => FiniteFallback("Command outdated"),
        Command::Search(_) => FiniteFallback("Command search"),
        Command::Mcp(args) => match &args.command {
            McpSubcommand::Serve(_) => PersistentProtocol,
            McpSubcommand::Install(_) => FiniteFallback("Command mcp install"),
            McpSubcommand::Status(_) => FiniteFallback("Command mcp status"),
            McpSubcommand::Upgrade(_) => FiniteFallback("Command mcp upgrade"),
            McpSubcommand::Uninstall(_) => FiniteFallback("Command mcp uninstall"),
        },
        Command::Aiui(args) => match &args.command {
            AiuiSubcommand::Render(_) => FiniteFallback("Command aiui render"),
            AiuiSubcommand::State(_) => FiniteFallback("Command aiui state"),
            AiuiSubcommand::Open(_) => FiniteFallback("Command aiui open"),
            AiuiSubcommand::Send(_) => FiniteFallback("Command aiui send"),
            AiuiSubcommand::Snapshot(_) => FiniteFallback("Command aiui snapshot"),
            AiuiSubcommand::Wait(_) => FiniteFallback("Command aiui wait"),
            AiuiSubcommand::Close(_) => FiniteFallback("Command aiui close"),
            AiuiSubcommand::Inspect(_) => FiniteFallback("Command aiui inspect"),
            AiuiSubcommand::PtyStop(_) => FiniteFallback("Command aiui pty-stop"),
            AiuiSubcommand::PtyStart(_) => FiniteFallback("Command aiui pty-start"),
            AiuiSubcommand::Scrollbar(_) => FiniteFallback("Command aiui scrollbar"),
        },
        Command::Term(_) | Command::Frame(_) => TerminalPassthrough,
        Command::Skill(args) => match &args.command {
            SkillSubcommand::List(_) => FastMetadata,
            SkillSubcommand::Install(_) => FiniteFallback("Command skill install"),
            SkillSubcommand::Uninstall(_) => FiniteFallback("Command skill uninstall"),
        },
        Command::Agentic(args) => match &args.command {
            AgenticSubcommand::Explain(_) => FiniteFallback("Command agentic explain"),
        },
        Command::Drain(_) => FiniteFallback("Command command"),
        Command::Uninstall(_) => FiniteFallback("Command uninstall"),
        Command::Update(_) => FiniteFallback("Command update"),
        Command::Reinstall(_) => FiniteFallback("Command reinstall"),
        Command::Check(_) => FiniteFallback("Command check"),
        Command::Doc(args) => match &args.command {
            DocCommand::Build(_) => FiniteFallback("Command doc build"),
            DocCommand::BuildSite(_) => FiniteFallback("Command doc build-site"),
            DocCommand::Check(_) => FiniteFallback("Command doc check"),
            DocCommand::Manifest(_) => FastMetadata,
            DocCommand::Serve(args) if args.print_shell => {
                FiniteFallback("Command doc serve --print-shell")
            }
            DocCommand::Serve(_) => PersistentProtocol,
            DocCommand::Shell(args) => match &args.command {
                None | Some(DocShellCommand::Status(_)) => FastMetadata,
                Some(DocShellCommand::Install(_)) => FiniteFallback("Command doc shell install"),
            },
            DocCommand::Surface(_) => FiniteFallback("Command doc surface"),
            DocCommand::Diff(_) => FiniteFallback("Command doc diff"),
            DocCommand::Todo(_) => FiniteFallback("Command doc todo"),
        },
        Command::Facts(args) => match &args.command {
            FactsSubcommand::Check(_)
            | FactsSubcommand::List { .. }
            | FactsSubcommand::Get { .. }
            | FactsSubcommand::Set { .. }
            | FactsSubcommand::Rm { .. }
            | FactsSubcommand::Sync { .. }
            | FactsSubcommand::Adopt { .. }
            | FactsSubcommand::Clean { .. }
            | FactsSubcommand::Report { .. } => FiniteFallback("Command facts"),
        },
        Command::Requirements(_) => FiniteFallback("Command requirements"),
        Command::Why(_) => FiniteFallback("Command why"),
        Command::Refactor(args) => match &args.command {
            RefactorCommand::ConvertSource(_)
            | RefactorCommand::ConvertPackageSrc(_)
            | RefactorCommand::ConvertSpecSrc(_) => FiniteFallback("Command refactor"),
        },
        Command::Friends(_) => FiniteFallback("Command friends"),
        Command::Show(args) => match &args.command {
            ShowSubcommand::Effective(_)
            | ShowSubcommand::Config(_)
            | ShowSubcommand::Features(_)
            | ShowSubcommand::Subskills(_)
            | ShowSubcommand::Purls(_)
            | ShowSubcommand::SourcePath(_) => FiniteFallback("Command show"),
        },
        Command::Prefs(args) => match &args.command {
            PrefsSubcommand::Ui(_) => Interactive,
            PrefsSubcommand::Set(_) => FiniteFallback("Command prefs set"),
            PrefsSubcommand::Migrate(_) => FiniteFallback("Command prefs migrate"),
            PrefsSubcommand::Get(_)
            | PrefsSubcommand::List(_)
            | PrefsSubcommand::Check(_)
            | PrefsSubcommand::ShowOrigins(_) => FastMetadata,
        },
        Command::Tree(args) if args.terminal && !args.plain => TerminalPassthrough,
        Command::Tree(args) if args.plain || !terminal_attached => FiniteFallback("Command tree"),
        Command::Tree(_) => Interactive,
        Command::Registry(args) => match &args.command {
            RegistrySubcommand::Sync(_)
            | RegistrySubcommand::Publish(_)
            | RegistrySubcommand::List(_)
            | RegistrySubcommand::Add(_)
            | RegistrySubcommand::SetMirror(_)
            | RegistrySubcommand::Test(_)
            | RegistrySubcommand::Redirect(_)
            | RegistrySubcommand::RedirectSync(_)
            | RegistrySubcommand::RedirectUpdate(_)
            | RegistrySubcommand::Vendor(_) => FiniteFallback("Command registry"),
            RegistrySubcommand::Remove(remove) => match &remove.target {
                RegistryRemoveTarget::Registry(_) | RegistryRemoveTarget::Mirror(_) => {
                    FiniteFallback("Command registry")
                }
            },
        },
        Command::Cache(args) => match &args.command {
            CacheSubcommand::Path
            | CacheSubcommand::List
            | CacheSubcommand::Add(_)
            | CacheSubcommand::Clean(_)
            | CacheSubcommand::Check(_) => FiniteFallback("Command cache"),
        },
        Command::Workspace(args) => match &args.command {
            WorkspaceSubcommand::Publish(_) => FiniteFallback("Command workspace publish"),
        },
        Command::Vvm(args) => match &args.command {
            VvmSubcommand::Install(_)
            | VvmSubcommand::Bootstrap(_)
            | VvmSubcommand::Update(_)
            | VvmSubcommand::Reinstall(_)
            | VvmSubcommand::Gc(_) => ExistingRoot,
            VvmSubcommand::Import(_) => FiniteFallback("Command self import"),
            VvmSubcommand::Use(_) => FiniteFallback("Command self use"),
            VvmSubcommand::Rollback => FiniteFallback("Command self rollback"),
            VvmSubcommand::Doctor(_) => FiniteFallback("Command self doctor"),
            VvmSubcommand::Remove(_) => FiniteFallback("Command self remove"),
            VvmSubcommand::Relocate(_) => FiniteFallback("Command self relocate"),
            VvmSubcommand::Ls
            | VvmSubcommand::Current
            | VvmSubcommand::Which(_)
            | VvmSubcommand::Source
            | VvmSubcommand::Env(_) => FastMetadata,
        },
        Command::Tools { .. } => FiniteFallback("Command tools"),
        Command::Bin { cmd } => match cmd {
            BinCmd::List => FiniteFallback("Command bin list"),
            BinCmd::Path { .. } => FiniteFallback("Command bin path"),
            BinCmd::Build { .. } => FiniteFallback("Command bin build"),
            BinCmd::Exec { .. } => TerminalPassthrough,
        },
        Command::Explain(_) => FiniteFallback("Command explain"),
        Command::Query(_) => FiniteFallback("Command query"),
        Command::Select(_) => FiniteFallback("Command select"),
        Command::Specmap(_) => FiniteFallback("Command specmap"),
        Command::Trace { .. } => TerminalPassthrough,
        Command::Vars(_) => FastMetadata,
        Command::Progress(args) => match &args.command {
            ProgressSubcommand::Scan(_)
            | ProgressSubcommand::Check(_)
            | ProgressSubcommand::Report(_)
            | ProgressSubcommand::Mirror(_)
            | ProgressSubcommand::Weave(_)
            | ProgressSubcommand::Rescan(_)
            | ProgressSubcommand::Baseline(_)
            | ProgressSubcommand::Resume(_)
            | ProgressSubcommand::Gate(_)
            | ProgressSubcommand::Seal(_) => FiniteFallback("Command progress"),
        },
        Command::Version => SilentMetadata,
    }
}

/// Resolve the documented flag/environment opt-out without ambient reads.
pub(crate) fn progress_disabled(flag: bool, environment: Option<&str>) -> bool {
    flag || environment.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Configure the invocation renderer and establish any finite fallback root.
/// Ambient terminal/environment reads are resolved by `main` and arrive here
/// only as values.
pub(crate) fn configure_context(
    root: Context,
    command: &Command,
    verbose: bool,
    stdout_interactive: bool,
    stderr_interactive: bool,
    disable_flag: bool,
    disable_environment: Option<&str>,
) -> (Context, CommandActivity) {
    let policy = policy(command, stdout_interactive);
    let disabled = progress_disabled(disable_flag, disable_environment);
    let root = root.with_progress(verbose, policy.progress_mode(stderr_interactive, disabled));
    let activity = CommandActivity::start(&root, policy);
    let scoped = activity.scoped_context(&root);
    (scoped, activity)
}

/// Owns the generic finite-command root until dispatch returns.
pub(crate) struct CommandActivity {
    task: Option<ProgressTask>,
}

impl CommandActivity {
    pub(crate) fn start(ctx: &Context, policy: CommandPolicy) -> Self {
        Self::start_with(&ctx.progress(), policy)
    }

    fn start_with(progress: &Progress, policy: CommandPolicy) -> Self {
        let task = policy.fallback_label().map(|label| progress.task(label));
        Self { task }
    }

    /// Put command-specific phases beneath the generic finite fallback.
    pub(crate) fn scoped_context(&self, ctx: &Context) -> Context {
        self.task.as_ref().map_or_else(
            || ctx.clone(),
            |task| ctx.with_progress_scope(task.progress()),
        )
    }

    /// Terminalize before ordinary error rendering starts.
    pub(crate) fn complete(&self, succeeded: bool) {
        if let Some(task) = &self.task {
            if succeeded {
                task.finish();
            } else {
                task.fail("command failed");
            }
        }
    }
}

#[cfg(test)]
#[path = "command_progress/tests.rs"]
mod tests;
