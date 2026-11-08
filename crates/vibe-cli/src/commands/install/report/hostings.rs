//! The hostings a plan would contact, aggregated from evidence it already holds.
//!
//! VIBE-NEEDS 3 (optional item): list, for a project or for the user home,
//! every hosting an operation would contact, so the effect of a
//! `--prefer-hosting` / `--exclude-hosting` switch is visible before the
//! press. The aggregate is a pure projection of the plan's own captured
//! source evidence and candidate observations: building it reads no
//! configuration and contacts nothing.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#HOSTING-OBSERVATION");

use std::collections::{BTreeMap, BTreeSet};

use vibe_core::manifest::HostingSelector;
use vibe_core::remap::endpoint::{EndpointObservation, observe_endpoint};
use vibe_wire::generated::install::e2::plan::{
    CandidateDecision, CandidateObservation, InstallPlan, SourceCandidate,
};
use vibe_wire::generated::shared::{OperationHosting, SourceEvidence};

#[derive(Default)]
struct Entry {
    excluded: bool,
    endpoints: BTreeSet<String>,
    packages: BTreeSet<String>,
}

/// Hosting key → aggregated observations, in key order.
#[derive(Default)]
pub(crate) struct Hostings(BTreeMap<String, Entry>);

/// The `--prefer-hosting` / `--exclude-hosting` key of a network URL: a
/// known managed family (`github`, `gitverse`) or `host:HOST`. Local and
/// opaque spellings have no hosting and are never listed.
fn hosting_key(url: &str) -> Option<String> {
    let EndpointObservation::Network(authority) = observe_endpoint(url) else {
        return None;
    };
    Some(
        [HostingSelector::Github, HostingSelector::Gitverse]
            .into_iter()
            .find(|family| family.host_keys().contains(&authority.host.as_str()))
            .map_or_else(
                || format!("host:{}", authority.host),
                |family| family.to_string(),
            ),
    )
}

impl Hostings {
    fn observe(&mut self, package: &str, url: Option<&str>, excluded: bool) {
        let Some(url) = url else { return };
        let Some(key) = hosting_key(url) else { return };
        let entry = self.0.entry(key).or_default();
        entry.excluded |= excluded;
        entry
            .endpoints
            .insert(vibe_core::progress::transfer::safe_remote(url));
        entry.packages.insert(package.to_owned());
    }

    /// The selected source and its differing acquisition route, if any.
    pub(crate) fn selected(&mut self, package: &str, evidence: Option<&SourceEvidence>) {
        let Some(evidence) = evidence else { return };
        self.observe(package, evidence.source_url.as_deref(), false);
        self.observe(package, evidence.fetched_from.as_deref(), false);
    }

    /// Probed candidates and policy-excluded ones; a candidate never probed
    /// was not contacted and is not listed.
    fn candidates(&mut self, package: &str, rows: Option<&[SourceCandidate]>) {
        for row in rows.unwrap_or_default() {
            if matches!(row.decision, CandidateDecision::ExcludedHost) {
                let endpoint = row
                    .excluded_endpoint
                    .as_deref()
                    .or(row.source_evidence.source_url.as_deref());
                self.observe(package, endpoint, true);
            } else if !matches!(row.observation, CandidateObservation::NotProbed) {
                self.observe(package, row.source_evidence.source_url.as_deref(), false);
            }
        }
    }

    /// Every package row and remap offer of a project plan.
    pub(crate) fn of_plan(plan: &InstallPlan) -> Self {
        let mut hostings = Self::default();
        for row in &plan.packages {
            hostings.selected(&row.package, row.source_evidence.as_ref());
            hostings.candidates(&row.package, row.source_candidates.as_deref());
        }
        for row in plan.remaps.iter().flatten() {
            hostings.selected(&row.package, row.source_evidence.as_ref());
            hostings.candidates(&row.package, row.source_candidates.as_deref());
        }
        hostings
    }

    pub(crate) fn finish(self) -> Vec<OperationHosting> {
        self.0
            .into_iter()
            .map(|(hosting, entry)| OperationHosting {
                hosting,
                excluded: entry.excluded,
                endpoints: entry.endpoints.into_iter().collect(),
                packages: entry.packages.into_iter().collect(),
            })
            .collect()
    }
}

/// Attach the aggregate to a finished project plan document.
pub(crate) fn with_hostings(mut plan: InstallPlan) -> InstallPlan {
    plan.hostings = Some(Hostings::of_plan(&plan).finish());
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_wire::generated::install::e2::plan::PlanEntry;

    fn evidence(url: &str) -> SourceEvidence {
        SourceEvidence {
            registry: None,
            source_url: Some(url.into()),
            source_ref: None,
            resolved_commit: None,
            source_kind: None,
            via_redirect: None,
            fetched_from: None,
            acquisition: None,
            store_reused: None,
        }
    }
    fn candidate(url: &str, decision: CandidateDecision) -> SourceCandidate {
        SourceCandidate {
            source_evidence: evidence(url),
            observation: CandidateObservation::NotProbed,
            decision,
            versions: None,
            failure: None,
            pinned_registry: None,
            excluded_endpoint: None,
            constraint: None,
        }
    }

    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#HOSTING-OBSERVATION"
    )]
    fn families_hosts_exclusions_and_unprobed_candidates_are_classified() {
        let plan = InstallPlan {
            command: "install:plan".into(),
            packages: vec![PlanEntry {
                package: "org.example/demo".into(),
                version: "1.0.0".into(),
                source_evidence: Some(evidence("https://user:secret@github.com/org/demo")),
                source_hosting: None,
                source_candidates: Some(vec![
                    candidate("https://gitverse.ru/org", CandidateDecision::ExcludedHost),
                    candidate("https://mirror.example/org", CandidateDecision::NotSelected),
                    candidate("file:///local/registry", CandidateDecision::ExcludedHost),
                ]),
            }],
            preview_complete: None,
            remaps: None,
            hostings: None,
        };
        let rows = with_hostings(plan).hostings.unwrap_or_default();
        let keys: Vec<_> = rows
            .iter()
            .map(|r| (r.hosting.as_str(), r.excluded))
            .collect();
        assert_eq!(keys, [("github", false), ("gitverse", true)]);
        assert!(rows[0].endpoints.iter().all(|e| !e.contains("secret")));
        assert_eq!(rows[0].packages, ["org.example/demo"]);
        assert_eq!(
            hosting_key("http://127.0.0.2:8080/x").as_deref(),
            Some("host:127.0.0.2")
        );
        assert_eq!(
            hosting_key("https://api.github.com/x").as_deref(),
            Some("github")
        );
        assert_eq!(hosting_key("/a/local/path"), None);
    }
}
