# DEF-C2 slice — the falsifier's repairs, landed

_2026-07-10 21:30. Owner-facing report per the reports law. The slice
executed the owner's post-close order («…вероятно нужно будет вначале
доделать DEF-C2-1…4») against the CLOSED campaign plan's §15
deferrals. Preceded by the rulings commit: MT-C2-01…04 signed, RP2
resolved ON, RP3 resolved settings.local.json._

## What was done

Three mechanical repairs and one pre-registration — one per Ф6
falsifier mechanism, each verified before commit:

1. **DEF-C2-2a — the staging toolchain (F24).** The trial's `env -i`
   boss env broke cargo TWICE: the rustup shim could not resolve a
   toolchain under the scratch USERPROFILE, and rustc's MSVC
   auto-detect (vswhere lives under `ProgramFiles(x86)`) silently
   fell back to Git's GNU `link.exe`, which cannot link test
   binaries. Repro'd both layers in scratch (`cargo build` hides the
   second — lib crates don't link; `cargo test --no-run` exposes
   it), then fixed the runner: RUSTUP_HOME/CARGO_HOME passthrough +
   the ProgramFiles family. Verified: link-fails without, links
   clean with.
2. **DEF-C2-1 — the mid-work nudge channel (F23).**
   `decide_midwork_nudge` in the engine + PostToolUse
   `additionalContext` emission in the hook. Same threshold, same
   session-level cooldown anchor (the fatigue bound stays "one nudge
   per window per session across ALL channels" — no new state), a
   distinct journal reason (`work-tool-threshold-midwork`) so the
   re-run can measure channels apart, its own `midwork_nudges`
   config switch under the same kill switch. D5 rewritten in place
   with the falsifier as the reason. Staged smoke: silent through
   event 6, fires at 7 with the exact text, cooldown-quiet at 8,
   kill switch and config-off both silent. P4 re-bench after the
   extra MC round-trip: **P95 50 ms** (was 51 — the localhost RTT is
   noise), 2× headroom intact.
3. **DEF-C2-3 — the cold-start board (F25).** When all-time runs =
   0, the board no longer renders zero counters ("all-time: 0 runs"
   was anti-proof at the only moment the injection speaks); it
   states the measured fact ("fabric ready — no delegated runs on
   this box yet") and leads with the route verb + spawn pointer +
   skill. D7-honest: no invented numbers. Three engine tests pin the
   text, the switch-back at one run, and the live-session line
   surviving; smoked live through both surfaces (SessionStart
   injection + `scoreboard`).
4. **DEF-C2-4 — MT-C2-05, the re-run protocol.** Pre-registered and
   UNFIRED: arms A′ (repaired baseline) / B′ (repaired initiative),
   3+3, frozen scoring identical to MT-C2-01, new fatigue facts
   (nudges by reason, acted-on proxy), predictions PR1 (A′ ≥ A —
   the confound theory), PR2 (every B′ run fires ≥ 1 mid-work
   nudge — mechanism proof), PR3 (B′ ≥ A′ + 30 — the original P3
   delta clause, now with a live channel). **Every paid run is gated
   on RP5 (OPEN)** — nothing fires without the owner's word.

## Decisions taken

- The mid-work channel ships **default ON**: the shared cooldown
  anchor already bounds fatigue to the same budget the owner
  accepted for the prompt channel; a separate default-OFF would
  reproduce exactly the F23 silence the owner ordered repaired.
  Fatigue is measured, not assumed — MT-C2-05 PR2/fatigue facts.
- DEF-C2-2b (worker-credibility facts on the boss surface) shipped
  as its honest thin slice ONLY: the cold board now tells a
  zero-run box the truth instead of implying disuse. The full form
  ("workers run cargo test green here: yes/no, last proven <when>")
  needs acceptance-schema plumbing MC does not have — explicitly
  left in §15 for the next campaign, not silently absorbed.
- The slice ran as a direct-order follow-up, not a new campaign:
  one session, four commits, every item pre-specified in §15 (the
  campaign-plans threshold sits right at this boundary; the
  started/completed dashboards + this report carry the record).

## Left undone / risks

- MT-C2-05's paid arms — RP5 is OPEN by design.
- DEF-C2-2b-full (acceptance-backed credibility facts) — next
  campaign, named in §15.
- The env whitelist is verified against THIS box's VS/rustup layout;
  other boxes may need more vars (the runner comment says extend
  there).
- Nudge-fatigue for the new channel is unmeasured until MT-C2-05
  runs — the cooldown bound is an argument, not yet field data.
