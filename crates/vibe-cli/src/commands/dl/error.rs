use vibe_net::probe::FailureClass;

/// A classified `vibe dl` failure: a stable code first (the refusal's
/// headline code), what was observed, and the fix. Never a credential.
///
/// ```text
/// dl_proxy_auth: the proxy answered `HTTP/1.1 407 …` (violates spec://…#dl; fix: …)
/// ```
#[derive(Debug)]
pub struct DlError {
    pub code: &'static str,
    pub exit: u8,
    pub detail: String,
    pub fix: String,
    /// The cause behind the headline, drawn as the refusal's `why`.
    pub why: Option<String>,
}

impl std::fmt::Display for DlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} (violates spec://org.vibevm.core/vibevm/common/PROP-068#dl; fix: {})",
            self.code, self.detail, self.fix
        )?;
        if let Some(why) = &self.why {
            write!(f, ": {why}")?;
        }
        Ok(())
    }
}

impl std::error::Error for DlError {}

/// A nested error's message without its `(violates …; fix: …)` tail —
/// what a refusal shows as the cause.
fn first_sentence(message: &str) -> String {
    match message.find(" (violates ") {
        Some(at) => message[..at].to_owned(),
        None => message.to_owned(),
    }
}

/// The `fix:` a nested error carries, when it carries one: a
/// configuration error knows its own fix better than any failure class
/// does.
fn nested_fix(message: &str) -> String {
    match message.find("fix: ") {
        Some(at) => message[at + "fix: ".len()..]
            .trim_end_matches(')')
            .trim()
            .to_owned(),
        None => "check the configured certificate files".to_owned(),
    }
}

impl DlError {
    pub(super) fn class(class: FailureClass, detail: impl Into<String>) -> Self {
        let detail: String = detail.into();
        // A nested error's own `(violates …; fix: …)` tail is replaced by
        // this class's fix, so the refusal names one fix.
        let detail = match detail.find(" (violates ") {
            Some(at) => detail[..at].to_owned(),
            None => detail,
        };
        // `connecting to <target>: <cause>` reads as a headline and its cause.
        let split = matches!(class, FailureClass::ConnectFailed | FailureClass::ConnectTimeout)
            .then(|| detail.strip_prefix("connecting to "))
            .flatten()
            .and_then(|rest| rest.split_once(": "))
            .map(|(target, cause)| (format!("could not connect to {target}"), cause.to_owned()));
        let (detail, why) = match split {
            Some((headline, cause)) => (headline, Some(cause)),
            None => (detail, None),
        };
        Self {
            code: class.code(),
            exit: class.exit_code(),
            detail,
            fix: class.fix().to_owned(),
            why,
        }
    }

    pub(super) fn local(code: &'static str, exit: u8, detail: impl Into<String>, fix: &str) -> Self {
        Self {
            code,
            exit,
            detail: detail.into(),
            fix: fix.to_owned(),
            why: None,
        }
    }

    pub(super) fn usage(detail: impl Into<String>, fix: &str) -> Self {
        Self::local("dl_usage", 2, detail, fix)
    }

    /// The process exit status of this failure.
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

pub(super) fn trust_refusal(error: &vibe_net::NetError) -> DlError {
    use vibe_net::{NetError, TrustError};
    let invalid_fix = "export the certificate as PEM (-----BEGIN CERTIFICATE-----) or as DER";
    match error {
        NetError::Trust(TrustError::Read { path, source }) => DlError {
            why: Some(source.to_string()),
            ..DlError::local(
                "tls_extra_ca_unreadable",
                2,
                format!("extra certificate {} cannot be read", path.display()),
                "point [tls] extra_ca at a readable certificate file, or at a directory of *.pem / *.crt / *.cer files",
            )
        },
        NetError::Trust(TrustError::Empty { path } | TrustError::EmptyDirectory { path }) => {
            DlError::local(
                "tls_extra_ca_invalid",
                2,
                format!("extra certificate {} holds no usable certificate", path.display()),
                invalid_fix,
            )
        }
        NetError::Trust(TrustError::Invalid { path, reason }) => DlError {
            why: Some(reason.clone()),
            ..DlError::local(
                "tls_extra_ca_invalid",
                2,
                format!("extra certificate {} holds no usable certificate", path.display()),
                invalid_fix,
            )
        },
        _ => DlError::usage(
            first_sentence(&error.to_string()),
            &nested_fix(&error.to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dl_shape_connect_failure_moves_the_cause_to_why() {
        let refusal = crate::output::refusal_of(&anyhow::Error::new(DlError::class(
            FailureClass::ConnectFailed,
            "connecting to 127.0.0.1:1: Connection refused (os error 111)",
        )));
        assert_eq!(refusal.code.as_deref(), Some("dl_connect_failed"));
        assert_eq!(refusal.headline, "could not connect to 127.0.0.1:1");
        assert_eq!(refusal.why.as_deref(), Some("Connection refused (os error 111)"));
    }

    #[test]
    fn dl_shape_unreadable_extra_ca_has_its_code() {
        let error = trust_refusal(&vibe_net::NetError::Trust(vibe_net::TrustError::Read {
            path: "/x/missing.pem".into(),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        }));
        assert_eq!(error.code, "tls_extra_ca_unreadable");
        assert_eq!(error.detail, "extra certificate /x/missing.pem cannot be read");
        assert!(error.why.is_some());
        assert_eq!(error.exit_code(), 2);
    }
}
