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

mod alternatives;
mod describe;
mod error;
mod fs_util;
mod index;
mod installer;
mod launch;
mod operations;
mod proxies;
mod resolve;
mod select;
mod session;
mod settings;
mod steps;
mod storage;

pub use alternatives::{
    DefaultChange, listing as alternatives_listing, reserved_commands, set_default,
};
pub use describe::describe;
pub use error::{
    AppError, InstallerError, LaunchError, ResolveError, StateError, StepError, StorageError,
};
pub use index::{
    AuthoredCommands, INDEX_PROTOCOL, Index, Installation, Location, Method, PackageCoordinate,
    StepOutcome, StepRecord,
};
pub use launch::{
    LaunchRequest, ResolvedLauncher, TemplateVars, app_root, exit_code, expand_template,
    listing as launcher_listing, resolve as resolve_launcher, run as run_launcher,
};
pub use operations::{
    Command, InstallRequest, OperationContext, Outcome, install, installed, list, plan, uninstall,
    update,
};
pub use proxies::{proxy_file_name, sync as sync_proxies};
pub use resolve::{
    ResolvedPackage, from_global, from_local_registry, from_manifest, qualified_ref,
};
pub use select::{
    Chooser, InstallChoice, detected_arch, detected_platform, options_listing, select_option,
};
pub use session::{SESSION_VARIABLE, SessionChoices};
pub use settings::{MutationLock, PROJECT_HOME, SettingsRoot};
pub use steps::{FeatureChooser, FeatureRequest, HostFacts, Plan, PlannedStep, Start};
pub use storage::{
    Availability, DEFAULT_LIBRARY, LIBRARY_MARKER, Library, LibraryMarker, Moved, Storage,
    StorageConfig, add as storage_add, attach as storage_attach, create_library,
    detach as storage_detach, gc as storage_gc, listing as storage_list, relocate as storage_move,
};
