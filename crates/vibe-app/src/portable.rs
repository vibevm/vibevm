//! The portable set of a home's applications (`PROP-059#portable-set`):
//! `vibe app export` projects the index and the user layer into one
//! `vibe-apps/1` document, and `vibe app import` reads it back on another
//! machine, deciding row by row whether to install, skip or refuse.
//!
//! The document is a projection, never a second inventory: it names exact
//! package keys and the choice made for each, and the importing host
//! resolves every key through its own registries and picks its own platform
//! row. This module owns the projection, the TOML and JSON spellings, the
//! per-row decision and the restoration of the launcher arrangement; the CLI
//! runs the installs between them.
//!
//! It is not the system-menu export of `PROP-059#launcher-runtime`
//! (`export.rs`), which writes `.desktop` files and shortcuts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#portable-set");

use std::collections::BTreeSet;

use specmark::spec;
use vibe_core::manifest::InstallerOptionDecl;
use vibe_wire::generated::apps_manifest as wire;

use crate::catalog::UserLaunchers;
use crate::error::AppError;
use crate::error_lifecycle::LifecycleError;
use crate::index::{Index, Installation, Mark};
use crate::resolve::ResolvedPackage;
use crate::select::{InstallChoice, detected_arch, detected_platform, select_option};
use crate::settings::SettingsRoot;
use crate::steps::PlanLicense;

#[path = "portable/arrangement.rs"]
mod arrangement;

pub use arrangement::restore_arrangement;

/// The protocol tag of the portable set.
pub const APPS_PROTOCOL: &str = "vibe-apps/1";

/// The protocol tag of the `vibe app import --json` answer.
pub const IMPORT_PROTOCOL: &str = "vibe-app-import/1";

/// A reader opening the file in an editor learns what it is and how it is
/// used without the spec at hand.
const HEADER: &str = "\
# vibe-apps/1 — a portable set of applications and their launcher arrangement
# (spec://org.vibevm.core/vibevm/common/PROP-059#portable-set).
# Install it on another machine with `vibe app import <this file>`.

";

/// What `vibe app export` puts into the set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#portable-set")]
pub struct PortableExport {
    /// `--all`: hidden installations and `auto` rows too.
    pub all: bool,
    /// The launcher arrangement (collections, pins, overrides); off with
    /// `--installations-only`.
    pub arrangement: bool,
}

/// Project a home into the portable set.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let request = vibe_app::PortableExport { all: false, arrangement: true };
/// let set = vibe_app::portable_set(&settings, &request).unwrap();
/// assert_eq!(set.protocol, "vibe-apps/1");
/// assert!(set.application.is_empty() && set.collection.is_empty());
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#portable-set")]
pub fn portable_set(
    settings: &SettingsRoot,
    request: &PortableExport,
) -> Result<wire::AppsManifest, AppError> {
    let index = Index::load(settings)?;
    let application = index
        .installations
        .values()
        // A kept previous version is rollback state of this machine, not a
        // choice anyone would carry to another one.
        .filter(|record| record.lifecycle.superseded_by.is_none())
        .filter(|record| request.all || !record.lifecycle.hidden)
        .filter(|record| request.all || record.lifecycle.mark == Mark::Manual)
        .map(row_of)
        .collect();
    let mut set = wire::AppsManifest {
        protocol: APPS_PROTOCOL.into(),
        exported: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        home: if settings.is_project() {
            wire::AppsManifestHome::Project
        } else {
            wire::AppsManifestHome::User
        },
        platform: detected_platform().unwrap_or(std::env::consts::OS).into(),
        arch: detected_arch().into(),
        application,
        collection: Vec::new(),
        pin: Vec::new(),
        launcher: Vec::new(),
    };
    if request.arrangement {
        arrangement::export_into(&UserLaunchers::load(settings)?, &mut set);
    }
    Ok(set)
}

