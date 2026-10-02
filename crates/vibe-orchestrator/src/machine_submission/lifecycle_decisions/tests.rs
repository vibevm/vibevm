use super::*;

#[test]
fn lifecycle_decisions_retention_accounting_checks_eight_mib_and_overflow() {
    assert!(capacity_available(
        3,
        MAX_RETAINED - CANONICAL_CAP,
        CANONICAL_CAP
    ));
    assert!(!capacity_available(
        3,
        MAX_RETAINED - CANONICAL_CAP,
        CANONICAL_CAP + 1
    ));
    assert!(!capacity_available(4, 0, 0));
    assert!(!capacity_available(0, usize::MAX, 1));
}
