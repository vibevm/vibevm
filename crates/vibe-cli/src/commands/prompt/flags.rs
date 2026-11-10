//! `vibe prompt flags` — flags only, never a program.
//!
//! That is the whole property: the output is appended to whatever wrapper,
//! alias or launcher a developer already has, which is why the argument may
//! be either a harness **kind** or a **launcher name** and why no program
//! ever appears in the answer (`#SYS-HARNESS-EXECUTABLE`).
//!
//! It also writes nothing. Decision D4 allowed `vibe prompt flags` to write
//! Codex's launch-time profile file beside `vibe agent`; it does not, by the
//! narrower rule that a command whose job is to print is a command that
//! prints. The note in the answer says which command writes the file.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use anyhow::{Result, bail};
use vibe_app::{AgentsConfig, AppError};
use vibe_core::manifest::HarnessDelivery;
use vibe_wire::generated::prompt_flags as wire;
use vibe_workspace::system_prompt::{HarnessKind, LaunchFlags, check_node_prompt, launch_flags};

use crate::commands::agent::launchers;
use crate::commands::app::global;
use crate::output;

use super::Selected;

/// The protocol tag `vibe-prompt-flags/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-flags/1";

pub(crate) fn run(ctx: &output::Context, selected: &Selected, harness: &str) -> Result<()> {
    let (kind, launcher) = resolve_kind(selected, harness)?;
    let composed = compose(selected, kind, launcher.as_ref())?;
    // **A command whose job is to print refuses what it may not print.**
    // `vibe agent` starts the harness anyway on exactly these reasons, with
    // no slot flag and one loud warning; here there is no session to save,
    // and flags pointing at a lane that is not there — or at a carrier this
    // harness may not read — are the one answer a shell alias turns
    // silently into a session with no lane in it (`#SYS-SOFT-LAUNCH`).
    let composed = match composed.undeliverable {
        Some(undeliverable) if !undeliverable.printable => {
            bail!("{}", undeliverable.sentence());
        }
        Some(undeliverable) => {
            let mut flags = composed.flags;
            flags.notes.push(format!(
                "{} — these flags point at the bytes on disk now",
                undeliverable.sentence()
            ));
            flags
        }
        None => composed.flags,
    };
    let report = wire::PromptFlagsReport {
        protocol: PROTOCOL.to_owned(),
        node: selected.node_rel.clone(),
        launcher: launcher.map(|launcher| launcher.name),
        kind: kind.as_str().to_owned(),
        delivery: kind.delivery(&selected.manifest).as_str().to_owned(),
        flags: composed.flags.clone(),
        notes: composed.notes.clone(),
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    // One line of flags on stdout, so `claude $(vibe prompt flags --harness
    // claude)` is the whole integration; everything else is stderr.
    println!("{}", shell_line(&composed.flags));
    for note in &composed.notes {
        eprintln!("note: {note}");
    }
    Ok(())
}

/// Either a kind or a launcher name. A kind is tried first, so `--harness
/// claude` means the dialect even on a machine with a launcher of another
/// name bound to it.
fn resolve_kind(
    selected: &Selected,
    harness: &str,
) -> Result<(HarnessKind, Option<launchers::Launcher>)> {
    if let Some(kind) = HarnessKind::parse(harness) {
        return Ok((kind, None));
    }
    launchers::refuse_project_launchers(&selected.workspace.root)?;
    let settings = global::settings_root()?;
    let config = AgentsConfig::load(&settings).map_err(AppError::from)?;
    let launcher = launchers::resolve(&settings, &config, harness)?;
    Ok((launcher.kind, Some(launcher)))
}

/// Whether this node has opted in to a system prompt at all.
///
/// `vibe agent` asks this before composing: a node with no authored template
/// has no lane, and the launch then starts the harness unchanged, which is
/// exactly the classic boot the block's router already handles. Printing
/// flags for such a node stays a refusal in [`compose`] — there is nothing
/// to print.
pub(crate) fn opted_in(selected: &Selected) -> Result<bool> {
    let tracked = selected.tracked();
    let request = selected.request(&tracked, None);
    Ok(check_node_prompt(&request)?.opted_in)
}

/// Why one kind's lane must not be put in the slot, and the command that
/// fixes it.
///
/// **The same facts serve two different answers**, which is why this is a
/// value and not a refusal. `vibe prompt flags` prints or refuses; `vibe
/// agent` starts the harness without the carrier and warns. One cell
/// deciding both would have to know which caller it is in
/// (`#SYS-SOFT-LAUNCH`).
pub(crate) struct Undeliverable {
    /// What is wrong, as one clause.
    pub(crate) reason: String,
    /// What the reader does about it, as one imperative clause naming the
    /// exact command.
    pub(crate) fix: String,
    /// Whether `vibe prompt flags` still prints the flags beside a note.
    ///
    /// True only for a lane that **exists** and is merely out of date: the
    /// flags then point at real bytes, and a human reading them can decide
    /// for themselves. False is a refusal there — flags pointing at a file
    /// that is not there, or at a carrier this harness may not read, are
    /// the one answer a shell alias turns silently into a session with no
    /// lane in it. A *launch* never delivers either case.
    pub(crate) printable: bool,
}

impl Undeliverable {
    /// A reason no surface may print flags for.
    pub(crate) fn hard(reason: String, fix: String) -> Self {
        Self {
            reason,
            fix,
            printable: false,
        }
    }

    /// A reason `vibe prompt flags` prints beside a note, and a launch
    /// still refuses to deliver.
    fn printable(reason: String, fix: String) -> Self {
        Self {
            reason,
            fix,
            printable: true,
        }
    }

    /// The one sentence both surfaces say it with.
    pub(crate) fn sentence(&self) -> String {
        format!("{} — {}", self.reason, self.fix)
    }
}

/// What one kind's launch would carry for this node.
pub(crate) struct Composed {
    /// The flags, notes and carrier as composed over the artifacts that are
    /// on disk now. Present even when the lane must not be delivered,
    /// because `vibe prompt flags` prints what is there for the one reason
    /// it may; a launch reads [`Composed::undeliverable`] first.
    pub(crate) flags: LaunchFlags,
    /// Why the lane must not be delivered, when it must not. `None` is a
    /// lane this build is willing to put in the slot.
    pub(crate) undeliverable: Option<Undeliverable>,
}

/// Compose the flags for one kind over this node's current artifacts.
///
/// The lane is read from disk rather than recomposed, and a lane that is not
/// there is **undeliverable** naming the build: flags pointing at a file
/// that does not exist are the one answer a launch turns silently into a
/// session with no boot lane at all. For the same reason the kind's carrier
/// is gated against the installed harness's version here, in the **one**
/// code path both `vibe prompt flags` and `vibe agent` take — a carrier
/// whose failure is silent must not be printed as a working answer either.
///
/// **Both of those apply only to a `launch` delivery**, which is constraint
/// C2 of `HARNESS-DELIVERY.md` §4: a `read` harness receives nothing, so a
/// lane it does not need and a carrier version it does not use are not its
/// to be refused over. It answers with no flags and the one line saying why.
///
/// **The one refusal left here is "nothing to compose".** A node that never
/// opted in has no lane, which is not a delivery failure but an absence of
/// anything to deliver; `vibe agent` asks [`opted_in`] before calling this
/// and starts the harness unchanged, so the refusal is reached only by the
/// surface that prints (`#SYS-SOFT-LAUNCH`).
pub(crate) fn compose(
    selected: &Selected,
    kind: HarnessKind,
    launcher: Option<&launchers::Launcher>,
) -> Result<Composed> {
    let tracked = selected.tracked();
    let request = selected.request(&tracked, None);
    let answer = check_node_prompt(&request)?;
    if !answer.opted_in {
        bail!(
            "`{}` has not opted in to a system prompt (no authored `{}`), so there is no lane to \
             deliver and no flag to pass",
            selected.node_rel,
            vibe_workspace::path_to_slash(&vibe_core::layout::current_boot_system_template())
        );
    }
    // **C2 — no injection for a harness that is not in `launch`.** The
    // project asked for the router to do the work, so there is no lane to
    // point at, no carrier to gate against a harness version and nothing a
    // user's own slot flag could conflict with. Asking those questions
    // anyway is how a launch comes to be refused over a delivery it was
    // never going to perform (`HARNESS-DELIVERY.md` §4).
    let delivery = kind.delivery(&selected.manifest);
    if delivery == HarnessDelivery::Read {
        return Ok(Composed {
            flags: launch_flags(kind, delivery, "", &answer.lane.path, "", 0),
            undeliverable: None,
        });
    }
    if !answer.lane.present {
        return Ok(Composed {
            flags: LaunchFlags::default(),
            undeliverable: Some(Undeliverable::hard(
                format!("`{}` has not been generated", answer.lane.path.display()),
                format!("run `{}`", answer.remedy()),
            )),
        });
    }
    let bytes = std::fs::read_to_string(&answer.lane.path)?;
    let limit = selected.manifest.system_prompt.as_ref().map_or(
        vibe_workspace::boot_artifacts::PROJECT_DOC_MAX_BYTES,
        |section| section.codex_doc_max_bytes(),
    );
    let node_id = vibe_workspace::boot_artifacts::router_node_id(
        Some(&selected.manifest),
        &vibe_spec::SelfCoordinate::new(None, String::new()),
    );
    let mut flags = launch_flags(kind, delivery, &node_id, &answer.lane.path, &bytes, limit);
    // **The carrier gate first, because it is the harder answer.** A
    // carrier this harness may not read makes the lane's own freshness
    // beside the point, and reporting the weaker reason over the stronger
    // one is how a reader comes to run the wrong command.
    let undeliverable = match super::harness_version::gate(kind, launcher)? {
        super::harness_version::Gate::Deliver(note) => {
            flags.notes.extend(note);
            (!answer.verdict.is_equal()).then(|| {
                Undeliverable::printable(
                    format!(
                        "the lane is {} ({}), so the bytes on disk are not this node's current \
                         lane",
                        answer.verdict.as_str(),
                        answer.verdict.reasons().join("; ")
                    ),
                    format!("run `{}`", answer.remedy()),
                )
            })
        }
        super::harness_version::Gate::Undeliverable(undeliverable) => Some(undeliverable),
    };
    Ok(Composed {
        flags,
        undeliverable,
    })
}

/// One pasteable line. Only an argument that needs quoting gets it, so the
/// common case stays copy-pasteable without a shell's help.
fn shell_line(flags: &[String]) -> String {
    flags
        .iter()
        .map(|flag| {
            if flag.is_empty() || flag.contains([' ', '\t', '"', '\'', '$', '`', '\\']) {
                format!("'{}'", flag.replace('\'', "'\\''"))
            } else {
                flag.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
