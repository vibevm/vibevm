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

mod activation;
mod agents;
mod alternatives;
mod catalog;
mod codes;
pub mod configuration;
mod derive;
mod describe;
mod desktop;
mod environment;
mod error;
mod error_catalog;
mod error_lifecycle;
mod error_paths;
pub use error_paths::PathsError;
mod error_versions;
pub use error_versions::VersionsError;
mod export;
mod exposure;
mod fs_util;
mod icons;
mod index;
mod installer;
mod launch;
mod migrate;
mod operations;
pub use operations::pending::command as resume_command;
mod paths;
mod portable;
mod proxies;
mod resolve;
mod select;
mod session;
mod settings;
mod steps;
mod storage;
mod versionfile;
mod versions;

pub use activation::{
    Activation, ActivationRequest, ChannelMembership, ProjectRequirement, SlotSelection,
    Source as ActivationSource, State as ActivationState, resolve as resolve_activation,
    slot_default,
};
pub use agents::{
    AgentDefinition, AgentError, AgentHooks, AgentNotice, AgentPrompt, AgentSource, AgentsConfig,
    ConsentPrompt, Delivery, ExternalAgent, Invocation, LauncherDefinition, PLACEHOLDERS,
    ParkedStep, Placeholders, Preset, ResolvedAgent, RunMode, SelectedAgent, agent_list,
    preset as agent_preset, preset_table as agent_preset_table, presets as agent_presets,
    select_agent,
};
pub use alternatives::{
    DefaultChange, listing as alternatives_listing, reserved_commands, set_default, set_slot_off,
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
pub use environment::blocks::{
    BlockMarkers, ENV_BLOCK, HOOK_BLOCK, ensure_block, has_block, managed_startup_files,
    remove_block,
};
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
    Installation, Lifecycle, Location, Mark, Method, PackageCoordinate, Requirement, SlotState,
    StepOutcome, StepRecord, ValueKind,
};
pub use installer::programs::{windows_cmd, windows_powershell, windows_system_program};
pub use installer::{
    DeclaredInput, Interrupt, ServedBy, ServedRow, StepLogs, StepPanel, fetch_declared_input,
};
pub use launch::{
    CommandSearch, LaunchRequest, ResolvedLauncher, TemplateVars, app_root, exit_code,
    expand_template, icon_choice, launch_arguments, listing as launcher_listing,
    listing_all as launcher_listing_all, resolve as resolve_launcher, run as run_launcher,
};
pub use migrate::{
    MIGRATION_PROTOCOL, MigrationPlan, MigrationRequest, kept_migration_work, migrate,
    migration_offer, migration_plan, slot_migration_work,
};
pub use operations::{
    Change, ChangeVerb, Command, InstallRequest, ModifyRequest, OperationContext, Outcome,
    PackageResolver, Removal, RemoveRequest, ResolverScope, ResumeInfo, ResumeRequest,
    SavedSourceReader, SlotExposure, autoremove, configuration_plan, expose, hide, hold,
    import_cohort, import_resume_info, install, installed, list, mark, modify, outdated, plan,
    remove_unrecorded, resume, resume_import, resume_info, rollback, uninstall, update,
};
pub use paths::{
    PATH_LINK_STEP, PathSync, SlotPaths, env_tag as slot_path_env_tag,
    path_entry as slot_path_entry, session_dirs as slot_session_dirs, slot_link_dirs,
    sync_locked as sync_slot_paths,
};
pub use portable::{
    APPS_PROTOCOL, FULL_APPS_PROTOCOL, IMPORT_PROTOCOL, ImportAction, ImportCandidate,
    ImportInstall, ImportSecrets, PortableDocument, PortableExport, PreparedImport, import_action,
    import_choice, import_license, import_row, parse_import_secrets, parse_import_target,
    parse_portable, parse_portable_document, portable_set, portable_toml, prepare_import,
    restore_arrangement,
};
pub use portable::{FullImportPreview, export_portable, preview_full_import, selected_import_keys};
pub use proxies::{proxy_file_name, sync as sync_proxies, sync_locked as sync_proxies_locked};
pub use resolve::{
    ResolvedPackage, directory_roots_from_global_with_operation, from_global,
    from_global_with_operation, from_local_registry, from_manifest, global_resolver,
    local_registry_carries, qualified_ref, versions_from_global_with_operation,
};
pub use select::{
    Choice, Chooser, InstallChoice, detected_arch, detected_platform, options_listing,
    select_option,
};
pub use session::{SESSION_OFF, SESSION_VARIABLE, SessionChoices};
pub use settings::{HOME_POINTER, MutationLock, PROJECT_HOME, SettingsRoot};
pub use steps::{
    FeatureChooser, FeatureRequest, FeatureResolution, HostFacts, Implied, ImpliedRule,
    LicensePrompt, ParamValue, ParameterPrompt, Plan, PlanLicense, PlannedDependency, PlannedStep,
    RequiredBy, Start, clean_import, clean_work, human_bytes, prune_runs, resolve_features, runs,
    step_env_name, work, work_import,
};
pub use storage::{
    Availability, DEFAULT_LIBRARY, INSTALL_ROOT_LIBRARY, LIBRARY_MARKER, Library, LibraryMarker,
    Moved, Storage, StorageConfig, add as storage_add, attach as storage_attach, create_library,
    detach as storage_detach, gc as storage_gc, listing as storage_list, relocate as storage_move,
};
pub use versionfile::{
    CACHE_PROTOCOL as VERSION_FILE_CACHE_PROTOCOL, Consulted as VersionFileConsulted,
    Declared as DeclaredVersionFiles, Found as VersionFileFound, RequestKind,
    Search as VersionFileSearch, ToolVersions, VERSION_FILES_PROTOCOL, VersionRequest,
    cache_path as version_files_cache_path, clear_cache as clear_version_file_cache,
    declared_path as declared_version_files_path, is_channel_name, plain_line,
    resolve as resolve_version_files, search as search_version_files, tool_versions,
};
pub use versions::{
    DEFAULT_LIMIT as VERSIONS_DEFAULT_LIMIT, Domain, DomainVersion, Domains, Origin, TemplateFacts,
    VERSIONS_PROTOCOL, VersionsRequest, versions_listing,
};

pub use operations::import_full;
pub use portable::{
    FullImportCandidate, FullImportPreparation, PreparedFullImport, prepare_full_import,
};

pub use operations::ImportScope;

pub use operations::{ImportResumeInfo, inspect_import_resume, resume_full_import};
