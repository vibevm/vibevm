//! Which version of each application a directory activates
//! (PROP-075 `#VERSION-FILE-FORMATS`, `#ACTIVATION-ENV-OUTPUT`).
//!
//! The precedence, highest first, is the one PROP-075 states: an explicit
//! `--use SLOT=VER` or `SLOT@VER` on the command; the project's `vibe.toml`
//! requirement on that application package; the nearest declared version
//! file, searched from the working directory upward and stopping at the
//! project root; the slot's global default; and a slot that is off, which is
//! none of the above and stays off.
//!
//! Resolution never reaches the network, a registry or an index server: it
//! reads this home's index, the sidecar that records the declared version
//! files, and the version files themselves. A version that is not installed
//! is **not** an error — it is reported with the exact `vibe install -g`
//! command, because a prompt hook must never install anything and must never
//! fail a `cd` (PROP-075 `#ACTIVATION-HOOK`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS");

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::VersionSpec;

use crate::index::Index;
use crate::session::SESSION_OFF;
use crate::settings::SettingsRoot;
use crate::versionfile::{Declared, RequestKind, VersionRequest};

/// What decided a slot's version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
pub enum Source {
    /// An explicit `--use SLOT=VER` or `SLOT@VER`.
    Flag,
    /// The project's `vibe.toml` requirement on the application package.
    ProjectRequirement,
    /// The nearest declared version file.
    VersionFile,
    /// The slot's global default.
    SlotDefault,
    /// The slot is off and stays off (`PROP-059#slot-off`).
    SlotOff,
    /// Nothing selected this slot.
    None,
}

impl Source {
    /// The stable spelling of this source.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::ProjectRequirement => "project_requirement",
            Self::VersionFile => "version_file",
            Self::SlotDefault => "slot_default",
            Self::SlotOff => "slot_off",
            Self::None => "none",
        }
    }
}

/// What came of a slot's selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
pub enum State {
    /// One installation was chosen.
    Selected,
    /// The requested version is not installed; `install` names the command.
    NotInstalled,
    /// The slot is switched off for this home.
    Off,
    /// A channel this vibe cannot lower yet (`PROP-075#CHANNEL-SYNTAX`
    /// implements `latest` and `stable` over the installed set; every other
    /// channel needs the template channel resolver).
    UnresolvedChannel,
    /// Several installations matched and nothing narrowed the request.
    Ambiguous,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::NotInstalled => "not_installed",
            Self::Off => "off",
            Self::UnresolvedChannel => "unresolved_channel",
            Self::Ambiguous => "ambiguous",
        }
    }
}

/// One slot's resolved selection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
pub struct SlotSelection {
    pub slot: String,
    pub source: Source,
    pub state: State,
    /// The requirement, channel or key as it was authored.
    pub requested: Option<String>,
    /// The installation key chosen, for a `Selected` slot; `none` for a slot
    /// an explicit `--use SLOT=none` switched off.
    pub key: Option<String>,
    pub version: Option<String>,
    /// The version file that decided it.
    pub file: Option<PathBuf>,
    /// The versions a `.tool-versions` line listed after the one taken.
    pub unused: Vec<String>,
    /// The keys that matched, when nothing narrowed the request.
    pub candidates: Vec<String>,
    /// The exact command that would supply a version that is not installed.
    pub install: Option<String>,
}

impl SlotSelection {
    /// Whether this selection withdraws the slot's link directories instead
    /// of putting a payload in front of them (`PROP-059#slot-off`).
    pub fn is_off(&self) -> bool {
        self.key.as_deref() == Some(SESSION_OFF)
    }

    /// The `SLOT=KEY` entry this selection contributes to `VIBE_APP_USE`,
    /// if any.
    pub fn choice(&self) -> Option<String> {
        self.key
            .as_deref()
            .map(|key| format!("{}={key}", self.slot))
    }
}

/// The whole resolution for one directory.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Activation {
    pub project: Option<PathBuf>,
    /// One per slot the request resolved, in slot order.
    pub selections: Vec<SlotSelection>,
    /// What the resolution reported and did not fail on.
    pub notices: Vec<String>,
}

impl Activation {
    pub fn of_slot(&self, slot: &str) -> Option<&SlotSelection> {
        self.selections.iter().find(|row| row.slot == slot)
    }
}

/// One project requirement on an application package, as the caller read it
/// from `vibe.toml`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRequirement {
    pub group: String,
    pub name: String,
    pub version: VersionSpec,
    /// The requirement as `vibe.toml` spells it.
    pub authored: String,
}

