//! One error enum per layer: durable state, package resolution and
//! selection, installer children, launch, and the operations that compose
//! them. Every message names the violated PROP-059 anchor and a fix.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#principle");

use specmark::spec;
use thiserror::Error;

/// Settings root, `storage.toml`, library markers and the index file.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#index")]
pub enum StateError {
    #[error(
        "{action} `{path}` failed: {source} (violates spec://org.vibevm.core/vibevm/common/PROP-059#principle; \
         fix: check that the settings root is writable and has free space)"
    )]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "another global application mutation holds `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-059#index; \
         fix: wait for the other vibe command to finish and retry)"
    )]
    Locked { path: String },
    #[error(
        "application index `{path}` is unreadable: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#index; \
         fix: restore the file from a backup or remove it to start an empty inventory)"
    )]
    Index { path: String, reason: String },
    #[error(
        "storage configuration `{path}` is invalid: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: give every [[library]] a unique portable id other than `default` and an absolute path)"
    )]
    Storage { path: String, reason: String },
    #[error(
        "user launcher layer `{path}` is invalid: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog; \
         fix: edit it through `vibe app collection`, `vibe app pin` and `vibe app launcher`, or remove the file to start an empty arrangement)"
    )]
    UserLaunchers { path: String, reason: String },
    #[error(
        "storage library `{id}` is not configured (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: use `default` or a library listed in storage.toml)"
    )]
    UnknownLibrary { id: String },
    #[error(
        "storage library `{id}` is unavailable: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#availability; \
         fix: attach or mount the library directory again)"
    )]
    LibraryUnavailable { id: String, reason: String },
}

/// `vibe storage` edits of `storage.toml`, and moves of owned installations
/// between libraries.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#move")]
pub enum StorageError {
    #[error(
        "library path `{path}` is not absolute (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: pass an absolute directory)"
    )]
    RelativePath { path: String },
    #[error(
        "library id `{id}` is invalid: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: pass --id with a lowercase portable token other than `default` that no configured library uses)"
    )]
    InvalidId { id: String, reason: &'static str },
    #[error(
        "directory `{path}` is already configured as library `{id}` (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: check `vibe storage list`)"
    )]
    AlreadyListed { path: String, id: String },
    #[error(
        "directory `{path}` is not an empty directory (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: `vibe storage add` creates a fresh library in a new or empty directory; use `vibe storage attach` for an existing library)"
    )]
    NotEmpty { path: String },
    #[error(
        "directory `{path}` is not a library: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: attach a directory that holds vibe-library.toml, or create one with `vibe storage add`)"
    )]
    NotALibrary { path: String, reason: String },
    #[error(
        "the default library cannot be detached (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: detach one of the libraries listed in storage.toml)"
    )]
    DetachDefault,
    #[error(
        "installation `{key}` is not portable: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#portability; \
         fix: reinstall it with `vibe install -g --library ID` instead of moving it)"
    )]
    NotPortable { key: String, reason: &'static str },
    #[error(
        "installation `{key}` already lives in library `{library}` (violates spec://org.vibevm.core/vibevm/common/PROP-059#move; \
         fix: name a different destination library)"
    )]
    SameLibrary { key: String, library: String },
    #[error(
        "destination `{path}` already exists and no index record owns it (violates spec://org.vibevm.core/vibevm/common/PROP-059#move; \
         fix: inspect it, then remove it with `vibe storage gc --delete`)"
    )]
    DestinationOccupied { path: String },
    #[error(
        "{action} `{path}` failed: {source} (violates spec://org.vibevm.core/vibevm/common/PROP-059#move; \
         fix: check the permissions and free space of both libraries; `vibe storage gc` reports any leftover directory)"
    )]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Package coordinates, registry resolution and installer-option selection.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-options")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-options")]
