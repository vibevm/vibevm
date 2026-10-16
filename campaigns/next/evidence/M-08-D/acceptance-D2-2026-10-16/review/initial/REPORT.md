# M-08-D.2 independent full-obligation review

**Verdict: changes required. Candidate remains unaccepted; Root owns acceptance.**

Review resumed 2026-10-16. Current candidate is unchanged from the pre-pause snapshot. No builds, tests, process management, source edits or Git mutations were performed. Reports and hash evidence only were written outside the source repository.

## R1 — P1: Validate opaque D1 grants before any composed file effect

Location: `crates/vibe-lifecycle/src/mechanism/deploy/client_deploy.rs:503`.

D2 checks the content plan hash and command count, then checks preparation currentness, but never checks each ProcessGrant against its Prepared. D1 assigns a fresh private prepared id on every prepare; identical inspect DTOs hash to the same D2 identity. Approval from preparation A passes D2 preflight for equivalent preparation B. C3 files.execute runs at line 802, and only subsequent D1 execute rejects the wrong prepared id.

Minimal correction: Expose a read-only D1 exact grant validation port that preserves its existing host/prepared/currentness checks, use it for every command in D2 complete preflight before file effects, and retain the execution recheck.

Required focused regression: Prepare identical command/file plans twice without state changes; approve first, execute second with that approval; assert typed NeedsAuthority and zero file/command effects. Then execute with a fresh matching grant successfully.

## R2 — P2: Preserve original C3 operation subject across repeated recovery

Location: `crates/vibe-lifecycle/src/mechanism/deploy/client_deploy/journal.rs:19`.

save writes file_subject from Files::identity on every save. Initial Apply identity equals the C3 binding grant_subject; a retained recovery plan identity is a different hash. C3 execute_retained_inner preserves grant_subject when plan.recovery is true (generic/inverse.rs 394-400). After the first recovery saves, a second prepare_recovery_with_files compares the new recovery identity to the original binding subject and refuses at client_deploy.rs 420. Even cooperative cancellation before dispatch invokes save and causes this mismatch.

Minimal correction: Keep the original C3 operation subject immutable in D2 retained state and separate it from the fresh retained-plan approval identity; validate target/resource/subject joins on every continuation. Do not weaken the C3 equality check.

Required focused regression: Combined file+command apply stops after a command effect; cold recover with fresh C3/D1 admission, cancel at a durable boundary, then cold recover again with fresh admission. Same run resumes, original C3 subject remains, landed effects execute once, foreign state survives.

## R3 — P1: Reconcile the receipt publication boundary within the retained run

Location: `crates/vibe-lifecycle/src/mechanism/deploy/client_deploy/journal.rs:291`.

Receipt is published before sidecar promotion, intent retirement and final client record save. The durable client record still contains the previous receipt_digest until that final save. Crash or publication error in this interval leaves an owned completed/partial prefix, but execute refuses its own new receipt as changed at client_deploy.rs 734-737. Fresh D1/C3 authority cannot repair this journal join.

Minimal correction: Retain a durable exact finalization intent/basis before receipt publication and reconcile a matching own receipt/sidecar/intent during recovery using existing DeployState/Transaction ownership. Refuse foreign joins; do not merely ignore receipt mismatch or create a new run.

Required focused regression: Deterministically stop after receipt publication and before final client-record publication (also cover sidecar/intent finalization stages). Cold inspect/recover with fresh authority must complete the same operation without repeating landed commands and must reject a genuinely foreign receipt.

## R4 — P2: Connect trusted host injection to the real CLI entry path

Location: `crates/vibe-cli/src/commands/machine_session.rs:133`.

The production --client-deploy branch always calls client_deploy::exchange with None. The helper immediately errors on None and is in a private module; its only Some(route) caller is a unit test with empty input. Lifecycle CLI tests subsequently drive ClientDeployMachineSessionRunner directly. Thus the declared CLI mode cannot execute any lifecycle verb through a reachable trusted composition path.

Minimal correction: Thread an explicitly supplied trusted client host through an actual callable CLI composition/dispatch entry, preserving default ungranted refusal and using the same frame runner. Do not create ambient grants or vendor discovery.

Required focused regression: Drive the real CLI dispatch entry with an injected controlled host and actual framed plan/apply/verify/update/remove/recover input; compare typed owner outcomes. Also assert ordinary ungranted startup refuses.

## Original scope disposition

| Obligation | Review |
|---|---|
| Existing journals and M16 durable admission | The route uses SubmissionJournal/submit, LifecycleLease and SubmissionReservations; new records attach to DeployState. No new physical launcher or reservation table. Receipt finalization remains broken (R3). |
| Independent observation, cancellation, recovery/remove | Real filesystem fixture effects and observer are present; command-only recovery skips observed landed steps. Combined file recovery is not repeatable (R2); exact grants are not fully prevalidated (R1). |
| CLI/API/MCP parity | API/MCP and shared framed runner use typed owner replies including partial effects. The actual CLI entry is unconditionally ungranted (R4). |
| Apply → verify → update → remove preserves foreign state | Covered for command fixtures, plus combined C3 JSON-member lifecycle helper. This does not exercise repeated combined recovery. |
| Response loss and owned-prefix recovery | BrokenPipe frame test retains exact response and reconnect retrieves same run. Command-only response loss is covered; receipt publication interruption is not (R3). |
| Changed executable/descriptor/grant refuses | Executable, policy/descriptor, expansion and file-grant checks covered; equivalent re-preparation with wrong opaque D1 grant is missing (R1). |
| One effect then provider error remains structured | Covered by real injected ProcessRunner mutation followed by error; MCP executed_failure preserves partial result. |
| Both acceptance criteria and safe stop | Not met until R1–R4 are corrected and focused evidence is rebound. No plan-only or vendor-qualification substitution. |

## Evidence and preservation

Producer logs show 13 nonignored new tests (4/3/2/2/2) and five exact existing owner cases, all exit 0. Defined function names match CASE-BINDINGS. This is observed historical evidence, not an independent test execution. V7 commands have separate exits 0/1/0; check-codegen failed before staging and remains failed. The two initial bootstrap codegen failures are retained. V8 frozen-wrapper failure is an assertion against its obsolete authored path list; supplemental rustfmt and whitespace exits are 0. Root repair of that wrapper and later staged codegen gate remain separate tasks, not product defects.

Independent byte comparison of the 238-file protected generated baseline found 57 files with exactly the approved ` / `schemas/client_deploy/e1/`` header insertion, three required registration owners, and no other old generated drift. Registration diffs add only the new families/formats. PROP054 parses and removal of the one added clause restores exact old text. Old C3 execution/grant bodies are unchanged except the approved additive ports, and D1 change is check visibility. FixedDeployRoute bodies are unchanged.

## Tested-source closure limitation

All 471 recorded per-group source hashes match current bytes, as does the final 114-file SOURCE-CLOSURE. However the per-group manifests omit the new lifecycle/client route/test modules, machine_session/state.rs and generic/grant.rs. A final source hash list cannot retroactively bind these bodies to the executed tests. SOURCE-HASHES.json lists every omitted final-closure path and records before/after snapshots with no drift. Corrected focused checks must snapshot the full consumed closure, including new code and approved extensions, before and after execution. Do not infer a blanket current-source green result from the existing 471 rows.

The review addresses the full original obligation set and changed integration bodies. It does not claim fresh runtime reproduction of the findings or blanket requalification of every unchanged owner. Source corrections should be performed by the managed producer; no patch was written here.