fn row_of(record: &Installation) -> wire::Application {
    wire::Application {
        key: record.package.key(),
        installer: record.option.clone(),
        features: record.features.clone(),
        parameters: record.parameters.clone(),
        license: record
            .license
            .as_ref()
            .map(|_| wire::ApplicationLicense::Accepted),
        mark: match record.lifecycle.mark {
            Mark::Manual => wire::ApplicationMark::Manual,
            Mark::Auto => wire::ApplicationMark::Auto,
        },
        held: record.lifecycle.held,
        hidden: record.lifecycle.hidden,
    }
}

/// The TOML spelling of the set, under a short header comment.
pub fn portable_toml(set: &wire::AppsManifest) -> Result<String, AppError> {
    let body = toml::to_string(set).map_err(|error| LifecycleError::PortableInvalid {
        file: "the exported set".into(),
        reason: error.to_string(),
    })?;
    Ok(format!("{HEADER}{body}"))
}

/// Read a set `vibe app export` wrote, as TOML or (from `--json`) JSON.
///
/// ```
/// let set = vibe_app::parse_portable(
///     "set.toml",
///     "protocol = \"vibe-apps/1\"\nexported = \"2026-11-01T12:00:00Z\"\nhome = \"user\"\n\
///      platform = \"linux\"\narch = \"x86_64\"\n\n[[application]]\n\
///      key = \"org.example/demo@1.0.0\"\ninstaller = \"main\"\nmark = \"manual\"\n",
/// )
/// .unwrap();
/// assert_eq!(set.application[0].installer, "main");
/// assert!(vibe_app::parse_portable("set.toml", "protocol = \"other/1\"").is_err());
/// ```
pub fn parse_portable(file: &str, text: &str) -> Result<wire::AppsManifest, AppError> {
    let invalid = |reason: String| LifecycleError::PortableInvalid {
        file: file.into(),
        reason,
    };
    let set: wire::AppsManifest = if text.trim_start().starts_with('{') {
        serde_json::from_str(text).map_err(|error| invalid(error.to_string()))?
    } else {
        toml::from_str(text).map_err(|error| invalid(error.to_string()))?
    };
    if set.protocol != APPS_PROTOCOL {
        return Err(invalid(format!(
            "protocol `{}` is not `{APPS_PROTOCOL}`",
            set.protocol
        ))
        .into());
    }
    Ok(set)
}

/// What an import does with one row.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#portable-set")]
pub enum ImportAction {
    /// Install with this selection and these wizard answers.
    Install(ImportInstall),
    /// Leave it; the reason says why.
    Skip(String),
    /// Refuse this row; the others go on.
    Refuse(AppError),
}

/// The selection and answers one imported row installs with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#portable-set")]
pub struct ImportInstall {
    pub choice: InstallChoice,
    /// The option this host will take when it is decided without a
    /// question; `None` when the attended chooser would ask.
    pub option: Option<String>,
    pub enable: Vec<String>,
    pub disable: Vec<String>,
    pub parameters: Vec<(String, String)>,
}

/// Decide one row against the home's index and the options the registries
/// offer for its package: the named installer when this host has it, the
/// host's own option otherwise (the rule `select_option` applies with no
/// flags), and a skip or refusal when the key is installed already.
///
/// ```
/// use vibe_app::{ImportAction, Index, import_action};
/// use vibe_wire::generated::apps_manifest::{Application, ApplicationMark};
///
/// let row = Application {
///     key: "org.example/demo@1.0.0".into(),
///     installer: "main".into(),
///     features: vec![],
///     parameters: Default::default(),
///     license: None,
///     mark: ApplicationMark::Manual,
///     held: false,
///     hidden: false,
/// };
/// let action = import_action(&Index::default(), &row, &[], false);
/// assert!(matches!(action, ImportAction::Install(_)));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#portable-set")]
pub fn import_action(
    index: &Index,
    row: &wire::Application,
    options: &[InstallerOptionDecl],
    skip_installed: bool,
) -> ImportAction {
    let host = detected_platform();
    let offered = options.iter().any(|option| {
        option.id == row.installer
            && Some(option.platform.as_str()) == host
            && option.arch == detected_arch()
    });
    let choice = if offered {
        InstallChoice {
            platform: host.map(str::to_owned),
            arch: Some(detected_arch().into()),
            installer: Some(row.installer.clone()),
        }
    } else {
        InstallChoice::default()
    };
    let chosen = select_option(options, &choice, None).ok();
    let install = answers(row, choice, chosen.as_ref());
    let Some(record) = index
        .installations
        .get(&row.key)
        .filter(|record| record.lifecycle.superseded_by.is_none())
    else {
        return ImportAction::Install(install);
    };
    let wanted = install.option.as_deref().unwrap_or(&row.installer);
    let same = record.option == wanted
        && sorted(&record.features) == sorted(&row.features)
        && record.parameters == row.parameters;
    if same {
        return ImportAction::Skip("already installed with the same choice".into());
    }
    if skip_installed {
        return ImportAction::Skip(format!(
            "installed with installer `{}`; kept as it is",
            record.option
        ));
    }
    ImportAction::Refuse(
        LifecycleError::ChoiceDiffers {
            key: row.key.clone(),
            installed: describe(&record.option, &record.features),
            wanted: describe(wanted, &row.features),
        }
        .into(),
    )
}

