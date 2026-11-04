//! Package resolution and option selection errors, reexported by the error layer.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-options");

use specmark::spec;
use thiserror::Error;

/// Package coordinates, registry resolution and installer-option selection.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-options")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-options")]
pub enum ResolveError {
    #[error(
        "this operation does not yet evaluate common configuration for `{package}` scope `{scope}` (violates REQ spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-SCOPES; fix surface: connect common configuration evaluation before this application operation)"
    )]
    ConfigurationUnsupported { package: String, scope: String },
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
