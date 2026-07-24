# PROP-006: Operating modes — codeword-triggered work postures {#root}

<status stage="freeze" state="done" comment="pilot markup 2026-07-24: extracted to the operating-modes flow; this entry is a live thin pointer; fact grain 2026-07-24"/>

##status-line **Status:** accepted 2026-05-06; the framework and its codewords were extracted to the `operating-modes` flow 2026-07-14 (reached via the redbook dependency). This entry is now a thin pointer.
**Related:** [`CLAUDE.md`](../../CLAUDE.md) (the four rules + session-end codeword), [PROP-000](PROP-000.md) (foundation). @freeze/done

##modes-extracted vibevm's codeword-triggered operating postures — the safety-first default, why a codeword is an explicit opt-in into an alternate posture for a work cycle, the activation lifecycle and reporting cadence, and the shape for writing a new codeword — are the **operating-modes** flow: @freeze/done

- ##flow-framework Framework: `spec://org.vibevm.world/operating-modes/flows/operating-modes/OPERATING-MODES-PROTOCOL#root` @freeze/done
- ##flow-writing-codeword Writing a new codeword: `spec://org.vibevm.world/operating-modes/flows/operating-modes/writing-a-codeword#root` @freeze/done

##CATALOGUE-AT-BOOT The codeword catalogue is surfaced at session boot by [`spec/boot/90-user.md`](../boot/90-user.md). @freeze/done

## «move fast and break things» {#mfbt}

<status stage="freeze" state="done" comment="codeword content lives in mfbt-mode; pointer verified against the flow 2026-07-24"/>

##mfbt-pointer The pre-authorised heads-down execution codeword — aim at the maximum version of the deliverable, phase the work into testable iterations, begin without asking, drive to completion without confirmation handshakes, and spend whatever time / tokens / reasoning depth it takes (the owner's verbatim description is recorded in the flow) — is the operating-modes flow's `spec://org.vibevm.world/operating-modes/flows/operating-modes/mfbt-mode#root`. @freeze/done

- ##MFBT-SUSPENDS It suspends **only** Rule 4's "ask before routine large changes". @freeze/done
- ##MFBT-RULES-SURVIVE The four non-negotiable rules survive unchanged. @freeze/done
- ##MFBT-RED-LINES Rule 4's non-routine red lines — rewriting published history, `git push --force` / `--force-with-lease`, large binary blobs, CI / signing / secrets changes, anything whose reversal would cost work — STILL require explicit owner confirmation even when the codeword is active. @freeze/done
