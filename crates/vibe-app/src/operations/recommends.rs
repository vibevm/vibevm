//! An application's `[recommends]`, installed by the application installer
//! best-effort (`PROP-077#APP-RECOMMENDS`,
//! `#APP-RECOMMENDS-MECHANICS`): a breadth-first walk in declaration order
//! over a visited set, each installation the ordinary one and each run after
//! the recommender's own record is committed, so a failed recommendation can
//! never leave the recommender half-installed and never changes the exit
//! status of the install the user asked for.
//!
//! `[suggests]` is read here too and installed nowhere: the level means
//! "never auto-installed", so the walk only reports it
//! (`PROP-077#SUGGESTS-UNCHANGED`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS");

use std::collections::{BTreeSet, VecDeque};

use specmark::spec;
use vibe_core::manifest::RequirementScope;

use crate::error::AppError;
use crate::index::{Index, Mark, Requirement};
use crate::resolve::{ResolvedPackage, qualified_ref};
use crate::select::{InstallChoice, select_option};

use super::{InstallRequest, OperationContext};

/// What became of one `[recommends]` reference
/// (`PROP-077#APP-RECOMMENDS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS")]
pub enum RecommendationOutcome {
    /// Installed by this walk, exposed, `auto`, with a weak requirer edge.
    Installed,
    /// An installed version already satisfies it; nothing was installed.
    Satisfied,
    /// Reported and not installed — never a failure of the install the user
    /// asked for.
    Skipped,
    /// `--plan-only`: this is what the walk would install.
    Planned,
}

impl RecommendationOutcome {
    /// The stable spelling the report and the plan carry.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Satisfied => "satisfied",
            Self::Skipped => "skipped",
            Self::Planned => "planned",
        }
    }
}

/// One line of the recommendation report: what was recommended, by whom, and
/// what became of it (`PROP-077#APP-RECOMMENDS`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS")]
pub struct Recommendation {
    /// The authored `group/name[@requirement]` spelling.
    pub package: String,
    /// The installation key of the package that recommended it.
    pub by: String,
    /// The installation key, when one is installed or already satisfied.
    pub key: Option<String>,
    pub outcome: RecommendationOutcome,
    /// Why it was skipped, in one line; absent otherwise.
    pub reason: Option<String>,
}

/// The `[recommends]` references of one resolved package, in declaration
/// order.
fn references(resolved: &ResolvedPackage) -> Vec<String> {
    resolved
        .recommends
        .packages
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Whether an installed version already satisfies `spelling`, and which key
/// it is. A kept previous installation does not count.
fn satisfied_by(index: &Index, spelling: &str) -> Option<String> {
    let requested = qualified_ref(spelling).ok()?;
    let group = requested.group.as_ref()?.to_string();
    let mut found: Vec<(semver::Version, String)> = index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.package.group == group
                && record.package.name == requested.name.as_str()
                && record.lifecycle.superseded_by.is_none()
        })
        .filter_map(|(key, record)| {
            let version = semver::Version::parse(&record.package.version).ok()?;
            requested
                .version
                .matches(&version)
                .then_some((version, key.clone()))
        })
        .collect();
    found.sort_by(|left, right| left.0.cmp(&right.0));
    found.pop().map(|(_, key)| key)
}

/// One reference resolved far enough to say whether the application plane
/// can install it at all. Every refusal is a report line and a skip, never
/// an error (`PROP-077#APP-RECOMMENDS`).
enum Admitted {
    Installable(Box<ResolvedPackage>),
    Skip(String),
}

fn admit(ctx: &OperationContext<'_>, spelling: &str) -> Admitted {
    let Some(resolver) = ctx.resolver else {
        return Admitted::Skip("no package resolver is available in this context".into());
    };
    if let Err(error) = qualified_ref(spelling) {
        return Admitted::Skip(error.to_string());
    }
    let resolved = match resolver(spelling) {
        Ok(resolved) => resolved,
        // One refusal covers the three the fact names separately — a package
        // no configured registry carries, a requirement nothing satisfies,
        // and an unreadable coordinate — because the resolver's own message
        // already says which it was.
        Err(error) => return Admitted::Skip(error.to_string()),
    };
    if resolved.kind != vibe_core::PackageKind::App {
        return Admitted::Skip(format!(
            "`{}` is kind `{}`, and only an application package can be installed beside one",
            resolved.package.key(),
            resolved.kind.as_str()
        ));
    }
    if let Err(error) = select_option(&resolved.options, &InstallChoice::default(), None) {
        return Admitted::Skip(error.to_string());
    }
    Admitted::Installable(Box::new(resolved))
}

