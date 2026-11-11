//! Short-name resolution at the **application** command boundary
//! (PROP-008 §2.6 `#short-name`, PROP-059 `#commands`).
//!
//! `vibe install -g node` is the same convenience `vibe install wal` has
//! had since M1.19, brought to the commands that act on user applications.
//! The machinery is the one that already exists — the index layer's
//! `by-name/<name>.json` candidate set reached through the ordinary
//! resolver, and the PROP-008 §2.7 collision refusal carrying
//! `InstallError::AmbiguousPackage` (exit 7). Nothing here enumerates
//! registries on its own: the resolver this calls is the resolver the
//! command would have resolved the package with, so a registry the
//! effective source policy excludes is not contacted here either.
//!
//! **Nothing bare is ever stored.** PROP-029 `#ADDR-SHORT-NAMES`: the
//! short name is a one-time human CLI input. Qualification happens before
//! the spelling reaches the library, so every record, lockfile, version
//! file and receipt downstream sees `<group>/<name>` exactly as if the
//! operator had typed it.
//!
//! **Two sources, in PROP-008's own order.** A verb acting on installed
//! state resolves from this home's inventory alone
//! (`#INSTALLED-STATE-RESOLVES-LOCALLY` — the application index is to
//! `vibe app hold` what `vibe.lock` is to `vibe uninstall`): no index, no
//! network, and a name the home does not carry is "not installed". A verb
//! that resolves a package out of a registry prefers what this home
//! already installed (`#LOCKFILE-AUTHORITATIVE`) and falls through to the
//! registries' index layer.
//!
//! **A registry may offer a short spelling of its own.** `qwen` reaches
//! `org.vibevm.qwen/qwen-code` when the registry carrying that package
//! says the spelling means it (PROP-008 §2.6, `vibe_index::aliases`).
//! The alias is the registry OWNER's, never the package author's, and it
//! is always outranked: a package actually named `qwen`, in any consulted
//! registry, wins outright and the alias is not even weighed. Two
//! registries offering the same target are one candidate; two targets are
//! the §2.7 collision. The one resolved line names the registry that
//! decided, because a spelling that reaches a differently-named package
//! must never arrive unattributed.
//!
//! **Trust narrows the registry candidates.** `org.vibevm.*` is trusted by
//! default and `untrusted_groups` takes that back (PROP-065
//! `#ordinary-trust`, `vibe_core::trust`). An untrusted group's package is
//! never *picked for* an operator who did not name it — installing it runs
//! that author's code — so it is listed as untrusted and never chosen.
//! Installed candidates are not filtered: the operator already consented
//! to that package by installing it, and a verb that acts on installed
//! state must be able to name what is there.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");

use std::path::Path;

use anyhow::Result;
use vibe_app::{Index, SettingsRoot};
use vibe_core::Group;
use vibe_core::trust::TrustPolicy;

use vibe_registry::short_name::{AliasCandidate, ShortNameCandidates};

use crate::exit_code::InstallError;
use crate::output;

/// What the boundary says: the provenance line, the two refusals and the
/// spelling helpers, pure over values.
mod render;
/// The commands whose argument may be an installed slot id as well as a
/// package, and the precedence that decides between them.
mod slots;

pub(crate) use slots::{slot_target, versions_target};

use render::{bare_name, provenance, render_collision, render_not_found, splice, splice_alias};

/// One candidate group of a bare name, with the trust verdict that decides
/// whether it may be picked for an operator who did not name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Candidate {
    group: Group,
    /// Set when this candidate was reached through a registry's ALIAS
    /// for the spelling rather than through a package that bears it
    /// (PROP-008 §2.6): the package the registry owner points the
    /// spelling at, and which registries point it there. An alias
    /// candidate only ever exists when no registry carries the name for
    /// real — a name somebody publishes is never displaced by a
    /// spelling somebody offers.
    alias: Option<AliasCandidate>,
    /// Whether the effective trust policy covers the group. An installed
    /// candidate is always eligible — see the module note.
    eligible: bool,
}

impl Candidate {
    /// A candidate the operator already installed: eligible whatever any
    /// trust list says, because installing it was the consent.
    fn installed(group: Group) -> Self {
        Candidate {
            group,
            alias: None,
            eligible: true,
        }
    }

    /// A registry candidate, eligible only from a trusted group.
    fn offered(group: Group, trust: &TrustPolicy) -> Self {
        let eligible = trust.trusts(group.as_str());
        Candidate {
            group,
            alias: None,
            eligible,
        }
    }

