# Ordinary contribution logging — implementation and evidence

Status: implementation candidate; final coherent acceptance pending.

## Ownership and behavior

The existing lifecycle process runner observes its own file-backed capture through
independent read cursors. Both ordinary and controlled runners poll bounded chunks
and final-drain the measured remaining extent before capped preview reads/removal.
The CLI reserves the existing per-package/run/contribution log and an in-flight
record before execution; it appends raw diagnostic bytes rather than rebuilding a
log from the capped report. No application runner, resolver or retention framework
is introduced. Binary-handler stdout remains reply protocol, not diagnostic output.

Persistent logging is best-effort. An unavailable destination or write/observation
failure does not change the actual installation result. A diagnostic and the
record's incomplete-log state distinguish failed evidence from a complete file.
`logTruncated` continues to describe the persistent log; stdout/stderr report
truncation flags still describe bounded previews. The install-steps schema shape,
protocol and epoch remain unchanged; its descriptions are clarified to cover full
streamed logs and unavailable/partial evidence.

The shared panel and its existing key reader are reused. Task-ID log association
avoids colliding author IDs/display labels. Completion tails scan logs using bounded
memory, retaining at most 30 bounded lines, including non-UTF8 input. No total order
between stdout and stderr is promised. Quiet/JSON/machine contexts never relay raw
child output; a missing-log diagnostic stays outside machine stdout.

## Executed focused evidence

- Lifecycle process group: 15 passed, including both runners retaining more than
  1MiB on both streams with bounded previews, early output while large stdin is
  consumed, cancellation with partial output, spawn failure and existing stdin
  failure kill/reap regression.
- CLI binary unit group: 1305 passed, 0 failed, 1 ignored. New sink-unavailable,
  binary-protocol exclusion, duplicate-label TaskId association and bounded invalid
  long-line failure-tail cases passed; existing fold/toggle terminal-model cases
  passed.
- Public contribution group: 12 passed. Full logs retain beginning/end markers
  beyond both preview limits; quiet retains logs without raw output; unavailable
  evidence leaves successful install work successful with an explicit warning;
  an actual PTY install uses `--log tail --log-tail 2` and retains its complete log.
- Adjacent application steps: 6 passed. Adjacent hook lifecycle: 3 passed.

These are focused candidate checks, not a substitute for the final frozen-subject
canonical gate. The PTY case is not a Windows Terminal hotkey test or a human
sign-off. Native/manual MT-04/05/06 remain separately accounted for.

## Failed attempts retained honestly

- `cargo test -p vibe-cli --lib` could not run because the package has no library
  target; corrected to `--bin vibe`. This was an invocation error, not a test fail.
- First strict pass found two collapsible-if lints in stdin writer polling; the
  next found two in log write/flush. Fixed with let-chains, without lint allowances.
  Final strict Clippy passed (all targets of lifecycle/install/CLI, exit 0).
  Conform passed: 141 existing findings, 0 new; no baseline change.

## Machine-local evidence pointers

- `/fast/tmp/prior-app-steps-process-tests.log`
- `/fast/tmp/prior-app-steps-cli-unit-bin.log`
- `/fast/tmp/prior-app-steps-public-tests.log`
- `/fast/tmp/prior-app-steps-public-pty.log`
- `/fast/tmp/prior-app-steps-strict-final.log`
- `/fast/tmp/prior-app-steps-conform.log`

No external publication, real provider call, credential/configuration change or
native human-only PASS is implied by this candidate report.
