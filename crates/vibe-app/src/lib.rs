//! User applications (PROP-059): a package of kind `app` describes how to
//! install an upstream application per platform, how to launch it and which
//! commands it puts on PATH. This crate owns every capability — settings,
//! storage libraries, the index, resolution, selection, installers, the
//! install / update / uninstall operations, launch, slot alternatives, PATH
//! proxy synchronisation, session choices and the `vibe storage` library
//! edits, moves and sweeps — and the CLI and the PATH proxy stay thin callers.
//!
//! Durable state is plain files replaced atomically: one index, one
//! `storage.toml`, library markers and directories. An interrupted operation
//! leaves at most an orphan directory.
//!
//! ```
//! use vibe_app::{Index, SettingsRoot, list};
//!
//! let dir = tempfile::tempdir().unwrap();
//! let settings = SettingsRoot::new(dir.path());
//! assert!(Index::load(&settings).unwrap().installations.is_empty());
//! assert!(list(&settings).unwrap().installations.is_empty());
//! ```

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#library-first");

mod agents;
mod alternatives;
mod catalog;
mod codes;
mod derive;
mod describe;
mod desktop;
mod environment;
mod error;
mod error_catalog;
mod error_lifecycle;
mod export;
mod exposure;
mod fs_util;
mod icons;
mod index;
mod installer;
mod launch;
mod operations;
mod portable;
mod proxies;
mod resolve;
mod select;
mod session;
mod settings;
mod steps;
mod storage;

pub use agents::{
    AgentDefinition, AgentError, AgentHooks, AgentNotice, AgentPrompt, AgentSource, AgentsConfig,
    ConsentPrompt, Delivery, ExternalAgent, Invocation, PLACEHOLDERS, ParkedStep, Placeholders,
    Preset, ResolvedAgent, RunMode, SelectedAgent, agent_list, preset as agent_preset,
    preset_table as agent_preset_table, presets as agent_presets, select_agent,
};
pub use alternatives::{
    DefaultChange, listing as alternatives_listing, reserved_commands, set_default,
};
pub use catalog::{
    ApplicationEvent, EVENTS_PROTOCOL, EventKind, EventTail, HomeEvent, LAUNCHER_KEY_SEPARATOR,
    LauncherCatalog, LauncherChange, LauncherUsage, PinTarget, ProcessState, ProcessTable,
    RUNNING_PROTOCOL, StartRequest, USER_LAUNCHERS_PROTOCOL, Usage, UserCollection, UserLauncher,
    UserLaunchers, UserPins, WatchRequest, bump_revision, collection_add as app_collection_add,
    collection_create as app_collection_create, collection_delete as app_collection_delete,
    collection_order as app_collection_order, collection_remove as app_collection_remove,
    collection_rename as app_collection_rename, collections, drop_package, launcher_key,
    launcher_override, pin as app_pin, record_event, revision as catalog_revision,
    revision_document, running, seed_package, split_launcher_key, start as start_launcher,
    stop_launcher, unpin as app_unpin, watch,
};
pub use codes::{ErrorClass, ErrorCode, error_codes};
pub use derive::{derive_launchers, hicolor as hicolor_icon_set, import_desktop};
pub use describe::describe;
pub use desktop::{DesktopFile, exec_words};
pub use environment::{ProfileEnvironment, UserEnvironment, recorded_environment};
pub use error::{
    AppError, InstallerError, LaunchError, ResolveError, StateError, StepError, StorageError,
};
pub use error_catalog::CatalogError;
pub use error_lifecycle::LifecycleError;
pub use export::{
    ApplicationExported, EXPORTED_PROTOCOL, ExportRequest, SystemPlatform, SystemTargets,
    export_system, remove_system,
};
pub use exposure::{Exposure, expose_for_session};
pub use icons::{ResolvedIcon, ResolvedIconFile};
pub use index::{
    AcceptedLicense, ApplicationMenu, AuthoredCommands, EnvEntry, EnvOp, INDEX_PROTOCOL, Index,
    Installation, Lifecycle, Location, Mark, Method, PackageCoordinate, Requirement, StepOutcome,
    StepRecord, ValueKind,
};
pub use installer::programs::{windows_cmd, windows_powershell, windows_system_program};
pub use installer::{Interrupt, ServedBy, ServedRow, StepLogs, StepPanel};
pub use launch::{
    LaunchRequest, ResolvedLauncher, TemplateVars, app_root, exit_code, expand_template,
    icon_choice, launch_arguments, listing as launcher_listing,
    listing_all as launcher_listing_all, resolve as resolve_launcher, run as run_launcher,
};
pub use operations::{
    Change, ChangeVerb, Command, InstallRequest, ModifyRequest, OperationContext, Outcome,
    PackageResolver, Removal, RemoveRequest, autoremove, expose, hide, hold, install, installed,
    list, mark, modify, outdated, plan, remove_unrecorded, rollback, uninstall, update,
};
pub use portable::{
    APPS_PROTOCOL, IMPORT_PROTOCOL, ImportAction, ImportInstall, PortableExport, import_action,
    parse_portable, portable_set, portable_toml, restore_arrangement,
};
pub use proxies::{proxy_file_name, sync as sync_proxies};
pub use resolve::{
    ResolvedPackage, from_global, from_local_registry, from_manifest, local_registry_carries,
    qualified_ref,
};
pub use select::{
    Choice, Chooser, InstallChoice, detected_arch, detected_platform, options_listing,
    select_option,
};
pub use session::{SESSION_VARIABLE, SessionChoices};
pub use settings::{MutationLock, PROJECT_HOME, SettingsRoot};
pub use steps::{
    FeatureChooser, FeatureRequest, FeatureResolution, HostFacts, Implied, ImpliedRule,
    LicensePrompt, ParamValue, ParameterPrompt, Plan, PlanLicense, PlannedDependency, PlannedStep,
    RequiredBy, Start, clean_work, human_bytes, prune_runs, resolve_features, runs, step_env_name,
    work,
};
pub use storage::{
    Availability, DEFAULT_LIBRARY, LIBRARY_MARKER, Library, LibraryMarker, Moved, Storage,
    StorageConfig, add as storage_add, attach as storage_attach, create_library,
    detach as storage_detach, gc as storage_gc, listing as storage_list, relocate as storage_move,
};
