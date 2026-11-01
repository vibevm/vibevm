//! The question, and the refusal when there is nobody to ask
//! (`PROP-066#consent`).
//!
//! The default is always a question. When a run would apply an offer that
//! is neither accepted nor rejected, an attended terminal shows the
//! package, its description, every row it carries with the computed
//! diagnostics of each target, the host footer, and asks `apply the N
//! remap rows?` with `no` as the default, through the shared prompt theme.
//! An unattended run refuses with `consent_required`, naming
//! `vibe remap accept <package>` and carrying the `vibe-remap-offer/1`
//! document, which is the same question a graphical client renders.
//!
//! The flags answer it without a terminal: `--accept-remap` accepts every
//! offer the run meets, `--accept-remap <package>` (repeatable) accepts
//! named ones, and `--reject-remap` drops every unanswered offer **for
//! this run only** and records nothing — the one answer that leaves the
//! ledger untouched, because "not now" is not "never".

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#consent");

use anyhow::{Context, Result};
use dialoguer::Confirm;
use vibe_core::remap::{OfferState, RemapPackage, RemapRow};
use vibe_wire::generated::remap_offer as wire;

use crate::output;

use super::diagnostics;
use super::home::RemapHome;
use super::render;

/// The `vibe-remap-offer/1` protocol.
pub(crate) const OFFER_PROTOCOL: &str = "vibe-remap-offer/1";

/// How this invocation answers an unanswered offer.
#[derive(Debug, Clone, Default)]
pub(crate) struct ConsentFlags {
    /// `--accept-remap` with no value: every offer this run meets.
    pub(crate) accept_all: bool,
    /// `--accept-remap <package>`, repeatable.
    pub(crate) accept: Vec<String>,
    /// `--reject-remap`: this run only, nothing recorded.
    pub(crate) reject: bool,
}

impl ConsentFlags {
    /// Read the flags out of the repeatable `--accept-remap` values and
    /// the `--reject-remap` switch. A bare `--accept-remap` arrives as an
    /// empty value, which is how one flag carries both spellings.
    pub(crate) fn from_args(accept_remap: &[String], reject_remap: bool) -> Self {
        ConsentFlags {
            accept_all: accept_remap.iter().any(|value| value.trim().is_empty()),
            accept: accept_remap
                .iter()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .collect(),
            reject: reject_remap,
        }
    }

    /// Whether this flag set answers the offer for `key` with yes.
    fn accepts(&self, key: &str) -> bool {
        self.accept_all || self.accept.iter().any(|named| named == key)
    }
}

/// A refused run: something needed the user's word and there was nobody to
/// ask (`PROP-066#consent`, `PROP-066#uninstall`).
///
/// The same code and class serve both questions, because they are the same
/// question — a remap changes **where** artifacts are fetched from, and
/// that is never decided for the user by a script.
#[derive(Debug, thiserror::Error)]
#[error(
    "consent_required: {count} {what} need(s) an answer, and this run has nobody to ask — \
     {packages} \
     (violates spec://org.vibevm.core/vibevm/common/PROP-066#consent; \
      fix: `{accept}`, or `{reject}`; `vibe remap list` shows every row involved)"
)]
pub(crate) struct ConsentRequired {
    /// How many things were unanswered.
    pub(crate) count: usize,
    /// What they are — `remap package offer(s)`, `orphaned remap row(s)`.
    pub(crate) what: &'static str,
    /// Their keys or spellings, comma-separated.
    pub(crate) packages: String,
    /// The exact command that answers yes.
    pub(crate) accept: String,
    /// The exact command that answers no.
    pub(crate) reject: String,
}

/// Decide which offers of `home` apply to this run, asking or refusing as
/// `PROP-066#consent` requires.
///
/// The answer is the offers whose rows the Locator should carry. Accepting
/// here writes the ledger and rebuilds the derived provider map, so the
/// next run does not ask again; `--reject-remap` writes nothing, which is
/// what makes it a this-run answer.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
pub(crate) fn resolve(
    ctx: &output::Context,
    home: &mut RemapHome,
    flags: &ConsentFlags,
    offline: bool,
) -> Result<Vec<RemapPackage>> {
    let pending = home.pending();
    if pending.is_empty() {
        return Ok(home.accepted());
    }
    if flags.reject {
        // Nothing recorded, nothing applied: the offers stay `offered`
        // and the next run asks again.
        return Ok(home.accepted());
    }
    let mut answered: Vec<(String, bool)> = Vec::new();
    let mut unanswered: Vec<&RemapPackage> = Vec::new();
    for offer in &pending {
        if flags.accepts(&offer.key) {
            answered.push((offer.key.clone(), true));
            continue;
        }
        unanswered.push(offer);
    }
    if !unanswered.is_empty() {
        if !attended(ctx) {
            let keys: Vec<&str> = unanswered.iter().map(|offer| offer.key.as_str()).collect();
            if ctx.is_json() {
                // The envelope carries the question itself, so a client
                // can render it and answer with `remap accept`.
                let document = offer_document(&unanswered, home, offline);
                ctx.emit_json(&document)
                    .context("emitting the remap offer document")?;
            }
            return Err(ConsentRequired {
                count: keys.len(),
                what: "remap package offer(s)",
                packages: keys.join(", "),
                accept: format!("vibe remap accept {}", keys.join(" ")),
                reject: format!("vibe remap reject {}", keys.join(" ")),
            }
            .into());
        }
        for offer in &unanswered {
            answered.push((offer.key.clone(), ask(ctx, offer, home, offline)?));
        }
    }
    if answered.is_empty() {
        return Ok(home.accepted());
    }
    let when = crate::commands::init::current_timestamp_utc();
    let _lock = home.lock()?;
    for (key, accepted) in &answered {
        let Some(entry) = home.installed.find(key) else {
            continue;
        };
        let hash = entry.package.content_hash.clone();
        if *accepted {
            home.ledger.accept(key, &hash, &when);
        } else {
            home.ledger.reject(key, &hash, &when);
        }
    }
    home.save()?;
    Ok(home.accepted())
}