/// The breadth-first frontier of the walk: the recommender's key and the
/// references it declares, in declaration order.
struct Frontier {
    by: String,
    references: Vec<String>,
}

/// Seed a walk at `resolved`: its own `(group, name)` is visited, so a
/// package that recommends itself terminates at once.
fn seed(resolved: &ResolvedPackage) -> (BTreeSet<(String, String)>, VecDeque<Frontier>) {
    let mut visited = BTreeSet::new();
    visited.insert((
        resolved.package.group.clone(),
        resolved.package.name.clone(),
    ));
    let mut queue = VecDeque::new();
    queue.push_back(Frontier {
        by: resolved.package.key(),
        references: references(resolved),
    });
    (visited, queue)
}

/// The key of the next reference, or `None` when it has already been
/// walked — the visited set is what makes a cycle terminate
/// (`PROP-077#APP-RECOMMENDS-MECHANICS`).
fn first_visit(visited: &mut BTreeSet<(String, String)>, spelling: &str) -> bool {
    match qualified_ref(spelling) {
        Ok(requested) => {
            let Some(group) = requested.group.as_ref().map(ToString::to_string) else {
                return true;
            };
            visited.insert((group, requested.name.to_string()))
        }
        // An unparseable spelling has no coordinate to remember; it is
        // reported once per recommender, which is where it was written.
        Err(_) => true,
    }
}

/// `--plan-only`: the whole walk, resolved and printed, downloading no
/// payload and installing nothing (`PROP-077#APP-RECOMMENDS-MECHANICS`).
pub(super) fn plan(
    ctx: &OperationContext<'_>,
    index: &Index,
    resolved: &ResolvedPackage,
) -> Vec<Recommendation> {
    let (mut visited, mut queue) = seed(resolved);
    let mut out = Vec::new();
    while let Some(frontier) = queue.pop_front() {
        for spelling in frontier.references {
            if !first_visit(&mut visited, &spelling) {
                continue;
            }
            let mut line = Recommendation {
                package: spelling.clone(),
                by: frontier.by.clone(),
                key: None,
                outcome: RecommendationOutcome::Planned,
                reason: None,
            };
            if let Some(key) = satisfied_by(index, &spelling) {
                line.key = Some(key);
                line.outcome = RecommendationOutcome::Satisfied;
                out.push(line);
                continue;
            }
            match admit(ctx, &spelling) {
                Admitted::Installable(next) => {
                    line.key = Some(next.package.key());
                    queue.push_back(Frontier {
                        by: next.package.key(),
                        references: references(&next),
                    });
                }
                Admitted::Skip(reason) => {
                    line.outcome = RecommendationOutcome::Skipped;
                    line.reason = Some(reason);
                }
            }
            out.push(line);
        }
    }
    out
}

