//! `vibe install|update|uninstall -g` for user applications: argument
//! checks, package resolution and rendering around the `vibe-app` library.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_app::{
    FeatureRequest, HostFacts, InstallChoice, InstallRequest, Interrupt, OperationContext, Plan,
    RemoveRequest, ResolvedPackage, SettingsRoot, Start, UserEnvironment,
};
use vibe_core::progress::Progress;

use super::agents::{AgentSetup, report_parked};
use super::display::{StepDisplay, asking};
use super::interrupt::Signals;
use super::report::{render, render_removal};
use super::wizard::{chooser, feature_chooser, license_prompt, parameter_prompt};
use crate::cli::{AppChoiceArgs, AppStepArgs, InstallArgs, UninstallArgs, UpdateArgs};
use crate::output;

/// The current user's settings root, resolved once at this boundary, with
/// the install root store its applications follow.
pub(crate) fn settings_root() -> Result<SettingsRoot> {
    let root = vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "the Vibe settings root is unavailable; set VIBE_SETTINGS or make the user home resolvable"
        )
    })?;
    let settings = SettingsRoot::new(&root);
    // Applications follow the install root vibe itself lives under, so
    // their commands sit beside vibe's shims (PROP-019 `#install-root`).
    Ok(match crate::composition::app_install_store() {
        Some(store) => settings.with_install_root(&store),
        None => settings,
    })
}

/// The `vibe-app-proxy` image shipped beside the running `vibe` executable;
/// `vibe-app` copies it into `opt/bin` for every reserved command.
pub(crate) fn proxy_image() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("locating the running vibe executable")?;
    Ok(exe.with_file_name(crate::commands::vvm::APP_PROXY_BINARY_NAME))
}

pub(crate) fn source_operation(
    invocation: vibe_core::manifest::SourcePolicyDecl,
) -> Result<vibe_package_source::SourceOperation> {
    source_operation_for_project(None, invocation)
}

/// The caller supplies the already selected project, never inferred from app-home paths.
pub(crate) fn source_operation_for_project(
    project_root: Option<&Path>,
    invocation: vibe_core::manifest::SourcePolicyDecl,
) -> Result<vibe_package_source::SourceOperation> {
    let manifest = match project_root {
        Some(root) => vibe_core::manifest::Manifest::read(root.join("vibe.toml"))?,
        None => vibe_core::manifest::Manifest::default(),
    };
    let cwd = std::env::current_dir()?;
    vibe_package_source::source_operation(
        &vibe_package_source::PackageSourceOptions {
            source_policy: invocation,
            ..Default::default()
        },
        &manifest,
        project_root.unwrap_or(&cwd),
        vibe_package_source::deferred_user_registry(crate::output::seeds_default_registry()),
    )
}

