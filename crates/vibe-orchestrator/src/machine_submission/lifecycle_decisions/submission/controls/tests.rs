//! Pure policy separation; actual boundary tests use the native owner below.
use super::*;
#[test]
fn exact_permissions_do_not_imply_the_other_cancellation_mode() {
    let pre = HostPolicyFacts {
        approve_captured_script: true,
        allow_detach: false,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: false,
    };
    assert!(pre.permits(&RequestedOptionsControl::CancelPreDispatch));
    assert!(!pre.permits(&RequestedOptionsControl::CancelDeferredBoundary));
    let deferred = HostPolicyFacts {
        allow_cancel_pre_dispatch: false,
        allow_cancel_deferred_boundary: true,
        ..pre
    };
    assert!(deferred.permits(&RequestedOptionsControl::CancelDeferredBoundary));
    assert!(!deferred.permits(&RequestedOptionsControl::CancelPreDispatch));
    assert!(!deferred.permits(&RequestedOptionsControl::Detach));
    assert!(deferred.permits(&RequestedOptionsControl::None));
}
