//! How a FAILED quiet command gets its trace suffix onto the one line it owns.
//!
//! A quiet command's whole output is a single line. On success that line is
//! the summary the command prints itself, and the suffix simply rides it. On
//! FAILURE there is no summary at all: the sole line is `main`'s final
//! `ctx.error(&err)`, printed after the command has already returned. There is
//! no seam between those two points — so this is one.
//!
//! The rule the design exists to keep: **the error object is never
//! reformatted.** A wrapper that stored `format!("{err:#}")` and rebuilt an
//! error from it would change the exit code (the chain is what
//! `as_exit_code` downcasts through), lose every structured variant, and turn
//! a rich error into prose at the exact moment the operator most needs it. So
//! the wrapper OWNS the original object, `main` takes it back out, and what
//! reaches the terminal is the same error it always was — plus, in
//! `HumanQuiet` alone, a suffix.
//!
//! Trace-disabled produces no suffix and therefore no wrapper, which is why
//! the old error line stays byte-identical rather than merely equivalent.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::fmt;

/// The transport. Constructed only at a command's final return, downcast and
/// consumed immediately by `main`.
///
/// It implements `Error` so it can ride `anyhow`, and its `Display`/`source`
/// delegate to the original — so even the impossible path where one escaped
/// un-downcast would print the same words rather than the type's name.
#[derive(Debug)]
pub(crate) struct QuietTraceSuffix {
    original: anyhow::Error,
    suffix: String,
}

impl fmt::Display for QuietTraceSuffix {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.original, formatter)
    }
}

impl std::error::Error for QuietTraceSuffix {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.original.source()
    }
}

/// Wrap only when there is really something to append.
///
/// An empty suffix returns the error UNTOUCHED — not a wrapper carrying an
/// empty string — so the trace-disabled path allocates nothing and `main`'s
/// downcast finds nothing to undo.
pub(crate) fn attach(original: anyhow::Error, suffix: &str) -> anyhow::Error {
    if suffix.is_empty() {
        return original;
    }
    anyhow::Error::new(QuietTraceSuffix {
        original,
        suffix: suffix.to_string(),
    })
}

/// `main`'s side: recover the SAME original object and its suffix.
///
/// Total — an unwrapped error is returned as it is with no suffix, which is
/// every other command in the binary.
pub(crate) fn detach(error: anyhow::Error) -> (anyhow::Error, Option<String>) {
    match error.downcast::<QuietTraceSuffix>() {
        Ok(QuietTraceSuffix { original, suffix }) => (original, Some(suffix)),
        Err(error) => (error, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("the slot refused")]
    struct Sentinel;

    /// The property the whole cell exists for: what comes out is the object
    /// that went in — same downcast identity, same context chain.
    #[test]
    fn the_original_object_survives_the_round_trip() {
        let original = anyhow::Error::new(Sentinel).context("while installing");
        let rendered = format!("{original:#}");
        let (recovered, suffix) = detach(attach(original, ", compile trace ok"));
        assert_eq!(suffix.as_deref(), Some(", compile trace ok"));
        assert_eq!(format!("{recovered:#}"), rendered);
        assert!(
            recovered.downcast_ref::<Sentinel>().is_some(),
            "the typed variant the exit code is read from must survive"
        );
    }

    #[test]
    fn an_empty_suffix_never_wraps() {
        let original = anyhow::Error::new(Sentinel);
        let attached = attach(original, "");
        assert!(attached.downcast_ref::<QuietTraceSuffix>().is_none());
        let (recovered, suffix) = detach(attached);
        assert!(suffix.is_none());
        assert!(recovered.downcast_ref::<Sentinel>().is_some());
    }

    #[test]
    fn an_unwrapped_error_detaches_to_itself() {
        let (recovered, suffix) = detach(anyhow::Error::new(Sentinel).context("ctx"));
        assert!(suffix.is_none());
        assert_eq!(format!("{recovered:#}"), "ctx: the slot refused");
    }
}
