use std::path::Path;

use specmark::spec;
use vibe_core::PackageName;
use vibe_core::lifecycle::{ExtensionPoint, SlotPoint};
use vibe_lifecycle::{
    DependencyExtensionSource, DependencyProviderId, ExtensionProvider, ExtensionWorld,
    HandlerExecution, HostExtensionSource, SelectorSubject, SlotTarget, collect_extensions,
};
use vibe_workspace::install::ResolvedDep;

use crate::error::{Error, Result};
use crate::lifecycle::judgment::{SlotConsentRequest, SlotJudgmentContext, condition_lines};

#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#SURFACE-THE-RITUAL")]
pub struct SlotLifecyclePlanEntry {
    pub key: String,
    pub reference: String,
    pub point: String,
    pub provider: String,
    pub handler: String,
    pub tier: String,
    pub version: Option<String>,
    pub slot_target: SlotTarget,
    /// The contribution's own `id`, as the author wrote it.
    pub id: String,
    /// The human line of PROP-065 `#ordinary-grammar`, when declared.
    pub title: Option<String>,
    /// The group of the package that DECLARED the row — `None` when the
    /// host's own manifest did, which is what exempts it from the trust
    /// question of PROP-065 `#ordinary-trust`.
    pub declarant_group: Option<String>,
    /// Whether this contribution will run in this pass.
    pub run: bool,
    /// The deciding condition, or the unmet need, when it will not.
    pub reason: Option<String>,
    /// The declared `needs` this host does not meet.
    pub unmet_needs: Vec<String>,
    /// An unmet need or a failure is recorded and does not stop the rest.
    pub optional: bool,
    /// The conditions the verdict was decided from, for the plan's
    /// continuation lines.
    pub conditions: Vec<String>,
    pub(super) execution: HandlerExecution,
}

#[derive(Debug, Clone, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#SURFACE-THE-RITUAL")]
pub struct SlotLifecyclePlan {
    pub entries: Vec<SlotLifecyclePlanEntry>,
}

impl SlotLifecyclePlan {
    pub(super) fn for_targets(
        &self,
        targets: &[vibe_workspace::install::SlotLifecycleTarget],
    ) -> Self {
        Self {
            entries: self
                .entries
                .iter()
                .filter(|entry| {
                    targets.iter().any(|target| {
                        entry.slot_target.group == target.group.to_string()
                            && entry.slot_target.name == target.name
                            && entry.slot_target.version == target.version.to_string()
                    })
                })
                .cloned()
                .collect(),
        }
    }
}

impl SlotLifecyclePlanEntry {
    /// The `needs` continuation line of a plan row: what the contribution
    /// requires and, when this host does not provide it, which of them.
    ///
    /// ```
    /// use vibe_install::SlotLifecyclePlan;
    ///
    /// assert!(SlotLifecyclePlan::default().entries.is_empty());
    /// ```
    pub fn needs_line(&self) -> Option<String> {
        (!self.unmet_needs.is_empty()).then(|| format!("missing: {}", self.unmet_needs.join(", ")))
    }
}

impl SlotLifecyclePlan {
    /// Every contribution that will run, in plan order.
    pub fn running(&self) -> impl Iterator<Item = &SlotLifecyclePlanEntry> {
        self.entries.iter().filter(|entry| entry.run)
    }

    /// Refuse the installation up front when a contribution that will run
    /// needs something this host has not got (PROP-065 `#needs`,
    /// `#ordinary-grammar`). Checked for every such contribution **before
    /// anything starts**, so a late failure becomes an early refusal.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
    pub(super) fn preflight(&self) -> Result<()> {
        for entry in self.running() {
            if let Some(need) = entry.unmet_needs.first() {
                return Err(Error::SlotNeedUnmet {
                    package: format!(
                        "{}/{}@{}",
                        entry.slot_target.group, entry.slot_target.name, entry.slot_target.version
                    ),
                    contribution: entry.id.clone(),
                    point: entry.point.clone(),
                    need: need.clone(),
                });
            }
        }
        Ok(())
    }

    /// The question of PROP-065 `#ordinary-trust`, asked once per package
    /// and only about a dependency whose group is not trusted and at least
    /// one of whose contributions will actually run.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
    pub(super) fn admit_consent(&self, judgment: &SlotJudgmentContext) -> Result<()> {
        if judgment.assume_yes {
            return Ok(());
        }
        admit_consent_requests(self.consent_requests(judgment), judgment)
    }

