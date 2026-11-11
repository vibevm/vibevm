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

/// The Qwen Code version this build measured its carrier question against.
///
/// Version-scoped for the same reason Codex's and OpenCode's are: the claim
/// is a measurement of how one program assembles a prompt. It gates nothing
/// — there is no delivery to gate — it scopes the *findings* that keep the
/// delivery closed.
pub const QWEN_MEASURED_VERSION: &str = "0.27.2";

/// Why Qwen Code has no deliverable launch tier, as one sentence naming both
/// of its slots, what closes each, and the version they were measured on.
///
/// **Qwen Code has two per-launch instruction slots, and the campaign
/// measured both.** Neither can carry this project's lane, and they fail for
/// opposite reasons — which is why the sentence names both rather than
/// stopping at the first.
///
/// **The additive slot is the right shape and the wrong medium.**
/// `--append-system-prompt` is per-launch, additive, lands inside the
/// prompt's cache-stable prefix and is **not** inherited by an ordinary
/// subagent, so `#SYS-WORKER-EXCLUSION` is satisfied — four properties out
/// of four. It takes the instructions as **text**, there is no file variant
/// of it, and nothing else adds to the prompt beside it. So the lane would
/// have to travel as one argument, and `#SYS-HARNESS-DELIVERY` puts no lane
/// byte on a command line under any delivery — **a rule, not a threshold**,
/// because one argument is capped at 128 KiB on Linux and a whole command
/// line at about 32,767 characters on Windows, so a project delivered by
/// measured size would arrive for one contributor and fail for the next.
/// The measurement behind the rule: the 202,349-byte sample lane is `E2BIG`
/// — «Argument list too long», errno 7 — at `execve`, before Qwen Code
/// starts, and an environment string is capped the same way.
///
/// **The file-based slot works, and the owner's composition with it works.**
/// `QWEN_SYSTEM_MD` names a file, is read only on the main session's own
/// prompt path — `isSystemMdActive` is consumed inside `getCoreSystemPrompt`,
/// which a subagent never enters — and it *replaces* the base prompt rather
/// than adding to it. The owner's answer to that was to supply Qwen Code's
/// own prompt followed by the lane, and it holds: `QWEN_WRITE_SYSTEM_MD`
/// makes the installed program write its base prompt to a file, the lane is
/// appended to that copy, and the captured request then begins with Qwen
/// Code's own prompt **byte-exact**, carries the lane **exactly once** at
/// byte 13,858, and continues byte-identically to a plain run after its
/// base. The probe costs nothing and calls nothing: with no prompt and stdin
/// closed, Qwen Code writes the dump and then refuses for want of input.
///
/// **What closes that slot is that the prompt it replaces is a function of
/// the session, not of the version.** The base prompt is built from the
/// interaction mode, the tool configuration, the model, whether the working
/// directory is a git repository and whether a sandbox is set — and the
/// interaction mode alone is decisive: measured on 0.27.2 under identical
/// probe flags, the headless variant is **13,856** bytes and the interactive
/// one **18,451**, differing by whole sections rather than a sentence, and a
/// probe with no terminal is **always** the headless one, because
/// `--prompt-interactive` refuses piped input and a run without a tty is
/// headless whatever is asked of it. So a launcher cannot know which copy a
/// given launch needs without allocating a pty and starting a second
/// session, and handing over the wrong copy would silently swap the
/// harness's own instructions — strictly worse than delivering no lane,
/// which is `#SYS-SOFT-LAUNCH`'s standing judgement. A stored copy per
/// verified version does not help either: it would be stale along every axis
/// above on the first launch, not merely at the next upgrade. One more cost
/// is recorded rather than discovered later: `QWEN_SYSTEM_MD` also disables
/// the user's `general.outputStyle`, which Qwen Code says itself
/// (`#SYS-QWEN`).
pub fn qwen_launch_blocker() -> String {
    format!(
        "Qwen Code has two per-launch instruction slots and neither can carry this project's \
         lane. `--append-system-prompt` is additive and is **not** inherited by an ordinary \
         subagent, so worker exclusion is satisfied — but it takes the instructions as TEXT, and \
         no lane byte goes on a command line in any delivery of this project, on any platform: \
         that is one rule rather than a size threshold, since a single argument is capped at 128 \
         KiB on Linux and a whole command line at about 32,767 characters on Windows. The \
         measurement behind the rule: the 202,349-byte sample lane is `E2BIG` at `execve`, before \
         the program starts, and an environment string is capped the same way. `QWEN_SYSTEM_MD` \
         IS file-based and subagent-safe, and composing Qwen Code's own base prompt — dumped by \
         `QWEN_WRITE_SYSTEM_MD` — with the lane after it was measured to arrive byte-exact and \
         exactly once; but that slot REPLACES the base prompt, and the prompt it replaces is a \
         function of the session rather than of the version: 13,856 bytes headless against 18,451 \
         interactive under identical probe flags, and a probe without a terminal is always the \
         headless one. Handing over the wrong copy would silently swap the harness's own \
         instructions, which is worse than delivering no lane, and it would also disable the \
         user's `general.outputStyle`. Measured against Qwen Code {QWEN_MEASURED_VERSION}"
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
            "{}. Both slots exist, which is why this note names them rather than denying them: \
             the argument would have been `--append-system-prompt \"$(cat {lane})\"`, the \
             lane's own bytes on the command line, and the file-based assignment would have \
             been `QWEN_SYSTEM_MD=<a vibe-owned copy of Qwen Code's base prompt with {lane} \
             after it>`. The agent reads the boot files itself instead",
            qwen_launch_blocker(),
            lane = lane.display()
        )),
    }
}
