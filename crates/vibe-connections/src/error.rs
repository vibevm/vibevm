//! The connection-metadata layer's refusals, each led by its stable code.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");

/// The connection-metadata layer's one error enum.
/// Every message is fixed text led by its stable code: it names the violated
/// rule and a fix, never an endpoint or a value.
///
/// ```
/// use vibe_connections::{Error, Metadata};
///
/// // Plain http is admitted only towards a loopback address.
/// let metadata = Metadata {
///     label: "Synthetic".into(),
///     endpoint: "http://provider.invalid/v1".into(),
///     api_kind: "openai-chat-completions".into(),
///     default_model: "synthetic".into(),
/// };
/// let error = metadata.validate().unwrap_err();
/// assert!(matches!(error, Error::Invalid));
/// assert!(error.to_string().contains("TS-KEYRING-CONSUMER"));
/// assert!(!error.to_string().contains("provider.invalid"));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER")]
pub enum Error {
    #[error(
        "connection_invalid: invalid nonsecret connection metadata \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
          fix: give a bounded label and model, a supported API kind and harness, and an \
          https or loopback http endpoint without credentials, query or fragment)"
    )]
    Invalid,
    #[error(
        "connection_metadata_unreadable: connection metadata serialization failed \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
          fix: remove the connection whose stored nonsecret metadata is missing or \
          unreadable and add it again)"
    )]
    Metadata,
    /// The agent's native settings have no connection to bind.
    #[error(
        "binding_unsupported: this agent's native settings cannot be bound to a connection \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
          fix: `vibe ai bindings show --help` names what each agent supports)"
    )]
    BindingUnsupported,
    /// The connection speaks an API the agent cannot use.
    #[error(
        "binding_incompatible: the connection's API kind is not one this agent can use \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
          fix: `vibe ai connections list` shows the API kind of each connection)"
    )]
    BindingIncompatible,
    /// The connection store changed after its revision was observed.
    #[error(
        "connection_store_stale: the connection store changed since its revision was read \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
          fix: `vibe ai connections list` shows the current store revision)"
    )]
    ConnectionStoreStale,
    /// A default selection or agent binding still refers to the connection.
    #[error(
        "connection_in_use: the connection is the default or is bound by an agent \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
          fix: `vibe ai connections default show` and `vibe ai bindings show --help` show what uses it)"
    )]
    ConnectionInUse,
    /// Another keyring refusal, delegated without adding a duplicate message.
    #[error(transparent)]
    Store(vibe_secrets::Error),
}

impl From<vibe_secrets::Error> for Error {
    fn from(error: vibe_secrets::Error) -> Self {
        match error {
            vibe_secrets::Error::Conflict => Self::ConnectionStoreStale,
            vibe_secrets::Error::RefusedReferenced { .. } => Self::ConnectionInUse,
            error => Self::Store(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn connection_refusals_carry_code_and_fix() {
        let tail =
            " (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; fix: ";
        let cases = [
            (Error::Invalid, "connection_invalid: "),
            (Error::Metadata, "connection_metadata_unreadable: "),
            (Error::BindingUnsupported, "binding_unsupported: "),
            (Error::BindingIncompatible, "binding_incompatible: "),
            (
                Error::from(vibe_secrets::Error::Conflict),
                "connection_store_stale: ",
            ),
            (
                Error::from(vibe_secrets::Error::RefusedReferenced {
                    consumers: vec!["ai:binding".into()],
                }),
                "connection_in_use: ",
            ),
        ];
        for (error, code) in cases {
            let text = error.to_string();
            assert!(text.starts_with(code), "{text}");
            assert!(text.contains(tail), "{text}");
            assert!(text.ends_with(')'), "{text}");
            assert!(!text.contains("ai:binding"), "{text}");
            assert!(std::error::Error::source(&error).is_none(), "{text}");
        }
        assert_eq!(
            Error::Store(vibe_secrets::Error::NotFound).to_string(),
            vibe_secrets::Error::NotFound.to_string()
        );
        assert!(matches!(
            Error::from(vibe_secrets::Error::Invalid),
            Error::Store(vibe_secrets::Error::Invalid)
        ));
    }
}