/// Which installed versions a named channel holds — the seam by which
/// `lts/jod` in a `.nvmrc` becomes a version, without the network, a
/// registry call or an index read (PROP-075 `#CHANNEL-SYNTAX`,
/// `#ACTIVATION-HOOK`).
///
/// `latest` and `stable` are **not** asked here: they are reserved and
/// vibe answers them over the installed set itself, so every slot has
/// them whether or not any package declares a channel. An implementor
/// that knows nothing about a package answers `None`, and the slot is
/// then reported `unresolved_channel` — never an error, because a prompt
/// hook must not fail a `cd`.
/// ```
/// use vibe_app::ChannelMembership;
///
/// /// A table a test states outright, standing in for the cached version
/// /// list a real resolution reads.
/// struct Table;
/// impl ChannelMembership for Table {
///     fn members(&self, package: &str, channel: &str) -> Option<Vec<semver::Version>> {
///         (package == "org.vibevm.node/node" && channel == "lts/jod")
///             .then(|| vec!["22.23.4".parse().unwrap()])
///     }
/// }
///
/// let table: &dyn ChannelMembership = &Table;
/// assert_eq!(
///     table.members("org.vibevm.node/node", "lts/jod").unwrap()[0].to_string(),
///     "22.23.4"
/// );
/// // An undeclared channel is `None`, which the activation reports as
/// // `unresolved_channel` rather than as an error.
/// assert!(table.members("org.vibevm.node/node", "canary").is_none());
/// ```
pub trait ChannelMembership {
    /// The versions `channel` holds for `package` (`<group>/<name>`), or
    /// `None` when that package declares no such channel.
    fn members(&self, package: &str, channel: &str) -> Option<Vec<semver::Version>>;
}

/// What to resolve.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
#[derive(Clone)]
pub struct ActivationRequest<'a> {
    /// Which installed versions a channel holds. `None` leaves every
    /// channel but `latest` and `stable` `unresolved_channel`.
    pub channels: Option<&'a dyn ChannelMembership>,
    /// `SLOT=KEY`, `SLOT=VERSION`, `SLOT=CHANNEL` or `SLOT=none` entries, in
    /// command order; a later entry wins for its slot.
    pub uses: &'a [String],
    /// Where the version-file search starts.
    pub directory: &'a Path,
    /// The project root the search stops at.
    pub project: Option<&'a Path>,
    /// The project's requirements on application packages.
    pub requirements: &'a [ProjectRequirement],
    /// Resolve the project requirements and the version files too, not only
    /// the explicit `uses` (`vibe app env --project`).
    pub consult_project: bool,
}

