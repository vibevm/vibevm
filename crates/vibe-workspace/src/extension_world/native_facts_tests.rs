use vibe_spec::{CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError};

use super::{CompilerNativeFactBinding, CompilerNativeFactError};

struct EmptyBinding;

impl CompilerNativeInvoker for EmptyBinding {
    fn invoke(&self, _call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        unreachable!("object-safety fixture is never invoked")
    }
}

impl CompilerNativeFactBinding for EmptyBinding {
    fn invoker(&self) -> &dyn CompilerNativeInvoker {
        self
    }

    fn take_pending_build_facts(
        &self,
        _pending: &vibe_spec::CompilerPendingSet,
    ) -> Result<Vec<super::PendingBuildFact>, CompilerNativeFactError> {
        Err(CompilerNativeFactError::already_taken())
    }

    fn finish_ready(&self) -> Result<(), CompilerNativeFactError> {
        Err(CompilerNativeFactError::already_taken())
    }
}

#[test]
fn binding_is_object_safe_and_lends_the_same_invoker() {
    let binding: &dyn CompilerNativeFactBinding = &EmptyBinding;
    let _: &dyn CompilerNativeInvoker = binding.invoker();
}

#[test]
fn errors_are_bounded_opaque_and_distinguish_recorder_laws() {
    for (error, word) in [
        (CompilerNativeFactError::poisoned(), "poisoned"),
        (CompilerNativeFactError::already_taken(), "already taken"),
        (CompilerNativeFactError::missing(u32::MAX), "missing"),
        (CompilerNativeFactError::extra(u32::MAX), "extra"),
        (CompilerNativeFactError::conflict(u32::MAX), "conflicts"),
        (
            CompilerNativeFactError::construction(u32::MAX),
            "construction",
        ),
    ] {
        assert!(error.to_string().contains(word));
        assert_eq!(format!("{error:?}"), "CompilerNativeFactError(..)");
        assert!(error.to_string().len() < 384);
    }
}