pub enum ResolveError {
    #[error(
        "global application package `{spelling}` is invalid: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#commands; \
         fix: name it as <group>/<name>[@<version>])"
    )]
    Coordinate { spelling: String, reason: String },
    #[error(
        "resolving application package `{spelling}` failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#commands; \
         fix: check the package name, the --registry path or the configured registries)"
    )]
    Registry { spelling: String, reason: String },
    #[error(
        "application package `{package}` is not usable: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#declaration; \
         fix: declare [package] kind = \"app\" with an [application] table)"
    )]
    Manifest { package: String, reason: String },
    #[error(
        "application package `{package}` declares no [[installer_options]] and is not globally installable \
         (violates spec://org.vibevm.core/vibevm/common/PROP-059#declaration; fix: install a package that declares installer options)"
    )]
    NotGloballyInstallable { package: String },
    #[error(
        "selection_required: {what}; use {flag} with one of: {choices} (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-options; \
         fix: pass the flag or run in an attended terminal)"
    )]
    SelectionRequired {
        what: String,
        flag: &'static str,
        choices: String,
    },
    #[error(
        "no installer option for {detail}; available: {choices} (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-options; \
         fix: inspect `vibe app installers PACKAGE` and pick a listed platform, architecture and installer)"
    )]
    NoOption { detail: String, choices: String },
    #[error(
        "{what} selection cancelled (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-options; \
         fix: choose an entry or pass --platform/--arch/--installer)"
    )]
    Cancelled { what: &'static str },
    #[error(
        "interactive selection failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-options; \
         fix: pass --platform, --arch and --installer explicitly)"
    )]
    Prompt { reason: String },
}

/// Installer, hook and download children and archive extraction.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub enum InstallerError {
    #[error(
        "starting {what} failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#child-output; \
         fix: check that the interpreter or installer program exists)"
    )]
    Spawn { what: String, reason: String },
    #[error(
        "{what} exited {code} (violates spec://org.vibevm.core/vibevm/common/PROP-059#child-output; \
         fix: read the child output above and correct the authored command)"
    )]
    Exited { what: String, code: String },
    #[error(
        "fetching installer source `{source_ref}` failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: check the URL or the package-relative path)"
    )]
    Fetch { source_ref: String, reason: String },
    #[error(
        "installer source `{source_ref}` does not match its declared sha256 (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: correct the digest in the package or the published file)"
    )]
    Digest { source_ref: String },
    #[error(
        "installer source `{source_ref}` needs the network but --offline is set (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: drop --offline or use a package-relative source)"
    )]
    Offline { source_ref: String },
    #[error(
        "archive is unsafe or malformed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: publish an archive of plain files and directories with portable relative names)"
    )]
    Archive { reason: String },
    #[error(
        "{method} installers run only on Windows (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: select an installer option for this platform)"
    )]
    WindowsOnly { method: &'static str },
    #[error(
        "interpreter `{interpreter}` is unavailable on this host (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: select an installer option whose interpreter this platform provides)"
    )]
    Interpreter { interpreter: &'static str },
    #[error(
        "source_build input `{name}` cannot be laid out: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#source-build; \
         fix: give the source a URL or path ending in a portable file name, and set `extract = true` only on .zip, .tar.gz or .tgz files)"
    )]
    SourceLayout { name: String, reason: String },
    #[error(
        "{action} `{path}` failed: {source} (violates spec://org.vibevm.core/vibevm/common/PROP-059#ownership; \
         fix: check the permissions and free space of the storage library)"
    )]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "installer source `{source_ref}` is larger than its declared size of {declared} bytes; the download was stopped \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#sizes; fix: correct the declared size in the package or the published file)"
    )]
    Oversize { source_ref: String, declared: u64 },
    #[error(
        "{what} was interrupted; the signal reached the child and vibe waited for it to stop \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#cancellation; fix: rerun with --resume)"
    )]
    Interrupted { what: String },
}