    /// A candidate reached through a registry's alias. Trust is asked
    /// about the TARGET's group — the group whose code an install would
    /// run — and never about the registry that offered the spelling: an
    /// alias moves no code and vouches for nothing.
    fn aliased(candidate: AliasCandidate, trust: &TrustPolicy) -> Self {
        let eligible = trust.trusts(candidate.target.group.as_str());
        Candidate {
            group: candidate.target.group.clone(),
            alias: Some(candidate),
            eligible,
        }
    }

    /// The package this candidate resolves to — the bare name itself for
    /// an ordinary candidate, the alias target's name for an alias.
    fn package_name<'a>(&'a self, name: &'a str) -> &'a str {
        match &self.alias {
            Some(candidate) => &candidate.target.name,
            None => name,
        }
    }

    /// `org.vibevm.node/node`, or the alias's own provenance spelling,
    /// each with the untrusted mark when it carries one.
    fn spelling(&self, name: &str) -> String {
        let base = match &self.alias {
            Some(candidate) => candidate.provenance(),
            None => format!("{}/{name}", self.group),
        };
        match self.eligible {
            true => base,
            false => format!("{base} (untrusted group)"),
        }
    }
}

/// What a lookup of one bare name found, and where it looked.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Lookup {
    /// Every candidate, trusted or not, sorted and de-duplicated by group.
    candidates: Vec<Candidate>,
    /// The sources that actually answered, named as the operator declared
    /// them, for the not-found sentence.
    consulted: Vec<String>,
    /// The hosting keys the effective source policy excludes, so the
    /// not-found sentence can name what was deliberately not asked.
    skipped: Vec<String>,
    /// Whether the run was `--offline`, which limits the walk to the
    /// cached index.
    offline: bool,
    /// Whether this lookup read **only** this home's inventory, so its
    /// refusal must not reach for a registry it never asked
    /// (`#INSTALLED-STATE-RESOLVES-LOCALLY`).
    local: bool,
}

impl Lookup {
    fn sorted(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
        // Keyed on the PACKAGE each candidate resolves to, not on its
        // group alone: two registries offering the same alias target
        // are one candidate, and two aliases into one group that name
        // different packages are two.
        let key = |c: &Candidate| (c.group.as_str().to_owned(), c.package_name("").to_owned());
        candidates.sort_by_key(key);
        candidates.dedup_by_key(|c| key(c));
        candidates
    }

    fn of_installed(groups: Vec<Group>) -> Self {
        Lookup {
            candidates: Self::sorted(groups.into_iter().map(Candidate::installed).collect()),
            local: true,
            ..Default::default()
        }
    }

    fn eligible(&self) -> Vec<&Candidate> {
        self.candidates.iter().filter(|c| c.eligible).collect()
    }
}

/// The verdict of one bare name, before it is rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    /// Exactly one eligible candidate.
    Resolved(Box<Candidate>),
    /// Two or more eligible candidates — PROP-008 §2.7, exit 7.
    Ambiguous,
    /// None eligible; there may still be untrusted candidates to name.
    NotFound,
}

/// The pure decision over a candidate set: one eligible candidate is the
/// answer, several are the collision, none is the not-found. Trust has
/// already been applied, so this is the same three-way verdict
/// `vibe install` has always made.
fn decide(lookup: &Lookup) -> Outcome {
    match lookup.eligible().as_slice() {
        [only] => Outcome::Resolved(Box::new((*only).clone())),
        [] => Outcome::NotFound,
        _ => Outcome::Ambiguous,
    }
}

/// Every group of this home's inventory that installed a package of the
/// bare `name` (`#INSTALLED-STATE-RESOLVES-LOCALLY`).
fn installed_groups(index: &Index, name: &str) -> Vec<Group> {
    index
        .installations
        .values()
        .filter(|record| record.package.name == name)
        .filter_map(|record| Group::parse(&record.package.group).ok())
        .collect()
}

/// Turn one verdict into the qualified spelling, printing the provenance
/// line, or into the refusal the verdict is.
fn settle(
    ctx: &output::Context,
    command: &str,
    spelling: &str,
    name: &str,
    lookup: &Lookup,
) -> Result<String> {
    match decide(lookup) {
        Outcome::Resolved(candidate) => {
            // One line, on stderr in every mode: it is what vibe decided,
            // not what the command answered, so `--json`'s stdout stays one
            // envelope and `--quiet`'s one line is still the command's own.
            // An alias says so, and says which registry offered it: a
            // spelling that reaches a differently-named package must
            // never arrive without naming who decided that.
            ctx.decision(&provenance(name, &candidate));
            Ok(match &candidate.alias {
                Some(alias) => splice_alias(spelling, &alias.target),
                None => splice(spelling, &candidate.group),
            })
        }
        // PROP-008 §2.7 — the typed refusal that carries exit 7, returned
        // unchanged so the exit mapper's downcast still finds it.
        Outcome::Ambiguous => {
            Err(InstallError::AmbiguousPackage(render_collision(command, name, lookup)).into())
        }
        Outcome::NotFound => anyhow::bail!(render_not_found(command, name, lookup)),
    }
}

