//! Existing failure projection and elapsed-time conversion.
use super::*;

pub(super) fn failed_transition(envelope: Context, source: DispatchError) -> LifecycleRunError {
    let transition = FailedExecutionTransition {
        envelope,
        message: source.to_string(),
        streams: source.streams().cloned().unwrap_or_default(),
    };
    LifecycleRunError::FailedTransition {
        transition: Box::new(transition),
        source: Box::new(source),
    }
}

pub(super) fn elapsed_ms(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}