    /// One request per untrusted dependency with running contributions, in
    /// plan order, each listing what it would run.
    fn consent_requests(&self, judgment: &SlotJudgmentContext) -> Vec<SlotConsentRequest> {
        let mut requests: Vec<SlotConsentRequest> = Vec::new();
        for entry in self.running() {
            let Some(group) = &entry.declarant_group else {
                continue;
            };
            if judgment.trusts(group) {
                continue;
            }
            let package = format!(
                "{}/{}@{}",
                entry.slot_target.group, entry.slot_target.name, entry.slot_target.version
            );
            let line = format!(
                "{} ({}) {}",
                entry.id,
                entry.handler,
                entry.title.as_deref().unwrap_or("-")
            );
            match requests
                .iter_mut()
                .find(|request| request.package == package)
            {
                Some(request) => request.contributions.push(line),
                None => requests.push(SlotConsentRequest {
                    package,
                    group: group.clone(),
                    contributions: vec![line],
                }),
            }
        }
        requests
    }
}

/// Ask the prepared requests without granting consent on absence or failure.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
fn admit_consent_requests(
    requests: Vec<SlotConsentRequest>,
    judgment: &SlotJudgmentContext,
) -> Result<()> {
    for request in requests {
        let granted = match &judgment.confirm {
            Some(ask) => ask(&request).map_err(|reason| {
                consent_refusal(&request, &format!("the question failed: {reason}"))
            })?,
            None => return Err(consent_refusal(&request, "this run has nobody to ask")),
        };
        if !granted {
            return Err(consent_refusal(&request, "the contributions were declined"));
        }
    }
    Ok(())
}

/// The trust refusal of one consent request: its contribution ids, the
/// reason the question did not grant it, and the agents.toml that would.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
fn consent_refusal(request: &SlotConsentRequest, reason: &str) -> Error {
    let ids: Vec<&str> = request
        .contributions
        .iter()
        .map(|line| line.split(' ').next().unwrap_or(line.as_str()))
        .collect();
    let exact_package = request
        .package
        .rsplit_once('@')
        .map(|(name, version)| format!("{name}@={}", version.trim_start_matches('=')))
        .unwrap_or_else(|| request.package.clone());
    Error::SlotConsentRequired {
        package: request.package.clone(),
        group: request.group.clone(),
        contributions: ids.join(", "),
        reason: reason.to_owned(),
        diagnostic: Box::new(crate::error::SlotConsentDiagnostic {
            retry: format!("vibe install {exact_package} --assume-yes"),
            agents_file: crate::error::agents_file_in(
                vibe_core::settings::settings_dir().as_deref(),
            ),
        }),
    }
}

/// Presentation-neutral pre-dispatch ritual observer.
///
/// ```
/// use vibe_install::{SlotLifecycleObserver, SlotLifecyclePlan};
/// struct Observer;
/// impl SlotLifecycleObserver for Observer {
///     fn observe(&self, plan: &SlotLifecyclePlan) -> Result<(), String> {
///         assert!(plan.entries.iter().all(|entry| entry.point.starts_with("slot:")));
///         Ok(())
///     }
/// }
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#SURFACE-THE-RITUAL")]
pub trait SlotLifecycleObserver {
    fn observe(&self, plan: &SlotLifecyclePlan) -> std::result::Result<(), String>;

    /// One contribution is about to run.
    ///
    /// The engine reports what a contribution DID; the moment it began is
    /// a clock reading, and the engine is handed no clock. A surface that
    /// draws a panel row or writes the step evidence of PROP-065
    /// `#ordinary-plan` needs that moment, and this is where it is taken.
    /// Called only for a contribution that will actually run: a skip is
    /// decided, never started.
    fn started(&self, _entry: &SlotLifecyclePlanEntry) -> std::result::Result<(), String> {
        Ok(())
    }

    /// Optional best-effort process output observation for this contribution.
    fn output_observer(
        &self,
        _entry: &SlotLifecyclePlanEntry,
    ) -> Option<std::sync::Arc<dyn vibe_lifecycle::process::ProcessOutputObserver>> {
        None
    }

