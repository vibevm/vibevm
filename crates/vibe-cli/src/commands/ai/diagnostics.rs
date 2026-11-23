//! Validated public metadata only, observed after typed connection refusals.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use vibe_connections::{Error, Store};
use vibe_secrets::{Expected, SecretId};

pub(super) fn binding(harness: &str, error: Error) -> anyhow::Error {
    if matches!(error, Error::BindingUnsupported)
        && vibe_secrets::known_reference_harness(harness)
        && !vibe_secrets::reference_binding_supported(harness)
    {
        return anyhow::Error::new(error).context(format!(
            "binding_unsupported: {harness} cannot be bound to a connection; \
             this binding operation does not change independent native authentication \
             (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
             fix: `vibe ai bindings show {harness}` inspects this harness's binding)"
        ));
    }
    error.into()
}

pub(super) fn removal(
    store: &Store,
    id: &SecretId,
    expected: &Expected,
    error: Error,
) -> anyhow::Error {
    if matches!(error, Error::ConnectionInUse)
        && let Some((harness, revision)) = explicit_binding(store, id, expected)
    {
        return anyhow::Error::new(error).context(format!(
            "connection_in_use: connection {} is explicitly bound by {harness} \
             in the current observation at connection store revision {revision} \
             (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
             fix: `vibe ai bindings set {harness} --mode native --expect-store {revision}`; \
             inspect again before retrying removal)",
            id.as_str()
        ));
    }
    error.into()
}

fn explicit_binding(
    store: &Store,
    id: &SecretId,
    expected: &Expected,
) -> Option<(&'static str, u64)> {
    let before = store.list().ok()?;
    let revision = before.store_revision;
    if revision != expected.store_revision
        || before.items.get(id)?.current_revision != expected.current_revision
    {
        return None;
    }
    let default = store.default_selection().ok()?;
    if default.store_revision != revision || default.connection.as_ref() == Some(id) {
        return None;
    }
    // Every read must agree on the monotonically increasing store revision.
    // Never attribute a default/inherited reference to an explicit binding,
    // or offer one replay command when more than one binding must be removed.
    let mut bound = None;
    for harness in vibe_secrets::REFERENCE_HARNESSES {
        let observation = store.binding(harness).ok()?;
        if observation.store_revision != revision || observation.harness != harness {
            return None;
        }
        if observation.connection.as_ref() == Some(id) {
            if observation.mode != "connection"
                || !vibe_secrets::reference_binding_supported(harness)
                || bound.is_some()
            {
                return None;
            }
            bound = Some(harness);
        }
    }
    if store.list().ok()?.store_revision != revision {
        return None;
    }
    bound.map(|harness| (harness, revision))
}