/// Whether this run may ask: a terminal, not `--json`, not `--unattended`.
fn attended(ctx: &output::Context) -> bool {
    !ctx.is_json() && !ctx.is_unattended() && console::user_attended()
}

/// Show one offer in the layout of `PROP-066#display` and ask.
fn ask(
    ctx: &output::Context,
    offer: &RemapPackage,
    home: &RemapHome,
    offline: bool,
) -> Result<bool> {
    let document = offer_document(std::slice::from_ref(&offer), home, offline);
    ctx.suspend_progress(|| {
        eprint!("{}", render::offer(&document));
        crate::output::prompts::space_before_question();
        Confirm::with_theme(&crate::output::prompt_theme())
            .with_prompt(format!(
                "apply the {} remap row{}?",
                offer.section.rows.len(),
                if offer.section.rows.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ))
            .default(false)
            .interact()
            .context("reading the remap consent answer")
    })
}

/// Build the `vibe-remap-offer/1` document for a set of offers.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#display")]
pub(crate) fn offer_document(
    offers: &[&RemapPackage],
    home: &RemapHome,
    offline: bool,
) -> wire::RemapOffer {
    let keys: Vec<&str> = offers.iter().map(|offer| offer.key.as_str()).collect();
    wire::RemapOffer {
        protocol: OFFER_PROTOCOL.into(),
        packages: offers
            .iter()
            .map(|offer| offered_package(offer, home, offline))
            .collect(),
        accept_command: format!("vibe remap accept {}", keys.join(" ")),
        reject_command: format!("vibe remap reject {}", keys.join(" ")),
    }
}

/// One package's header block and rows.
fn offered_package(offer: &RemapPackage, home: &RemapHome, offline: bool) -> wire::OfferedPackage {
    let rows: Vec<RemapRow> = offer
        .loaded(vibe_core::remap::Layer::UserPackage)
        .map(|loaded| loaded.rows)
        .unwrap_or_default();
    let mut targets: Vec<(String, _, Option<String>)> = Vec::new();
    let mut blocks = Vec::with_capacity(rows.len());
    for (position, row) in rows.iter().enumerate() {
        let found = diagnostics::diagnose(&row.key, &row.target, offline);
        targets.push((
            found.scheme.clone(),
            found.reach.clone(),
            found.host.clone(),
        ));
        blocks.push(wire::OfferedRow {
            number: position as u32 + 1,
            kind: render::offer_kind(&row.key),
            what: row.key.what().to_owned(),
            to: row.target.display(),
            deny: row.target.is_deny(),
            scheme: found.scheme,
            reach: render::offer_reach(found.reach),
            host: found.host,
            why: row.reason.clone(),
            scope: row.scope.as_ref().map(|scope| match &scope.step {
                Some(step) => format!("{}, step {step}", scope.package),
                None => scope.package.clone(),
            }),
            notes: found.notes.into_iter().map(render::offer_note).collect(),
        });
    }
    wire::OfferedPackage {
        package: offer.key.clone(),
        version: offer.version.clone(),
        content_hash: offer.content_hash.clone(),
        state: offer_state(home.state(&offer.key).unwrap_or(OfferState::Offered)),
        description: offer.section.description.clone(),
        registry: home
            .installed
            .find(&offer.key)
            .and_then(|entry| entry.registry.clone()),
        policy: (!offer.policy.is_default()).then(|| "deny".to_owned()),
        rows: blocks,
        hosts: diagnostics::host_summary(&targets)
            .into_iter()
            .map(|host| {
                format!(
                    "{} row{} → {}",
                    host.rows,
                    if host.rows == 1 { "" } else { "s" },
                    host.host
                )
            })
            .collect(),
    }
}

/// The wire spelling of an offer state.
pub(crate) fn offer_state(state: OfferState) -> wire::OfferState {
    match state {
        OfferState::Offered => wire::OfferState::Offered,
        OfferState::Accepted => wire::OfferState::Accepted,
        OfferState::Rejected => wire::OfferState::Rejected,
        OfferState::Disabled => wire::OfferState::Disabled,
    }
}
