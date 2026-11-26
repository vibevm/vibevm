//! The source of the invocation’s agent identity.

/// Resolved provenance for `--invoked-by` / `VIBE_INVOKED_BY`. Drives
/// `vibe show config` reporting and lets tests assert which layer
/// supplied the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokedByProvenance {
    /// `--invoked-by <agent>` was passed on the command line.
    CliFlag,
    /// `VIBE_INVOKED_BY` was set in the environment; CLI flag was absent.
    EnvVar,
    /// Neither layer set the value.
    Default,
}

impl InvokedByProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            InvokedByProvenance::CliFlag => "cli-flag",
            InvokedByProvenance::EnvVar => "env",
            InvokedByProvenance::Default => "default",
        }
    }
}