pub(crate) fn resolve_package_with_operation(
    settings: &SettingsRoot,
    registry: Option<&Path>,
    embedded_root: Option<&Path>,
    spelling: &str,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<ResolvedPackage> {
    if let Some(root) = registry {
        let local = open_local(root)?;
        return Ok(vibe_app::from_local_registry(&local, root, spelling)
            .map_err(vibe_app::AppError::from)?);
    }
    if let Some(root) = embedded_root {
        let local = open_local(root)?;
        if vibe_app::local_registry_carries(&local, spelling)? {
            let mut resolved = vibe_app::from_local_registry(&local, root, spelling)
                .map_err(vibe_app::AppError::from)?;
            if let Some(evidence) = &mut resolved.source_evidence {
                evidence.source_kind =
                    Some(vibe_wire::generated::shared::SourceSelectionKind::Embedded);
            }
            return Ok(resolved);
        }
    }
    Ok(
        vibe_app::from_global_with_operation(settings.path(), spelling, offline, operation)
            .map_err(vibe_app::AppError::from)?,
    )
}

fn open_local(root: &Path) -> Result<vibe_registry::LocalRegistry> {
    crate::registry::application_local_registry(root.to_path_buf())
        .with_context(|| format!("opening the application registry `{}`", root.display()))
}

#[cfg(test)]
#[path = "global/source_tests.rs"]
mod source_tests;

/// The package resolver of one command: the same explicit (or embedded)
/// registry as the requested package, otherwise the configured registries
/// (PROP-065 `#app-dependencies`: requirements resolve like any package).
pub(crate) struct Packages {
    pub(crate) operation: vibe_package_source::SourceOperation,
    pub(crate) settings: SettingsRoot,
    pub(crate) registry: Option<PathBuf>,
    pub(crate) embedded: Option<PathBuf>,
    pub(crate) offline: bool,
}

impl Packages {
    pub(crate) fn scope(&self) -> vibe_app::ResolverScope<'_> {
        match self.registry.as_deref() {
            Some(root) => vibe_app::ResolverScope::Registry(root),
            None => vibe_app::ResolverScope::Chain {
                embedded: self.embedded.as_deref(),
            },
        }
    }
    pub(crate) fn resolve(&self, spelling: &str) -> Result<ResolvedPackage, vibe_app::AppError> {
        resolve_package_with_operation(
            &self.settings,
            self.registry.as_deref(),
            self.embedded.as_deref(),
            spelling,
            self.offline,
            &self.operation,
        )
        .map_err(|error| match error.downcast::<vibe_app::AppError>() {
            Ok(error) => error,
            Err(error) => vibe_app::ResolveError::Registry {
                spelling: spelling.into(),
                reason: format!("{error:#}"),
            }
            .into(),
        })
    }
}

pub(super) fn attended(ctx: &output::Context) -> bool {
    !ctx.is_json() && !ctx.is_unattended() && console::user_attended()
}

/// PATH and environment facts for step conditions and needs.
pub(crate) fn host_facts() -> HostFacts<'static> {
    HostFacts {
        command_present: &super::facts::command_on_path,
        env_present: &super::facts::env_set,
        resolve_path: &super::facts::absolute_path,
    }
}

/// A run's one-line notice printer (PROP-065 `#work`).
type Notice = Box<dyn Fn(&str)>;

/// What one operation observes: the progress handle (the machine stream
/// only; the human renderer keeps showing child output as before) and the
/// caught signals of a mutating command.
pub(crate) struct Observation<'d> {
    pub(crate) http_client: vibe_package_source::OperationHttpClient,
    pub(crate) progress: Progress,
    pub(crate) signals: Option<Signals>,
    /// The durable user environment env steps change (PROP-065 `#env-step`).
    pub(crate) environment: Option<Box<dyn UserEnvironment>>,
    /// The caller's PATH, behind the requirements a step sees first.
    pub(crate) search_path: Option<std::ffi::OsString>,
    /// One-line run notices on stderr, so `--json` stays one envelope
    /// (PROP-065 `#work`); absent when the caller asked for quiet.
    notice: Option<Notice>,
    /// What this run shows of its children (PROP-065 `#display`); the flat
    /// display of a command that runs no panel.
    display: &'d StepDisplay,
}

impl<'d> Observation<'d> {
    /// `mutating` operations catch Ctrl-C / SIGTERM (PROP-065 `#cancellation`).
    pub(crate) fn new(
        ctx: &output::Context,
        mutating: bool,
        display: &'d StepDisplay,
    ) -> Result<Self> {
        Ok(Self {
            http_client: source_operation(Default::default())?
                .http_client(std::time::Duration::from_secs(600)),
            // Every human renderer draws the steps as blocks (PROP-067
            // `#collapse`, `#plain`); a run with none observes nothing but
            // the machine stream.
            progress: if ctx.is_machine_progress() || ctx.draws_progress() {
                ctx.progress()
            } else {
                // Unrendered, still journaled (`PROP-068#journal`).
                Progress::default()
                    .with_transfers(ctx.transfers())
                    .with_cancellation(ctx.progress().cancellation().clone())
            },
            signals: if mutating {
                Some(Signals::install().context("catching interrupt signals")?)
            } else {
                None
            },
            environment: super::environment::for_host(
                settings_root()?.path(),
                super::facts::user_home().as_deref(),
            ),
            search_path: super::facts::search_path(),
            notice: (!ctx.is_quiet())
                .then(|| Box::new(|text: &str| asking(|| notice(text))) as Notice),
            display,
        })
    }

