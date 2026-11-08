//! `vibe self apply` — configure a whole installation from one site
//! profile (`PROP-019#site-profile`; corporate-env campaign mandate 9/10).
//!
//! The profile is validated in full before anything is written, stored
//! verbatim as its own layer (`~/.vibe/site-profile.toml`, beneath the
//! user's own files), recorded in a receipt naming exactly the files apply
//! owns, and followed by the consented PATH step and a report of where a
//! person puts vibe releases and package payloads in the closed network.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

#[path = "site/apply.rs"]
mod apply;
#[path = "site/diff.rs"]
mod diff;
#[path = "site/layers.rs"]
mod layers;
#[path = "site/manage.rs"]
mod manage;
#[path = "site/report.rs"]
mod report;
#[path = "site/steps.rs"]
mod steps;
#[path = "site/view.rs"]
mod view;

pub(super) use apply::{apply_after_bootstrap, prepare_profile, run_apply_cmd};
pub(crate) use layers::ConfigLayers;
