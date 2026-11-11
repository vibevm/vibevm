//! The sentences a launch prints about a harness whose carrier is not a
//! plain flag: what that carrier is, what it costs, and the version each
//! claim was measured on.
//!
//! **Prose, in a cell of its own, on purpose.** These are the longest
//! strings in the system-prompt surface and they are not configuration: each
//! one is a measurement written out so a reader can act on it without
//! opening the campaign's documents. Keeping them beside the flag dialect
//! made that file about two different things at once.
//!
//! Every claim here is **version-scoped**, because each is a measurement of
//! how one program assembles a prompt rather than a floor. What they gate is
//! nothing: OpenCode's carrier is gated by its installed version's own
//! `PROVENANCE.toml` and the `verified` flag in it, and Qwen Code has no
//! tier to gate (`#SYS-OPENCODE`, `#SYS-QWEN`, `#SYS-SUPPLIED-BASE`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-DELIVERY");

use std::path::Path;

use super::HarnessKind;

/// The OpenCode version this build measured the carrier's *shape* against.
///
/// Version-scoped for the same reason Codex's is: the claim is a
/// measurement of how one program assembles a prompt, and nothing here is a
/// floor. It gates **nothing** — the gate is the installed version's own
/// `PROVENANCE.toml` and its `verified` flag, which is data in OpenCode's
/// package rather than a number in vibe's source. What this constant scopes
/// is the *findings* the carrier's design rests on: that an agent's
/// `system` replaces the base, that a `subagent` child carries none of it,
/// and that the configuration `instructions` array does the opposite
/// (`#SYS-OPENCODE`, `#SYS-SUPPLIED-BASE`).
pub const OPENCODE_MEASURED_VERSION: &str = "2.2.7";

/// What OpenCode's launch carrier is, as one sentence a note prints.
///
/// **The slot that works is an agent's `system`, and the cost is that it
/// replaces.** Measured on 2.2.7: the configuration `instructions` array is
/// additive and file-based, and a `subagent` child's request came back
/// byte-identical to the primary's with the lane in it — which
/// `#SYS-WORKER-EXCLUSION` forbids outright, so that carrier is closed for
/// good. An agent's `system` is the opposite on both counts: the child's own
/// agent has a `system` of its own and receives none of ours, and the value
/// *replaces* OpenCode's base prompt instead of adding to it. So vibe
/// supplies that base — for exactly the installed version and the launch's
/// model, from versioned data with provenance in OpenCode's own package —
/// with the lane after it (`#SYS-SUPPLIED-BASE`).
///
/// **Two flags are not optional.** `--standalone` because OpenCode's
/// default mode serves sessions from one long-lived background service that
/// never sees this launch's environment, so the document would either not
/// arrive or arrive for every session that service goes on to serve; and
/// the document's own `default_agent` key, because the top-level command
/// takes no `--agent` argument at all — measured on 2.2.7, where
/// `opencode --standalone --agent <name>` prints the usage and starts
/// nothing.
pub fn opencode_carrier_description() -> String {
    format!(
        "OpenCode's launch carrier is `OPENCODE_CONFIG_CONTENT`: one per-launch configuration          layer declaring vibe's own `mode = \"primary\"` agent and selecting it with that          document's `default_agent`, whose prompt is OpenCode's OWN base prompt for the          installed version and this launch's model followed by the lane's path in a `{{file:}}`          token — so no lane byte is on a command line or in an environment value. The base is          supplied because that slot REPLACES OpenCode's own prompt rather than adding to it,          and it is read from versioned data with provenance in OpenCode's own vibe package,          never from vibe's source. `--standalone` is required: OpenCode's default mode serves          sessions from one long-lived background service that never sees this launch's          environment. The configuration `instructions` array, which IS additive, is closed for          good — on {OPENCODE_MEASURED_VERSION} a `subagent` child's request came back          byte-identical to the primary's with the lane in it, which          `PROP-076#SYS-WORKER-EXCLUSION` forbids. Carrier shape measured against OpenCode          {OPENCODE_MEASURED_VERSION}"
    )
}

/// The Qwen Code version this build measured the identity carrier on, and
/// the only one a launch delivers it to.
///
/// **One version and not a floor**, for the reason Codex's constant is one:
/// the claim is a measurement of how one program assembles a prompt, and the
/// failure mode of a carrier that stops being honoured is **silence**. It
/// gates the launch here — `vibe_workspace::system_prompt::
/// QWEN_VERIFIED_VERSION` is the same number, read by the gate — because
/// Qwen Code's carrier is an environment variable rather than data in a
/// package with a `verified` flag of its own.
pub const QWEN_MEASURED_VERSION: &str = "0.27.2";

