//! Args for `vibe vars` (PROP-019 §2.14).

specmark::scope!("spec://vibevm/common/PROP-019#vars");

#[derive(Debug, clap::Args)]
pub struct VarsArgs {
    /// Optional modes: `full` (two tables — actual and environment) and/or
    /// `diff` (mark where the environment differs). E.g. `vibe vars full diff`.
    pub modes: Vec<String>,
}