/// Planning and running the steps of an installer option (`PROP-065`).
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#plan")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#execution")]
pub enum StepError {
    #[error(
        "feature `{feature}` is not declared by installer option `{option}`; declared: {declared} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#features; fix: name a declared feature with --feature / --no-feature)"
    )]
    UnknownFeature {
        option: String,
        feature: String,
        declared: String,
    },
    #[error(
        "feature selection failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-065#features; \
         fix: pass --feature / --no-feature explicitly)"
    )]
    FeaturePrompt { reason: String },
    #[error(
        "profile `{profile}` is not declared by installer option `{option}`; declared: {declared} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#profiles; fix: name a declared profile with --profile)"
    )]
    UnknownProfile {
        option: String,
        profile: String,
        declared: String,
    },
    #[error(
        "the feature choice of installer option `{option}` cannot be honoured: {reason}; nothing was downloaded or run \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#feature-tree; fix: drop or change the conflicting --feature / --no-feature / --profile)"
    )]
    FeatureConflict { option: String, reason: String },
    #[error(
        "parameter `{parameter}` of installer option `{option}` is invalid: {reason}; nothing was downloaded or run \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#parameters; fix: pass a valid value with --set {parameter}=VALUE)"
    )]
    ParameterInvalid {
        option: String,
        parameter: String,
        reason: String,
    },
    #[error(
        "installing `{key}` needs acceptance of the license `{license}` ({file}); nothing was downloaded or run \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#license; fix: read the license and pass --accept-license, \
         or accept it in an attended terminal)"
    )]
    LicenseRequired {
        key: String,
        license: String,
        file: String,
    },
    #[error(
        "the license file `{file}` of `{key}` cannot be read: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-065#license; \
         fix: ship the declared license file in the package)"
    )]
    LicenseUnreadable {
        key: String,
        file: String,
        reason: String,
    },
    #[error(
        "step `{step}` of `{key}` needs `{need}`, which this host does not provide; nothing was downloaded or run \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#needs; fix: provide `{need}` and retry, or select another installer option)"
    )]
    UnmetNeed {
        key: String,
        step: String,
        need: String,
    },
    #[error(
        "agent step `{step}` of `{key}` is required but no agent is configured: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-064#selection; fix: configure an agent with --agent NAME, \
         the VIBE_AGENT_COMMAND registration or `default` in agents.toml, or select another installer option)"
    )]
    AgentRequired {
        key: String,
        step: String,
        reason: String,
    },
    #[error(
        "installer option `{option}` of `{key}` declares an owned result, but with the chosen features and this host \
         no running step writes $VIBE_OUT (violates spec://org.vibevm.core/vibevm/common/PROP-065#plan; \
         fix: choose the features whose steps produce the payload, or select another installer option)"
    )]
    NoResultStep { key: String, option: String },
    #[error(
        "{field} of step `{step}` cannot be expanded: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-065#templates; \
         fix: correct the template in the package)"
    )]
    Template {
        step: String,
        field: &'static str,
        reason: String,
    },
    #[error(
        "copy step `{step}` is unsafe: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-065#kinds; \
         fix: copy a package-relative file or directory to a destination inside the payload)"
    )]
    Copy { step: String, reason: String },
    #[error(
        "step `{step}` ({kind}) of `{key}` failed; nothing was placed or registered and the index is unchanged; \
         its work directory `{work}` is kept for inspection (violates spec://org.vibevm.core/vibevm/common/PROP-065#failure; \
         fix: correct the cause below, then rerun with --resume, or with --from-step ID to restart at a named step)"
    )]
    Failed {
        key: String,
        step: String,
        kind: &'static str,
        work: String,
        #[source]
        source: Box<AppError>,
    },
    #[error(
        "the steps of `{key}` finished but $VIBE_OUT `{path}` is not a non-empty directory; its work directory is kept \
         for inspection (violates spec://org.vibevm.core/vibevm/common/PROP-065#result; fix: make a step write the payload into $VIBE_OUT)"
    )]
    EmptyResult { key: String, path: String },
    #[error(
        "finish step `{step}` ({kind}) of `{key}` failed; the payload or root stays in place, the installation is recorded \
         and `vibe uninstall` remains available (violates spec://org.vibevm.core/vibevm/common/PROP-065#failure; \
         fix: correct the cause below and install again)"
    )]
    FinishFailed {
        key: String,
        step: String,
        kind: &'static str,
        #[source]
        source: Box<AppError>,
    },
    #[error(
        "the installation of `{key}` was interrupted {during}; no step is marked done by the interruption, no payload \
         was placed and the index is unchanged; its work directory `{work}` is kept \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#cancellation; fix: rerun the same command with --resume)"
    )]
    Interrupted {
        key: String,
        during: String,
        step: Option<String>,
        work: String,
    },
    #[error(
        "--resume has nothing to continue for `{key}`: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-065#resume; \
         fix: install without --resume, or restart a kept work directory at one of its steps with --from-step ID)"
    )]
    NothingToResume {
        key: String,
        work: String,
        reason: String,
    },
    #[error(
        "the kept work directory `{work}` cannot be resumed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-065#resume; \
         fix: rerun with the original option and features, or install without --resume)"
    )]
    ResumeMismatch { work: String, reason: String },
    #[error(
        "--from-step `{step}` names no step of installer option `{option}`; steps: {steps} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#resume; fix: name a listed step id)"
    )]
    UnknownStep {
        option: String,
        step: String,
        steps: String,
    },
    #[error(
        "no kept work directory of `{package}` is in `{home}` (violates spec://org.vibevm.core/vibevm/common/PROP-065#work; \
         fix: list the kept directories with `vibe app work`, or install with --keep-work to keep one)"
    )]
    WorkMissing { package: String, home: String },
}

