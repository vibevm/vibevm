# M-08-D.2 correction rereview

**Bounded clear: R1–R4 resolved. No new actionable finding in the correction scope. This is not Root acceptance.**

Candidate collection sequence 1955 was read from END-2. The 2,655-source snapshot was frozen before source review and matches the final snapshot. No builds, tests, process operations, model launches or source/Git mutations were performed.

## R1: resolved

D1 validate_process_grant checks exact host/prepared identity and currentness. D2 validates every command grant before C3 effects; D1 execute reuses validation. Equal-DTO/different-private-prepared regression checks zero file/command effects for wrong grant and success for matching grant.

## R2: resolved

save retains old.file_subject after checking plan/file-target join; fresh C3 approval identity is separate. Combined cold recovery/cancel/cold recovery regression rebuilds C3/D1 hosts and checks immutable original subject, same run, two total command effects and retained foreign JSON member.

## R3: resolved

Exact future receipt digest/generation and final reply are durable before receipt publication. Recovery validates own receipt plus sidecar/intent under incumbent locks and finalizes same run. Table-driven regression covers 3 publication stages x 2 prefix states x 4 foreign-join choices (24 cases inside one test); successful cases have exactly two total command effects, foreign receipt/sidecar/intent refuse. Before-receipt interruption retains prior join and can proceed through existing owned-prefix transaction.

## R4: resolved_for_authorized_controlled_embedding

run now calls shared dispatch(args, cli, client_host, reader, writer). Dispatch performs normal CLI validation and owns mode selection; capability injection is explicit. New regression invokes this actual entry with parsed CLI arguments and negotiated frames for plan/apply/verify/update/remove/partial apply/recover/cleanup, compares API results and observes 10 command effects. Default None refuses. This is in-process trusted CLI composition, not standalone vendor commissioning.

## Actual evidence

Latest producer logs contain 17 nonignored focused tests (7 lifecycle, 3 orchestrator, 2 MCP, 3 CLI, 2 wire) plus five exact owner regressions; all exit 0. Function names, source definitions and CASE-BINDINGS hashes agree. R3 is meaningful fault injection after receipt publication, after sidecar promotion and after intent retirement, with completed and partial prefixes and foreign receipt/sidecar/intent refusal. R4 drives the entry also called by production run, rather than only invoking the leaf helper or MCP runner.

Every latest V1–V8 per-command and group before/after manifest has 2,655 rows and exactly equals the review snapshot. Formerly omitted new source files, tests and approved extensions are included. The old provenance gap is resolved for these correction runs. Earlier attempts remain historical; no failed result is relabelled.

V8 scoped formatting/whitespace exits are 0/0 with successful preservation receipt. V7 remains 0/1/0: codegen succeeded, preintegration check-codegen failed, wire-diff succeeded. Root must still perform the separate staged integrated gate.

## Spec and scope

PROP054 is byte-identical to the correction baseline. The original full lifecycle, exact private approval, fresh recovery authority, structured partial effects, CLI parity and controlled-fixture safe stop requirements remain intact. New deploy-client-state finalization fields are optional data paired and checked by the reader; they supply no grant. D1 private-grant semantics are preserved by extracting the old checks and retaining execution validation. No new task/process owner is introduced.

Default CLI startup still refuses an absent host, as authorized. The repaired path is explicitly supplied in-process trusted composition; this review does not claim a standalone real-vendor or production PodBay adapter.

END-2 says candidate_collected with known tail and process_stopped=false. Source hashes showed no live drift during this review; that is not proof that every managed process has retired. No process inspection was performed. Acceptance remains with Root.