/// The licence an imported row still needs accepted, so a caller can show
/// its text before the run instead of meeting a `license_required` refusal
/// (`PROP-065#license`). `None` when the option the row installs with
/// declares none, when this home already accepted that licence for the key
/// (the plan admits it on its own), or when `--accept-license` meets a row
/// that says the licence was accepted where the set was exported. `file` is
/// the absolute path of the text in the resolved package, as `describe`
/// reports it.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#portable-set")]
pub fn import_license(
    index: &Index,
    row: &wire::Application,
    install: &ImportInstall,
    resolved: &ResolvedPackage,
    accept_license: bool,
) -> Option<PlanLicense> {
    if accept_license && matches!(row.license, Some(wire::ApplicationLicense::Accepted)) {
        return None;
    }
    // The same choice `import_action` made, so the licence is the one of the
    // option the install will plan with.
    let option = select_option(&resolved.options, &install.choice, None).ok()?;
    let license = option.license.as_ref()?;
    let accepted_here = index
        .installations
        .get(&row.key)
        .and_then(|record| record.license.as_ref())
        .is_some_and(|accepted| accepted.name == license.name);
    if accepted_here {
        return None;
    }
    Some(PlanLicense {
        name: license.name.clone(),
        file: resolved.root.join(&license.file).display().to_string(),
    })
}

/// The wizard answers of a row. When the host's option is known, features
/// and parameters it does not declare are dropped and the defaults the row
/// left out are deselected, so the import reproduces the recorded set rather
/// than the package's defaults.
fn answers(
    row: &wire::Application,
    choice: InstallChoice,
    chosen: Option<&InstallerOptionDecl>,
) -> ImportInstall {
    let Some(option) = chosen else {
        return ImportInstall {
            choice,
            option: None,
            enable: row.features.clone(),
            disable: Vec::new(),
            parameters: row.parameters.clone().into_iter().collect(),
        };
    };
    let declared = |id: &str| option.features.iter().any(|feature| feature.id == id);
    ImportInstall {
        choice,
        option: Some(option.id.clone()),
        enable: row
            .features
            .iter()
            .filter(|id| declared(id))
            .cloned()
            .collect(),
        disable: option
            .features
            .iter()
            .filter(|feature| feature.default && !feature.required)
            .filter(|feature| !row.features.contains(&feature.id))
            .map(|feature| feature.id.clone())
            .collect(),
        parameters: row
            .parameters
            .iter()
            .filter(|(id, _)| option.parameters.iter().any(|declared| &declared.id == *id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect(),
    }
}

fn sorted(features: &[String]) -> BTreeSet<&str> {
    features.iter().map(String::as_str).collect()
}

fn describe(option: &str, features: &[String]) -> String {
    if features.is_empty() {
        format!("installer `{option}`")
    } else {
        format!("installer `{option}` and features {}", features.join(", "))
    }
}

#[cfg(test)]
#[path = "portable/tests.rs"]
mod tests;
