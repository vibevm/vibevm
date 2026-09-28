//! Exact selected local Git contribution and retained byte coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vibe_safefs::Project;
use vibe_wire::generated::preview::e1::capture::Capture;
use vibe_wire::generated::preview::e1::release_impact::{
    AcceptanceState, ImplementationSubjectKind, ReleaseImpact,
};
use vibe_wire::generated::preview::e1::transition::{CaptureRef, PendingCaptureState, Transition};

use super::{CheckFailure, CheckReport, CheckReportVerdict, Finding, FindingState, capture};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CONTRIBUTION-SCOPE-PROOF");

#[path = "contribution_evidence.rs"]
mod evidence;

fn pending(report: &mut CheckReport, code: &str, subject: &str, reason: &str, state: FindingState) {
    report.verdict = CheckReportVerdict::Pending;
    report.findings.push(Finding {
        code: code.into(),
        subject: subject.into(),
        reason: reason.into(),
        state,
    });
}

fn fault(code: &'static str, subject: &str, reason: &str) -> CheckFailure {
    CheckFailure::new(code, subject, reason)
}

fn text(bytes: Vec<u8>) -> Result<String, CheckFailure> {
    String::from_utf8(bytes)
        .map_err(|_| fault("git-output", "git", "Git identity output is not UTF-8"))
}

fn ancestor(root: &Path, before: &str, after: &str) -> Result<bool, CheckFailure> {
    let output = capture::git_output(root, &["merge-base", "--is-ancestor", before, after])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(fault(
            "git-ancestry",
            "git",
            "Captured ancestry could not be checked",
        )),
    }
}

struct Captures<'a> {
    release_dir: &'a Path,
    release: Project,
    references: Vec<CaptureRef>,
    verified: BTreeMap<(String, String), Capture>,
}

impl Captures<'_> {
    fn get(
        &mut self,
        id: &str,
        transition: &Transition,
        report: &mut CheckReport,
    ) -> Result<Option<Capture>, CheckFailure> {
        let matching: Vec<_> = self
            .references
            .iter()
            .filter(|reference| reference.capture_id == id)
            .cloned()
            .collect();
        if matching.is_empty() {
            if matches!(transition.dp2.state, PendingCaptureState::Pending) {
                pending(
                    report,
                    "capture-unresolved",
                    id,
                    "Required capture is not supplied while DP2 remains pending",
                    FindingState::Pending,
                );
                return Ok(None);
            }
            return Err(fault(
                "capture-unresolved",
                id,
                "Required capture ID is not supplied",
            ));
        }
        let mut result = None;
        for reference in matching {
            let key = (reference.capture_id.clone(), reference.source.clone());
            if !self.verified.contains_key(&key) {
                let verified = capture::verify_ref(
                    self.release_dir,
                    &self.release,
                    &reference,
                    &format!("capture:{id}"),
                )?;
                report.checked_claims.extend(verified.claims);
                report.subject_ids.push(id.into());
                self.verified.insert(key.clone(), verified.capture);
            }
            result = self.verified.get(&key).cloned();
        }
        Ok(result)
    }
}

fn record_path(id: &str) -> String {
    vibe_core::layout::current_specs_root()
        .join("changes")
        .join(id)
        .join("release-impact.json")
        .to_string_lossy()
        .replace('\\', "/")
}

fn changed_paths(
    root: &Path,
    commit: &str,
    report: &mut CheckReport,
) -> Result<(BTreeSet<String>, bool), CheckFailure> {
    let parents = text(capture::git(
        root,
        &["rev-list", "--parents", "-n", "1", commit],
    )?)?;
    let first_parent = parents.split_whitespace().nth(1);
    let mut args = vec![
        "diff-tree",
        "--no-commit-id",
        "--name-only",
        "-z",
        "--no-renames",
        "--no-ext-diff",
        "-r",
    ];
    if let Some(parent) = first_parent {
        args.extend([parent, commit]);
    } else {
        args.extend(["--root", commit]);
    }
    let raw = capture::git(root, &args)?;
    let mut paths = BTreeSet::new();
    let mut complete = true;
    for path in raw.split(|byte| *byte == 0).filter(|path| !path.is_empty()) {
        match std::str::from_utf8(path) {
            Ok(path) => {
                paths.insert(path.into());
            }
            Err(_) => {
                pending(
                    report,
                    "git-path-unsupported",
                    commit,
                    "A changed path cannot be represented by the UTF-8 wire format",
                    FindingState::Unsupported,
                );
                complete = false;
            }
        }
    }
    Ok((paths, complete))
}

fn excluded(path: &str, before: &Capture) -> bool {
    before.excluded_from_product_delta.iter().any(|exclusion| {
        if exclusion.path.ends_with('/') {
            path.starts_with(&exclusion.path)
        } else {
            path == exclusion.path
        }
    })
}

