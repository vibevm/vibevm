//! The step evidence of one package's installation contributions
//! (PROP-065 `#ordinary-plan`, format `vibe-install-steps/1`).
//!
//! One directory per package version per run —
//! `<home>/cache/install-steps/<group>/<name>/<version>/<timestamp>/` —
//! holding `steps.json` and one `logs/<id>.log` per contribution that
//! produced output. The record is rewritten as each contribution ends, so a
//! run that is interrupted or killed leaves the rows it had reached rather
//! than nothing at all.
//!
//! **Derived state, and it behaves like it.** Nothing resumes from this
//! directory and nothing in the install depends on it, so **no write failure
//! here may fail an installation**: a full disk, a read-only cache or a
//! settings root that cannot be resolved costs the evidence and not the
//! work. That is the one rule this cell is built around, and it is why every
//! path here ends in a discarded result rather than a `?`.
//!
//! Written at the CLI boundary, like the run records of the application
//! plane: the settings home is resolved here, the clock is read here, and
//! the engine that runs the contributions is handed neither.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use vibe_install::SlotLifecycleReport;
use vibe_wire::generated::install_steps as wire;

/// The protocol string every `steps.json` carries.
pub(crate) const PROTOCOL: &str = "vibe-install-steps/1";

/// `<group>/<name>/<version>` of one package — the key a record is kept
/// under and the subtree it is written into.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Coordinate {
    group: String,
    name: String,
    version: String,
}

impl Coordinate {
    fn of(target: &vibe_wire::generated::lifecycle::e1::context::SlotTarget) -> Self {
        Self {
            group: target.group.clone(),
            name: target.name.clone(),
            version: target.version.clone(),
        }
    }
}

/// One package's open record: where it is written and what it says so far.
struct Record {
    dir: PathBuf,
    document: wire::InstallSteps,
}

/// The run's evidence: one record per package whose contributions it judged.
pub(crate) struct StepEvidence {
    /// `<home>/cache/install-steps`.
    root: PathBuf,
    /// One timestamp for the whole run, so every package of one install
    /// lands under directories a reader can line up.
    stamp: String,
    open: Mutex<BTreeMap<Coordinate, Record>>,
    /// When each running contribution started, by execution key. The engine
    /// reports an outcome and no start, and a start this cell invented at
    /// outcome time would be a measurement of nothing.
    started: Mutex<BTreeMap<String, String>>,
}

impl StepEvidence {
    /// The evidence of a run writing under `settings_root`.
    pub(crate) fn new(settings_root: &Path) -> Self {
        Self {
            root: settings_root.join("cache").join("install-steps"),
            stamp: stamp(),
            open: Mutex::new(BTreeMap::new()),
            started: Mutex::new(BTreeMap::new()),
        }
    }

    /// A contribution is about to run: take its start instant, by the
    /// execution key the outcome will come back under.
    pub(crate) fn started(&self, key: &str) {
        if let Ok(mut started) = self.started.lock() {
            started.insert(key.to_owned(), now());
        }
    }

    /// One contribution ended: append its row, write its log, and rewrite
    /// `steps.json`. Answers with the log it wrote, which is the path a
    /// failure's warning names (PROP-065 `#ordinary-failure`).
    pub(crate) fn record(&self, report: &SlotLifecycleReport) -> Option<PathBuf> {
        let Some(target) = &report.slot_target else {
            // A host-declared contribution belongs to no package version, so
            // there is no package directory to write it under. It is still
            // reported in the install document; only this cache is keyed by
            // a coordinate it has not got.
            return None;
        };
        let coordinate = Coordinate::of(target);
        let started = self
            .started
            .lock()
            .ok()
            .and_then(|mut started| started.remove(&report.key));
        let Ok(mut open) = self.open.lock() else {
            return None;
        };
        let record = open.entry(coordinate.clone()).or_insert_with(|| Record {
            dir: self
                .root
                .join(&coordinate.group)
                .join(&coordinate.name)
                .join(&coordinate.version)
                .join(&self.stamp),
            document: wire::InstallSteps {
                protocol: PROTOCOL.to_owned(),
                package: wire::Package {
                    group: coordinate.group.clone(),
                    name: coordinate.name.clone(),
                    version: coordinate.version.clone(),
                },
                started: now(),
                finished: None,
                contributions: Vec::new(),
            },
        });
        let id = contribution_id(&report.key);
        let log = write_log(&record.dir, &id, report);
        record.document.contributions.push(wire::Contribution {
            id: id.clone(),
            key: report.key.clone(),
            point: report.point.clone(),
            handler: report.handler.clone(),
            outcome: outcome(&report.status),
            reason: report.message.clone(),
            started,
            finished: Some(now()),
            log: log.then(|| format!("logs/{id}.log")),
            log_truncated: (report.stdout_truncated || report.stderr_truncated).then_some(true),
        });
        write_document(record);
        log.then(|| record.dir.join("logs").join(format!("{id}.log")))
    }