/// Qualify a package spelling for a command that acts on **installed**
/// applications: `vibe uninstall -g`, `vibe app hold`, `vibe app
/// launchers` and their kin. The home's inventory is the whole source
/// (`#INSTALLED-STATE-RESOLVES-LOCALLY`) — no index is read and no
/// request is made.
pub(crate) fn installed(
    ctx: &output::Context,
    command: &str,
    settings: &SettingsRoot,
    spelling: &str,
) -> Result<String> {
    let Some(name) = bare_name(spelling) else {
        return Ok(spelling.to_owned());
    };
    let index = Index::load(settings)?;
    let lookup = Lookup::of_installed(installed_groups(&index, &name));
    settle(ctx, command, spelling, &name, &lookup)
}

/// The sources a registry-backed qualification walks — exactly the ones
/// the command itself would have resolved the package through, so no
/// second registry chain and no registry filter of our own exists here.
pub(crate) struct Sources<'a> {
    /// The home whose installed applications answer first — the project
    /// home inside a project, the user settings root otherwise.
    pub(crate) home: &'a SettingsRoot,
    /// The user settings root the configured registries and the trust
    /// lists are read from.
    pub(crate) settings: &'a SettingsRoot,
    /// The registry inputs of the command itself.
    pub(crate) inputs: Inputs<'a>,
}

/// The registry inputs one command already carries, as one value: the
/// boundary walks exactly these and composes no chain of its own.
#[derive(Clone, Copy)]
pub(crate) struct Inputs<'a> {
    /// An explicit `--registry DIR`: that directory is then the whole
    /// source, as it is for the resolution that follows.
    pub(crate) registry: Option<&'a Path>,
    pub(crate) embedded: Option<&'a Path>,
    pub(crate) offline: bool,
    pub(crate) operation: &'a vibe_package_source::SourceOperation,
}

/// [`Inputs`] from the four values a command already holds, so a call site
/// stays one expression.
pub(crate) fn inputs<'a>(
    registry: Option<&'a Path>,
    embedded: Option<&'a Path>,
    offline: bool,
    operation: &'a vibe_package_source::SourceOperation,
) -> Inputs<'a> {
    Inputs {
        registry,
        embedded,
        offline,
        operation,
    }
}