pub(super) fn check(
    path: &Path,
    transition: &Transition,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    report.verdict = CheckReportVerdict::Passed;
    let Some(scope) = &transition.contribution_scope else {
        pending(
            report,
            "contribution-scope-missing",
            "contribution",
            "Exact contribution scope has not been supplied",
            FindingState::Pending,
        );
        return Ok(());
    };
    let release_dir = path.parent().unwrap_or(Path::new("."));
    let root = PathBuf::from(
        text(capture::git(
            release_dir,
            &["rev-parse", "--show-toplevel"],
        )?)?
        .trim(),
    );
    let project = capture::open_project(&root)?;
    let references = transition
        .capture_references()
        .map_err(|error| fault("capture-meaning", error.field, error.reason))?;
    let mut captures = Captures {
        release_dir,
        release: capture::open_project(release_dir)?,
        references,
        verified: BTreeMap::new(),
    };
    let before = captures.get(&scope.before_capture_id, transition, report)?;
    let after = captures.get(&scope.after_capture_id, transition, report)?;
    let (Some(before), Some(after)) = (before, after) else {
        return Ok(());
    };
    if !ancestor(
        &root,
        &before.repository_commit_sha1,
        &after.repository_commit_sha1,
    )? {
        return Err(fault(
            "scope-ancestry",
            "contribution",
            "Scope before capture is not an ancestor of its after capture",
        ));
    }
    let range = format!(
        "{}..{}",
        before.repository_commit_sha1, after.repository_commit_sha1
    );
    let commits: BTreeSet<String> = text(capture::git(&root, &["rev-list", &range])?)?
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let mut records = Vec::new();
    let mut record_bounds = BTreeMap::new();
    for binding in &scope.record_digests {
        let bytes = capture::read_within(
            &project,
            &record_path(&binding.change_id),
            &binding.change_id,
            "record-read",
        )?;
        if capture::sha256(&bytes) != binding.sha256 {
            return Err(fault(
                "record-digest",
                &binding.change_id,
                "Selected impact record bytes changed",
            ));
        }
        let record: ReleaseImpact = serde_json::from_slice(&bytes).map_err(|_| {
            fault(
                "record-shape",
                &binding.change_id,
                "Impact record does not match the registered shape",
            )
        })?;
        record
            .validate_release_impact()
            .map_err(|error| fault("record-meaning", &binding.change_id, error.reason))?;
        if record.change_id != binding.change_id {
            return Err(fault(
                "record-identity",
                &binding.change_id,
                "Impact ID does not match its selected path",
            ));
        }
        if matches!(record.acceptance.state, AcceptanceState::Rejected) {
            return Err(fault(
                "record-rejected",
                &record.change_id,
                "Rejected impact record cannot establish coverage",
            ));
        }
        let rb = captures.get(&record.before_capture_id, transition, report)?;
        let ra = captures.get(&record.after_capture_id, transition, report)?;
        if let (Some(rb), Some(ra)) = (rb, ra) {
            if !ancestor(
                &root,
                &rb.repository_commit_sha1,
                &ra.repository_commit_sha1,
            )? {
                return Err(fault(
                    "record-ancestry",
                    &record.change_id,
                    "Record before is not an ancestor of its after capture",
                ));
            }
            record_bounds.insert(
                record.change_id.clone(),
                (rb.repository_commit_sha1, ra.repository_commit_sha1),
            );
        }
        if record.affected_paths.is_none() {
            pending(
                report,
                "affected-paths-missing",
                &record.change_id,
                "Record paths have not been bound",
                FindingState::Pending,
            );
        }
        report
            .checked_claims
            .push(format!("record-bytes:{}", record.change_id));
        report.subject_ids.push(record.change_id.clone());
        records.push(record);
    }
    let ids: BTreeSet<_> = records
        .iter()
        .map(|record| record.change_id.as_str())
        .collect();
    let mut by_commit: BTreeMap<&str, Vec<&ReleaseImpact>> = BTreeMap::new();
    let mut actual: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut historical_paths = BTreeMap::new();
    let mut current_subjects: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for record in &records {
        for relation in &record.relations {
            if !ids.contains(relation.change_id.as_str()) {
                return Err(fault(
                    "record-relation",
                    &record.change_id,
                    "Related impact ID is not selected",
                ));
            }
        }
        for subject in &record.implementation_subjects {
            if !matches!(subject.kind, ImplementationSubjectKind::Commit) {
                pending(
                    report,
                    "implementation-unsupported",
                    &record.change_id,
                    "Local scope cannot establish external/content subject authority",
                    FindingState::Unsupported,
                );
                continue;
            }
            if subject.identity.len() != 40
                || !subject
                    .identity
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(fault(
                    "commit-outside-scope",
                    &record.change_id,
                    "Selected local commit subject is not an exact Git identity",
                ));
            }
            if let Some((rb, ra)) = record_bounds.get(&record.change_id)
                && (&subject.identity == rb
                    || !ancestor(&root, rb, &subject.identity)?
                    || !ancestor(&root, &subject.identity, ra)?)
            {
                return Err(fault(
                    "commit-outside-record",
                    &record.change_id,
                    "Local commit is not between its record capture subjects",
                ));
            }
            if commits.contains(&subject.identity) {
                current_subjects
                    .entry(&record.change_id)
                    .or_default()
                    .insert(subject.identity.clone());
                by_commit.entry(&subject.identity).or_default().push(record);
            } else if ancestor(&root, &subject.identity, &before.repository_commit_sha1)? {
                if !historical_paths.contains_key(subject.identity.as_str()) {
                    historical_paths.insert(
                        subject.identity.as_str(),
                        changed_paths(&root, &subject.identity, report)?,
                    );
                }
                let (paths, complete) = &historical_paths[subject.identity.as_str()];
                actual
                    .entry(&record.change_id)
                    .or_default()
                    .extend(paths.iter().cloned());
                report.subject_ids.push(subject.identity.clone());
                if *complete {
                    report
                        .checked_claims
                        .push(format!("historical-commit-paths:{}", subject.identity));
                }
            } else {
                return Err(fault(
                    "commit-outside-scope",
                    &record.change_id,
                    "Local subject is future or unrelated to the selected scope",
                ));
            }
        }
    }
    let selected_paths: BTreeSet<_> = records
        .iter()
        .map(|record| record_path(&record.change_id))
        .collect();
    for commit in &commits {
        let (mut paths, paths_complete) = changed_paths(&root, commit, report)?;
        let raw_paths = paths.clone();
        if paths_complete
            && !paths.is_empty()
            && paths.iter().all(|path| selected_paths.contains(path))
        {
            paths.clear();
        }
        paths.retain(|path| !excluded(path, &before));
        let matching = by_commit.get(commit.as_str());
        if !paths.is_empty() && matching.is_none() {
            return Err(fault(
                "commit-uncovered",
                commit,
                "Actual product commit has no selected local implementation subject",
            ));
        }
        if let Some(matching) = matching {
            let mut covered = BTreeSet::new();
            let mut complete = true;
            for record in matching {
                actual
                    .entry(&record.change_id)
                    .or_default()
                    .extend(raw_paths.iter().cloned());
                if let Some(declared) = &record.affected_paths {
                    covered.extend(declared.iter().cloned());
                } else {
                    complete = false;
                }
            }
            if complete && !paths.is_subset(&covered) {
                return Err(fault(
                    "paths-uncovered",
                    commit,
                    "Actual product paths are omitted from matching impact records",
                ));
            }
        }
        if paths_complete {
            report
                .checked_claims
                .push(format!("git-commit-paths:{commit}"));
        }
        report.subject_ids.push(commit.clone());
    }
    for record in &records {
        if record
            .implementation_subjects
            .iter()
            .any(|subject| matches!(subject.kind, ImplementationSubjectKind::Commit))
            && let Some(declared) = &record.affected_paths
        {
            let observed = actual
                .get(record.change_id.as_str())
                .cloned()
                .unwrap_or_default();
            if declared.iter().any(|path| !observed.contains(path)) {
                return Err(fault(
                    "paths-invented",
                    &record.change_id,
                    "Declared path belongs to no covered local commit",
                ));
            }
        }
        let binding = scope
            .record_digests
            .iter()
            .find(|binding| binding.change_id == record.change_id)
            .ok_or_else(|| {
                fault(
                    "record-selection",
                    &record.change_id,
                    "Record binding is missing",
                )
            })?;
        let empty = BTreeSet::new();
        evidence::check(
            &project,
            record,
            binding.required_claim_ids.as_deref(),
            current_subjects
                .get(record.change_id.as_str())
                .unwrap_or(&empty),
            scope,
            report,
        )?;
    }
    if !report.findings.iter().any(|finding| {
        matches!(
            finding.code.as_str(),
            "git-path-unsupported" | "affected-paths-missing" | "capture-unresolved"
        )
    }) {
        report
            .checked_claims
            .push("selected-local-git-coverage".into());
    }
    for (code, subject, reason) in [
        (
            "interface-linkage-later",
            "interfaces",
            "Contribution mode does not verify the complete interface or topic census",
        ),
        (
            "native-linkage-later",
            "bindings",
            "Contribution mode does not verify native role adoption or linkage",
        ),
        (
            "semantic-review-later",
            "contribution",
            "Byte and Git coverage does not confer central semantic acceptance",
        ),
    ] {
        report.findings.push(Finding {
            code: code.into(),
            subject: subject.into(),
            reason: reason.into(),
            state: FindingState::Pending,
        });
    }
    Ok(())
}