/// The walk itself, run under the held lock **after** the recommender's
/// record is committed. Every failure is a report line and exit 0 for the
/// install the user asked for (`PROP-077#APP-RECOMMENDS`).
pub(super) fn install(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Vec<Recommendation>, AppError> {
    let (mut visited, mut queue) = seed(resolved);
    let mut out = Vec::new();
    while let Some(frontier) = queue.pop_front() {
        for spelling in frontier.references {
            if !first_visit(&mut visited, &spelling) {
                continue;
            }
            let mut line = Recommendation {
                package: spelling.clone(),
                by: frontier.by.clone(),
                key: None,
                outcome: RecommendationOutcome::Skipped,
                reason: None,
            };
            let index = Index::load(ctx.settings)?;
            // Already present: nothing is installed and the walk stops
            // here, exactly as `plan` stops — but the edge is recorded
            // anyway, because a recommender's `requires` list is rewritten
            // from its plan at every install and a second `install -g` of
            // it would otherwise drop the edge and orphan the companion.
            if let Some(key) = satisfied_by(&index, &spelling) {
                line.key = Some(key.clone());
                line.outcome = RecommendationOutcome::Satisfied;
                record_edge(lock, ctx, &frontier.by, &spelling, &key)?;
                out.push(line);
                continue;
            }
            match admit(ctx, &spelling) {
                Admitted::Skip(reason) => line.reason = Some(reason),
                Admitted::Installable(next) => {
                    let key = next.package.key();
                    line.key = Some(key.clone());
                    // The ordinary installation: the ordinary resolution,
                    // the ordinary installer options and defaults, the
                    // ordinary record — `auto`, and **exposed**, because a
                    // recommended maven the user cannot run is pointless
                    // (`PROP-077#WEAK-OPEN` W3).
                    let child = InstallRequest {
                        mark: Mark::Auto,
                        accept_license: request.accept_license,
                        ..InstallRequest::default()
                    };
                    match super::install_locked(
                        lock,
                        ctx,
                        &next,
                        &child,
                        &mut Vec::new(),
                        None,
                        None,
                    ) {
                        Ok(_) => {
                            line.outcome = RecommendationOutcome::Installed;
                            record_edge(lock, ctx, &frontier.by, &spelling, &key)?;
                            queue.push_back(Frontier {
                                by: key,
                                references: references(&next),
                            });
                        }
                        Err(error) => line.reason = Some(error.to_string()),
                    }
                }
            }
            out.push(line);
        }
    }
    Ok(out)
}

/// Record the weak requirer edge on the recommender's own record: `auto`
/// with the recommender as its requirer, exactly as a step requirement
/// already is, with the one attribute that distinguishes it
/// (`PROP-077#APP-RECOMMENDS-MECHANICS`, PROP-065 `#uninstall-tree`).
fn record_edge(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    by: &str,
    spelling: &str,
    key: &str,
) -> Result<(), AppError> {
    let mut index = Index::load(ctx.settings)?;
    let Some(owner) = index.installations.get_mut(by) else {
        return Ok(());
    };
    if owner
        .lifecycle
        .requires
        .iter()
        .any(|edge| edge.key == key && edge.weak)
    {
        return Ok(());
    }
    owner.lifecycle.requires.push(Requirement {
        package: spelling.to_owned(),
        key: key.to_owned(),
        scope: RequirementScope::Runtime,
        step: None,
        weak: true,
    });
    // The weak edge is the first thing in a home that needs the
    // `vibe-user-applications/8` reader, so the publication admits and
    // refreshes the capable proxy image first (PROP-059
    // `#proxy-index-upgrade`).
    let image = crate::proxies::for_publication(ctx.settings, &index, ctx.proxy_image)?;
    index.publish_locked(
        ctx.settings,
        &crate::catalog::HomeEvent::about(crate::catalog::EventKind::Updated, by.to_owned()),
        lock,
        image.as_ref(),
    )?;
    Ok(())
}

/// The walk, run after one command's own installation is published, and the
/// `[suggests]` line that rides with it. Entered only when the command the
/// user typed asked for it, and never for a requirement install, an import
/// replay, a resume or a recommendation of the walk itself. A failure of the
/// walk is reported on the outcome and never changes the exit status of the
/// install the user asked for (`PROP-077#APP-RECOMMENDS`).
pub(super) fn after(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
    mut outcome: super::Outcome,
) -> super::Outcome {
    // A suggestion installs nothing, so it is reported whether or not the
    // user declined the recommendations (`PROP-077#SUGGESTS-UNCHANGED`).
    outcome.suggested = resolved
        .suggests
        .packages
        .iter()
        .map(ToString::to_string)
        .collect();
    if !request.recommends {
        return outcome;
    }
    match install(lock, ctx, resolved, request) {
        Ok(recommended) => outcome.recommended = recommended,
        // The walk's own machinery failing — an unreadable inventory, a
        // refused save — is a report line of its own rather than a failure
        // of the install that already succeeded.
        Err(error) => {
            outcome.recommended = vec![Recommendation {
                package: String::new(),
                by: resolved.package.key(),
                key: None,
                outcome: RecommendationOutcome::Skipped,
                reason: Some(error.to_string()),
            }];
        }
    }
    outcome
}
