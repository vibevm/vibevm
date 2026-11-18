# Native authority and attended stop repairs — 2026-11-18

Base: `ddc7e6863405f504a8a4ef0e868cc12dad1cbdb7`.
Status: bounded repair candidate, final coherent/native verdict pending.

## Measured defects

1. Current native Windows ordinary install failed after boot publication because
   the specmap authority publisher unconditionally requested exact Unix0600.
   The strict filesystem owner correctly refused unsupported Unix permissions.
2. The full canonical diagnostic run encountered an attended cancellation hang
   after the contribution panel was introduced. Thread-local panel release did
   not transfer terminal ownership to the stop-question thread, which also held
   renderer suspension over a blocking prompt.

The old canonical run is **aborted/nonPASS**, not a green result. Its hung test
chain was stopped by exact owned process identity and the later diagnostic run
was ended with exit143. Prior Windows native build success does not turn its
functional installation failure into PASS. All original logs/receipts remain.

## Corrections at existing owners

- Authority publisher requests0600 on Unix and no exact Unix mode on other
  platforms. `vibe-safefs` stays strict and unchanged. Inherited Windows permissions
  are ordinary same-user behavior, not a promised private ACL.
- Context clones share an invocation-owned weak terminal-panel handle. The stop
  question releases/joins the panel key reader before taking terminal input and
  prompts without a renderer lock. A no-panel prompt holds progress without
  retaining the renderer lock. RAII restores ownership after answer/error/unwind.
- Existing panel close/reclaim transitions are serialized; a closed session is
  not restarted by a late reclaim. No global singleton or second key reader.
- The existing PTY test has finite timeout/drain failure and owned descendant/
  process-group cleanup. It does not signal an already reaped child PID or call
  timeout a skip. Existing rollback/leftover assertions remain unchanged.

## Latest executed focused checks

- CLI binary unit tests: **1308 passed, 0 failed, 1 ignored**. Includes cross-thread
  release/unwind, actual closed PanelSession reclaim and no-panel render callback.
- Public contribution logging: **12/12 passed**, including actual attended PTY,
  full logs beyond both bounded previews, quiet and unavailable evidence.
- Public project cancellation: **11/11 passed**, including attended yes/no, rollback
  and retained no-leftovers assertions. Previously hung journeys finish normally.
- Public specmap authority: **6/6 passed**, including portable ordinary empty install,
  stable repeated bytes/mtime and newly created Unix0600.
- Forced1second PTY timeout with a separately grouped real child: explicit failure,
  transcript retained, owned child absent afterward. This is negative cleanup proof,
  not a successful install.

Strict/conform and final source-bound native/full-gate results are recorded only
when terminal. No human GUI/hardware/real-agent assertion follows from these tests.

## Local evidence

- `/fast/tmp/prior-repair-final-unit.log`
- `/fast/tmp/prior-repair-final-public.log`
- `/fast/tmp/prior-repair-final-strict.log`
- `/fast/tmp/prior-repair-final-conform.log`
- `/fast/tmp/prior-pty-timeout-cleanup-proof.json`
- `/fast/tmp/prior-ddc7-canonical-aborted-20261118.json`
- `/fast/tmp/prior-ddc7-cancellation-hang-20261118.json`
- `/fast/tmp/prior-ddc7-hung-test-chain-stop-20261118.json`

Owner boundary: finish PRIOR-APP-STEPS, then stop for subsequent server-maintenance
instructions. Do not automatically start Hermes/Pi, NST repair or maintenance.
