use super::*;
fn row(key: String) -> LifecycleContributionReport {
    LifecycleContributionReport {
        key,
        phase: "validate".into(),
        status: "ok".into(),
        provider: "local".into(),
        handler: "script".into(),
        point: "phase:validate".into(),
        tier: "project".into(),
        flagged: None,
        message: None,
        reference: None,
        slot_target: None,
        stderr: None,
        stderr_truncated: None,
        stdout: None,
        stdout_truncated: None,
        version: None,
    }
}
#[test]
fn bounded_rows_keep_measured_counts_and_independent_last_phase() {
    let mut ledger = Ledger::default();
    ledger.total = Some(40);
    for i in 0..40 {
        ledger.terminal(&row(i.to_string()));
    }
    ledger.phase("validate", Some("fail"));
    assert_eq!(ledger.terminals, Some(40));
    assert_eq!(ledger.work.len(), ROWS);
    assert!(ledger.partial);
    assert_eq!(ledger.last.unwrap().phase(), "validate");
}
#[test]
fn utf8_is_counted_before_clone_and_invalid_phase_is_unavailable() {
    let mut ledger = Ledger::default();
    ledger.terminal(&row("é".repeat(513)));
    assert!(ledger.work.is_empty());
    assert_eq!(ledger.terminals, Some(1));
    ledger.phase("validate", None);
    ledger.phase(&"é".repeat(129), None);
    assert!(ledger.last.is_none());
    assert!(ledger.phase_unavailable.is_some());
    assert!(ledger.partial);
}
#[test]
fn identity_budget_and_counter_overflow_are_explicit() {
    let mut ledger = Ledger::default();
    for _ in 0..32 {
        let mut r = row("k".repeat(1024));
        r.provider = "p".repeat(1024);
        r.handler = "h".repeat(1024);
        ledger.terminal(&r);
    }
    assert!(ledger.bytes <= BYTES - 384);
    assert!(ledger.work.len() < 32);
    assert!(ledger.partial);
    ledger.terminals = Some(u64::MAX);
    ledger.terminal(&row("overflow".into()));
    assert_eq!(ledger.terminals, None);
}

#[test]
fn actual_error_chain_is_bounded_before_allocation_and_marks_utf8_loss() {
    let error = anyhow::anyhow!("physical root changed").context("incumbent inspection");
    let (cause, truncated) = diagnostic(&error);
    assert_eq!(cause, "incumbent inspection: physical root changed");
    assert!(!truncated);
    let (cause, truncated) = diagnostic(&anyhow::anyhow!("é".repeat(4097)));
    assert_eq!(cause.len(), 4096);
    assert!(truncated);
    let (cause, truncated) = diagnostic(&anyhow::anyhow!(format!("x{}", "é".repeat(4097))));
    assert_eq!(cause.len(), 4095);
    assert!(truncated);
}
