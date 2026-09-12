//! The registry builder — the site, rendered from what the registry and
//! the host currently hold (PROP-057 §9.2, `##SITE-TWO-SOURCES`).
//!
//! Everything under `vibe doc build` answers about ONE package the caller
//! points at. This module answers about the whole site: which packages
//! exist, which of their versions moved since the last render, and where
//! each rendered version goes. It is the half `vibe doc build-site` is a
//! surface over, and the half the renderer container runs.
//!
//! ## Two sources, and neither of them is a list this project keeps
//!
//! `##SITE-SOURCE-REGISTRY` — a package registry, whose index is the
//! change feed: the site asks it what exists now and compares that with
//! what it rendered. `##SITE-SOURCE-HOST` — the host's own repository,
//! which is not a package at all (its root is a `[project]`), so it is
//! read as a checkout on disk that the deploy keeps current
//! (`##SITE-HOST-CHECKOUT`). Both are configured in the form a project's
//! `[[registry]]` already has, because a second shape for «where packages
//! come from» would be a second answer to a question this project settled
//! once.
//!
//! ## What this module refuses to remember
//!
//! No publication history, no «new since», no revision. A version number
//! may be published ten times a day and the site shows the last
//! publication (`##SITE-VERSION-SHOWS-CURRENT`); what the builder keeps
//! is what it rendered and the content hash it rendered it from, which is
//! the smallest fact that answers «has this moved» without pretending to
//! remember a past (§14, campaign decision D-27).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-TWO-SOURCES");

pub mod config;

pub use config::{Analytics, Host, Registry, Site, Theme};
