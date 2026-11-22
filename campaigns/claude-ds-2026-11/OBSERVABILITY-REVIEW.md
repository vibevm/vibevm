# Observability integrated architectural review

Date: 2026-11-15. Current verdict: **final-v2 portable observer architecturally
cleared; O1/O2 resolved**. The original findings remain below as historical evidence.
This is local observability review, not package promotion, owner acceptance,
source publication or cleanup.

## Candidate and evidence

Frozen candidate: `/fast/tmp/observer-final-review-bwm4mpo3`.
Manifest SHA-256:
`39ca5cf03213e2690e861f31c581bf5960d28360b8d4537e0f93688fa3a49fb8`.
All 15 module/test file hashes independently verified. Private runner is excluded
from this manifest. Sanitized integration description was read separately; actual
private launcher/config/auth/endpoint/transcript contents were not read.

Builder reports 97 synthetic tests and 12 independent checks passed. These were
not rerun by architect. Earlier live full-tools smoke reportedly showed seven
heartbeats, exit zero and no observer errors, but no admitted milestones. A
Write-only diagnostic produced prose without tool calls and likewise correctly
produced no milestones. The exact shell channel has synthetic evidence only;
no real correlated milestone or attached monitoring UI is claimed.

## O1 — Shell channel does not establish foreground completion

`observer_stream.py:154-158` captures a canonical Bash command without inspecting
execution controls such as `run_in_background`. At lines 201-207, empty result text
and an omitted exit code are treated as successful completion; arbitrary missing
`is_error` also coerces to false. A canonical command submitted in the background
can therefore become a milestone if its submission result has empty content.
The command string alone does not establish completed foreground execution.

Require admitted foreground tool-input semantics and a known completed result
shape. Reject background=true, invalid execution-control types and unknown controls
that can change completion semantics. Where a known harness convention permits
implicit exit zero, document and test exactly that convention, rather than accepting
all absent-status objects. An explicit exit status must be an integer, not bool.
Missing/invalid/unknown success semantics fail closed. Add synthetic canonical
command plus background flag/empty receipt, malformed result and valid documented
foreground completion cases. Preserve exact original-string canonical syntax and
strict captured JSON; never execute or reread the file.

## O2 — Independent diagnostic/status sink failures can disappear

`observer_files.py:238-242,260-264` silently abandons status publication when the
status name changes or its truncate/write fails. `_diag_bound` at 451-457 returns
false on capture identity failure without marking lost diagnostics. The caller
then returns without exposing that capture failure. A substituted diagnostics file
can therefore discard errors without a diagnostic-failure flag; an unusable status
sink can also be invisible when other evidence channels remain available.

Latch bounded status/diagnostic sink failures, expose them in returned summary or
another available independent callback/healthy sink, and mark observer degraded.
Do not recursively write failures to the failed sink or relabel the child outcome.
Do not claim a receipt persisted merely because publication was attempted. Tests
must inject status-write failure and replace diagnostics with a foreign entry,
assert foreign bytes unchanged, assert visible failure, and retain correct child
wait/drain results. File substitution must never cause adoption of another inode.

## Positive observations and remaining limits

Component-relative no-follow directory admission now distinguishes private owned
leaf from trusted root/current-user system ancestry and refuses nonsticky writable
ancestors. The process loop starts post-exit draining from actual observed child
exit and does not reclassify an exited child as timed out because a descendant
holds pipes. The aggregate runtime-error failure remains separate from the actual
exit result. Canonical shell syntax and strict milestone JSON reject malformed,
fenced, duplicate-key and nonstandard-constant payloads. These static observations
do not waive O1/O2.

The sanitized runner description binds a dedicated writable progress mount,
separate observer logs, explicit environment and existing isolation. This is a
structural review, not an independent runtime re-verification of private wiring.
The worker-writable progress mount must remain distinct from host/source/log targets;
shell redirection itself is not no-follow protection. Live provider availability,
complete milestone emission, monitoring UI attachment and package readiness are
not established by this review.

## Final-v2 re-review

Candidate: `/fast/tmp/observer-final-v2-review-eppnd1_c`, manifest SHA-256
`cfc8f1cc2ea37495b08a8bf6462a7e6234659cc04709961aa8ecbb8debd31b65`.
All 15 file hashes independently verified. The reviewed five-file delta changes
stream admission, file-sink flags, observer degradation, returned summary and
regression tests; remaining reviewed modules are unchanged.

**O1 resolved:** shell input admits only the documented command/description/timeout/
background controls. Background must be false (or the documented foreground
default); description is bounded text, timeout is finite positive nonboolean numeric
within the tool limit. Completion requires explicit boolean `is_error=false`, exact
empty or documented `(Bash completed with no output)` content, and integer zero if
exit status is supplied (bool refused). Unknown inputs, asynchronous/background
submission, invalid status types and missing success indication do not admit a
milestone. Exact canonical original-string syntax, captured payload and matching
call-id correlation remain. Positive wrapper and negative execution-control cases
are included. Empty completion is an explicitly admitted harness convention, not
universal proof for arbitrary future tool protocols.

**O2 resolved:** status and diagnostics failures are latched independently, exposed
in returned summary and included in observer degraded classification. Failure does
not recurse through the failed sink or change child exit evidence. Regression tests
inject failed status truncation and diagnostics-name replacement, require flags and
preserve foreign bytes. These flags do not claim failed persistence succeeded.

Builder reports complete 99-test suite passed in 9.033 seconds and 12 independent
checks passed. Architect read the exact implementation/test delta but did not rerun
these tests. An offline replay of retained tool events reportedly admits two
correct milestones after the documented wrapper correction; this is replay evidence,
not a new live provider run or proof that every future worker emits milestones.

No remaining blocking defect was established in this targeted re-review. The exact
portable observer content is architecturally cleared for local observability use
with the reviewed constraints. Private runner integration is reviewed only through
its sanitized structural description and builder's prior smoke evidence; actual
private launcher/config bytes are outside this candidate and not independently
re-verified. Monitoring UI attachment is not established. Package promotion,
functional global-launcher acceptance, owner final acceptance, source commit,
publication, future provider execution and run deletion remain separate decisions.

