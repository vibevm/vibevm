//! How a failed push is diagnosed. Split from `mod.rs` for the
//! file-length budget, by the same seam as `mirror/probe.rs`: the fan-out
//! decides WHAT to push, this decides what to TELL the operator when a
//! push does not land.

/// Why one push failed. A summary that asserts a single cause sends the
/// operator to fix the wrong thing: told «a target diverged, reconcile by
/// hand» when the host was simply unreachable, they go looking for a rewrite
/// that never happened. These are the two outcomes that read alike in an exit
/// code and need opposite responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FailureKind {
    /// The remote answered and refused the update — the target moved on its
    /// own. This is the one that needs a human reconciliation, never `--force`.
    Diverged,
    /// The remote never answered: connection refused or closed, host key,
    /// auth, DNS, or the repository not being there. Nothing about the local
    /// history is implicated, and there is nothing to reconcile.
    Unreachable,
}

impl FailureKind {
    /// Classify from git's stderr. Divergence has stable wording git prints on
    /// every rejection (`non-fast-forward`, `[rejected]`, «fetch first»);
    /// anything else is the transport, so the default is `Unreachable` — the
    /// safer error, because it never invents a rewrite that did not happen.
    pub(super) fn classify(stderr: &str) -> FailureKind {
        let s = stderr.to_ascii_lowercase();
        if s.contains("non-fast-forward") || s.contains("[rejected]") || s.contains("fetch first") {
            FailureKind::Diverged
        } else {
            FailureKind::Unreachable
        }
    }
}

/// One failed push, with the reason kept rather than thrown away.
pub(super) struct PushFailure {
    pub(super) target: String,
    pub(super) kind: FailureKind,
}

/// Build the summary from what actually happened, naming each cause only for
/// the targets it applies to.
pub(super) fn failure_summary(failures: &[PushFailure]) -> String {
    let names = |k: FailureKind| -> Vec<&str> {
        failures
            .iter()
            .filter(|f| f.kind == k)
            .map(|f| f.target.as_str())
            .collect()
    };
    let diverged = names(FailureKind::Diverged);
    let unreachable = names(FailureKind::Unreachable);
    let mut out = format!("mirror: {} push(es) failed", failures.len());
    if !diverged.is_empty() {
        out.push_str(&format!(
            " -- DIVERGED (the target moved on its own; reconcile by hand, never --force): {}",
            diverged.join(", ")
        ));
    }
    if !unreachable.is_empty() {
        out.push_str(&format!(
            " -- UNREACHABLE (the host never answered; nothing local diverged and there is nothing \
             to reconcile -- fix the connection, then re-run): {}",
            unreachable.join(", ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{FailureKind, PushFailure, failure_summary};

    #[test]
    fn an_unreachable_host_is_not_reported_as_a_divergence() {
        // The verbatim stderr that produced the wrong diagnosis: ssh to the
        // target was intercepted, and the summary announced that the target
        // had diverged and needed hand reconciliation.
        let stderr = "Connection closed by 127.92.0.49 port 22\n\
                      fatal: Could not read from remote repository.";
        assert_eq!(FailureKind::classify(stderr), FailureKind::Unreachable);

        let s = failure_summary(&[PushFailure {
            target: "github:main".into(),
            kind: FailureKind::classify(stderr),
        }]);
        assert!(s.contains("UNREACHABLE"), "{s}");
        assert!(s.contains("nothing to reconcile"), "{s}");
        // Not a bare substring check: the summary legitimately contains the
        // word inside «nothing local diverged», which is the sentence doing
        // the correcting. What must not appear is the DIVERGED heading, which
        // is what would route the reader to a reconciliation.
        assert!(
            !s.contains("DIVERGED"),
            "an unreachable host must never be filed under the divergence heading: {s}"
        );
        assert!(
            !s.contains("reconcile by hand"),
            "an unreachable host must not be sent to a hand reconciliation: {s}"
        );
    }

    #[test]
    fn a_real_rejection_still_reads_as_a_divergence() {
        for stderr in [
            " ! [rejected]        main -> main (non-fast-forward)",
            "Updates were rejected because the remote contains work that you do\n\
             not have locally. This is usually caused by another repository pushing\n\
             to the same ref. You may want to first integrate the remote changes\n\
             (e.g., 'git pull ...') before pushing again. hint: fetch first",
        ] {
            assert_eq!(
                FailureKind::classify(stderr),
                FailureKind::Diverged,
                "{stderr}"
            );
        }
        let s = failure_summary(&[PushFailure {
            target: "gitverse:main".into(),
            kind: FailureKind::Diverged,
        }]);
        assert!(s.contains("DIVERGED"), "{s}");
        assert!(
            s.contains("never --force"),
            "the force ban stays stated: {s}"
        );
    }

    #[test]
    fn a_mixed_run_names_each_cause_for_its_own_targets() {
        // The case the old single-sentence summary could not express at all.
        let s = failure_summary(&[
            PushFailure {
                target: "github:main".into(),
                kind: FailureKind::Unreachable,
            },
            PushFailure {
                target: "gitverse:main".into(),
                kind: FailureKind::Diverged,
            },
        ]);
        assert!(s.contains("2 push(es) failed"), "{s}");
        let d = s.find("DIVERGED").expect("divergence named");
        let u = s.find("UNREACHABLE").expect("unreachability named");
        assert!(s[d..u].contains("gitverse:main"), "{s}");
        assert!(s[u..].contains("github:main"), "{s}");
        assert!(
            !s[d..u].contains("github:main"),
            "an unreachable target must not be listed under DIVERGED: {s}"
        );
    }
}