/// What Qwen Code's launch carrier is, why it is clean, and what remains
/// closed beside it — as one sentence a note prints.
///
/// **The slot that works replaces exactly one paragraph.**
/// `QWEN_SYSTEM_IDENTITY_MD` names a file whose contents replace the base
/// prompt's first paragraph — its single identity sentence — and nothing
/// else; upstream's own tests pin both halves, «should replace only the
/// identity sentence when identity env points to a file» and «keeps the
/// style section under a `QWEN_SYSTEM_IDENTITY_MD` override». So the file is
/// Qwen Code's own sentence, a blank line, and the lane, and the prompt the
/// program builds is its own prompt with one insertion in it.
///
/// **Why that is clean where `QWEN_SYSTEM_MD` was not.** Replacing the whole
/// base prompt needed vibe to know a prompt that is a function of the
/// *session* — the interaction mode, the tool surface, the model, whether
/// the directory is a git repository, whether a sandbox is set — which a
/// launcher cannot learn without starting a second session. The identity
/// **sentence** is a function of two things, the interaction mode and
/// whether an output style is active, and vibe never stores it: it is
/// sampled at launch from the installed package's own `getCoreSystemPrompt`,
/// the way upstream's own test samples it, and every other byte of the
/// prompt is still built live by Qwen Code for the session it is starting.
///
/// **And the two slots that stay closed.** `--append-system-prompt` is
/// additive and subagent-safe and takes the instructions as **text**, where
/// no lane byte of this project may go under any delivery; `QWEN_SYSTEM_MD`
/// is file-based and subagent-safe and replaces the whole session-shaped
/// base prompt. [`qwen_closed_slots`] is that half, kept separate because it
/// is what a `read` delivery and a soft launch have to say and the carrier
/// description is not.
pub fn qwen_carrier_description() -> String {
    format!(
        "Qwen Code's launch carrier is `QWEN_SYSTEM_IDENTITY_MD`: a path to a file holding \
         Qwen Code's OWN identity sentence for this launch's session shape, a blank line, \
         and the lane — so no lane byte is on a command line or in an environment value. \
         That variable replaces the base prompt's FIRST PARAGRAPH and nothing else, which \
         upstream's own tests pin, so Qwen Code still builds every other byte of the prompt \
         itself, live, for the session it is actually starting. The sentence is never \
         stored: it is sampled at launch from the INSTALLED package's own \
         `getCoreSystemPrompt`, the way upstream's own test samples it, and a package that \
         no longer honours the variable or replaces more than that paragraph fails the \
         launch's own check and starts softly. A subagent never enters that path and \
         receives none of it. Measured against Qwen Code {QWEN_MEASURED_VERSION}, which is \
         also the one version a launch delivers it to"
    )
}

/// The two Qwen Code slots that remain closed, and what closes each.
///
/// **The additive slot is the right shape and the wrong medium.**
/// `--append-system-prompt` is per-launch, additive, lands inside the
/// prompt's cache-stable prefix and is **not** inherited by an ordinary
/// subagent — four properties out of four. It takes the instructions as
/// **text**, there is no file variant of it, and
/// `#SYS-HARNESS-DELIVERY` puts no lane byte on a command line under any
/// delivery — **a rule, not a threshold**, because one argument is capped at
/// 128 KiB on Linux and a whole command line at about 32,767 characters on
/// Windows, so a project delivered by measured size would arrive for one
/// contributor and fail for the next. The measurement behind the rule: the
/// 202,349-byte sample lane is `E2BIG` — «Argument list too long», errno 7 —
/// at `execve`, before Qwen Code starts, and an environment string is capped
/// the same way.
///
/// **The whole-prompt slot works and is still refused.** `QWEN_SYSTEM_MD`
/// names a file, is read only on the main session's own prompt path and
/// *replaces* the base prompt; composing Qwen Code's own base with the lane
/// after it was measured end to end and arrives byte-exact. What closes it
/// is the prompt it replaces: measured on 0.27.2 under identical probe
/// flags, the headless base is **13,856** bytes and the interactive one
/// **18,451**, differing by whole sections, and a probe with no terminal is
/// always the headless one. It also disables the user's
/// `general.outputStyle`, which Qwen Code says itself. The identity carrier
/// is the same idea reduced to the one paragraph that *is* knowable, which
/// is why it ships and this does not.
pub fn qwen_closed_slots() -> String {
    format!(
        "Qwen Code's other two instruction slots stay closed, for opposite reasons. \
         `--append-system-prompt` is additive and is **not** inherited by an ordinary subagent, \
         so worker exclusion is satisfied — but it takes the instructions as TEXT, and no lane \
         byte goes on a command line in any delivery of this project, on any platform: that is \
         one rule rather than a size threshold, since a single argument is capped at 128 KiB on \
         Linux and a whole command line at about 32,767 characters on Windows. The measurement \
         behind the rule: the 202,349-byte sample lane is `E2BIG` at `execve`, before the \
         program starts, and an environment string is capped the same way. `QWEN_SYSTEM_MD` IS \
         file-based and subagent-safe, and composing Qwen Code's own base prompt with the lane \
         after it was measured to arrive byte-exact and exactly once; but that slot REPLACES the \
         whole base prompt, and the prompt it replaces is a function of the session rather than \
         of the version — 13,856 bytes headless against 18,451 interactive under identical probe \
         flags — and it disables the user's `general.outputStyle` besides. The identity carrier \
         is that idea reduced to the one paragraph a launcher CAN know. Measured against Qwen \
         Code {QWEN_MEASURED_VERSION}"
    )
}

/// The note a kind with no deliverable slot prints: what the carrier is,
/// why it is not a flag, and why vibe does not use it.
///
/// It is printed under **both** deliveries, because the question it answers
/// — "where would this harness's lane have gone" — does not depend on which
/// tier the project declared.
pub(super) fn no_slot_note(kind: HarnessKind, lane: &Path) -> Option<String> {
    match kind {
        HarnessKind::Claude | HarnessKind::Codex => None,
        HarnessKind::OpenCode => Some(format!(
            "{}. The carrier is never a flag: the document travels in the environment and names \
             `{}` inside it, so under `launch` the flags are only the ones that make it \
             reachable. The agent reads the boot files itself instead",
            opencode_carrier_description(),
            lane.display()
        )),
        HarnessKind::Qwen => Some(format!(
            "{}. {} The carrier is never a flag: the file travels as a PATH in the environment \
             and holds `{}` inside it, so under `launch` the flags are only the user's own. The \
             agent reads the boot files itself instead",
            qwen_carrier_description(),
            qwen_closed_slots(),
            lane.display()
        )),
    }
}