    pub(crate) fn with_source_operation(
        mut self,
        operation: &vibe_package_source::SourceOperation,
    ) -> Self {
        self.http_client = operation.http_client(std::time::Duration::from_secs(600));
        self
    }

    fn notice(&self) -> Option<&dyn Fn(&str)> {
        self.notice.as_deref()
    }

    fn interrupt(&self) -> Option<&dyn Interrupt> {
        self.signals
            .as_ref()
            .map(|signals| signals as &dyn Interrupt)
    }
}

/// A run notice: a dim line between blocks.
fn notice(text: &str) {
    use output::language::{Kind, Role, Stream, before, paint};
    eprintln!(
        "{}{}",
        before(Stream::Err, Kind::Block),
        paint(Role::Dim, text)
    );
}

/// Every application mutation's context: attended prompts only in an
/// attended terminal, the plan shown through `show`.
pub(super) fn operation<'a>(
    ctx: &output::Context,
    settings: &'a SettingsRoot,
    offline: bool,
    proxy_image: &'a Path,
    show: Option<&'a dyn Fn(&Plan)>,
    observe: &'a Observation<'a>,
) -> OperationContext<'a> {
    let attended = attended(ctx);
    OperationContext {
        http_client: Some(&observe.http_client),
        import_scope: None,
        settings,
        json: ctx.is_json(),
        offline,
        chooser: attended.then_some(&chooser as vibe_app::Chooser<'_>),
        proxy_image,
        facts: host_facts(),
        feature_chooser: attended.then_some(&feature_chooser as vibe_app::FeatureChooser<'_>),
        parameter_prompt: attended.then_some(&parameter_prompt as vibe_app::ParameterPrompt<'_>),
        license_prompt: attended.then_some(&license_prompt as vibe_app::LicensePrompt<'_>),
        on_plan: show,
        on_notice: observe.notice(),
        progress: &observe.progress,
        quiet_children: ctx.is_machine_progress() || observe.display.quiet_children(),
        panel: observe.display.panel(),
        on_step_log: Some(observe.display as &dyn vibe_app::StepLogs),
        interrupt: observe.interrupt(),
        agent: vibe_app::AgentHooks::default(),
        resolver: None,
        resolver_scope: vibe_app::ResolverScope::Caller,
        source_reader: Some(&super::project::read_saved_source as vibe_app::SavedSourceReader<'_>),
        environment: observe.environment.as_deref(),
        search_path: observe.search_path.as_deref(),
    }
}

/// Print the plan before the first step runs (stderr, so `--json` stays
/// one envelope and still shows the plan and its warnings to whoever
/// watches); the machine stream carries no prose.
pub(super) fn plan_printer(ctx: &output::Context) -> impl Fn(&Plan) + '_ {
    use output::language::{Kind, Stream, before};
    move |plan: &Plan| {
        if !ctx.is_quiet() && !ctx.is_machine_progress() {
            let text = super::plan_text::render(plan, ctx.invoked_by());
            asking(|| eprintln!("{}{text}", before(Stream::Err, Kind::Block)));
        }
    }
}

/// The selection, wizard answers and resume point of an install or update.
pub(super) fn request(
    choice: InstallChoice,
    library: Option<String>,
    force: bool,
    steps: &AppStepArgs,
) -> Result<InstallRequest> {
    let start = match (&steps.from_step, steps.resume) {
        (Some(step), _) => Start::FromStep(step.clone()),
        (None, true) => Start::Resume,
        (None, false) => Start::Fresh,
    };
    Ok(InstallRequest {
        library,
        force,
        start,
        keep_work: steps.keep_work,
        ..plan_request(choice, &steps.choice)?
    })
}

/// The selection and wizard answers of `vibe app plan`.
pub(super) fn plan_request(
    choice: InstallChoice,
    answers: &AppChoiceArgs,
) -> Result<InstallRequest> {
    Ok(InstallRequest {
        transfer: None,
        configuration: super::configuration::read(answers)?,
        choice,
        library: None,
        force: false,
        features: FeatureRequest {
            enable: answers.feature.clone(),
            disable: answers.no_feature.clone(),
            profile: answers.profile.clone(),
        },
        parameters: answers.set.clone(),
        accept_license: answers.accept_license,
        start: Start::Fresh,
        ..InstallRequest::default()
    })
}

/// `vibe app plan` and `install --plan-only`: JSON or text on stdout.
pub(crate) fn render_plan(ctx: &output::Context, plan: &Plan) -> Result<()> {
    if ctx.is_json() {
        let mut wire = plan.to_wire()?;
        let mut hostings = crate::commands::install::report::hostings::Hostings::default();
        hostings.selected(
            &format!("{}/{}", wire.package.group, wire.package.name),
            wire.source_evidence.as_ref(),
        );
        wire.hostings = Some(hostings.finish());
        return ctx.emit_json(&wire);
    }
    ctx.block(&super::plan_text::render(plan, ctx.invoked_by()));
    Ok(())
}

pub fn install(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    if args.packages.len() != 1 {
        bail!("global install requires exactly one fully qualified application package");
    }
    if args.path != Path::new(".")
        || args.language.is_some()
        || !args.features.is_empty()
        || args.no_default_features
        || args.all_features
        || args.exact
        || args.auth_required
        || args.solver.is_some()
        || args.prefer_embedded
        || args.no_prefer_embedded
        || args.no_default_registry
        || args.embedded_short_circuit
        || args.prefer_local
        || args.no_prefer_local
        || args.git.is_some()
        || args.tag.is_some()
        || args.branch.is_some()
        || args.rev.is_some()
        || args.git_auth.is_some()
        || args.git_token_env.is_some()
        || args.trace_compile
    {
        bail!("global install received project-only package flags");
    }
    let offline = root_offline || args.offline;
    let settings = settings_root()?;
    let proxy_image = proxy_image()?;
    let source_operation = source_operation(args.source_policy.declaration())?;
    let resolved = resolve_package_with_operation(
        &settings,
        args.registry.as_deref(),
        embedded_root.as_deref(),
        &args.packages[0],
        offline,
        &source_operation,
    )?;
    let choice = InstallChoice {
        platform: args.platform,
        arch: args.arch,
        installer: args.installer,
    };
    let request = InstallRequest {
        hidden: args.hidden,
        ..request(choice, args.library, args.force, &args.steps)?
    };
    let agents = AgentSetup::new(ctx, args.agent.as_deref(), args.assume_yes)?;
    let packages = Packages {
        operation: source_operation.clone(),
        settings: settings.clone(),
        registry: args.registry.clone(),
        embedded: embedded_root.clone(),
        offline,
    };
    let resolver = |spelling: &str| packages.resolve(spelling);
    if args.plan_only {
        let flat = StepDisplay::flat();
        let observe =
            Observation::new(ctx, false, &flat)?.with_source_operation(&packages.operation);
        let operation = OperationContext {
            agent: agents.hooks(),
            resolver: Some(&resolver),
            resolver_scope: packages.scope(),
            ..operation(ctx, &settings, offline, &proxy_image, None, &observe)
        };
        let plan = vibe_app::plan(&operation, &resolved, &request)?;
        return render_plan(ctx, &plan);
    }
    let show = plan_printer(ctx);
    let display = StepDisplay::open(ctx, &args.steps);
    let observe = Observation::new(ctx, true, &display)?.with_source_operation(&packages.operation);
    let outcome = display.around(ctx, || {
        let operation = OperationContext {
            agent: agents.hooks(),
            resolver: Some(&resolver),
            resolver_scope: packages.scope(),
            ..operation(ctx, &settings, offline, &proxy_image, Some(&show), &observe)
        };
        report_parked(ctx, vibe_app::install(&operation, &resolved, &request))
    });
    let outcome = display.report(ctx, outcome)?;
    render(ctx, &outcome)?;
    // The slot may have gained a version whose package declares state to
    // carry; the offer is one line and never migrates (PROP-075
    // `#MIGRATION-FAILURE`).
    super::migrate::offer(ctx, &settings, &outcome.slot, &outcome.key);
    Ok(())
}

pub fn update(
    ctx: &output::Context,
    args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    if args.all && args.packages.is_empty() {
        return super::lifecycle::update_all(ctx, args, embedded_root, root_offline);
    }
    if args.packages.len() != 1 || args.all {
        bail!("global update requires exactly one fully qualified application package, or --all");
    }
    if args.path != Path::new(".") || args.exact || args.auth_required || args.trace_compile {
        bail!("global update received project-only package flags");
    }
    let settings = settings_root()?;
    let source_operation = source_operation(args.source_policy.declaration())?;
    let resolved = resolve_package_with_operation(
        &settings,
        args.registry.as_deref(),
        embedded_root.as_deref(),
        &args.packages[0],
        root_offline,
        &source_operation,
    )?;
    let outcome = update_one(
        ctx,
        &args,
        embedded_root,
        root_offline,
        &resolved,
        &source_operation,
    )?;
    render(ctx, &outcome)?;
    super::migrate::offer(ctx, &settings, &outcome.slot, &outcome.key);
    Ok(())
}

/// One application update with the flags of `args` (PROP-065
/// `#update-and-rollback`): requirements resolve through the same registry.
pub(crate) fn update_one(
    ctx: &output::Context,
    args: &UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
    resolved: &ResolvedPackage,
    source_operation: &vibe_package_source::SourceOperation,
) -> Result<vibe_app::Outcome> {
    let settings = settings_root()?;
    let proxy_image = proxy_image()?;
    let choice = InstallChoice {
        platform: args.platform.clone(),
        arch: args.arch.clone(),
        installer: args.installer.clone(),
    };
    let request = InstallRequest {
        purge_previous: args.purge_previous,
        ..request(choice, args.library.clone(), args.force, &args.steps)?
    };
    let agents = AgentSetup::new(ctx, args.agent.as_deref(), args.assume_yes)?;
    let packages = Packages {
        operation: source_operation.clone(),
        settings: settings.clone(),
        registry: args.registry.clone(),
        embedded: embedded_root,
        offline: root_offline,
    };
    let resolver = |spelling: &str| packages.resolve(spelling);
    let show = plan_printer(ctx);
    let display = StepDisplay::open(ctx, &args.steps);
    let observe = Observation::new(ctx, true, &display)?.with_source_operation(&packages.operation);
    let outcome = display.around(ctx, || {
        let operation = OperationContext {
            agent: agents.hooks(),
            resolver: Some(&resolver),
            resolver_scope: packages.scope(),
            ..operation(
                ctx,
                &settings,
                root_offline,
                &proxy_image,
                Some(&show),
                &observe,
            )
        };
        report_parked(ctx, vibe_app::update(&operation, resolved, &request))
    });
    display.report(ctx, outcome).map_err(Into::into)
}

/// The tree-removal flags of `uninstall` (PROP-065 `#uninstall-tree`).
pub(crate) fn remove_request(args: &UninstallArgs) -> RemoveRequest {
    RemoveRequest {
        force: args.force,
        cascade: args.cascade,
        autoremove: args.autoremove,
        plan_only: args.plan_only,
    }
}

pub fn uninstall(ctx: &output::Context, args: UninstallArgs, root_offline: bool) -> Result<()> {
    vibe_app::qualified_ref(&args.package)?;
    if args.path != Path::new(".") {
        bail!("global uninstall received project-only --path");
    }
    let settings = settings_root()?;
    let proxy_image = proxy_image()?;
    let flat = StepDisplay::flat();
    let observe = Observation::new(ctx, true, &flat)?;
    let removal = flat.around(ctx, || {
        let operation = OperationContext {
            chooser: None,
            feature_chooser: None,
            ..operation(ctx, &settings, root_offline, &proxy_image, None, &observe)
        };
        vibe_app::uninstall(&operation, &args.package, &remove_request(&args))
    })?;
    render_removal(ctx, &removal)
}