/// Resolve the selection of every slot the request names.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS",
    r = 1
)]
pub fn resolve(
    settings: &SettingsRoot,
    index: &Index,
    request: &ActivationRequest<'_>,
) -> Result<Activation, crate::error::AppError> {
    let mut activation = Activation {
        project: request.project.map(Path::to_path_buf),
        ..Activation::default()
    };
    // 1. The explicit flags, which win outright.
    let mut explicit: Vec<(String, String)> = Vec::new();
    for entry in request.uses {
        let (slot, value) = entry
            .split_once('=')
            .map(|(slot, value)| (slot.trim(), value.trim()))
            .ok_or_else(|| crate::error::AppError::SessionChoice {
                entry: entry.trim().to_owned(),
            })?;
        if slot.is_empty() || value.is_empty() {
            return Err(crate::error::AppError::SessionChoice {
                entry: entry.trim().to_owned(),
            });
        }
        explicit.retain(|(seen, _)| seen != slot);
        explicit.push((slot.to_owned(), value.to_owned()));
    }
    for (slot, value) in &explicit {
        activation.selections.push(from_flag(
            index,
            slot,
            value,
            &mut activation.notices,
            request.channels,
        )?);
    }
    if !request.consult_project {
        activation.selections.sort_by(|a, b| a.slot.cmp(&b.slot));
        return Ok(activation);
    }
    // 2. The project's requirements, by the slot each package provides.
    for requirement in request.requirements {
        let Some(slot) = slot_of_package(index, &requirement.group, &requirement.name) else {
            continue;
        };
        if activation.of_slot(&slot).is_some() {
            continue;
        }
        let spelling = format!("{}/{}", requirement.group, requirement.name);
        let mut selection = choose(
            index,
            &slot,
            Source::ProjectRequirement,
            &VersionRequest {
                authored: requirement.authored.clone(),
                kind: RequestKind::Spec(requirement.version.clone()),
            },
            Some(&spelling),
            request.channels,
        );
        selection.requested = Some(requirement.authored.clone());
        activation.selections.push(selection);
    }
    // 3. The nearest declared version file of each remaining slot.
    let declared = Declared::load(settings);
    let search =
        crate::versionfile::resolve(settings, request.directory, request.project, &declared);
    activation.notices.extend(search.faults.iter().cloned());
    for found in &search.found {
        if activation.of_slot(&found.slot).is_some() {
            continue;
        }
        let mut selection = choose(
            index,
            &found.slot,
            Source::VersionFile,
            &found.request,
            None,
            request.channels,
        );
        selection.file = Some(found.path.clone());
        selection.unused = found.unused.clone();
        if !found.unused.is_empty() {
            activation.notices.push(format!(
                "`{}` lists {} after `{}` for slot `{}`; only the first is used",
                found.path.display(),
                found
                    .unused
                    .iter()
                    .map(|version| format!("`{version}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
                found.request.authored,
                found.slot
            ));
        }
        activation.selections.push(selection);
    }
    activation.selections.sort_by(|a, b| a.slot.cmp(&b.slot));
    Ok(activation)
}

/// An explicit `--use SLOT=VALUE`: an installation key (which always carries
/// a `/`), the bare word `none`, or a version requirement or channel.
fn from_flag(
    index: &Index,
    slot: &str,
    value: &str,
    notices: &mut Vec<String>,
    channels: Option<&dyn ChannelMembership>,
) -> Result<SlotSelection, crate::error::AppError> {
    let empty = SlotSelection {
        slot: slot.to_owned(),
        source: Source::Flag,
        state: State::Selected,
        requested: Some(value.to_owned()),
        key: None,
        version: None,
        file: None,
        unused: Vec::new(),
        candidates: Vec::new(),
        install: None,
    };
    if value == SESSION_OFF {
        return Ok(SlotSelection {
            state: State::Off,
            key: Some(SESSION_OFF.to_owned()),
            ..empty
        });
    }
    // An installation key is `group/name@version`: it carries both. A
    // channel may carry a `/` (`lts/jod`) and never an `@`, so the two
    // spellings cannot be confused.
    if value.contains('/') && value.contains('@') {
        // An installation key names one record exactly.
        let Some(record) = index.installations.get(value) else {
            return Err(crate::error::AppError::NotInstalled {
                package: value.to_owned(),
                home: "the selected application home".to_owned(),
                fix: format!("install it with `vibe install -g {value}`"),
            });
        };
        if record.slot != slot {
            return Err(crate::error::AppError::NotProvider {
                key: value.to_owned(),
                slot: slot.to_owned(),
                providers: index
                    .installations
                    .iter()
                    .filter(|(_, provider)| provider.slot == slot)
                    .map(|(key, _)| key.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
        return Ok(SlotSelection {
            key: Some(value.to_owned()),
            version: Some(record.package.version.clone()),
            ..empty
        });
    }
    match VersionRequest::parse(value) {
        Ok(request) => Ok(choose(index, slot, Source::Flag, &request, None, channels)),
        Err(reason) => {
            notices.push(format!("`--use {slot}={value}`: {reason}"));
            Ok(SlotSelection {
                state: State::NotInstalled,
                install: install_command(index, slot, value),
                ..empty
            })
        }
    }
}

/// The slot a `group/name` provides, as the home's installations record it.
fn slot_of_package(index: &Index, group: &str, name: &str) -> Option<String> {
    index
        .installations
        .values()
        .find(|record| record.package.group == group && record.package.name == name)
        .map(|record| record.slot.clone())
}

/// Pick the installation of `slot` that `request` names: the newest matching
/// version wins, which is what a version manager means by `use 22`. Several
/// distinct installations of the *same* newest version are what
/// `selection_required` is about — nothing in the request narrows them.
#[allow(clippy::too_many_arguments)]
fn choose(
    index: &Index,
    slot: &str,
    source: Source,
    request: &VersionRequest,
    package: Option<&str>,
    channels: Option<&dyn ChannelMembership>,
) -> SlotSelection {
    let mut selection = SlotSelection {
        slot: slot.to_owned(),
        source,
        state: State::Selected,
        requested: Some(request.authored.clone()),
        key: None,
        version: None,
        file: None,
        unused: Vec::new(),
        candidates: Vec::new(),
        install: None,
    };
    if index.slots.get(slot).is_some_and(|state| state.off) && source == Source::SlotDefault {
        selection.source = Source::SlotOff;
        selection.state = State::Off;
        return selection;
    }
    // Every installation of this slot, with its parsed version.
    let mut providers: Vec<(&String, semver::Version)> = index
        .installations
        .iter()
        .filter(|(_, record)| record.slot == slot)
        .filter(|(_, record)| {
            package.is_none_or(|spelling| {
                format!("{}/{}", record.package.group, record.package.name) == spelling
            })
        })
        .filter_map(|(key, record)| {
            semver::Version::parse(&record.package.version)
                .ok()
                .map(|version| (key, version))
        })
        .collect();
    providers.sort_by(|left, right| right.1.cmp(&left.1));
    let matching: Vec<&(&String, semver::Version)> = match &request.kind {
        RequestKind::Spec(spec) => providers
            .iter()
            .filter(|(_, version)| spec.matches(version))
            .collect(),
        // `latest` and `stable` are reserved and implemented by vibe for
        // every package (`PROP-075#CHANNEL-SYNTAX`); over an installed set
        // they need neither a network nor a template.
        RequestKind::Channel(name) if name == "latest" => providers.iter().collect(),
        RequestKind::Channel(name) if name == "stable" => providers
            .iter()
            .filter(|(_, version)| version.pre.is_empty())
            .collect(),
        // Every other channel is the package's own, declared by its
        // `[[template.channel]]` rules and answered from the cached
        // version list — no network, no registry call, no index read
        // (PROP-075 `#CHANNEL-SYNTAX`, `#ACTIVATION-HOOK`). A slot whose
        // package declares no such channel, or whose cache holds nothing,
        // stays `unresolved_channel`: a prompt hook must not fail a `cd`.
        RequestKind::Channel(name) => {
            let coordinate = package.map(str::to_owned).or_else(|| {
                let record = index
                    .installations
                    .values()
                    .find(|record| record.slot == slot)?;
                Some(format!("{}/{}", record.package.group, record.package.name))
            });
            let members = coordinate
                .as_deref()
                .zip(channels)
                .and_then(|(coordinate, resolver)| resolver.members(coordinate, name));
            let Some(members) = members else {
                selection.state = State::UnresolvedChannel;
                return selection;
            };
            providers
                .iter()
                .filter(|(_, version)| members.contains(version))
                .collect()
        }
    };
    let Some(newest) = matching.first() else {
        selection.state = State::NotInstalled;
        selection.install = install_command(index, slot, &request.authored);
        return selection;
    };
    let tied: Vec<String> = matching
        .iter()
        .filter(|(_, version)| *version == newest.1)
        .map(|(key, _)| (*key).clone())
        .collect();
    if tied.len() > 1 {
        selection.state = State::Ambiguous;
        selection.candidates = tied;
        return selection;
    }
    selection.key = Some(newest.0.clone());
    selection.version = Some(newest.1.to_string());
    selection
}

/// The exact `vibe install -g` that would supply `requested` for `slot`,
/// derived from a package the slot already has. `None` when the home knows
/// no package for the slot, in which case the caller's own refusal names the
/// unknown slot.
fn install_command(index: &Index, slot: &str, requested: &str) -> Option<String> {
    let record = index
        .installations
        .values()
        .find(|record| record.slot == slot)?;
    Some(format!(
        "vibe install -g {}/{}@{requested}",
        record.package.group, record.package.name
    ))
}

/// The selection of `slot` with nothing explicit about it: the slot's global
/// default, or `Off` when the slot is off. Used by `vibe app exec SLOT` and
/// by the activation report of a slot a version file named and nothing
/// selected.
pub fn slot_default(index: &Index, slot: &str) -> SlotSelection {
    let mut selection = SlotSelection {
        slot: slot.to_owned(),
        source: Source::SlotDefault,
        state: State::Selected,
        requested: None,
        key: None,
        version: None,
        file: None,
        unused: Vec::new(),
        candidates: Vec::new(),
        install: None,
    };
    let state = index.slots.get(slot);
    if state.is_some_and(|state| state.off) {
        selection.source = Source::SlotOff;
        selection.state = State::Off;
        return selection;
    }
    match state
        .and_then(|state| state.default.clone())
        .filter(|default| index.installations.contains_key(default.as_str()))
    {
        Some(default) => {
            selection.version = index
                .installations
                .get(default.as_str())
                .map(|record| record.package.version.clone());
            selection.key = Some(default);
            selection
        }
        None => {
            selection.source = Source::None;
            selection.state = State::NotInstalled;
            selection
        }
    }
}

#[cfg(test)]
#[path = "activation/tests.rs"]
mod tests;
