# Explicit force in fixed one-script Validate

The generated requested-options `force` bit now reaches the actual owner
request and run-identity/freshness choice after exact selection and approval.
The unchanged v4 script-source capture remains a comparison component; the
admitted wrapper is the effective C1 request and plan and binds the approved
force bit. Same key/same intent attaches one run, changed force conflicts, and
new key alone keeps ordinary `force=false` semantics. Trace and deploy profile
remain explicit `ImplementationGap` refusals.

Focused Linux proof in the worker and primary targets:
`machine_submission::lifecycle_decisions` 45/45,
`lifecycle_adapter::sequential_runs::native_tests` 4/4, and
`machine_operation::fixed_lifecycle` 4/4. The worker also passed the exact
concurrency/C1 replay, generic no-profile refusal, approved-force and
freshness cases, plus `session_facade` 7/7. The sequential tests exercised
both force values against 11 damaged or nonterminal predecessor variants,
with no new provider effect.

The worker's broad adapter family was red under the host Cranelift backend:
one unchanged deliberate-panic test aborted, and two generic tests expected
fresh-key refusal but observed `Invoked`. The accepting parent reproduced each
of those two generic failures on the unmodified force baseline, selecting one
case at a time. Neither was counted as a force regression or as a passing gate;
no oracle was removed or weakened. No full gate panel or routine mutation
campaign was run.