/// Launch-point resolution and native program execution.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations")]
pub enum LaunchError {
    #[error(
        "unknown_launcher: `{id}` is not declared by the installation (violates spec://org.vibevm.core/vibevm/common/PROP-059#launch-points; \
         fix: list the IDs with `vibe app launchers PACKAGE`)"
    )]
    UnknownLauncher { id: String },
    #[error(
        "launcher_unavailable: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#launcher-paths; \
         fix: inspect `vibe app launchers PACKAGE` for the resolved paths)"
    )]
    Unavailable { reason: String },
    #[error(
        "launcher_unavailable: starting `{id}` failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations; \
         fix: check that the resolved executable runs on this platform)"
    )]
    Spawn { id: String, reason: String },
    #[error(
        "icon_unavailable: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#menu; \
         fix: inspect `vibe app launchers PACKAGE --json` for the icon set and the files that exist)"
    )]
    NoIcon { reason: String },
}

/// Install, update, uninstall and inventory operations.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub enum AppError {
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    Catalog(#[from] crate::error_catalog::CatalogError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    #[error(transparent)]
    Installer(#[from] InstallerError),
    #[error(transparent)]
    Launch(#[from] LaunchError),
    #[error(transparent)]
    Step(#[from] StepError),
    #[error(transparent)]
    Agent(#[from] crate::agents::AgentError),
    #[error(transparent)]
    Lifecycle(#[from] crate::error_lifecycle::LifecycleError),
    #[error(
        "not_installed: application `{package}` is not installed in {home} (violates spec://org.vibevm.core/vibevm/common/PROP-059#commands; \
         fix: {fix})"
    )]
    NotInstalled {
        package: String,
        /// `the user home` or `the project home <path>`.
        home: String,
        fix: String,
    },
    #[error(
        "selection_required: several installed versions of `{package}` match (violates spec://org.vibevm.core/vibevm/common/PROP-059#commands; \
         fix: name one exactly as <group>/<name>@=<version>)"
    )]
    AmbiguousInstallation { package: String },
    #[error(
        "--force is unsupported for `{key}`: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#force-and-hooks; \
         fix: run without --force or select an option that declares the forced variant)"
    )]
    ForceUnsupported { key: String, reason: &'static str },
    #[error(
        "--library applies only to owned methods, but `{key}` uses the external `{method}` method \
         (violates spec://org.vibevm.core/vibevm/common/PROP-059#commands; fix: drop --library, or select an archive installer option)"
    )]
    LibraryForExternal { key: String, method: &'static str },
    #[error(
        "directory `{path}` exists but no index record owns it (violates spec://org.vibevm.core/vibevm/common/PROP-059#ownership; \
         fix: rerun with --force to replace it, or remove the orphan directory)"
    )]
    ForeignDirectory { path: String },
    #[error(
        "{hook} failed for `{key}`: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#force-and-hooks; \
         fix: correct the authored hook; the previous index record is unchanged)"
    )]
    Hook {
        hook: &'static str,
        key: String,
        reason: String,
    },
    #[error(
        "installation `{key}` declares no update command (violates spec://org.vibevm.core/vibevm/common/PROP-059#commands; \
         fix: rerun with --force to use force_install, or reinstall)"
    )]
    MissingUpdate { key: String },
    #[error(
        "the update of `{key}` changes its application slot from `{from}` to `{to}` (violates spec://org.vibevm.core/vibevm/common/PROP-059#alternatives; \
         fix: install the new package with `vibe install -g` instead)"
    )]
    SlotChanged {
        key: String,
        from: String,
        to: String,
    },
    #[error(
        "installation `{key}` is unavailable: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#availability; \
         fix: attach the storage library again; vibe never removes or replaces an unavailable installation)"
    )]
    Unavailable { key: String, reason: String },
    #[error(
        "installer for `{key}` did not produce app_root `{path}`: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: make the authored installer create the declared app_root directory)"
    )]
    MissingAppRoot {
        key: String,
        path: String,
        reason: String,
    },
    #[error(
        "command `{command}` is reserved by slot `{owner}` and cannot be claimed by slot `{slot}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-059#alternatives; fix: uninstall the providers of `{owner}` first, \
         or install a package whose public commands do not collide)"
    )]
    CommandReserved {
        command: String,
        slot: String,
        owner: String,
    },
    #[error(
        "slot `{slot}` has no installed provider (violates spec://org.vibevm.core/vibevm/common/PROP-059#alternatives; \
         fix: check `vibe app list` for the installed slots)"
    )]
    UnknownSlot { slot: String },
    #[error(
        "`{key}` is not an installed provider of slot `{slot}`; providers: {providers} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-059#alternatives; fix: name one installation key exactly as listed)"
    )]
    NotProvider {
        slot: String,
        key: String,
        providers: String,
    },
    #[error(
        "the PATH proxy image `{path}` is missing (violates spec://org.vibevm.core/vibevm/common/PROP-059#proxy; \
         fix: reinstall vibe so that vibe-app-proxy ships beside the vibe executable)"
    )]
    ProxyImage { path: String },
    #[error(
        "malformed session choice `{entry}`; expected SLOT=KEY entries separated by `;` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-059#session-selection; fix: correct VIBE_APP_USE or the --use argument)"
    )]
    SessionChoice { entry: String },
    #[error(
        "the launchers of `{key}` are incomplete: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#desktop-entries; \
         fix: make every launcher row complete, or ship the desktop entry its override row and public commands name)"
    )]
    Launchers { key: String, reason: String },
}

impl AppError {
    /// `not_installed` for `package` in `home`, naming the home searched and,
    /// for a project home, pointing at `-g` (`PROP-059#project-scope`).
    pub fn not_installed(home: &crate::settings::SettingsRoot, package: impl Into<String>) -> Self {
        Self::NotInstalled {
            package: package.into(),
            home: home.home_name(),
            fix: if home.is_project() {
                "use -g for the user home, or check `vibe app list` for the installed keys".into()
            } else {
                "check `vibe app list` for the installed keys".into()
            },
        }
    }
}