    /// The run reached its end: close every open record.
    ///
    /// A record left open — because the process was killed, or because an
    /// interrupt ended the pass — simply has no `finished`, which is the
    /// honest reading of "this run did not come back".
    pub(crate) fn close(&self) {
        let Ok(mut open) = self.open.lock() else {
            return;
        };
        for record in open.values_mut() {
            record.document.finished = Some(now());
            write_document(record);
        }
    }
}

/// The author-written `id` inside an execution key `<provider>#<id>@…`.
///
/// The key is the lifecycle state's own spelling and carries the provider
/// and the slot target around the id; the record's `id` member is what the
/// package author wrote, which is what a reader recognises. A key in an
/// unexpected shape yields the whole key rather than an empty string: a
/// wrong-looking id is debuggable, a missing one is not.
fn contribution_id(key: &str) -> String {
    key.split_once('#')
        .map(|(_, tail)| tail.split_once('@').map_or(tail, |(id, _)| id))
        .unwrap_or(key)
        .to_owned()
}

/// The engine's transition status as the format's closed vocabulary.
///
/// Carried verbatim where the two agree; anything this build does not know
/// is recorded as `fail`, because an outcome a reader cannot classify must
/// not read as a success.
fn outcome(status: &str) -> wire::Outcome {
    match status {
        "ok" => wire::Outcome::Ok,
        "skip" => wire::Outcome::Skip,
        "fresh" => wire::Outcome::Fresh,
        "delegated" => wire::Outcome::Delegated,
        "interrupted" => wire::Outcome::Interrupted,
        _ => wire::Outcome::Fail,
    }
}

/// Write this contribution's captured output to `logs/<id>.log`; `true`
/// when there was anything to write.
///
/// `stdout` then `stderr`, each as the engine captured it. A run whose
/// child wrote straight to the terminal (the attended default) captures
/// nothing, and such a row carries no log rather than an empty one.
fn write_log(dir: &Path, id: &str, report: &SlotLifecycleReport) -> bool {
    let mut bytes = report.stdout.clone().unwrap_or_default();
    bytes.push_str(report.stderr.as_deref().unwrap_or_default());
    if bytes.is_empty() {
        return false;
    }
    let logs = dir.join("logs");
    if std::fs::create_dir_all(&logs).is_err() {
        return false;
    }
    std::fs::write(logs.join(format!("{id}.log")), bytes).is_ok()
}

/// Rewrite one record's `steps.json`. A failure is discarded: this is
/// evidence, and evidence may not fail an install.
fn write_document(record: &Record) {
    if std::fs::create_dir_all(&record.dir).is_err() {
        return;
    }
    if let Ok(mut bytes) = serde_json::to_vec_pretty(&record.document) {
        bytes.push(b'\n');
        let _ = std::fs::write(record.dir.join("steps.json"), bytes);
    }
}

/// The record directory's name: sortable, to the millisecond, UTC.
fn stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ").to_string()
}