    fn outcome(&self, _report: &super::SlotLifecycleReport) -> std::result::Result<(), String> {
        Ok(())
    }
}

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct NoSlotLifecycleObserver;
impl SlotLifecycleObserver for NoSlotLifecycleObserver {
    fn observe(&self, _plan: &SlotLifecyclePlan) -> std::result::Result<(), String> {
        Ok(())
    }
}

pub(super) fn build_slot_plan(
    installed: &[DependencyExtensionSource],
    host: &HostExtensionSource,
    resolution: &[ResolvedDep],
    judgment: &SlotJudgmentContext,
    offline: bool,
) -> Result<SlotLifecyclePlan> {
    let mut entries = Vec::new();
    for point in [SlotPoint::PreInstall, SlotPoint::PostInstall] {
        for target_dep in resolution {
            append_target_plan(
                &mut entries,
                installed,
                host,
                target_dep,
                point,
                judgment,
                offline,
            )?;
        }
    }
    Ok(SlotLifecyclePlan { entries })
}

fn append_target_plan(
    entries: &mut Vec<SlotLifecyclePlanEntry>,
    installed: &[DependencyExtensionSource],
    host: &HostExtensionSource,
    target_dep: &ResolvedDep,
    point: SlotPoint,
    judgment: &SlotJudgmentContext,
    offline: bool,
) -> Result<()> {
    let target_id = DependencyProviderId::new(
        target_dep.group.clone(),
        PackageName::parse(&target_dep.name)?,
    );
    let target_source = installed
        .iter()
        .find(|source| source.provider.id == target_id)
        .ok_or_else(|| {
            Error::Lifecycle(format!(
                "planned slot target `{target_id}` is absent from provisional world"
            ))
        })?;
    let target = slot_target(target_dep, &target_source.provider.root);
    let registry = collect_extensions(ExtensionWorld {
        installed: installed.to_vec(),
        host: host.clone(),
        effective_stack: None,
    })
    .map_err(|error| Error::Lifecycle(error.to_string()))?;
    for row in registry.plan(
        ExtensionPoint::Slot(point),
        SelectorSubject::package(&target_id),
    ) {
        let execution = HandlerExecution::from_row(row).with_slot_target(target.clone());
        let declared = row.declaration().judgment.as_ref();
        let slot = std::path::Path::new(&target.root);
        let (mut run, mut reason) = judgment.decide(declared, offline, slot);
        let needs = declared.map(|j| j.needs.as_slice()).unwrap_or_default();
        let optional = declared.is_some_and(|j| j.optional);
        let unmet = if run {
            judgment.unmet(needs, offline)
        } else {
            Vec::new()
        };
        // `#needs`: an optional contribution with an unmet need is skipped
        // and reported; a required one refuses the whole installation, which
        // the plan-wide preflight does once every verdict is known.
        if run && optional && !unmet.is_empty() {
            run = false;
            reason = Some(format!("unmet need `{}`", unmet.join("`, `")));
        }
        entries.push(SlotLifecyclePlanEntry {
            key: execution.key(),
            reference: execution.reference(),
            point: row.declaration().point.to_string(),
            provider: row.provider().to_string(),
            handler: row.declaration().handler.kind().to_string(),
            tier: tier_name(row.effective_tier()).into(),
            version: provider_version(row.provider()),
            slot_target: target.clone(),
            id: row.declaration().id.clone(),
            title: declared.and_then(|j| j.title.clone()),
            declarant_group: declarant_group(row.provider()),
            run,
            reason,
            unmet_needs: unmet,
            optional,
            conditions: condition_lines(declared),
            execution,
        });
    }
    Ok(())
}

fn slot_target(dep: &ResolvedDep, root: &Path) -> SlotTarget {
    SlotTarget {
        group: dep.group.to_string(),
        name: dep.name.clone(),
        version: dep.version.to_string(),
        kind: dep.kind.to_string(),
        root: vibe_core::machine_json_path(root),
    }
}

fn tier_name(tier: vibe_lifecycle::ContributionTier) -> &'static str {
    match tier {
        vibe_lifecycle::ContributionTier::Dependency => "dependency",
        vibe_lifecycle::ContributionTier::Preset => "preset",
        vibe_lifecycle::ContributionTier::HostDeclaration => "host-declaration",
        vibe_lifecycle::ContributionTier::HostActivation => "host-activation",
    }
}