impl Sources<'_> {
    /// The hosting keys the effective policy excludes, named so a
    /// not-found says what was deliberately not asked. A policy that
    /// cannot be read says nothing rather than failing the lookup — the
    /// resolution that follows reports that fault properly.
    fn skipped(&self) -> Vec<String> {
        self.inputs
            .operation
            .source_policy()
            .map(|policy| {
                policy
                    .excluded_selectors()
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The names of the registries a resolver actually walks, in declared
/// order. Read off the resolver the command uses, never re-enumerated
/// from configuration: a registry the effective source policy excluded is
/// already absent here.
fn consulted(resolver: &vibe_package_source::InstallResolver) -> Vec<String> {
    use vibe_package_source::InstallResolver;
    let local = |registry: &vibe_registry::LocalRegistry| registry.root().display().to_string();
    let declared = |multi: &vibe_registry::MultiRegistryResolver| -> Vec<String> {
        multi
            .sources()
            .map(|sources| {
                sources
                    .iter()
                    .map(|source| source.name().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    match resolver {
        InstallResolver::Local(registry, _) => vec![local(registry)],
        InstallResolver::LockedLocal {
            local: registry,
            held,
            ..
        } => {
            let mut names = vec![local(registry)];
            names.extend(declared(held));
            names
        }
        InstallResolver::Multi(multi, _) => declared(multi),
        InstallResolver::Embedded {
            locals,
            declared: d,
            ..
        } => {
            let mut names: Vec<String> = locals.iter().map(local).collect();
            if let Some(multi) = d {
                names.extend(declared(multi));
            }
            names
        }
    }
}

/// Qualify a package spelling for a command that resolves the package out
/// of a registry: `vibe install -g`, `vibe app installers|plan|steps|
/// describe|versions`.
///
/// This home's installed packages are preferred, exactly as `vibe.lock` is
/// for a project (`#LOCKFILE-AUTHORITATIVE`); with an explicit
/// `--registry` that preference is skipped, because the named directory is
/// then the whole source of the resolution that follows and a preference
/// pointing elsewhere would only fail later.
pub(crate) fn from_registries(
    ctx: &output::Context,
    command: &str,
    sources: &Sources<'_>,
    spelling: &str,
) -> Result<String> {
    let Some(name) = bare_name(spelling) else {
        return Ok(spelling.to_owned());
    };
    let lookup = lookup(sources, &name)?;
    settle(ctx, command, spelling, &name, &lookup)
}

/// As [`from_registries`], but a bare name **no** candidate matches is
/// handed back unchanged instead of refused, as `Ok(None)`.
///
/// It exists for `vibe app versions`, whose argument may also be a slot
/// id: its own refusal names the slots this home records, which is the
/// fact that command's reader needs and this boundary cannot know. A
/// collision still refuses here, and so does a candidate set that exists
/// but is entirely untrusted — those are facts the library's refusal
/// cannot have either way.
fn from_registries_or_unchanged(
    ctx: &output::Context,
    command: &str,
    sources: &Sources<'_>,
    name: &str,
) -> Result<Option<String>> {
    let lookup = lookup(sources, name)?;
    if lookup.candidates.is_empty() {
        return Ok(None);
    }
    settle(ctx, command, name, name, &lookup).map(Some)
}

/// The candidate set of one bare name over the command's own sources.
fn lookup(sources: &Sources<'_>, name: &str) -> Result<Lookup> {
    let explicit = sources.inputs.registry.is_some();
    if !explicit {
        // What this home already installed answers first and locally.
        let installed = Lookup::of_installed(installed_groups(&Index::load(sources.home)?, name));
        if !installed.candidates.is_empty() {
            return Ok(installed);
        }
    }
    let trust = trust_policy(sources.settings)?;
    // One gatherer for every source: the real names become ordinary
    // candidates, the registry-offered spellings become alias ones. The
    // gather step has already dropped every alias when any registry
    // carries the name for real, so nothing here has to rank them.
    let offered = |found: ShortNameCandidates| -> Vec<Candidate> {
        Lookup::sorted(
            found
                .exact
                .into_iter()
                .map(|group| Candidate::offered(group, &trust))
                .chain(
                    found
                        .aliases
                        .into_iter()
                        .map(|candidate| Candidate::aliased(candidate, &trust)),
                )
                .collect(),
        )
    };
    let from_local = |root: &Path| -> Result<ShortNameCandidates> {
        let registry = open_local(root)?;
        Ok(ShortNameCandidates::gather([(
            root.display().to_string(),
            registry.indexed_name_lookup(name)?,
        )]))
    };
    if let Some(root) = sources.inputs.registry {
        return Ok(Lookup {
            candidates: offered(from_local(root)?),
            consulted: vec![root.display().to_string()],
            skipped: sources.skipped(),
            offline: sources.inputs.offline,
            local: false,
        });
    }
    // The embedded registry is first in the chain, not the whole chain
    // (PROP-030 `#precedence`): a name it does not carry falls through.
    if let Some(root) = sources.inputs.embedded {
        let found = from_local(root)?;
        if !found.is_empty() {
            return Ok(Lookup {
                candidates: offered(found),
                consulted: vec![root.display().to_string()],
                skipped: sources.skipped(),
                offline: sources.inputs.offline,
                local: false,
            });
        }
    }
    let resolver = vibe_app::global_resolver(
        sources.settings.path(),
        name,
        sources.inputs.offline,
        sources.inputs.operation,
    )
    .map_err(vibe_app::AppError::from)?;
    Ok(Lookup {
        candidates: offered(resolver.short_name_candidates(name)?),
        consulted: consulted(&resolver),
        skipped: sources.skipped(),
        offline: sources.inputs.offline,
        local: false,
    })
}

fn open_local(root: &Path) -> Result<vibe_registry::LocalRegistry> {
    crate::registry::application_local_registry(root.to_path_buf()).map_err(|error| {
        anyhow::anyhow!(
            "opening the application registry `{}`: {error}",
            root.display()
        )
    })
}

/// The effective trust of this run — the one policy `vibe_core::trust`
/// resolves from the base list, the site profile and `agents.toml`.
///
/// A trust entry vibe cannot read refuses the lookup rather than being
/// dropped, for the reason the install boundary gives: a dropped entry
/// would either grant trust nobody wrote or withhold trust someone did.
fn trust_policy(settings: &SettingsRoot) -> Result<TrustPolicy> {
    let config = vibe_app::AgentsConfig::load(settings).map_err(vibe_app::AppError::from)?;
    Ok(vibe_app::effective_trust(settings, Some(&config)).map_err(vibe_app::AppError::from)?)
}

#[cfg(test)]
#[path = "short_name/tests.rs"]
mod tests;