/// A field instant: RFC 3339, milliseconds, UTC.
fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(version: &str) -> vibe_wire::generated::lifecycle::e1::context::SlotTarget {
        vibe_wire::generated::lifecycle::e1::context::SlotTarget {
            group: "org.example".into(),
            name: "hooked".into(),
            version: version.into(),
            kind: "tool".into(),
            root: "/slot".into(),
        }
    }

    fn report(key: &str, status: &str, stdout: Option<&str>) -> SlotLifecycleReport {
        SlotLifecycleReport {
            key: key.into(),
            reference: "ref".into(),
            slot_target: Some(target("0.1.0")),
            point: "slot:post-install".into(),
            provider: "org.example/hooked".into(),
            handler: "script".into(),
            tier: "dependency".into(),
            version: Some("0.1.0".into()),
            status: status.into(),
            flagged: false,
            message: Some("because".into()),
            stdout: stdout.map(ToOwned::to_owned),
            stderr: None,
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    /// The id a reader recognises is the author's, not the engine's key.
    #[test]
    fn the_row_carries_the_authors_own_id() {
        assert_eq!(
            contribution_id("org.example/hooked#build@slot(org.example/hooked@0.1.0)"),
            "build"
        );
        assert_eq!(contribution_id("org.example/hooked#build"), "build");
        // An unexpected shape is debuggable rather than blank.
        assert_eq!(contribution_id("odd-key"), "odd-key");
    }

    /// Every status the engine can report maps onto the closed vocabulary,
    /// and an unknown one is never a success.
    #[test]
    fn an_unclassifiable_outcome_is_never_read_as_a_success() {
        assert_eq!(outcome("ok"), wire::Outcome::Ok);
        assert_eq!(outcome("skip"), wire::Outcome::Skip);
        assert_eq!(outcome("fresh"), wire::Outcome::Fresh);
        assert_eq!(outcome("delegated"), wire::Outcome::Delegated);
        assert_eq!(outcome("interrupted"), wire::Outcome::Interrupted);
        assert_eq!(outcome("fail"), wire::Outcome::Fail);
        assert_eq!(
            outcome("something-this-build-never-wrote"),
            wire::Outcome::Fail
        );
    }

    /// The record directory, the rows, the log and the closing stamp.
    #[test]
    fn a_run_writes_one_record_per_package_with_a_log_per_noisy_row() {
        let home = tempfile::tempdir().unwrap();
        let evidence = StepEvidence::new(home.path());
        let ran = report("org.example/hooked#build@slot(x)", "ok", Some("built\n"));

        evidence.started("org.example/hooked#build@slot(x)");
        let log = evidence.record(&ran).expect("the noisy row wrote a log");
        assert!(
            evidence
                .record(&report("org.example/hooked#register@slot(x)", "skip", None))
                .is_none(),
            "a silent row writes no log",
        );
        evidence.close();

        let dir = log
            .parent()
            .and_then(Path::parent)
            .expect("the log sits under the record directory")
            .to_path_buf();
        let document: wire::InstallSteps =
            serde_json::from_slice(&std::fs::read(dir.join("steps.json")).unwrap()).unwrap();
        assert_eq!(document.protocol, PROTOCOL);
        assert_eq!(document.package.name, "hooked");
        assert!(document.finished.is_some(), "the closed run has an end");
        assert_eq!(document.contributions.len(), 2);

        let built = &document.contributions[0];
        assert_eq!(built.id, "build");
        assert_eq!(built.outcome, wire::Outcome::Ok);
        assert!(built.started.is_some(), "a row that ran has a start");
        assert_eq!(built.log.as_deref(), Some("logs/build.log"));
        assert_eq!(
            std::fs::read_to_string(dir.join("logs/build.log")).unwrap(),
            "built\n"
        );

        let skipped = &document.contributions[1];
        assert_eq!(skipped.outcome, wire::Outcome::Skip);
        assert!(
            skipped.started.is_none(),
            "a skip is decided, never started",
        );
        assert!(skipped.log.is_none(), "and produces no log");
        assert_eq!(skipped.reason.as_deref(), Some("because"));
    }

    /// A settings root that cannot be written costs the evidence and not
    /// the install: nothing panics and nothing propagates.
    #[test]
    fn an_unwritable_home_costs_the_evidence_and_not_the_install() {
        let evidence = StepEvidence::new(Path::new("/vibe-no-such-root-anywhere"));
        evidence.started("org.example/hooked#build@slot(x)");
        assert!(
            evidence
                .record(&report("org.example/hooked#build@slot(x)", "ok", Some("x")))
                .is_none(),
            "a log that could not be written is not reported as one",
        );
        evidence.close();
    }
}