/// The group of the package that declared the row, or `None` for the
/// host's own manifest — a project's own `vibe.toml` is the user's own
/// word, so it is never asked about (PROP-065 `#ordinary-trust`).
fn declarant_group(provider: &ExtensionProvider) -> Option<String> {
    match provider {
        ExtensionProvider::Dependency(provider) => Some(provider.id.group().to_string()),
        ExtensionProvider::Host(_) => None,
    }
}

fn provider_version(provider: &ExtensionProvider) -> Option<String> {
    match provider {
        ExtensionProvider::Dependency(provider) => Some(provider.version.clone()),
        ExtensionProvider::Host(provider) => {
            (!provider.version.is_empty()).then(|| provider.version.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic request, with prose deliberately unrelated to its package.
    fn request(package: &str) -> SlotConsentRequest {
        SlotConsentRequest {
            package: package.into(),
            group: "org.example".into(),
            contributions: vec![
                "judged (script) install org.other/wrong@9.9.9".into(),
                "maybe (agent) an optional explanation".into(),
            ],
        }
    }

    #[test]
    fn consent_refusal_pins_raw_and_exact_versions_without_parsing_prose() {
        for package in ["org.example/hooked@0.1.0", "org.example/hooked@=0.1.0"] {
            let Error::SlotConsentRequired {
                package: actual,
                group,
                contributions,
                reason,
                diagnostic,
            } = consent_refusal(&request(package), "synthetic refusal")
            else {
                panic!("expected a typed consent refusal");
            };
            assert_eq!(actual, package);
            assert_eq!(group, "org.example");
            assert_eq!(
                diagnostic.retry,
                "vibe install org.example/hooked@=0.1.0 --assume-yes"
            );
            assert_eq!(contributions, "judged, maybe");
            assert_eq!(reason, "synthetic refusal");
            assert_eq!(
                diagnostic.agents_file,
                crate::error::agents_file_in(vibe_core::settings::settings_dir().as_deref())
            );
        }
    }

    #[test]
    fn consent_refusal_uses_the_last_at_and_preserves_an_unversioned_package() {
        for (package, exact) in [
            ("org.example/hooked", "org.example/hooked"),
            (
                "org.example/hooked@label@1.0.0",
                "org.example/hooked@label@=1.0.0",
            ),
        ] {
            let Error::SlotConsentRequired { diagnostic, .. } =
                consent_refusal(&request(package), "declined")
            else {
                panic!("expected a typed consent refusal");
            };
            assert_eq!(
                diagnostic.retry,
                format!("vibe install {exact} --assume-yes")
            );
        }
    }

    #[test]
    fn consent_question_failure_refuses_without_granting() {
        let judgment = SlotJudgmentContext {
            confirm: Some(std::sync::Arc::new(|_| Err("synthetic failure".into()))),
            assume_yes: false,
            ..SlotJudgmentContext::consenting()
        };
        assert_refusal(judgment, "the question failed: synthetic failure");
    }

    #[test]
    fn consent_without_a_question_refuses_without_granting() {
        assert_refusal(
            SlotJudgmentContext {
                assume_yes: false,
                ..SlotJudgmentContext::consenting()
            },
            "this run has nobody to ask",
        );
    }

    #[test]
    fn declined_consent_refuses_without_granting() {
        let judgment = SlotJudgmentContext {
            confirm: Some(std::sync::Arc::new(|_| Ok(false))),
            assume_yes: false,
            ..SlotJudgmentContext::consenting()
        };
        assert_refusal(judgment, "the contributions were declined");
    }

    /// All three question outcomes retain the same typed, fail-closed refusal.
    fn assert_refusal(judgment: SlotJudgmentContext, expected_reason: &str) {
        let error = admit_consent_requests(vec![request("org.example/hooked@0.1.0")], &judgment)
            .expect_err("no refusal outcome may grant consent");
        let Error::SlotConsentRequired {
            contributions,
            reason,
            diagnostic,
            ..
        } = error
        else {
            panic!("expected a typed consent refusal");
        };
        assert_eq!(reason, expected_reason);
        assert_eq!(contributions, "judged, maybe");
        assert_eq!(
            diagnostic.retry,
            "vibe install org.example/hooked@=0.1.0 --assume-yes"
        );
    }
}
